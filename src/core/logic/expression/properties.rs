//! Conservative finite-scalar proofs over the stored DAG.
//!
//! The facts of a node are a function of the node, proved once from its
//! children's facts and kept with the interned node, so a repeated query is one
//! atomic load.

#![expect(
    unsafe_code,
    reason = "base positions of compact polynomial storage index its base table and any per-base table built with the same length"
)]

use core::{
    ptr::{eq, null},
    sync::atomic::{AtomicU32, AtomicUsize, Ordering},
};

use alloc::{vec, vec::Vec};

use num_anafis::Number;

use crate::{collections::FixedMap, sync::Mutex};

use super::{BuiltinFunction, Expr, Node, Polynomial, Postorder, StoredForm};

/// Number of lower payload bits reserved in `Stored::facts` (10 proven + 10
/// refuted) and `Stored::settled` (up to 20 stage bits).
const PAYLOAD_BITS: u32 = 20;

/// Mask covering the 20-bit payload of an epoch-tagged word.
const PAYLOAD_MASK: usize = if let Some(limit) = 1_usize.checked_shl(PAYLOAD_BITS) {
    limit - 1
} else {
    usize::MAX
};

/// Largest even epoch representable in the upper bits of `usize`, or zero when
/// `usize` is too narrow to hold both payload and epoch bits.
const MAX_EVEN_EPOCH: usize = if let Some(shifted) = usize::MAX.checked_shr(PAYLOAD_BITS) {
    shifted & !1
} else {
    0
};

/// Odd sentinel entered once `MAX_EVEN_EPOCH` is exhausted; readers bypass node
/// caching rather than ever wrapping an epoch tag.
const EPOCH_SATURATED: usize = MAX_EVEN_EPOCH | 1;

/// Starting property epoch: even (`2`) when epoch tagging fits in `usize`,
/// otherwise `EPOCH_SATURATED` (`1`).
const INITIAL_EPOCH: usize = if MAX_EVEN_EPOCH >= 2 { 2 } else { 1 };

/// Generation seqlock for symbol property updates. Even values in
/// `2..=MAX_EVEN_EPOCH` are stable cached epochs; odd values indicate either an
/// in-progress write or permanent saturation (`EPOCH_SATURATED`).
static FACTS_EPOCH: AtomicUsize = AtomicUsize::new(INITIAL_EPOCH);

/// Serializes symbol property writers around the seqlock transition.
static PROPERTIES_WRITE_LOCK: Mutex<()> = Mutex::new(());

/// The result of a conservative mathematical property query.
/// Unknown facts do not authorize transformations requiring a proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Truth {
    /// The property holds for every admissible assignment.
    True,
    /// The property fails for every admissible assignment.
    False,
    /// The analysis cannot establish either result.
    Unknown,
}

/// Membership predicates for finite real and complex scalars.
///
/// Sign predicates require a finite real value; nonreal and undefined values
/// do not satisfy them. Nonzero means a finite complex value unequal to zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Property {
    /// A defined finite complex value.
    Finite,
    /// A finite real value.
    Real,
    /// A real integer value, independently of its literal representation.
    Integer,
    /// A real rational value.
    Rational,
    /// Exactly zero.
    Zero,
    /// A finite complex value unequal to zero.
    Nonzero,
    /// A strictly positive real value.
    Positive,
    /// A strictly negative real value.
    Negative,
    /// A nonnegative real value.
    Nonnegative,
    /// A nonpositive real value.
    Nonpositive,
}

/// Proven and refuted facts for one expression.
///
/// By default, symbols range over finite complex scalars (`Property::Finite`).
/// Callers may attach stronger closed facts to symbols via
/// [`crate::Symbol::set_properties`] and return-domain facts to custom
/// functions via [`crate::UserFunction::returns`]. Facts describe mathematical
/// values; they do not predict overflow or rounding in numerical evaluation.
/// Unknown function domains remain unknown. Facts cannot change expression
/// identity or canonical construction.
/// Nonfinite inputs do not refute a composition's finite value without a domain
/// proof: extended values or limits may require additional rules.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Properties {
    facts: Facts,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    proven: u16,
    refuted: u16,
}

