//! The shared handle owner and its private node representation.

#![expect(
    clippy::self_named_module_files,
    reason = "the type owner keeps fields private while its descendants implement operations; mod.rs files contain plumbing only"
)]

use core::{ptr::null, slice::Iter};

use alloc::{borrow::Cow, sync::Arc, vec, vec::Vec};

use num_anafis::Number;

use super::{
    BuiltinFunction, ExprView, FunctionDefinition, FunctionName, Interner, NumericValue, SYMBOLS,
    Substitution, Substitutions, Symbol, SymbolData,
};

static NODES: Interner<Node> = Interner::new();

/// An immutable handle to a globally interned expression DAG.
///
/// Equal live expressions share one allocation. Cloning shares ownership; equality
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
}

#[derive(Debug)]
struct WeightedOperand {
    original: Expr,
    base: Expr,
    count: i64,
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
}

#[derive(Debug)]
enum RemainingOperand {
    Original(Expr),
    Counted { base: Expr, count: i64 },
    Fixed(Expr),
    Scaled { base: Expr, coefficient: Expr },
    Divisor(Expr),
    FixedDivisor(Expr),
    DivisorCounted { base: Expr, count: i64 },
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
        order: u32,
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
    bases: Vec<Expr>,
    terms: Vec<PolynomialTerm>,
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

#[derive(Debug)]
struct OperandSource<'entries> {
    sum: bool,
    operands: Vec<Expr>,
    destination: &'entries Expr,
}

impl Expr {
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

    /// Fold finite division and remove an exact unit denominator.
    fn div_expr(left: Self, right: Self) -> Self {
        if right.is_one_num() {
            return left;
        }
        if let (Some(numerator), Some(denominator)) = (left.literal(), right.literal())
            && numerator.value().is_finite()
            && denominator.value().is_finite()
            && let Some(value) = numerator.div(denominator)
        {
            return Self::number(value);
        }
        Self::intern(Node::Div(left, right))
    }

    /// Fold finite powers while preserving symbolic and undefined zero powers.
    fn pow_static(base: Self, exponent: Self) -> Self {
        if exponent.is_one_num() {
            return base;
        }
        if let (Some(value), Some(power)) = (base.literal(), exponent.literal())
            && value.value().is_finite()
            && power.value().is_finite()
            && let Some(folded) = value.pow(power)
        {
            return Self::number(folded);
        }
        Self::intern(Node::Pow(base, exponent))
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
mod constructors;
mod display;
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
mod powers;
mod projection;
mod reclaim;
mod subsets;
mod transform;
mod view;

#[cfg(test)]
mod tests;
