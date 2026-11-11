//! Conservative finite-scalar proofs over the stored DAG.
//!
//! The facts of a node are a function of the node, proved once from its
//! children's facts and kept with the interned node, so a repeated query is one
//! atomic load.

use core::{iter::once, sync::atomic::Ordering};

use alloc::vec;

use num_anafis::Number;

use super::{BuiltinFunction, Expr, Node, StoredForm};

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
/// Symbols range over finite complex scalars. No realness, sign, integrality,
/// or nonzero assumptions are attached to them. Facts describe mathematical
/// values; they do not predict overflow or rounding in numerical evaluation.
/// Unknown function domains remain unknown. Facts are kept with their node and
/// cannot change expression identity or canonical construction.
/// Nonfinite inputs do not refute a composition's finite value without a domain
/// proof: extended values or limits may require additional rules.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Properties {
    facts: Facts,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Facts {
    proven: u16,
    refuted: u16,
}

impl Expr {
    /// Prove scalar properties conservatively in children-before-parent order.
    /// Each node is analyzed once in its lifetime and its facts are kept, so a
    /// repeated query reads them without a traversal. Compact polynomials are
    /// inspected without materializing their logical monomials. Unknown facts
    /// remain unknown.
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
        if let Some(facts) = Facts::stored(self) {
            return Properties { facts };
        }
        // A node with stored facts has stored children, so it ends this walk
        // and no visited set is needed: a node is completed before any earlier
        // stack entry for it is popped.
        let mut pending = vec![(self, false)];
        while let Some((node, ready)) = pending.pop() {
            if Facts::stored(node).is_some() {
                continue;
            }
            if ready {
                Facts::of(node).store(node);
            } else {
                pending.push((node, true));
                pending.extend(
                    node.stored_children()
                        .filter(|child| Facts::stored(child).is_none())
                        .map(|child| (child, false)),
                );
            }
        }
        Properties {
            facts: Facts::stored(self).unwrap_or_default(),
        }
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
    /// The facts of one node from the stored facts of its children. Children
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
    fn of(expression: &Expr) -> Self {
        let child = |operand: &Expr| Self::stored(operand).unwrap_or_default();
        let mut facts = match expression.node() {
            Node::Number(value) => Self::literal(value.value()),
            Node::ImaginaryUnit => Self::literal(&Number::i()),
            Node::Pi | Node::Euler => {
                let mut facts = Self::default();
                facts.prove(Property::Positive);
                facts.refute(Property::Rational);
                facts
            }
            Node::Symbol(_) => {
                let mut facts = Self::default();
                facts.prove(Property::Finite);
                facts
            }
            Node::Sum(terms) => Self::sum(terms.iter().map(child)),
            Node::Product(factors) => Self::product(factors.iter().map(child)),
            Node::Div(numerator, denominator) => {
                Self::quotient(child(numerator), child(denominator))
            }
            Node::Pow(base, exponent) => {
                Self::power(child(base), child(exponent), exponent.as_number())
            }
            Node::Function { name, args } => Self::function(
                BuiltinFunction::of(name),
                if let [argument] = args.as_slice() {
                    Some(child(argument))
                } else {
                    None
                },
            ),
            Node::Poly(polynomial) => {
                Self::sum(polynomial.monomials().map(|(coefficient, powers)| {
                    Self::product(
                        once(child(coefficient)).chain(powers.map(|(base, exponent)| {
                            Self::integer_power(child(base), i64::from(exponent))
                        })),
                    )
                }))
            }
            Node::Derivative { .. } => Self::default(),
        };
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
            if facts.proven & premises == premises && facts.refuted & refuted == refuted {
                facts.prove(conclusion);
            }
        }
        facts
    }

    /// The facts kept with an interned node, packed as `proven << 16 | refuted`.
    /// The ten property bits leave bit 15 free to mark a stored value. Facts
    /// are a function of the node, so a relaxed load sees either nothing or
    /// the one value any query stores.
    fn stored(expression: &Expr) -> Option<Self> {
        let [refuted_low, refuted_high, proven_low, proven_high] =
            expression.facts().load(Ordering::Relaxed).to_le_bytes();
        (refuted_high & 0x80 != 0).then(|| Self {
            proven: u16::from_le_bytes([proven_low, proven_high]),
            refuted: u16::from_le_bytes([refuted_low, refuted_high & 0x7F]),
        })
    }

    fn store(self, expression: &Expr) {
        let [refuted_low, refuted_high] = self.refuted.to_le_bytes();
        let [proven_low, proven_high] = self.proven.to_le_bytes();
        let packed =
            u32::from_le_bytes([refuted_low, refuted_high | 0x80, proven_low, proven_high]);
        expression.facts().store(packed, Ordering::Relaxed);
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

    fn prove(&mut self, property: Property) {
        let implied = match property {
            Property::Positive => [
                Property::Finite,
                Property::Real,
                Property::Nonzero,
                Property::Nonnegative,
            ]
            .as_slice(),
            Property::Negative => [
                Property::Finite,
                Property::Real,
                Property::Nonzero,
                Property::Nonpositive,
            ]
            .as_slice(),
            Property::Zero => [
                Property::Finite,
                Property::Real,
                Property::Rational,
                Property::Integer,
                Property::Nonnegative,
                Property::Nonpositive,
            ]
            .as_slice(),
            Property::Integer => [Property::Finite, Property::Real, Property::Rational].as_slice(),
            Property::Rational | Property::Nonnegative | Property::Nonpositive => {
                [Property::Finite, Property::Real].as_slice()
            }
            Property::Real | Property::Nonzero => [Property::Finite].as_slice(),
            Property::Finite => [].as_slice(),
        };
        for fact in once(property).chain(implied.iter().copied()) {
            self.proven |= fact.bit();
        }
        let excluded = match property {
            Property::Positive => {
                [Property::Zero, Property::Negative, Property::Nonpositive].as_slice()
            }
            Property::Negative => {
                [Property::Zero, Property::Positive, Property::Nonnegative].as_slice()
            }
            Property::Zero => {
                [Property::Nonzero, Property::Positive, Property::Negative].as_slice()
            }
            Property::Nonzero => [Property::Zero].as_slice(),
            Property::Nonnegative => [Property::Negative].as_slice(),
            Property::Nonpositive => [Property::Positive].as_slice(),
            Property::Finite | Property::Real | Property::Integer | Property::Rational => {
                [].as_slice()
            }
        };
        for fact in excluded {
            self.refuted |= fact.bit();
        }
    }

    fn refute(&mut self, property: Property) {
        let excluded = match property {
            Property::Finite => [
                Property::Real,
                Property::Integer,
                Property::Rational,
                Property::Zero,
                Property::Nonzero,
                Property::Positive,
                Property::Negative,
                Property::Nonnegative,
                Property::Nonpositive,
            ]
            .as_slice(),
            Property::Real => [
                Property::Integer,
                Property::Rational,
                Property::Zero,
                Property::Positive,
                Property::Negative,
                Property::Nonnegative,
                Property::Nonpositive,
            ]
            .as_slice(),
            Property::Rational => [Property::Integer, Property::Zero].as_slice(),
            Property::Integer => [Property::Zero].as_slice(),
            Property::Zero
            | Property::Nonzero
            | Property::Positive
            | Property::Negative
            | Property::Nonnegative
            | Property::Nonpositive => [].as_slice(),
        };
        for fact in once(property).chain(excluded.iter().copied()) {
            self.refuted |= fact.bit();
        }
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
