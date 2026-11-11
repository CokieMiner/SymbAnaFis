//! Costs for selecting among completed simplification candidates.

use core::convert::Infallible;

use crate::{
    collections::FixedSet,
    core::{BuiltinFunction, ExactIdentity, Expr, ExprView, Mapped, Postorder, StoredForm},
};

use super::SimplificationTarget;

/// Lexicographic objective followed by logical expression size.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CandidateCost {
    primary: usize,
    size: usize,
}

/// The cost facts of one logical node, read by its parent.
#[derive(Clone, Copy)]
struct Term {
    size: usize,
    penalty: usize,
    composite: bool,
    sum: bool,
}

impl CandidateCost {
    /// Compute logical costs once per distinct node, children before parents;
    /// compact storage in symbols and constants is scored from its terms
    /// without building them. Saturation keeps costs totally ordered when the
    /// logical size exceeds `usize`.
    pub fn new(
        expression: &Expr,
        target: SimplificationTarget,
        custom_score: Option<&(dyn Fn(&Expr) -> usize + Send + Sync)>,
    ) -> Self {
        let objective = if custom_score.is_some() {
            SimplificationTarget::Compact
        } else {
            target
        };
        let mut native_cost = 0_usize;
        let Ok(root) = Postorder::compact(expression).evaluate(|node| {
            let mut term = Self::term(node, objective);
            let group = Self::group(node);
            if let Some((size, composite, _)) = group {
                term.size = term.size.saturating_add(size);
                if objective == SimplificationTarget::Factored {
                    term.penalty = term.penalty.saturating_add(composite);
                }
            }
            if objective == SimplificationTarget::Compilation {
                native_cost = native_cost
                    .saturating_add(Self::compilation_cost(node.expression()))
                    .saturating_add(group.map_or(0, |(_, _, operations)| operations));
            }
            Ok::<_, Infallible>(term)
        });
        let primary = match objective {
            SimplificationTarget::Compact => root.size,
            SimplificationTarget::Expanded | SimplificationTarget::Factored => root.penalty,
            SimplificationTarget::Compilation => native_cost,
        };
        Self {
            primary: custom_score.map_or(primary, |scorer| scorer(expression)),
            size: root.size,
        }
    }

    /// Size counts repeated edges. Expanded penalties count distributable
    /// sites; factored penalties count composite terms under sums, except below
    /// a product, which is already the desired factored form.
    fn term(node: &Mapped<'_, '_, Term>, objective: SimplificationTarget) -> Term {
        let view = node.expression().view();
        let mut size = 1_usize;
        let mut penalty = 0_usize;
        let mut composite = 0_usize;
        let mut sum_factor = false;
        for child in node.values() {
            size = size.saturating_add(child.size);
            penalty = penalty.saturating_add(child.penalty);
            composite = composite.saturating_add(usize::from(child.composite));
            sum_factor |= child.sum;
        }
        let local = match (objective, &view) {
            (SimplificationTarget::Factored, ExprView::Sum(_)) => composite,
            (SimplificationTarget::Factored, ExprView::Product(_)) => {
                penalty = 0;
                0
            }
            (SimplificationTarget::Expanded, ExprView::Product(_)) => usize::from(sum_factor),
            (SimplificationTarget::Expanded, ExprView::Pow(base, exponent)) => usize::from(
                matches!(base.view(), ExprView::Sum(_))
                    && matches!(exponent.exact_ratio(), Some((count, 1)) if count > 1),
            ),
            _ => 0,
        };
        Term {
            size,
            penalty: penalty.saturating_add(local),
            composite: Self::composite_term(&view),
            sum: matches!(view, ExprView::Sum(_)),
        }
    }

