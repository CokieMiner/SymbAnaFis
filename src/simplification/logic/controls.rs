//! Sequence bounds, compiled rule plans, structural pass gates, and domain guards.

use alloc::{sync::Arc, vec};

use crate::{
    collections::FixedSet,
    core::{BuiltinFunction, Expr, ExprView, StoredForm, Truth},
    errors::SimplifyError,
    rewrite::RuleSet,
};

use super::{DomainSafety, PreparedStep, SimplificationStep};

/// Reusable compiled rule storage, shared when a sequence is cloned.
#[derive(Clone)]
pub struct PreparedPlan {
    preserving: Option<Arc<RuleSet>>,
    /// The preserving rules followed by the assumption-dependent rules, when
    /// the step has any of the latter.
    assumed: Option<Arc<RuleSet>>,
}

/// Sweep, size, matching, expansion, and polynomial work bounds. `usize::MAX`
/// is unbounded.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    iterations: usize,
    nodes: usize,
    depth: usize,
    matches: usize,
    expansion: usize,
    polynomial: usize,
}

/// The structural capabilities of one expression.
///
/// A pass whose rule templates and direct identities cannot mention any
/// present capability is a complete no-op. One borrowed traversal of the
/// stored DAG gathers them: compact polynomials contribute their stored
/// bases, so no logical monomial is built, and scalar leaves are never queued.
#[derive(Clone, Copy)]
pub struct PassFeatures {
    bits: u8,
}

/// Namespace for domain-contract admission checks.
pub struct Domain;

impl PreparedPlan {
    /// Every builtin table compiles, which the tests check, so an error
    /// leaves the step without rules instead of failing sequence construction.
    pub fn new(step: &SimplificationStep) -> Self {
        let rules = PreparedStep::rules(step, DomainSafety::Preserving).unwrap_or_default();
        Self {
            preserving: (!rules.is_empty()).then(|| Arc::new(RuleSet::new(rules))),
            assumed: None,
        }
    }

    pub fn enable_assumptions(&mut self, step: &SimplificationStep) {
        if self.assumed.is_some() {
            return;
        }
        let extra =
            PreparedStep::rules(step, DomainSafety::AssumptionDependent).unwrap_or_default();
        if !extra.is_empty() {
            let mut rules = PreparedStep::rules(step, DomainSafety::Preserving).unwrap_or_default();
            rules.extend(extra);
            self.assumed = Some(Arc::new(RuleSet::new(rules)));
        }
    }

    /// The rules `safety` admits, or `None` when the step has none.
    pub fn rules(&self, safety: DomainSafety) -> Option<&RuleSet> {
        if safety == DomainSafety::AssumptionDependent {
            self.assumed.as_deref().or(self.preserving.as_deref())
        } else {
            self.preserving.as_deref()
        }
    }
}

impl Limits {
    /// Sixteen sweeps and 10,000 candidate attempts per core rewrite pass;
    /// stored size, expansion, and polynomial work are unbounded.
    pub const DEFAULT: Self = Self {
        iterations: 16,
        nodes: usize::MAX,
        depth: usize::MAX,
        matches: 10_000,
        expansion: usize::MAX,
        polynomial: usize::MAX,
    };

    pub const fn with_iterations(mut self, value: usize) -> Self {
        self.iterations = value;
        self
    }

    pub const fn with_nodes(mut self, value: usize) -> Self {
        self.nodes = value;
        self
    }

    pub const fn with_depth(mut self, value: usize) -> Self {
        self.depth = value;
        self
    }

    pub const fn with_matches(mut self, value: usize) -> Self {
        self.matches = value;
        self
    }

    pub const fn with_expansion(mut self, value: usize) -> Self {
        self.expansion = value;
        self
    }

    pub const fn with_polynomial(mut self, value: usize) -> Self {
        self.polynomial = value;
        self
    }

    pub const fn iterations(self) -> usize {
        self.iterations
    }

    pub const fn matches(self) -> usize {
        self.matches
    }

    pub const fn expansion(self) -> usize {
        self.expansion
    }

    pub const fn polynomial(self) -> usize {
        self.polynomial
    }

    /// Check the stored node count and depth of one candidate.
    pub fn validate(self, expression: &Expr) -> Result<(), SimplifyError> {
        // An unbounded limit cannot be exceeded, so its traversal is skipped.
        if self.nodes != usize::MAX && expression.node_count() > self.nodes {
            return Err(SimplifyError::NodeLimitExceeded { limit: self.nodes });
        }
        if self.depth != usize::MAX && expression.max_depth() > self.depth {
            return Err(SimplifyError::DepthLimitExceeded { limit: self.depth });
        }
        Ok(())
    }
}

impl PassFeatures {
    const SUM: u8 = 1 << 0;
    const TRIGONOMETRIC: u8 = 1 << 1;
    const HYPERBOLIC: u8 = 1 << 2;
    const EXPONENTIAL: u8 = 1 << 3;
    const ROOTS: u8 = 1 << 4;

