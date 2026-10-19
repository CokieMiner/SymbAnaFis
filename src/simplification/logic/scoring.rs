//! Memoized costs for selecting among completed simplification candidates.

use crate::{
    collections::HashMap,
    core::{BuiltinFunction, Expr, ExprView, FunctionDefinition},
};

use super::{PreparedStep, SimplificationTarget};

/// Lexicographic objective followed by logical expression size.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CandidateCost {
    primary: usize,
    size: usize,
}

impl CandidateCost {
    /// Compute logical costs once per distinct node, preserving repeated edge counts.
    /// Saturation keeps costs totally ordered when logical size exceeds `usize`.
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
        // The final fold visit is the root. Carry its completed cost directly
        // rather than retrieving the root from the child memo afterward.
        let (_, size, penalty) = expression.fold(
            (HashMap::<Expr, (usize, usize)>::new(), 0, 0),
            |(mut values, _, _), node| {
                if objective == SimplificationTarget::Compilation {
                    native_cost = native_cost.saturating_add(Self::compilation_cost(node));
                }
                let mut size = 1_usize;
                let mut penalty = 0_usize;
                for child in node.children() {
                    let (child_size, child_penalty) = values
                        .get(&child)
                        .copied()
                        .expect("logical cost visits children before parents");
                    size = size.saturating_add(child_size);
                    penalty = penalty.saturating_add(child_penalty);
                }
                let local = match (objective, node.view()) {
                    (SimplificationTarget::Factored, ExprView::Sum(terms)) => {
                        terms.filter(Self::composite_term).count()
                    }
                    (SimplificationTarget::Expanded, ExprView::Product(factors))
                        if factors
                            .iter()
                            .any(|factor| matches!(factor.view(), ExprView::Sum(_))) =>
                    {
                        1
                    }
                    (SimplificationTarget::Expanded, ExprView::Pow(base, exponent))
                        if matches!(base.view(), ExprView::Sum(_))
                            && PreparedStep::exact_integer(exponent)
                                .is_some_and(|count| count > 1) =>
                    {
                        1
                    }
                    _ => 0,
                };
                penalty = penalty.saturating_add(local);
                values.insert(node.clone(), (size, penalty));
                (values, size, penalty)
            },
        );
        let primary = match objective {
            SimplificationTarget::Compact => size,
            SimplificationTarget::Expanded | SimplificationTarget::Factored => penalty,
            SimplificationTarget::Compilation => native_cost,
        };
        Self {
            primary: custom_score.map_or(primary, |scorer| scorer(expression)),
            size,
        }
    }

    fn composite_term(expression: &Expr) -> bool {
        match expression.view() {
            ExprView::Pow(_, exponent) => PreparedStep::exact_integer(exponent).is_some_and(|count| count > 1),
            ExprView::Product(factors) => factors.iter().filter(|factor| factor.as_number().is_none()).count() > 1
                || factors.iter().any(|factor| matches!(factor.view(), ExprView::Pow(_, exponent) if PreparedStep::exact_integer(exponent).is_some_and(|count| count > 1))),
            ExprView::Number(_) | ExprView::ImaginaryUnit | ExprView::Pi | ExprView::Euler
            | ExprView::Symbol(_) | ExprView::Function { .. } | ExprView::Sum(_)
            | ExprView::Div(..) | ExprView::Derivative { .. } => false,
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
            ExprView::Pow(_, exponent) => PreparedStep::exact_integer(exponent)
                .filter(|count| *count > 0)
                .map_or(24, Self::integer_power_cost),
            ExprView::Function { name, .. } => {
                let builtin = FunctionDefinition::get(name).map(FunctionDefinition::builtin);
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
