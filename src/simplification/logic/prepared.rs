//! Prepare captured rules once and reuse them over all sequence sweeps.

use crate::{
    collections::HashSet,
    core::{Expr, ExprView, Function},
    errors::SimplifyError,
};

use super::{DomainSafety, Limits, PolynomialWork, PreparedPlan, SimplificationStep};

pub struct PreparedStep<'step> {
    step: &'step SimplificationStep,
    plan: &'step PreparedPlan,
    safety: DomainSafety,
}

impl<'step> PreparedStep<'step> {
    pub const fn new(
        step: &'step SimplificationStep,
        plan: &'step PreparedPlan,
        safety: DomainSafety,
    ) -> Self {
        Self { step, plan, safety }
    }

    pub fn apply(&self, expression: &Expr, limits: Limits) -> Result<Expr, SimplifyError> {
        let mut remaining = limits.expansion();
        let mut polynomial_work = PolynomialWork::new(limits.polynomial());
        let mut result = match self.step {
            SimplificationStep::Rewrite(rule) => {
                rule.apply_all(expression, limits.matches(), None)?
            }
            SimplificationStep::Map(callback) => {
                expression.map(|node| Ok::<_, SimplifyError>(callback(node)))?
            }
            SimplificationStep::Expand => {
                expression.map(|node| Self::expand(node, &mut remaining, limits.expansion()))?
            }
            SimplificationStep::Factor => expression.clone(),
            SimplificationStep::Algebraic => expression.map(|node| {
                let direct = Self::identities(node, self.step, self.safety);
                let candidate = direct.as_ref().unwrap_or(node);
                Ok::<_, SimplifyError>(
                    Self::polynomial_cancellation(candidate, self.safety, &mut polynomial_work)?
                        .or(direct),
                )
            })?,
            SimplificationStep::Trigonometric
            | SimplificationStep::Hyperbolic
            | SimplificationStep::Exponential
            | SimplificationStep::Roots
            | SimplificationStep::Compilation => expression.map(|node| {
                Ok::<_, SimplifyError>(Self::identities(node, self.step, self.safety))
            })?,
        };
        if result != *expression {
            limits.validate(&result)?;
        }
        let mut functions = if self.plan.is_empty(self.safety) {
            HashSet::new()
        } else {
            Self::function_set(&result)
        };
        for rule in self.plan.rules(self.safety) {
            let next = rule.apply(&result, &functions, limits.matches())?;
            if next != result {
                limits.validate(&next)?;
                result = next;
                functions = Self::function_set(&result);
            }
        }
        if matches!(
            self.step,
            SimplificationStep::Factor | SimplificationStep::Compilation
        ) {
            let factored = result.map(|node| Ok::<_, SimplifyError>(Self::factor(node)))?;
            if factored != result {
                limits.validate(&factored)?;
            }
            return Ok(factored);
        }
        Ok(result)
    }

    fn function_set(expression: &Expr) -> HashSet<Function> {
        expression.fold(HashSet::new(), |mut names, node| {
            if let ExprView::Function { name, .. } = node.view() {
                names.insert(name.clone());
            }
            names
        })
    }
}
