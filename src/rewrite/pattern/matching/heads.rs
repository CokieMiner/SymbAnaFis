//! Logical node heads, node shapes, counted-operand indices, and exact power
//! template queries.
//!
//! A head set is one bit per logical node shape. A rule's pattern carries the
//! heads it can match at its root and the heads each of its operands must
//! have, so a node or an operand outside them is rejected with one mask test
//! before any stage runs or any candidate is built.

use core::num::NonZeroU64;

use super::{
    AlgebraicOperands, BuiltinFunction, Expr, ExprView, Function, OperandDemand, Pattern,
    PatternNode,
};

/// Logical node heads, one bit each, at which a rule's pattern can match.
///
/// Every matching stage fails at a node outside the set before it counts an
/// attempt or calls a guard, so testing the set first changes no result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Heads(pub u8);

/// The heads a counted additive operand can project to, and the heads of the
/// factors of the value it projects to.
#[derive(Clone, Copy, Debug)]
pub struct CountedSlot {
    /// A superset of the heads of the value the operand projects to.
    pub value: Heads,
    /// A superset of the heads of that value's factors.
    pub factors: Heads,
}

/// The node-only half of a rule's head test, read once per node.
///
/// `heads` is the bit the node contributes, so the per-rule test is one mask
/// and one branch instead of a twelve-arm `ExprView` dispatch. `CALL` is the
/// one head that also depends on the rule, because a function pattern accepts
/// only a call of its own function; the node therefore carries its function and
/// a call rule compares it.
#[derive(Clone, Copy, Debug)]
pub struct NodeShape<'node> {
    pub heads: Heads,
    pub function: Option<&'node Function>,
}

/// A fixed exact exponent exposed by a power, reciprocal, or square-root
/// template, retaining its principal base template.
pub struct PowerTemplate<'pattern> {
    pub base: &'pattern Pattern,
    pub numerator: i64,
    pub denominator: NonZeroU64,
    pub inverse: bool,
}

impl Heads {
    pub const SUM: Self = Self(1);
    pub const PRODUCT: Self = Self(2);
    pub const DIV: Self = Self(4);
    pub const POW: Self = Self(8);
    pub const SQRT: Self = Self(16);
    /// A call of the pattern root's function.
    pub const CALL: Self = Self(32);
    pub const OTHER: Self = Self(64);
    /// The heads that supply counted factor groups.
    pub const COUNTED: Self = Self(Self::PRODUCT.0 | Self::DIV.0 | Self::POW.0 | Self::SQRT.0);
    pub const ALL: Self = Self(127);

    /// The set holding every head of either operand.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether every head of `other` is in this set.
    pub const fn covers(self, other: Self) -> bool {
        other.0 & !self.0 == 0
    }

    /// Whether a value of head `other` can satisfy this set.
    pub const fn accepts(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// The head of one expression, for rejecting an operand before a candidate
    /// is built.
    ///
    /// A call is `CALL` whatever its function, because a function template
    /// accepts any call of its own name; the `SQRT` bit belongs to the root
    /// stage alone, which tests the pattern's function against the node's.
    pub fn of(expression: &Expr) -> Self {
        match expression.view() {
            ExprView::Sum(_) => Self::SUM,
            ExprView::Product(_) => Self::PRODUCT,
            ExprView::Div(..) => Self::DIV,
            ExprView::Pow(..) => Self::POW,
            ExprView::Function { .. } => Self::CALL,
            ExprView::Number(_)
            | ExprView::ImaginaryUnit
            | ExprView::Pi
            | ExprView::Euler
            | ExprView::Symbol(_)
            | ExprView::Derivative { .. } => Self::OTHER,
        }
    }

    /// A superset of the heads the value a counted additive operand projects
    /// to can have, and of the heads of that value's factors.
    ///
    /// Such an operand projects to its original term, to the base of its exact
    /// coefficient, or to a coefficient times that base, so the value set holds
    /// the heads of the first two and `PRODUCT` for the third. The base is the
    /// original term unless the term is a product with a nonzero exact integer
    /// coefficient and a nonempty remainder, in which case it is that
    /// remainder's single factor or its product, which is itself a product. The
    /// factor set therefore holds the heads of the original's factors, and
    /// `OTHER` for the coefficient a projection may inject.
    pub fn counted(original: &Expr) -> CountedSlot {
        let mut value = Self::of(original).union(Self::PRODUCT);
        let mut factors = Self::OTHER;
        match original.view() {
            ExprView::Product(items) => {
                for factor in items {
                    factors = factors.union(Self::of(factor));
                }
                if let Some((coefficient, rest)) = items.split_first()
                    && AlgebraicOperands::integer(coefficient)
                        .and_then(i64::checked_abs)
                        .is_some_and(|count| count > 0)
                    && let [base] = rest
                {
                    value = value.union(Self::of(base));
                }
            }
            ExprView::Sum(_)
            | ExprView::Div(..)
            | ExprView::Pow(..)
            | ExprView::Function { .. }
            | ExprView::Number(_)
            | ExprView::ImaginaryUnit
            | ExprView::Pi
            | ExprView::Euler
            | ExprView::Symbol(_)
            | ExprView::Derivative { .. } => factors = factors.union(Self::of(original)),
        }
        CountedSlot { value, factors }
    }
}

impl<'node> NodeShape<'node> {
    pub fn of(node: &'node Expr) -> Self {
        let (heads, function) = match node.view() {
            ExprView::Sum(_) => (Heads::SUM, None),
            ExprView::Product(_) => (Heads::PRODUCT, None),
            ExprView::Div(..) => (Heads::DIV, None),
            ExprView::Pow(..) => (Heads::POW, None),
            ExprView::Function { name, .. } if *name == BuiltinFunction::Sqrt.function() => {
                (Heads::SQRT, Some(name))
            }
            ExprView::Function { name, .. } => (Heads::CALL, Some(name)),
            ExprView::Number(_)
            | ExprView::ImaginaryUnit
            | ExprView::Pi
            | ExprView::Euler
            | ExprView::Symbol(_)
            | ExprView::Derivative { .. } => (Heads::OTHER, None),
        };
        Self { heads, function }
    }
}

impl PatternNode {
    /// The heads a value must have for `Search::compatible` to accept it, or
    /// `ALL` when acceptance depends on the bindings or on nothing.
    ///
    /// A candidate outside the set is rejected by `compatible` anyway, so
    /// skipping it before it is built changes no result.
    pub fn operand_heads(&self) -> Heads {
        match self {
            Self::Function { .. } => Heads::CALL,
            Self::Div(..) => Heads::DIV,
            Self::Pow(..) => Heads::POW,
            // An exact template holds the interned value it accepts, so the
            // value's head is the template's own.
            Self::Exact(value) => Heads::of(value),
            Self::Capture { .. } | Self::Sum(_) | Self::Product(_) => Heads::ALL,
        }
    }