    pub fn of(expression: &Expr) -> Self {
        let mut features = Self { bits: 0 };
        let mut pending = vec![expression];
        let mut visited = FixedSet::default();
        while let Some(node) = pending.pop() {
            if !visited.insert(node) {
                continue;
            }
            match node.view() {
                ExprView::Sum(_) => features.bits |= Self::SUM,
                ExprView::Function { name, .. } => {
                    if let Some(builtin) = BuiltinFunction::of(name) {
                        features.bits |= Self::classify(builtin);
                    }
                }
                ExprView::Number(_)
                | ExprView::ImaginaryUnit
                | ExprView::Pi
                | ExprView::Euler
                | ExprView::Symbol(_)
                | ExprView::Product(_)
                | ExprView::Div(..)
                | ExprView::Pow(..)
                | ExprView::Derivative { .. } => {}
            }
            // Construction records whether every compact base is a scalar leaf;
            // such storage holds only leaves and numeric coefficients.
            if node.simple_bases() == Some(true) {
                continue;
            }
            pending.extend(node.stored_children().filter(|child| {
                !matches!(
                    child.view(),
                    ExprView::Number(_)
                        | ExprView::ImaginaryUnit
                        | ExprView::Pi
                        | ExprView::Euler
                        | ExprView::Symbol(_)
                )
            }));
        }
        features
    }

    pub const fn allows(self, step: &SimplificationStep) -> bool {
        match step {
            SimplificationStep::Roots => self.bits & Self::ROOTS != 0,
            SimplificationStep::Exponential => self.bits & Self::EXPONENTIAL != 0,
            // Hyperbolic rules also recognize exponential representations.
            SimplificationStep::Hyperbolic => {
                self.bits & (Self::HYPERBOLIC | Self::EXPONENTIAL) != 0
            }
            SimplificationStep::Trigonometric => self.bits & Self::TRIGONOMETRIC != 0,
            // Expansion has no rules and rewrites only products and powers
            // with a sum operand.
            SimplificationStep::Factor | SimplificationStep::Expand => self.bits & Self::SUM != 0,
            SimplificationStep::Algebraic
            | SimplificationStep::Compilation
            | SimplificationStep::Rewrite(_)
            | SimplificationStep::Map(_) => true,
        }
    }

    const fn classify(function: BuiltinFunction) -> u8 {
        match function {
            BuiltinFunction::Sin
            | BuiltinFunction::Cos
            | BuiltinFunction::Tan
            | BuiltinFunction::Cot
            | BuiltinFunction::Sec
            | BuiltinFunction::Csc
            | BuiltinFunction::Asin
            | BuiltinFunction::Acos
            | BuiltinFunction::Atan
            | BuiltinFunction::Acot
            | BuiltinFunction::Asec
            | BuiltinFunction::Acsc
            | BuiltinFunction::Sinc => Self::TRIGONOMETRIC,
            BuiltinFunction::Sinh
            | BuiltinFunction::Cosh
            | BuiltinFunction::Tanh
            | BuiltinFunction::Coth
            | BuiltinFunction::Sech
            | BuiltinFunction::Csch
            | BuiltinFunction::Asinh
            | BuiltinFunction::Acosh
            | BuiltinFunction::Atanh
            | BuiltinFunction::Acoth
            | BuiltinFunction::Asech
            | BuiltinFunction::Acsch => Self::HYPERBOLIC,
            BuiltinFunction::Exp
            | BuiltinFunction::ExpM1
            | BuiltinFunction::ExpNeg
            | BuiltinFunction::Ln
            | BuiltinFunction::Ln1p
            | BuiltinFunction::Log
            | BuiltinFunction::Log10
            | BuiltinFunction::Log2 => Self::EXPONENTIAL,
            BuiltinFunction::Sqrt | BuiltinFunction::Cbrt => Self::ROOTS,
            BuiltinFunction::Floor
            | BuiltinFunction::Ceil
            | BuiltinFunction::Round
            | BuiltinFunction::Trunc
            | BuiltinFunction::Fract
            | BuiltinFunction::Abs
            | BuiltinFunction::Signum
            | BuiltinFunction::Erf
            | BuiltinFunction::Erfc
            | BuiltinFunction::Gamma
            | BuiltinFunction::Lgamma
            | BuiltinFunction::Digamma
            | BuiltinFunction::Trigamma
            | BuiltinFunction::Tetragamma
            | BuiltinFunction::Zeta
            | BuiltinFunction::EllipticK
            | BuiltinFunction::EllipticE
            | BuiltinFunction::Polygamma
            | BuiltinFunction::BesselJ
            | BuiltinFunction::BesselY
            | BuiltinFunction::BesselI
            | BuiltinFunction::BesselK
            | BuiltinFunction::ZetaDeriv
            | BuiltinFunction::Hermite
            | BuiltinFunction::LambertW
            | BuiltinFunction::Beta
            | BuiltinFunction::Atan2
            | BuiltinFunction::AssocLegendre => 0,
        }
    }
}

impl Domain {
    /// A proved fact is admitted and a refuted fact never is; an unknown fact
    /// is admitted only when the caller supplies the hypotheses.
    pub const fn admits(fact: Truth, safety: DomainSafety) -> bool {
        match fact {
            Truth::True => true,
            Truth::False => false,
            Truth::Unknown => matches!(safety, DomainSafety::AssumptionDependent),
        }
    }
}
