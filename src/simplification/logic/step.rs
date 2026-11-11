//! One builtin or caller step, bound to its compiled rules and domain contract.

use crate::{
    core::{ExactIdentity, Expr, ExprView, Postorder, Property, SettledStage, Truth},
    errors::SimplifyError,
    polynomial::PolynomialWork,
};

use super::{
    Algebraic, Branches, DomainSafety, Identities, Limits, Polynomials, Powers, PreparedPlan,
    SimplificationStep, SimplificationTarget,
};

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

    /// Run the step's direct identities, then one pass of its compiled rules,
    /// then, for Factor, factoring. Each stage that changes the expression
    /// validates its output. At each node the direct identities are tried in
    /// order and the first that applies is kept.
    ///
    /// The builtin identities number compact storage in symbols and constants
    /// as a leaf: its terms, a literal times positive powers of distinct
    /// scalar leaves, are fixed points of every builtin step, so the storage is
    /// read whole and its terms are never built. Caller steps visit every
    /// logical node.
    ///
    /// A builtin step's direct identities, and its factoring, read only the
    /// node and the domain contract: a node they keep is kept again, even
    /// under a smaller budget, since an exhausted budget is an error and never
    /// a kept node. So each such map records the nodes it keeps, one bit per
    /// step and contract, and later runs, in this or any sequence, skip them
    /// with their descendants. The rule pass reads the builtins below each
    /// node and visits every node.
    pub fn apply(
        &self,
        expression: &Expr,
        limits: Limits,
        target: SimplificationTarget,
    ) -> Result<Expr, SimplifyError> {
        let safety = self.safety;
        // The size goal is the sequence's: `compact()` keeps a grouped sum only
        // when it stores no more nodes; `factored()` and `compilation()` keep
        // an explicit `Factor` step's nested arithmetic.
        let minimize = !matches!(
            target,
            SimplificationTarget::Factored | SimplificationTarget::Compilation
        );
        let Some(stage) = self.stage_bit(minimize) else {
            let next = if let SimplificationStep::Rewrite(rule) = self.step {
                rule.apply_all(expression, limits.matches, None)?
            } else if let SimplificationStep::Map(callback) = self.step {
                expression.map(|node| Ok::<_, SimplifyError>(callback(node)))?
            } else {
                return Ok(expression.clone());
            };
            if next != *expression {
                limits.validate(&next)?;
            }
            return Ok(next);
        };
        let mut result = match self.step {
            SimplificationStep::Expand => {
                let mut remaining = limits.expansion;
                Postorder::settled(expression, stage)
                    .map(|node| Polynomials::expand(node, &mut remaining, limits.expansion))?
            }
            SimplificationStep::Algebraic => {
                let mut work = PolynomialWork::new(limits.polynomial);
                Postorder::settled(expression, stage).map(|node| {
                    let direct = Algebraic::cancel(node, safety)
                        .or_else(|| Powers::combine(node, safety))
                        .or_else(|| Algebraic::combine_fractions(node))
                        .or_else(|| Algebraic::absolute(node));
                    let candidate = direct.as_ref().unwrap_or(node);
                    Ok::<_, SimplifyError>(
                        Algebraic::polynomial_cancellation(candidate, safety, &mut work)?
                            .or(direct),
                    )
                })?
            }
            SimplificationStep::Roots => Postorder::settled(expression, stage).map(|node| {
                Ok::<_, SimplifyError>(
                    Branches::combine_roots(node, safety)
                        .or_else(|| Identities::root(node, safety)),
                )
            })?,
            SimplificationStep::Exponential => {
                Postorder::settled(expression, stage).map(|node| {
                    Ok::<_, SimplifyError>(
                        Branches::combine_logarithms(node, safety)
                            .or_else(|| Identities::logarithm(node, safety)),
                    )
                })?
            }
            SimplificationStep::Trigonometric => {
                Postorder::settled(expression, stage).map(|node| {
                    Ok::<_, SimplifyError>(
                        Identities::angle(node).or_else(|| Identities::parity(node)),
                    )
                })?
            }
            SimplificationStep::Hyperbolic => Postorder::settled(expression, stage)
                .map(|node| Ok::<_, SimplifyError>(Identities::parity(node)))?,
            // $x^{1/3}=\operatorname{cbrt}(x)$ for positive $x$. The exact
            // exponent must be stored; inferred power groups introduce new roots.
            SimplificationStep::Compilation => {
                Postorder::settled(expression, stage).map(|node| {
                    Ok::<_, SimplifyError>(
                        if let ExprView::Pow(base, exponent) = node.view()
                            && exponent.exact_ratio() == Some((1, 3))
                            && base.properties().get(Property::Positive) == Truth::True
                        {
                            Some(base.cbrt())
                        } else {
                            None
                        },
                    )
                })?
            }
            SimplificationStep::Factor
            | SimplificationStep::Rewrite(_)
            | SimplificationStep::Map(_) => expression.clone(),
        };
        if result != *expression {
            limits.validate(&result)?;
        }
        if let Some(rules) = self.plan.rules(safety) {
            let next = rules.apply(&result, limits.matches)?;
            if next != result {
                limits.validate(&next)?;
                result = next;
            }
        }
        if matches!(self.step, SimplificationStep::Factor) {
            // The Factor step also owns the binomial identities.
            let factored = Postorder::settled(&result, stage)
                .map(|node| Ok::<_, SimplifyError>(Polynomials::factor(node, minimize)))?;
            if factored != result {
                limits.validate(&factored)?;
            }
            return Ok(factored);
        }
        Ok(result)
    }

    const fn stage_bit(&self, minimize: bool) -> Option<SettledStage> {
        // Trigonometric, Hyperbolic, Expand, and Compilation read only the
        // node; Algebraic, Roots, and Exponential read the domain contract;
        // Factor also depends on whether minimizing size keeps reassociated
        // groupings out.
        let index = match (self.step, self.safety) {
            (SimplificationStep::Algebraic, DomainSafety::Preserving) => 0,
            (SimplificationStep::Algebraic, DomainSafety::AssumptionDependent) => 1,
            (SimplificationStep::Roots, DomainSafety::Preserving) => 2,
            (SimplificationStep::Roots, DomainSafety::AssumptionDependent) => 3,
            (SimplificationStep::Exponential, DomainSafety::Preserving) => 4,
            (SimplificationStep::Exponential, DomainSafety::AssumptionDependent) => 5,
            (SimplificationStep::Trigonometric, _) => 6,
            (SimplificationStep::Hyperbolic, _) => 7,
            (SimplificationStep::Expand, _) => 8,
            (SimplificationStep::Factor, _) if minimize => 9,
            (SimplificationStep::Factor, _) => 10,
            (SimplificationStep::Compilation, _) => 11,
            (SimplificationStep::Rewrite(_) | SimplificationStep::Map(_), _) => return None,
        };
        Some(SettledStage::bit(index))
    }
}
