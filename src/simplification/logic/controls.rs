//! Sequence bounds, compiled rule plans, structural pass gates, and domain guards.

use alloc::{sync::Arc, vec};

use crate::{
    collections::FixedSet,
    core::{BuiltinFunction, Expr, ExprView, StoredForm, Truth},
    errors::SimplifyError,
    rewrite::RuleSet,
    sync::Mutex,
};

use super::{DomainSafety, PreparedStep, SimplificationStep};

/// The builtin steps whose rule tables are compiled once and shared.
const CACHED_STEPS: usize = 6;

/// One cache slot per cached step and domain contract.
const SLOTS: usize = CACHED_STEPS * 2;

/// Compiled builtin tables, indexed by step and contract.
///
/// A table is a pure function of its step and contract, so it is compiled once
/// and shared: a sequence construction reuses it instead of rebuilding several
/// hundred templates.
static TABLES: Mutex<[Cached; SLOTS]> = Mutex::new([const { Cached::Unset }; SLOTS]);

/// One cache slot: not yet compiled, or the table a step contributes.
#[derive(Clone)]
enum Cached {
    Unset,
    /// The compiled table, or `None` when the step contributes no distinct
    /// table for this slot.
    Compiled(Option<Arc<RuleSet>>),
}

/// Reusable compiled rule storage, shared when a sequence is cloned.
#[derive(Clone)]
pub struct PreparedPlan {
    preserving: Option<Arc<RuleSet>>,
    /// The complete assumption-dependent table, when the step has rules whose
    /// behavior differs from `preserving`.
    assumed: Option<Arc<RuleSet>>,
}

/// Sweep, size, matching, expansion, and polynomial work bounds. `usize::MAX`
/// is unbounded.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub iterations: usize,
    pub nodes: usize,
    pub depth: usize,
    pub matches: usize,
    pub expansion: usize,
    pub polynomial: usize,
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
    /// The step's preserving rule table.
    pub fn new(step: &SimplificationStep) -> Self {
        Self {
            preserving: Self::builtin_table(step, DomainSafety::Preserving),
            assumed: None,
        }
    }

    /// Add the assumption-dependent rules to a second table.
    pub fn enable_assumptions(&mut self, step: &SimplificationStep) {
        if self.assumed.is_none() {
            self.assumed = Self::builtin_table(step, DomainSafety::AssumptionDependent);
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

    /// The compiled table of one builtin step and contract, or `None` when the
    /// step contributes no distinct rule table for `safety`.
    ///
    /// Compilation runs outside the cache lock, so two threads may build the
    /// same table once; the result is identical either way.
    fn builtin_table(step: &SimplificationStep, safety: DomainSafety) -> Option<Arc<RuleSet>> {
        let slot = match (step, safety) {
            (SimplificationStep::Algebraic, DomainSafety::Preserving) => 0,
            (SimplificationStep::Algebraic, DomainSafety::AssumptionDependent) => 1,
            (SimplificationStep::Roots, DomainSafety::Preserving) => 2,
            (SimplificationStep::Roots, DomainSafety::AssumptionDependent) => 3,
            (SimplificationStep::Exponential, DomainSafety::Preserving) => 4,
            (SimplificationStep::Exponential, DomainSafety::AssumptionDependent) => 5,
            (SimplificationStep::Trigonometric, DomainSafety::Preserving) => 6,
            (SimplificationStep::Trigonometric, DomainSafety::AssumptionDependent) => 7,
            (SimplificationStep::Hyperbolic, DomainSafety::Preserving) => 8,
            (SimplificationStep::Hyperbolic, DomainSafety::AssumptionDependent) => 9,
            (SimplificationStep::Compilation, DomainSafety::Preserving) => 10,
            (SimplificationStep::Compilation, DomainSafety::AssumptionDependent) => 11,
            (
                SimplificationStep::Expand
                | SimplificationStep::Factor
                | SimplificationStep::Rewrite(_)
                | SimplificationStep::Map(_),
                _,
            ) => return None,
        };
        let cached = TABLES.lock().get(slot).cloned();
        if let Some(Cached::Compiled(table)) = cached {
            return table;
        }
        let rules = PreparedStep::rules(step, safety);
        let table = (!rules.is_empty()).then(|| Arc::new(RuleSet::new(rules)));
        if let Some(entry) = TABLES.lock().get_mut(slot) {
            *entry = Cached::Compiled(table.clone());
        }
        table
    }
}

impl Limits {
    /// Sixteen sweeps and 10,000 candidate attempts per match attempt beyond
    /// its linear operand scan; stored size, expansion, and polynomial work are unbounded.
    pub const DEFAULT: Self = Self {
        iterations: 16,
        nodes: usize::MAX,
        depth: usize::MAX,
        matches: 10_000,
        expansion: usize::MAX,
        polynomial: usize::MAX,
    };

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
        match expression.view() {
            ExprView::Number(_)
            | ExprView::ImaginaryUnit
            | ExprView::Pi
            | ExprView::Euler
            | ExprView::Symbol(_) => return Self { bits: 0 },
            ExprView::Sum(_) if expression.simple_bases() == Some(true) => {
                return Self { bits: Self::SUM };
            }
            ExprView::Sum(_)
            | ExprView::Product(_)
            | ExprView::Div(..)
            | ExprView::Pow(..)
            | ExprView::Function { .. }
            | ExprView::Derivative { .. } => {}
        }
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

    /// Whether admission of `step` depends on the expression's features.
    ///
    /// A sequence whose steps are all ungated never scans the expression, so
    /// the scan is skipped entirely rather than repeated per sweep.
    pub const fn is_gated(step: &SimplificationStep) -> bool {
        match step {
            SimplificationStep::Roots
            | SimplificationStep::Exponential
            | SimplificationStep::Hyperbolic
            | SimplificationStep::Trigonometric
            | SimplificationStep::Factor
            | SimplificationStep::Expand => true,
            SimplificationStep::Algebraic
            | SimplificationStep::Compilation
            | SimplificationStep::Rewrite(_)
            | SimplificationStep::Map(_) => false,
        }
    }

    pub const fn allows(self, step: &SimplificationStep) -> bool {
        if !Self::is_gated(step) {
            return true;
        }
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
            // `is_gated` admits exactly the steps above.
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
            | BuiltinFunction::Re
            | BuiltinFunction::Im
            | BuiltinFunction::Conj
            | BuiltinFunction::Heaviside
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
            | BuiltinFunction::Hypot
            | BuiltinFunction::Max
            | BuiltinFunction::Min
            | BuiltinFunction::Modulo
            | BuiltinFunction::Rem
            | BuiltinFunction::AssocLegendre
            | BuiltinFunction::Clamp => 0,
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
