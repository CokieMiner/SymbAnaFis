//! Composable simplification passes, presets, and execution outcomes.

use core::fmt::{Debug, Formatter, Result as FmtResult};

use alloc::{boxed::Box, sync::Arc, vec, vec::Vec};

use crate::{
    collections::FixedSet,
    core::{Expr, Mutex},
    errors::SimplifyError,
    rewrite::RewriteRule,
};

use super::{CandidateCost, Limits, PassFeatures, PreparedPlan, PreparedStep};

type NodeTransform = dyn Fn(&Expr) -> Option<Expr> + Send + Sync;

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
/// let x = Expr::symbol("sequence_x")?;
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

/// One complete DAG pass in an ordered simplification sequence.
///
/// Builtin passes preserve domains by default. [`DomainSafety`] controls
/// whether additional caller-assumed domain and branch hypotheses are permitted.
/// User rules and callbacks retain the core's caller-owned equivalence contract.
#[derive(Clone)]
#[non_exhaustive]
pub enum SimplificationStep {
    /// Guarded scalar and exact polynomial cancellation, signs, and fractions.
    Algebraic,
    /// Pythagorean, angle, parity, period, inverse, and quotient identities.
    Trigonometric,
    /// Hyperbolic identities, angle formulas, and exponential representations.
    Hyperbolic,
    /// Principal logarithm and exponential identities with conservative guards.
    Exponential,
    /// Principal square roots and real cube roots on proved domains.
    Roots,
    /// Distribute products and positive exact integer powers over sums.
    Expand,
    /// Extract common factors and recognize squares, cubes, and differences of squares.
    Factor,
    /// Select native numerical intrinsics and common factors for compilation.
    Compilation,
    /// Apply an existing core rewrite once; use capture predicates for guards.
    Rewrite(RewriteRule),
    /// Apply a pure callback bottom-up to each distinct logical input node.
    /// Return `None` to retain the rebuilt node. Inserted nodes are not revisited.
    /// Callback panics propagate; equal input nodes must produce equal results.
    Map(Arc<NodeTransform>),
}

/// Which builtin rule contracts a sequence may use.
///
/// Custom rewrite rules and mapping callbacks retain their caller-owned
/// correctness contract in both modes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum DomainSafety {
    /// Preserve poles and principal branches using conservative input proofs.
    #[default]
    Preserving,
    /// Also use identities whose domain and branch conditions the caller assumes.
    ///
    /// These rules may remove zero exclusions or require real inputs, positive
    /// bases, or a principal inverse-function range. Selecting this mode asserts
    /// those rule-specific hypotheses; it does not establish them.
    AssumptionDependent,
}

/// The desired form among candidates visited by a sequence.
///
/// Presets pair a target with suitable passes; selecting a target alone does
/// not add passes or perform a global search over equivalent expressions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum SimplificationTarget {
    /// Minimize logical expression size, counting repeated edges.
    #[default]
    Compact,
    /// First minimize distributable products and positive integer powers of sums.
    Expanded,
    /// First minimize composite polynomial terms under sums, then logical size.
    Factored,
    /// Minimize estimated native operation cost over distinct logical DAG nodes.
    Compilation,
}

/// Why execution stopped. The returned expression is the best visited candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SimplificationTermination {
    /// Every step in the final sweep left its input unchanged, or the sequence is empty.
    FixedPoint,
    /// A complete sweep returned to a previously visited sweep boundary.
    Cycle,
    /// The configured number of complete sweeps was consumed.
    IterationLimit,
}

