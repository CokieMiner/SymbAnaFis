//! Reusable ordered passes and target configuration.

use core::fmt::{Debug, Formatter, Result as FmtResult};

use alloc::{sync::Arc, vec, vec::Vec};

use crate::{collections::HashSet, core::Expr, errors::SimplifyError};

use super::{
    CandidateCost, DomainSafety, Limits, PreparedPlan, PreparedStep, SimplificationStep,
    SimplificationTarget, SimplificationTermination,
};

type Score = dyn Fn(&Expr) -> usize + Send + Sync;

/// An immutable, reusable simplification program.
///
/// Steps execute in insertion order, including temporary increases in cost.
/// The input and every completed step compete under the target; equal scores
/// select the latest candidate. Repetition stops at a fixed point, a cycle, or
/// an iteration limit. Selection is heuristic over visited candidates.
///
/// ```
/// use symb_anafis::{Expr, Pattern, RewriteRule, SimplificationSequence, SimplificationStep};
/// let x = Expr::symbol("sequence_x");
/// let a = Pattern::capture("a", None);
/// let identity = RewriteRule::new(a.sin().pow(2) + a.cos().pow(2), Pattern::exact(1))?;
/// let sequence = SimplificationSequence::new()
///     .then(identity)
///     .then(SimplificationStep::Factor);
/// assert_eq!(sequence.simplify(&(x.sin().pow(2) + x.cos().pow(2)))?.into_expression(), Expr::number(1));
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Clone)]
pub struct SimplificationSequence {
    steps: Vec<SimplificationStep>,
    // Each step and its compiled plan are inserted and removed together.
    // Plans own no references into steps; clones share immutable rule storage.
    plans: Vec<PreparedPlan>,
    target: SimplificationTarget,
    domain_safety: DomainSafety,
    score: Option<Arc<Score>>,
    limits: Limits,
}

/// The best visited candidate and the reason a sequence stopped.
#[derive(Clone, Debug)]
pub struct Simplification {
    expression: Expr,
    termination: SimplificationTermination,
    iterations: usize,
}