impl Expr {
    /// Prove scalar properties conservatively in children-before-parent order.
    /// Each node is analyzed once per symbol-property epoch and its facts are
    /// kept, so a repeated query reads them without a traversal. Compact
    /// polynomials are inspected without materializing their logical monomials.
    /// Unknown facts remain unknown.
    ///
    /// ```
    /// use symb_anafis::{Expr, Property, Truth};
    /// let x = Expr::symbol("properties_example_x")?;
    /// assert_eq!(x.properties().get(Property::Real), Truth::Unknown);
    /// assert_eq!(x.abs().properties().get(Property::Nonnegative), Truth::True);
    /// assert_eq!(x.exp().properties().get(Property::Positive), Truth::Unknown);
    /// # Ok::<(), symb_anafis::SymbolError>(())
    /// ```
    #[must_use]
    pub fn properties(&self) -> Properties {
        for _ in 0..2 {
            let Some(epoch) = Facts::cache_epoch() else {
                break;
            };
            if let Some(facts) = Facts::stored(self, epoch) {
                return facts.into_properties();
            }
            // A node with stored facts for `epoch` has stored children for
            // `epoch`, so it ends this walk and no visited set is needed: a
            // node is completed before any earlier stack entry for it is popped.
            let mut pending = vec![(self, false)];
            let mut root_facts = None;
            let mut stale = false;
            while let Some((node, ready)) = pending.pop() {
                if let Some(facts) = Facts::stored(node, epoch) {
                    if node == self {
                        root_facts = Some(facts);
                    }
                    continue;
                }
                if ready {
                    let computed = Facts::of(node, |operand| {
                        Facts::stored_or_leaf(operand, epoch).unwrap_or_default()
                    });
                    if !Facts::is_current_epoch(epoch) {
                        stale = true;
                        break;
                    }
                    Facts::publish(node, epoch, computed);
                    if node == self {
                        root_facts = Some(computed);
                    }
                } else if let Some(computed) = Facts::leaf(node) {
                    if !Facts::is_current_epoch(epoch) {
                        stale = true;
                        break;
                    }
                    Facts::publish(node, epoch, computed);
                    if node == self {
                        root_facts = Some(computed);
                    }
                } else {
                    pending.push((node, true));
                    pending.extend(
                        node.stored_children()
                            .filter(|child| {
                                !matches!(
                                    child.node(),
                                    Node::Number(_)
                                        | Node::ImaginaryUnit
                                        | Node::Pi
                                        | Node::Euler
                                        | Node::Symbol(_)
                                ) && Facts::stored(child, epoch).is_none()
                            })
                            .map(|child| (child, false)),
                    );
                }
            }
            if !stale && let Some(facts) = root_facts {
                return facts.into_properties();
            }
        }
        Facts::compute_uncached(self).into_properties()
    }
}

impl Properties {
    /// Read one proven, refuted, or unknown property without traversing the DAG.
    #[must_use]
    pub const fn get(self, property: Property) -> Truth {
        self.facts.get(property)
    }
}

impl Facts {
    /// Bit position where the property epoch begins inside `Stored::facts` and
    /// `Stored::settled` (`20` on 32- and 64-bit targets).
    pub const EPOCH_SHIFT: u32 = if usize::BITS > PAYLOAD_BITS {
        PAYLOAD_BITS
    } else {
        0
    };

    /// Default facts for a symbol: a finite complex scalar.
    pub const SYMBOL_DEFAULT: Self = Self {
        proven: Property::Finite.bit(),
        refuted: 0,
    };

    /// Wrap these facts in the public [`Properties`] handle.
    #[must_use]
    pub const fn into_properties(self) -> Properties {
        Properties { facts: self }
    }

    /// The active property epoch when node caching is enabled (an even value in
    /// `2..=MAX_EVEN_EPOCH`), or `None` during a concurrent symbol update or
    /// after epoch saturation.
    #[must_use]
    pub fn cache_epoch() -> Option<usize> {
        let epoch = FACTS_EPOCH.load(Ordering::Acquire);
        (epoch & 1 == 0).then_some(epoch)
    }

    /// Whether `epoch` is still the active global property epoch.
    #[must_use]
    pub fn is_current_epoch(epoch: usize) -> bool {
        FACTS_EPOCH.load(Ordering::Acquire) == epoch
    }

