//! The shared handle owner and its private node representation.

#![expect(
    clippy::self_named_module_files,
    reason = "the type owner keeps fields private while its descendants implement operations; mod.rs files contain plumbing only"
)]

use core::{
    num::{NonZeroI128, NonZeroU32, NonZeroU64},
    ptr::null,
    slice::Iter,
};

use alloc::{borrow::Cow, sync::Arc, vec, vec::Vec};

use num_anafis::Number;

use crate::collections::HashMap;

use super::{
    BuiltinFunction, ExprView, FunctionDefinition, FunctionName, Interner, NumericValue, Pattern,
    SYMBOLS, Substitution, Substitutions, Symbol, SymbolData,
};

static NODES: Interner<Node> = Interner::new();

/// An immutable handle to a globally interned expression DAG.
///
/// Equal live expressions share one canonical node. Cloning shares ownership; equality
/// and hashing use the canonical pointer. Nodes store neither IDs nor cached hashes.
/// Literals retain their `NumAnaFis` representation. Float components have bitwise
/// identity, including signed zeros and NaN payloads; exact values stay exact.
/// Numeric folds use the backend's native arithmetic and promotion policies,
/// including wrapping integers and rational capacity panics.
///
/// The handle occupies one pointer. Its private `Option` uses the null-pointer niche
/// and is taken only during iterative destruction, avoiding unsafe field moves.
/// Public views and transformations expose logical sums; compact storage is private.
///
/// ```compile_fail
/// use symb_anafis::Polynomial;
/// ```
///
/// ```compile_fail
/// use symb_anafis::Expr;
/// let expression = Expr::pow_static(Expr::number(2), Expr::number(3));
/// ```
///
/// ```compile_fail
/// use symb_anafis::Expr;
/// let expression = Expr::symbol("x");
/// let terms = expression.additive_operands();
/// ```
#[derive(Clone)]
pub struct Expr(Option<Arc<Node>>);

/// A lazy iterator over an expression's immediate logical children.
///
/// Stored children share their handles. Compact monomials become canonical
/// expressions as they are requested; additive storage groups are flattened.
/// Yielded expressions own their data and can outlive this iterator and its root.
///
/// # Panics
/// Iteration and formatting can propagate numeric backend capacity panics while
/// reconstructing terms. Inspecting the remaining count does not reconstruct them.
#[derive(Clone)]
pub struct ExprChildren<'expr> {
    source: &'expr Expr,
    index: usize,
    monomials: Iter<'expr, PolynomialTerm>,
}

/// Ordered additive projection shared by sealed matching kernels.
/// The trait stays inside the logic facade so storage methods are not public
/// expression APIs. Ordinary sums retain their borrowed matching path.
pub trait AdditiveOperands {
    /// Borrow ordinary terms or reconstruct compact terms in structural order.
    fn additive_operands(&self) -> Option<Cow<'_, [Expr]>>;
}

/// Compressed arithmetic operands used only by sealed matching kernels.
#[derive(Debug)]
pub struct AlgebraicOperands {
    sum: bool,
    slots: Vec<WeightedOperand>,
}

/// The original arithmetic contribution selected by a matching attempt.
#[derive(Clone, Debug)]
pub enum OperandMatch {
    /// Disjoint units of a stored power or collected addend.
    Count { index: usize, amount: i64 },
    /// One scalar factor whose quotient is reconstructed after selection.
    Coefficient { index: usize, divisor: Expr },
}

/// A requested arithmetic projection without expanding repeated operands.
#[derive(Clone, Copy, Debug)]
pub enum OperandDemand {
    /// One scalar base or additive term.
    Atom,
    /// One unit remaining after a whole-operand capture has been tried.
    Split(i64),
    /// The entire available operand.
    Whole,
    /// A proved positive integer multiplicative power.
    Power(i64),
    /// A proved positive integer additive coefficient.
    Copies(i64),
    /// A positive integer block raised to a fixed positive integer exponent.
    Root { exponent: i64, block: i64 },
    /// One exact positive rational power on its unchanged principal base.
    Rational {
        numerator: i64,
        denominator: NonZeroU64,
    },
}

#[derive(Debug)]
struct WeightedOperand {
    original: Expr,
    base: Expr,
    count: i64,
    denominator: NonZeroU64,
    coefficient: bool,
    negative: bool,
    changed: bool,
    side: FactorSide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FactorSide {
    Numerator,
    Denominator,
}

struct LinearOperands {
    terms: Vec<LinearTerm>,
}

struct LinearTerm {
    original: Expr,
    base: Option<Expr>,
    coefficient: Expr,
    changed: bool,
}

struct LinearMatch {
    terms: Vec<(usize, Expr)>,
    scale: Number,
    factor: Option<AlgebraicOperands>,
}

struct PowerProjection<'expr> {
    base: &'expr Expr,
    exponent: NonZeroI128,
    denominator: NonZeroU64,
    nonzero: bool,
    projected: bool,
}