impl SimplificationSequence {
    /// Create an empty sequence with the compact target and default limits.
    /// Canonical construction has already folded and collected basic arithmetic.
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
        Self::identities(SimplificationStep::Factor)
    }

    /// Build the identity preset followed by bounded polynomial expansion.
    #[must_use]
    pub fn expanded() -> Self {
        Self::identities(SimplificationStep::Expand).target(SimplificationTarget::Expanded)
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
    /// let x = Expr::symbol("compilation_example_x")?;
    /// let result = SimplificationSequence::compilation().simplify(&(x.exp() - 1))?;
    /// assert_eq!(result.into_expression(), x.exp_m1());
    /// # Ok::<(), Box<dyn core::error::Error>>(())
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

    /// Bound stored nodes in the input, intermediate stage outputs, and completed candidates (default unlimited, `usize::MAX`).
    #[must_use]
    pub const fn max_nodes(mut self, limit: usize) -> Self {
        self.limits = self.limits.with_nodes(limit);
        self
    }

    /// Bound stored depth in the input, intermediate stage outputs, and completed candidates (default unlimited, `usize::MAX`).
    #[must_use]
    pub const fn max_depth(mut self, limit: usize) -> Self {
        self.limits = self.limits.with_depth(limit);
        self
    }

    /// Bound candidate attempts per core rewrite pass (default 10,000;
    /// `usize::MAX` is unbounded). Commutative matching can try exponentially
    /// many assignments without producing a match, so the default is finite. A
    /// builtin rule that exhausts the bound leaves its pass unchanged; a
    /// [`SimplificationStep::Rewrite`] step returns the error.
    #[must_use]
    pub const fn match_limit(mut self, limit: usize) -> Self {
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
    /// Propagates panics of mapping, scoring, and rule callbacks.
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
        let mut best_score = None;
        let mut current = expression.clone();
        let mut seen = FixedSet::default();
        for _ in 0..self.limits.iterations() {
            let mut changed = false;
            let mut features = PassFeatures::of(&current);
            for (step, plan) in self.steps.iter().zip(&self.plans) {
                if !features.allows(step) {
                    continue;
                }
                let next = PreparedStep::new(step, plan, self.domain_safety)
                    .apply(&current, self.limits)?;
                if next == current {
                    continue;
                }
                changed = true;
                let baseline = *best_score.get_or_insert_with(|| {
                    CandidateCost::new(expression, self.target, self.score.as_deref())
                });
                let candidate_score = CandidateCost::new(&next, self.target, self.score.as_deref());
                if candidate_score <= baseline {
                    best_score = Some(candidate_score);
                    result.expression = next.clone();
                }
                current = next;
                features = PassFeatures::of(&current);
            }
            // At most limits.iterations complete sweeps, so this sum fits usize.
            result.iterations = result.iterations.saturating_add(1);
            if !changed {
                result.termination = SimplificationTermination::FixedPoint;
                break;
            }
            if seen.is_empty() {
                seen.insert(expression.clone());
            }
            if !seen.insert(current.clone()) {
                result.termination = SimplificationTermination::Cycle;
                break;
            }
        }
        Ok(result)
    }

    /// The identity steps shared by the presets, followed by `last`.
    fn identities(last: SimplificationStep) -> Self {
        let steps = vec![
            SimplificationStep::Algebraic,
            SimplificationStep::Roots,
            SimplificationStep::Exponential,
            SimplificationStep::Trigonometric,
            SimplificationStep::Hyperbolic,
            last,
        ];
        let plans = steps.iter().map(PreparedPlan::new).collect();
        Self {
            steps,
            plans,
            ..Self::new()
        }
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

impl From<RewriteRule> for SimplificationStep {
    fn from(rule: RewriteRule) -> Self {
        Self::Rewrite(rule)
    }
}

impl Debug for SimplificationStep {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Algebraic => formatter.write_str("Algebraic"),
            Self::Trigonometric => formatter.write_str("Trigonometric"),
            Self::Hyperbolic => formatter.write_str("Hyperbolic"),
            Self::Exponential => formatter.write_str("Exponential"),
            Self::Roots => formatter.write_str("Roots"),
            Self::Expand => formatter.write_str("Expand"),
            Self::Factor => formatter.write_str("Factor"),
            Self::Compilation => formatter.write_str("Compilation"),
            Self::Rewrite(rule) => formatter.debug_tuple("Rewrite").field(rule).finish(),
            Self::Map(_) => formatter.write_str("Map(<callback>)"),
        }
    }
}

impl Expr {
    /// Simplify this expression to minimize logical size using scalar identities and factoring.
    ///
    /// The preset's rules compile on the first call; later calls reuse them.
    ///
    /// # Errors
    /// Returns [`SimplifyError`] if simplification limits are exceeded.
    ///
    /// ```
    /// use symb_anafis::Expr;
    /// let x = Expr::symbol("compact_x").expect("symbol");
    /// let expr = x.pow(2) - x.pow(2);
    /// assert_eq!(expr.compact()?, Expr::number(0));
    /// # Ok::<(), symb_anafis::SimplifyError>(())
    /// ```
    pub fn compact(&self) -> Result<Self, SimplifyError> {
        // The compiled preset is leaked once, like global symbol metadata, so
        // the lock is held only to read or publish its address.
        static COMPACT: Mutex<Option<&'static SimplificationSequence>> = Mutex::new(None);
        let sequence = *COMPACT
            .lock()
            .get_or_insert_with(|| Box::leak(Box::new(SimplificationSequence::compact())));
        sequence.simplify(self).map(Simplification::into_expression)
    }

    /// Expand products and integer powers in this expression into distributed sums.
    ///
    /// The preset's rules compile on the first call; later calls reuse them.
    ///
    /// # Errors
    /// Returns [`SimplifyError`] if simplification limits are exceeded.
    ///
    /// ```
    /// use symb_anafis::Context;
    /// let ctx = Context::new()?;
    /// let x = ctx.symb("expand_x")?;
    /// let expr = (x + 1).pow(2);
    /// assert_eq!(expr.expand()?, x.pow(2) + 2 * x + 1);
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn expand(&self) -> Result<Self, SimplifyError> {
        static EXPANDED: Mutex<Option<&'static SimplificationSequence>> = Mutex::new(None);
        let sequence = *EXPANDED
            .lock()
            .get_or_insert_with(|| Box::leak(Box::new(SimplificationSequence::expanded())));
        sequence.simplify(self).map(Simplification::into_expression)
    }

    /// Factor common polynomial powers and subexpressions.
    ///
    /// The preset's rules compile on the first call; later calls reuse them.
    ///
    /// # Errors
    /// Returns [`SimplifyError`] if simplification limits are exceeded.
    ///
    /// ```
    /// use symb_anafis::Context;
    /// let ctx = Context::new()?;
    /// let x = ctx.symb("factor_x")?;
    /// let y = ctx.symb("factor_y")?;
    /// let expr = x.pow(2) + x * y;
    /// assert_eq!(expr.factor()?, x * (x + y));
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn factor(&self) -> Result<Self, SimplifyError> {
        static FACTORED: Mutex<Option<&'static SimplificationSequence>> = Mutex::new(None);
        let sequence = *FACTORED
            .lock()
            .get_or_insert_with(|| Box::leak(Box::new(SimplificationSequence::factored())));
        sequence.simplify(self).map(Simplification::into_expression)
    }
}