    /// Update a symbol's atomic facts slot and advance the seqlock epoch when
    /// the packed facts change. Entering an odd epoch before writing `slot`
    /// with `Release` guarantees that any reader observing the new symbol facts
    /// via `Acquire` also observes the epoch change before publishing to a node
    /// cache.
    pub fn write_symbol_facts(slot: &AtomicU32, facts: Self) {
        let next = facts.pack();
        let _guard = PROPERTIES_WRITE_LOCK.lock();
        if slot.load(Ordering::Relaxed) == next {
            return;
        }
        let current = FACTS_EPOCH.load(Ordering::Relaxed);
        if current < EPOCH_SATURATED {
            FACTS_EPOCH.store(current.saturating_add(1), Ordering::Release);
        }
        slot.store(next, Ordering::Release);
        if current < EPOCH_SATURATED {
            let next_even = current.saturating_add(2);
            if next_even <= MAX_EVEN_EPOCH {
                FACTS_EPOCH.store(next_even, Ordering::Release);
            }
        }
    }

    /// Pack the 10 proven and 10 refuted bits into the low 20 bits of a `u32`.
    #[must_use]
    pub const fn pack(self) -> u32 {
        let [refuted_low, refuted_high] = (self.refuted & 0x03FF).to_le_bytes();
        let [proven_low, proven_high] = ((self.proven & 0x03FF) << 2).to_le_bytes();
        u32::from_le_bytes([refuted_low, refuted_high | proven_low, proven_high, 0])
    }

    /// Unpack the low 20 bits produced by [`Self::pack`].
    #[must_use]
    pub const fn unpack(packed: u32) -> Self {
        let [low, mid, high, _] = packed.to_le_bytes();
        Self {
            proven: (u16::from_le_bytes([mid, high]) >> 2) & 0x03FF,
            refuted: u16::from_le_bytes([low, mid & 0x03]),
        }
    }

    /// Build closed facts from user-declared `properties` (starting with
    /// [`Property::Finite`] when `finite_base` is set), returning `None` when
    /// the declarations contradict each other.
    #[must_use]
    pub fn from_properties(properties: &[Property], finite_base: bool) -> Option<Self> {
        let mut facts = if finite_base {
            Self::SYMBOL_DEFAULT
        } else {
            Self::default()
        };
        for &property in properties {
            facts.prove(property);
        }
        if facts.proven & facts.refuted != 0 {
            return None;
        }
        facts.close();
        (facts.proven & facts.refuted == 0).then_some(facts)
    }

    /// Compute facts bottom-up over the stored DAG without reading or writing
    /// per-node caches, used during concurrent property updates or after epoch
    /// saturation.
    fn compute_uncached(root: &Expr) -> Self {
        let order = Postorder::stored(root);
        let mut table = FixedMap::default();
        table.reserve(order.descendants().len());
        for visit in order.descendants() {
            let expression = visit.expression();
            let facts = Self::of(expression, |operand| {
                table.get(&operand.pointer()).copied().unwrap_or_default()
            });
            table.insert(expression.pointer(), facts);
        }
        Self::of(root, |operand| {
            table.get(&operand.pointer()).copied().unwrap_or_default()
        })
    }