enum ProductFrame {
    Numerator { denominators: Vec<Expr> },
    Denominator { numerator: Expr },
}

#[derive(Debug)]
pub enum RemainingOperand {
    Original(Expr),
    Counted {
        base: Expr,
        count: i64,
    },
    Fixed(Expr),
    Scaled {
        base: Expr,
        coefficient: Expr,
    },
    Divisor(Expr),
    FixedDivisor(Expr),
    DivisorCounted {
        base: Expr,
        count: i64,
    },
    Power {
        base: Expr,
        exponent: NonZeroI128,
        denominator: NonZeroU64,
    },
    Factored {
        factor: AlgebraicOperands,
        inserted: Expr,
    },
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum Node {
    Number(NumericValue),
    ImaginaryUnit,
    Pi,
    Euler,
    Symbol(Arc<SymbolData>),
    Function {
        name: FunctionName,
        args: Vec<Expr>,
    },
    Sum(Vec<Expr>),
    Poly(Polynomial),
    Product(Vec<Expr>),
    Div(Expr, Expr),
    Pow(Expr, Expr),
    Derivative {
        inner: Expr,
        var: Arc<SymbolData>,
        order: NonZeroU32,
    },
}

/// A canonical sparse polynomial in shared scalar expression bases.
///
/// Bases may be symbols or opaque expressions. Powers are positive `u32`
/// integers; negative and fractional powers retain their general expression
/// representation. Numeric coefficients are shared literal handles, so their
/// `Number` payloads are not copied into each term. Standalone constants belong
/// to the outer sum; this representation has no degree-zero terms.
#[derive(Debug, PartialEq, Eq, Hash)]
struct Polynomial {
    terms: Vec<PolynomialTerm>,
}

/// Borrowed stored edges and compact monomials for numerical engines.
#[derive(Clone, Copy, Debug)]
pub struct EvaluationView<'expr> {
    expression: &'expr Expr,
}

/// One nonzero-coefficient monomial in a canonical sparse polynomial.
///
/// Sparse powers own canonical expression handles, ordered by base structure.
/// Construction is private; bases are unique within a monomial and exponents
/// are positive. Coefficients own shared finite numeric nodes.
#[derive(Debug, PartialEq, Eq, Hash)]
struct PolynomialTerm {
    coefficient: Expr,
    powers: Vec<(Expr, u32)>,
}

#[derive(Debug)]
pub enum Selection {
    Whole(Expr),
    Partial(PartialSelection),
}

#[derive(Debug)]
pub struct PartialSelection {
    sum: bool,
    remainder: Vec<RemainingOperand>,
    inserted: Vec<Expr>,
}

#[derive(Debug)]
pub struct ConcreteMatcher<'entries> {
    exact: ExactSources<'entries>,
    groups: Vec<OperandSource<'entries>>,
    arithmetic: u8,
}

#[derive(Debug)]
pub enum ExactSources<'entries> {
    One(&'entries Substitution),
    Many(&'entries Substitutions),
}

#[derive(Clone)]
enum SubstitutionInput<'root> {
    Stored(&'root Expr),
    Logical(Expr),
}

#[derive(Debug)]
struct OperandSource<'entries> {
    sum: bool,
    operands: Vec<Expr>,
    destination: &'entries Expr,
}

impl Expr {
    fn substitute_logical_with(&self, mut select: impl FnMut(&Self) -> Option<Selection>) -> Self {
        let mut memo = HashMap::new();
        let mut projections = HashMap::new();
        let mut partial: HashMap<Self, PartialSelection> = HashMap::new();
        let mut pending = vec![(SubstitutionInput::Stored(self), false)];
        while let Some((input, ready)) = pending.pop() {
            let original = match &input {
                SubstitutionInput::Stored(original) => *original,
                SubstitutionInput::Logical(original) => original,
            };
            if memo.contains_key(original) {
                continue;
            }
            if ready {
                let rebuilt = partial.remove(original).map_or_else(
                    || Node::rebuild_logical(original, &memo, &projections),
                    |selection| {
                        selection.rebuild(&mut |child| {
                            if child == original {
                                return Node::rebuild_logical(original, &memo, &projections);
                            }
                            memo.get(child)
                                .expect("unmatched original operands precede their parent")
                                .clone()
                        })
                    },
                );
                projections.remove(original);
                memo.insert(original.clone(), rebuilt);
                continue;
            }
            match select(original) {
                Some(Selection::Whole(replacement)) => {
                    memo.insert(original.clone(), replacement);
                }
                Some(Selection::Partial(selection)) => {
                    pending.push((input.clone(), true));
                    for operand in &selection.remainder {
                        match operand {
                            RemainingOperand::Original(child)
                            | RemainingOperand::Divisor(child)
                            | RemainingOperand::Counted { base: child, .. }
                            | RemainingOperand::Power { base: child, .. }
                            | RemainingOperand::DivisorCounted { base: child, .. }
                            | RemainingOperand::Scaled { base: child, .. } => {
                                if child == original {
                                    // A residual power may retain this input as its base.
                                    // Its children are original; its new power is not.
                                    input.expand_children(&mut pending, &mut projections);
                                } else {
                                    pending
                                        .push((SubstitutionInput::Logical(child.clone()), false));
                                }
                            }
                            RemainingOperand::Factored { factor, .. } => pending.extend(
                                factor
                                    .remaining_inputs()
                                    .cloned()
                                    .map(|child| (SubstitutionInput::Logical(child), false)),
                            ),
                            RemainingOperand::Fixed(_) | RemainingOperand::FixedDivisor(_) => {}
                        }
                    }
                    partial.insert(original.clone(), selection);
                }
                None => {
                    pending.push((input.clone(), true));
                    input.expand_children(&mut pending, &mut projections);
                }
            }
        }
        memo.remove(self)
            .expect("the original root is visited once")
    }