    /// The heads that must occur among the factors of a value this template
    /// accepts, or the empty set when it constrains none.
    ///
    /// A product template matches its factors commutatively, so each of them
    /// must match a distinct factor of the value. A leading exact coefficient
    /// is excluded because a missing one is inferred by dividing the whole
    /// value, which adds a numeric factor instead of requiring one.
    pub fn required_factors(&self) -> Heads {
        let Self::Product(items) = self else {
            return Heads(0);
        };
        let mut required = Heads(0);
        for factor in items {
            let heads = factor.node().operand_heads();
            if heads == Heads::ALL {
                return Heads(0);
            }
            required = required.union(heads);
        }
        // The coefficient is inferred, so it need not occur in the value.
        match items.first().map(Pattern::node) {
            Some(Self::Exact(value)) => Heads(required.0 & !Heads::of(value).0),
            _ => required,
        }
    }

    /// Prove a fixed exponent or coefficient before projecting a virtual operand.
    pub fn operand_demand(&self, sum: bool, power: Option<&PowerTemplate<'_>>) -> OperandDemand {
        if let Some(source) = power {
            return if source.denominator == NonZeroU64::MIN {
                OperandDemand::Power(source.numerator)
            } else {
                OperandDemand::Rational {
                    numerator: source.numerator,
                    denominator: source.denominator,
                }
            };
        }
        match self {
            Self::Exact(value) => AlgebraicOperands::demand(value, sum),
            Self::Pow(..) if !sum => OperandDemand::Whole,
            Self::Product(factors) if sum => factors
                .first()
                .and_then(|factor| {
                    if let Self::Exact(value) = factor.node() {
                        AlgebraicOperands::integer(value).and_then(i64::checked_abs)
                    } else {
                        None
                    }
                })
                .filter(|count| *count > 0)
                .map_or(OperandDemand::Single, OperandDemand::Copies),
            Self::Capture { .. }
            | Self::Function { .. }
            | Self::Sum(_)
            | Self::Product(_)
            | Self::Div(..)
            | Self::Pow(..) => OperandDemand::Single,
        }
    }

    /// Expose only a fixed exact exponent, retaining the template's principal base.
    pub fn power_template(&self) -> Option<PowerTemplate<'_>> {
        let reciprocal = match self {
            Self::Div(unit, denominator) if matches!(unit.node(), Self::Exact(value) if value.is_exact_one()) => {
                Some(denominator)
            }
            Self::Capture { .. }
            | Self::Exact(_)
            | Self::Function { .. }
            | Self::Sum(_)
            | Self::Product(_)
            | Self::Div(..)
            | Self::Pow(..) => None,
        };
        let node = reciprocal.map_or(self, Pattern::node);
        if reciprocal.is_some()
            && matches!(node, Self::Div(unit, _) if matches!(unit.node(), Self::Exact(value) if value.is_exact_one()))
        {
            return None;
        }
        let projected = match node {
            Self::Pow(base, exponent) => {
                let ratio = if let Self::Exact(value) = exponent.node() {
                    AlgebraicOperands::ratio(value)
                } else {
                    None
                };
                ratio.map(|(numerator, denominator)| (base, numerator, denominator))
            }
            Self::Function { name, args } if *name == BuiltinFunction::Sqrt.function() => {
                let [base] = args.as_slice() else { return None };
                Some((base, 1, NonZeroU64::new(2)?))
            }
            Self::Capture { .. }
            | Self::Exact(_)
            | Self::Function { .. }
            | Self::Sum(_)
            | Self::Product(_)
            | Self::Div(..) => None,
        };
        let Some((base, signed, denominator)) = projected else {
            return reciprocal.map(|base| PowerTemplate {
                base,
                numerator: 1,
                denominator: NonZeroU64::MIN,
                inverse: true,
            });
        };
        if signed < 0 && reciprocal.is_some() {
            return None;
        }
        let numerator = signed.checked_abs()?;
        (numerator > 0).then_some(PowerTemplate {
            base,
            numerator,
            denominator,
            inverse: signed < 0 || reciprocal.is_some(),
        })
    }
}