    /// The facts of one node from the facts of its children. Children
    /// are proved first; a missing child fact would only lose facts.
    ///
    /// The rules of each node prove single facts and their implications;
    /// conjunctions close afterwards: a real value that is not negative is
    /// nonnegative, one that is not positive is nonpositive, a finite value
    /// that is not zero is nonzero, and then nonnegative and nonzero values
    /// are positive, nonpositive and nonzero values negative, and values both
    /// nonnegative and nonpositive zero. The first three only supply premises
    /// of the last three, whose conclusions imply all their premises, so one
    /// pass in this order reaches the closure.
    fn of(expression: &Expr, child: impl Fn(&Expr) -> Self) -> Self {
        let mut facts = match expression.node() {
            Node::Number(value) => Self::literal(value.value()),
            Node::ImaginaryUnit => Self::literal(&Number::i()),
            Node::Pi | Node::Euler => {
                let mut facts = Self::default();
                facts.prove(Property::Positive);
                facts.refute(Property::Rational);
                facts
            }
            Node::Symbol(data) => data.facts(),
            Node::Sum(terms) => Self::sum(terms.iter().map(&child)),
            Node::Product(factors) => Self::product(factors.iter().map(&child)),
            Node::Div(numerator, denominator) => {
                Self::quotient(child(numerator), child(denominator))
            }
            Node::Pow(base, exponent) => {
                Self::power(child(base), child(exponent), exponent.as_number())
            }
            Node::Function { name, args } => name.definition().map_or_else(
                || {
                    let mut arg_facts = [Self::default(); 3];
                    let mut count = 0_usize;
                    for (slot, argument) in arg_facts.iter_mut().zip(args) {
                        *slot = child(argument);
                        count = count.saturating_add(1);
                    }
                    Self::function(
                        BuiltinFunction::of(name),
                        arg_facts.get(..count).unwrap_or(&[]),
                    )
                },
                |custom| {
                    if args
                        .iter()
                        .all(|argument| child(argument).get(Property::Finite) == Truth::True)
                    {
                        custom.properties().facts
                    } else {
                        Self::default()
                    }
                },
            ),
            Node::Poly(polynomial) => Self::polynomial(polynomial, &child),
            Node::Derivative { .. } => Self::default(),
        };
        facts.close();
        facts
    }

    fn polynomial(polynomial: &Polynomial, child: impl Fn(&Expr) -> Self) -> Self {
        if polynomial
            .bases
            .iter()
            .all(|base| child(base) == Self::SYMBOL_DEFAULT)
        {
            return Self::SYMBOL_DEFAULT;
        }
        let base_powers: Vec<[Self; 2]> = polynomial
            .bases
            .iter()
            .map(|base| {
                let base_facts = child(base);
                let mut even = Self::integer_power(base_facts, 2);
                even.close();
                let mut odd = Self::integer_power(base_facts, 1);
                odd.close();
                [even, odd]
            })
            .collect();
        let mut coeff_cache = [(null::<Node>(), Self::default()); 64];
        Self::sum(polynomial.terms().map(|(coefficient, powers)| {
            let pointer = coefficient.pointer();
            let addr = pointer.addr();
            let slot_index = ((addr >> 4) ^ (addr >> 10)) & 63;
            let coeff_facts = if let Some(entry) = coeff_cache.get_mut(slot_index) {
                if eq(entry.0, pointer) {
                    entry.1
                } else {
                    let computed = child(coefficient);
                    *entry = (pointer, computed);
                    computed
                }
            } else {
                child(coefficient)
            };
            Self::polynomial_monomial(coeff_facts, powers, &base_powers)
        }))
    }