    /// Keep a wide integer exponent in a logarithmic number of native powers.
    fn integer_power(base: &Self, exponent: i128) -> Self {
        if let Ok(native) = i64::try_from(exponent) {
            if let Some(value) = base.literal()
                && value.value().is_finite()
                && !value.value().is_zero()
            {
                if native > 0 {
                    return AlgebraicOperands::repeat(base.clone(), native, false);
                }
                if let Some(magnitude) = native.checked_neg().filter(|count| *count > 0) {
                    return Self::scalar_quotient(
                        &NumericValue::new(Number::one()),
                        value,
                        magnitude,
                    );
                }
            } else {
                return base.pow(native);
            }
        }
        let mut remaining = exponent.unsigned_abs();
        let mut block = if exponent < 0 {
            base.literal()
                .filter(|value| value.value().is_finite() && !value.value().is_zero())
                .map_or_else(
                    || base.pow(-1),
                    |value| Self::scalar_quotient(&NumericValue::new(Number::one()), value, 1),
                )
        } else {
            base.clone()
        };
        let mut factors = Vec::new();
        loop {
            if let Ok(native) = i64::try_from(remaining) {
                factors.push(if block.literal().is_some() {
                    AlgebraicOperands::repeat(block, native, false)
                } else {
                    block.pow(native)
                });
                break;
            }
            if remaining & 1 != 0 {
                factors.push(block.clone());
            }
            remaining >>= 1;
            block = if block.literal().is_some() {
                AlgebraicOperands::repeat(block, 2, false)
            } else {
                block.pow(2)
            };
        }
        Self::product(factors)
    }

    /// Divide scalar coefficients without materializing oversized integer powers.
    fn scalar_quotient(numerator: &NumericValue, denominator: &NumericValue, count: i64) -> Self {
        let (coefficient, remaining) = numerator.divide_power(denominator, count);
        if remaining == 0 {
            Self::number(coefficient)
        } else {
            Self::product(vec![
                Self::number(coefficient),
                Self::number(denominator.value().to_float()).pow(remaining.saturating_neg()),
            ])
        }
    }

    fn intern(node: Node) -> Self {
        Self(Some(NODES.intern(node)))
    }

    fn literal(&self) -> Option<&NumericValue> {
        match self.node() {
            Node::Number(value) => Some(value),
            Node::ImaginaryUnit => Some(NumericValue::imaginary_unit()),
            Node::Pi
            | Node::Euler
            | Node::Symbol(_)
            | Node::Function { .. }
            | Node::Sum(_)
            | Node::Poly(_)
            | Node::Product(_)
            | Node::Div(..)
            | Node::Pow(..)
            | Node::Derivative { .. } => None,
        }
    }

    fn node(&self) -> &Node {
        self.0
            .as_deref()
            .expect("live expression handles own their node")
    }

    // Keep this single-handle projection available to generic Hash callers.
    #[inline]
    fn pointer(&self) -> *const Node {
        self.0.as_ref().map_or(null(), Arc::as_ptr)
    }
}

mod analysis;
mod calculus;
mod candidates;
mod constructors;
mod display;
mod evaluation;
mod functions;
mod identity;
mod linear;
mod normalize;
#[cfg(feature = "num-traits")]
mod num_traits;
mod operands;
mod operators;
mod ordering;
mod polynomial;
mod power_matching;
mod powers;
mod projection;
mod properties;
mod reclaim;
mod subsets;
mod transform;
mod view;

pub use properties::{Properties, Property, Truth};

#[cfg(test)]
mod tests;