impl SimplificationSequence {
    /// Create an empty sequence with the compact target and default limits.
    /// Canonical construction has already folded and collected basic arithmetic.
    /// Default work and node bounds are capped at `usize::MAX` on 16-bit targets.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            steps: Vec::new(),
            plans: Vec::new(),
            target: SimplificationTarget::Compact,
            domain_safety: DomainSafety::Preserving,
            score: None,
            limits: Limits::DEFAULT,
        }
    }

    /// Build the compact identity and common-factor preset.
    #[must_use]
    pub fn compact() -> Self {
        let steps = vec![
            SimplificationStep::Algebraic,
            SimplificationStep::Roots,
            SimplificationStep::Exponential,
            SimplificationStep::Trigonometric,
            SimplificationStep::Hyperbolic,
            SimplificationStep::Factor,
        ];
        let plans = steps.iter().map(PreparedPlan::new).collect();
        Self {
            steps,
            plans,
            ..Self::new()
        }
    }

    /// Build the identity preset followed by bounded polynomial expansion.
    #[must_use]
    pub fn expanded() -> Self {
        let mut sequence = Self::compact().target(SimplificationTarget::Expanded);
        sequence.steps.pop();
        sequence.plans.pop();
        sequence.then(SimplificationStep::Expand)
    }

    /// Build the identity and common-factor preset with a factored objective.
    #[must_use]
    pub fn factored() -> Self {
        Self::compact().target(SimplificationTarget::Factored)
    }

    /// Build the identity preset followed by native-intrinsic and factoring passes.
    ///
    /// Selection estimates operation cost over the shared DAG; weights are a
    /// backend-independent heuristic, and callers may supply their own scorer.
    ///
    /// ```
    /// use symb_anafis::{Expr, SimplificationSequence};
    /// let x = Expr::symbol("compilation_example_x");
    /// let result = SimplificationSequence::compilation().simplify(&(x.exp() - 1))?;
    /// assert_eq!(result.into_expression(), x.exp_m1());
    /// # Ok::<(), symb_anafis::SimplifyError>(())
    /// ```
    #[must_use]
    pub fn compilation() -> Self {
        Self::compact()
            .then(SimplificationStep::Compilation)
            .target(SimplificationTarget::Compilation)
    }

    /// Append a builtin pass, an existing rewrite, or a custom mapping pass.
    #[must_use]
    pub fn then(mut self, step: impl Into<SimplificationStep>) -> Self {
        let operation = step.into();
        let mut plan = PreparedPlan::new(&operation);
        if self.domain_safety == DomainSafety::AssumptionDependent {
            plan.enable_assumptions(&operation);
        }
        self.steps.push(operation);
        self.plans.push(plan);
        self
    }

    /// Inspect or clone the preset's ordered steps to build another sequence.
    #[must_use]
    pub fn steps(&self) -> &[SimplificationStep] {
        &self.steps
    }

    /// Select a builtin objective and clear any custom scoring function.
    /// Pass order remains caller-controlled.
    #[must_use]
    pub fn target(mut self, target: SimplificationTarget) -> Self {
        self.target = target;
        self.score = None;
        self
    }

    /// Select which builtin rule contracts may execute (default preserving).
    ///
    /// Assumption-dependent rules require the caller to supply the domain and
    /// branch hypotheses documented for each identity. Custom rules and mapping
    /// callbacks remain caller-validated in either mode. Steps and scoring stay
    /// unchanged.
    #[must_use]
    pub fn domain_safety(mut self, safety: DomainSafety) -> Self {
        if safety == DomainSafety::AssumptionDependent && self.domain_safety != safety {
            for (step, plan) in self.steps.iter().zip(&mut self.plans) {
                plan.enable_assumptions(step);
            }
        }
        self.domain_safety = safety;
        self
    }

    /// Minimize a pure caller score, breaking ties by logical expression size.
    /// Equal complete scores select the latest candidate. Callback panics propagate.
    #[must_use]
    pub fn score_with(mut self, score: impl Fn(&Expr) -> usize + Send + Sync + 'static) -> Self {
        self.score = Some(Arc::new(score));
        self
    }

    /// Set the maximum complete sweeps (default 16). Zero returns the input.
    #[must_use]
    pub const fn max_iterations(mut self, limit: usize) -> Self {
        self.limits = self.limits.with_iterations(limit);
        self
    }

    /// Bound stored nodes in the input and each completed candidate (default unlimited, `usize::MAX`).
    #[must_use]
    pub const fn max_nodes(mut self, limit: usize) -> Self {
        self.limits = self.limits.with_nodes(limit);
        self
    }

    /// Bound stored depth in the input and each completed candidate (default unlimited, `usize::MAX`).
    #[must_use]
    pub const fn max_depth(mut self, limit: usize) -> Self {
        self.limits = self.limits.with_depth(limit);
        self
    }

    /// Bound candidate attempts per core rewrite pass (default `None`, unrestricted matching).
    #[must_use]
    pub const fn match_limit(mut self, limit: Option<usize>) -> Self {
        self.limits = self.limits.with_matches(limit);
        self
    }

    /// Bound cumulative generated terms per expansion pass (default unlimited, `usize::MAX`).
    /// The bound is checked before allocating each distributive product.
    #[must_use]
    pub const fn expansion_limit(mut self, limit: usize) -> Self {
        self.limits = self.limits.with_expansion(limit);
        self
    }

    /// Bound sparse polynomial work per algebraic pass (default unlimited, `usize::MAX`).
    /// Parsing, term products, and remainder steps consume this shared budget.
    #[must_use]
    pub const fn polynomial_limit(mut self, limit: usize) -> Self {
        self.limits = self.limits.with_polynomial(limit);
        self
    }

    /// Execute the ordered passes and return the best visited candidate and stop reason.
    /// A result makes no claim of a globally minimal algebraic form.
    ///
    /// # Errors
    /// Returns [`SimplifyError`] if a node, depth, expansion, polynomial, or rewrite
    /// bound is exceeded.
    /// Failed candidates are discarded; the input is immutable.
    ///
    /// # Panics
    /// Propagates callback and numeric backend capacity panics.
    pub fn simplify(&self, expression: &Expr) -> Result<Simplification, SimplifyError> {
        self.limits.validate(expression)?;
        let mut result = Simplification {
            expression: expression.clone(),
            termination: SimplificationTermination::IterationLimit,
            iterations: 0,
        };
        if self.steps.is_empty() {
            result.termination = SimplificationTermination::FixedPoint;
            return Ok(result);
        }
        if self.limits.iterations() == 0 {
            return Ok(result);
        }
        let mut best_score = CandidateCost::new(expression, self.target, self.score.as_deref());
        let mut current = expression.clone();
        let mut seen: HashSet<Expr> = HashSet::from([expression.clone()]);
        for _ in 0..self.limits.iterations() {
            let mut changed = false;
            for (step, plan) in self.steps.iter().zip(&self.plans) {
                let prepared = PreparedStep::new(step, plan, self.domain_safety);
                let next = prepared.apply(&current, self.limits)?;
                if next == current {
                    continue;
                }
                changed = true;
                let candidate_score = CandidateCost::new(&next, self.target, self.score.as_deref());
                if candidate_score <= best_score {
                    best_score = candidate_score;
                    result.expression = next.clone();
                }
                current = next;
            }
            // At most limits.iterations complete sweeps, so this sum fits usize.
            result.iterations = result.iterations.saturating_add(1);
            if !changed {
                result.termination = SimplificationTermination::FixedPoint;
                break;
            }
            if !seen.insert(current.clone()) {
                result.termination = SimplificationTermination::Cycle;
                break;
            }
        }
        Ok(result)
    }
}

impl Simplification {
    /// Borrow the selected canonical expression.
    #[must_use]
    pub const fn expression(&self) -> &Expr {
        &self.expression
    }

    /// Take ownership of the selected canonical expression.
    #[must_use]
    pub fn into_expression(self) -> Expr {
        self.expression
    }

    /// Read the termination reason, independently of which candidate won.
    #[must_use]
    pub const fn termination(&self) -> SimplificationTermination {
        self.termination
    }

    /// Count completed sweeps; an empty sequence completes zero sweeps.
    #[must_use]
    pub const fn iterations(&self) -> usize {
        self.iterations
    }
}

impl Default for SimplificationSequence {
    fn default() -> Self {
        Self::compact()
    }
}

impl Debug for SimplificationSequence {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter
            .debug_struct("SimplificationSequence")
            .field("steps", &self.steps)
            .field("target", &self.target)
            .field("domain_safety", &self.domain_safety)
            .field("custom_score", &self.score.is_some())
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}