    #[expect(
        clippy::as_conversions,
        reason = "stored base positions are `u32` indices into `polynomial.bases` and `base_powers`, and `exponent & 1` is 0 or 1"
    )]
    fn polynomial_monomial(
        coefficient_facts: Self,
        powers: &[(u32, u32)],
        base_powers: &[[Self; 2]],
    ) -> Self {
        if coefficient_facts.get(Property::Finite) == Truth::False {
            return Self::default();
        }
        let mut all = coefficient_facts.proven;
        let mut some = coefficient_facts.proven;
        let mut negative = false;
        let mut signed = true;
        if coefficient_facts.get(Property::Nonnegative) != Truth::True {
            if coefficient_facts.get(Property::Nonpositive) == Truth::True {
                negative = !negative;
            } else {
                signed = false;
            }
        }
        for &(position, exponent) in powers {
            // SAFETY: `base_powers` mirrors `polynomial.bases`, and
            // `(exponent & 1)` is in `0..2`.
            let factor = unsafe {
                *base_powers
                    .get_unchecked(position as usize)
                    .get_unchecked((exponent & 1) as usize)
            };
            if factor.get(Property::Finite) == Truth::False {
                return Self::default();
            }
            all &= factor.proven;
            some |= factor.proven;
            if factor.get(Property::Nonnegative) != Truth::True {
                if factor.get(Property::Nonpositive) == Truth::True {
                    negative = !negative;
                } else {
                    signed = false;
                }
            }
        }
        let mut facts = Self::default();
        if all & Property::Finite.bit() == 0 {
            return facts;
        }
        facts.prove(Property::Finite);
        if some & Property::Zero.bit() != 0 {
            facts.prove(Property::Zero);
            return facts;
        }
        if all & Property::Real.bit() != 0 {
            facts.prove(Property::Real);
        }
        if all & Property::Integer.bit() != 0 {
            facts.prove(Property::Integer);
        }
        if all & Property::Rational.bit() != 0 {
            facts.prove(Property::Rational);
        }
        if all & Property::Nonzero.bit() != 0 {
            facts.prove(Property::Nonzero);
        }
        if !signed {
            return facts;
        }
        facts.prove(if negative {
            Property::Nonpositive
        } else {
            Property::Nonnegative
        });
        if facts.get(Property::Nonzero) == Truth::True {
            facts.prove(if negative {
                Property::Negative
            } else {
                Property::Positive
            });
        }
        facts
    }

    fn leaf(expression: &Expr) -> Option<Self> {
        let mut facts = match expression.node() {
            Node::Number(value) => Self::literal(value.value()),
            Node::ImaginaryUnit => Self::literal(&Number::i()),
            Node::Pi | Node::Euler => {
                let mut facts = Self::default();
                facts.prove(Property::Positive);
                facts.refute(Property::Rational);
                facts
            }
            Node::Symbol(data) => data.facts(),
            Node::Function { .. }
            | Node::Sum(_)
            | Node::Poly(_)
            | Node::Product(_)
            | Node::Div(..)
            | Node::Pow(..)
            | Node::Derivative { .. } => return None,
        };
        facts.close();
        Some(facts)
    }

    fn publish(node: &Expr, epoch: usize, computed: Self) {
        let packed = (epoch << Self::EPOCH_SHIFT) | usize::try_from(computed.pack()).unwrap_or(0);
        node.facts()
            .try_update(Ordering::Release, Ordering::Relaxed, |current| {
                ((current >> Self::EPOCH_SHIFT) < epoch).then_some(packed)
            })
            .ok();
    }

    fn stored_or_leaf(expression: &Expr, epoch: usize) -> Option<Self> {
        if let Some(facts) = Self::stored(expression, epoch) {
            return Some(facts);
        }
        let computed = Self::leaf(expression)?;
        if Self::is_current_epoch(epoch) {
            Self::publish(expression, epoch, computed);
        }
        Some(computed)
    }

    fn close(&mut self) {
        for (premises, refuted, conclusion) in [
            (
                Property::Real.bit(),
                Property::Negative.bit(),
                Property::Nonnegative,
            ),
            (
                Property::Real.bit(),
                Property::Positive.bit(),
                Property::Nonpositive,
            ),
            (
                Property::Finite.bit(),
                Property::Zero.bit(),
                Property::Nonzero,
            ),
            (
                Property::Nonnegative.bit() | Property::Nonzero.bit(),
                0,
                Property::Positive,
            ),
            (
                Property::Nonpositive.bit() | Property::Nonzero.bit(),
                0,
                Property::Negative,
            ),
            (
                Property::Nonnegative.bit() | Property::Nonpositive.bit(),
                0,
                Property::Zero,
            ),
        ] {
            if self.proven & premises == premises && self.refuted & refuted == refuted {
                self.prove(conclusion);
            }
        }
    }

    /// The facts kept with an interned node, packed as
    /// `(epoch << EPOCH_SHIFT) | (proven << 10) | refuted`.
    fn stored(expression: &Expr, epoch: usize) -> Option<Self> {
        let packed = expression.facts().load(Ordering::Acquire);
        ((packed >> Self::EPOCH_SHIFT) == epoch)
            .then(|| Self::unpack(u32::try_from(packed & PAYLOAD_MASK).unwrap_or(0)))
    }

    const fn get(self, property: Property) -> Truth {
        let bit = property.bit();
        if self.proven & bit != 0 {
            Truth::True
        } else if self.refuted & bit != 0 {
            Truth::False
        } else {
            Truth::Unknown
        }
    }

    fn literal(value: &Number) -> Self {
        let mut facts = Self::default();
        if !value.is_finite() {
            facts.refute(Property::Finite);
            return facts;
        }
        facts.prove(Property::Finite);
        if !value.is_real() {
            facts.refute(Property::Real);
            facts.prove(Property::Nonzero);
            return facts;
        }
        facts.prove(Property::Real);
        facts.prove(Property::Rational);
        if value.is_int() {
            facts.prove(Property::Integer);
        } else {
            facts.refute(Property::Integer);
        }
        if value.is_zero() {
            facts.prove(Property::Zero);
        } else if value.is_positive() {
            facts.prove(Property::Positive);
        } else {
            facts.prove(Property::Negative);
        }
        facts
    }

    const fn prove(&mut self, property: Property) {
        let (proven, refuted) = match property {
            Property::Positive => (
                Property::Positive.bit()
                    | Property::Finite.bit()
                    | Property::Real.bit()
                    | Property::Nonzero.bit()
                    | Property::Nonnegative.bit(),
                Property::Zero.bit() | Property::Negative.bit() | Property::Nonpositive.bit(),
            ),
            Property::Negative => (
                Property::Negative.bit()
                    | Property::Finite.bit()
                    | Property::Real.bit()
                    | Property::Nonzero.bit()
                    | Property::Nonpositive.bit(),
                Property::Zero.bit() | Property::Positive.bit() | Property::Nonnegative.bit(),
            ),
            Property::Zero => (
                Property::Zero.bit()
                    | Property::Finite.bit()
                    | Property::Real.bit()
                    | Property::Rational.bit()
                    | Property::Integer.bit()
                    | Property::Nonnegative.bit()
                    | Property::Nonpositive.bit(),
                Property::Nonzero.bit() | Property::Positive.bit() | Property::Negative.bit(),
            ),
            Property::Integer => (
                Property::Integer.bit()
                    | Property::Finite.bit()
                    | Property::Real.bit()
                    | Property::Rational.bit(),
                0,
            ),
            Property::Rational => (
                Property::Rational.bit() | Property::Finite.bit() | Property::Real.bit(),
                0,
            ),
            Property::Nonnegative => (
                Property::Nonnegative.bit() | Property::Finite.bit() | Property::Real.bit(),
                Property::Negative.bit(),
            ),
            Property::Nonpositive => (
                Property::Nonpositive.bit() | Property::Finite.bit() | Property::Real.bit(),
                Property::Positive.bit(),
            ),
            Property::Real => (Property::Real.bit() | Property::Finite.bit(), 0),
            Property::Nonzero => (
                Property::Nonzero.bit() | Property::Finite.bit(),
                Property::Zero.bit(),
            ),
            Property::Finite => (Property::Finite.bit(), 0),
        };
        self.proven |= proven;
        self.refuted |= refuted;
    }

    const fn refute(&mut self, property: Property) {
        self.refuted |= match property {
            Property::Finite => {
                Property::Finite.bit()
                    | Property::Real.bit()
                    | Property::Integer.bit()
                    | Property::Rational.bit()
                    | Property::Zero.bit()
                    | Property::Nonzero.bit()
                    | Property::Positive.bit()
                    | Property::Negative.bit()
                    | Property::Nonnegative.bit()
                    | Property::Nonpositive.bit()
            }
            Property::Real => {
                Property::Real.bit()
                    | Property::Integer.bit()
                    | Property::Rational.bit()
                    | Property::Zero.bit()
                    | Property::Positive.bit()
                    | Property::Negative.bit()
                    | Property::Nonnegative.bit()
                    | Property::Nonpositive.bit()
            }
            Property::Rational => {
                Property::Rational.bit() | Property::Integer.bit() | Property::Zero.bit()
            }
            Property::Integer => Property::Integer.bit() | Property::Zero.bit(),
            Property::Zero
            | Property::Nonzero
            | Property::Positive
            | Property::Negative
            | Property::Nonnegative
            | Property::Nonpositive => property.bit(),
        };
    }
}

impl Property {
    const fn bit(self) -> u16 {
        match self {
            Self::Finite => 1,
            Self::Real => 2,
            Self::Integer => 4,
            Self::Rational => 8,
            Self::Zero => 16,
            Self::Nonzero => 32,
            Self::Positive => 64,
            Self::Negative => 128,
            Self::Nonnegative => 256,
            Self::Nonpositive => 512,
        }
    }
}

mod arithmetic;
mod functions;