    /// The size, composite count, and operation weight of the terms of the
    /// compact storage in symbols and constants that the numbering keeps
    /// whole at a node, read without building them; `None` when it keeps none.
    ///
    /// Each term is the product of its literal, unless that is the unit, and
    /// of its powers, where a power above one is a power node over a leaf base
    /// and an exponent leaf. This gives the logical size and penalties
    /// exactly. Operation weights count each distinct power once within the
    /// storage; the additions joining the terms belong to the sum.
    fn group(node: &Mapped<'_, '_, Term>) -> Option<(usize, usize, usize)> {
        let monomials = node.group()?.monomials()?;
        let mut size = 0_usize;
        let mut composite = 0_usize;
        let mut cost = 0_usize;
        let mut powers = FixedSet::default();
        for (coefficient, factors) in monomials {
            let scaled = !coefficient.is_exact_one();
            let count = usize::from(scaled).saturating_add(factors.len());
            let mut raised = false;
            // A product node holds several factors; one factor is the term.
            let mut term = usize::from(count > 1).saturating_add(usize::from(scaled));
            for (base, exponent) in factors.clone() {
                if exponent > 1 {
                    raised = true;
                    term = term.saturating_add(3);
                    if powers.insert((base, exponent)) {
                        cost = cost.saturating_add(Self::integer_power_cost(exponent.into()));
                    }
                } else {
                    term = term.saturating_add(1);
                }
            }
            size = size.saturating_add(term);
            composite = composite.saturating_add(usize::from(factors.len() > 1 || raised));
            cost = cost.saturating_add(count.saturating_sub(1));
        }
        Some((size, composite, cost))
    }

    /// A positive integer power, or a product of several nonnumeric factors or
    /// with such a power.
    fn composite_term(view: &ExprView<'_>) -> bool {
        let integer_power =
            |exponent: &Expr| matches!(exponent.exact_ratio(), Some((count, 1)) if count > 1);
        match view {
            ExprView::Pow(_, exponent) => integer_power(exponent),
            ExprView::Product(factors) => {
                factors
                    .iter()
                    .filter(|factor| factor.as_number().is_none())
                    .count()
                    > 1
                    || factors.iter().any(|factor| {
                        matches!(factor.view(), ExprView::Pow(_, exponent) if integer_power(exponent))
                    })
            }
            ExprView::Number(_)
            | ExprView::ImaginaryUnit
            | ExprView::Pi
            | ExprView::Euler
            | ExprView::Symbol(_)
            | ExprView::Function { .. }
            | ExprView::Sum(_)
            | ExprView::Div(..)
            | ExprView::Derivative { .. } => false,
        }
    }

    /// Portable operation weights, summed once per distinct logical DAG node.
    fn compilation_cost(expression: &Expr) -> usize {
        match expression.view() {
            ExprView::Number(_)
            | ExprView::ImaginaryUnit
            | ExprView::Pi
            | ExprView::Euler
            | ExprView::Symbol(_) => 0,
            ExprView::Sum(children) => children.len().saturating_sub(1),
            ExprView::Product(factors) => factors.len().saturating_sub(1),
            ExprView::Div(..) => 4,
            ExprView::Pow(_, exponent) => exponent
                .exact_ratio()
                .filter(|(count, denominator)| *count > 0 && *denominator == 1)
                .map_or(24, |(count, _)| Self::integer_power_cost(count)),
            ExprView::Function { name, .. } => {
                let builtin = BuiltinFunction::of(name);
                if matches!(
                    builtin,
                    Some(
                        BuiltinFunction::Abs
                            | BuiltinFunction::Floor
                            | BuiltinFunction::Ceil
                            | BuiltinFunction::Round
                            | BuiltinFunction::Trunc
                            | BuiltinFunction::Fract
                            | BuiltinFunction::Signum
                    )
                ) {
                    1
                } else if builtin == Some(BuiltinFunction::Sqrt) {
                    4
                } else if builtin == Some(BuiltinFunction::Cbrt) {
                    8
                } else {
                    16
                }
            }
            ExprView::Derivative { .. } => 32,
        }
    }

    /// The multiplication count of binary exponentiation for `count > 0`.
    fn integer_power_cost(mut count: i64) -> usize {
        let mut cost = 0_usize;
        // Positive native powers use at most 62 squares and 62 additional products.
        while count > 1 {
            cost = cost
                .saturating_add(1)
                .saturating_add(usize::from(count & 1 != 0));
            count >>= 1;
        }
        cost
    }
}
