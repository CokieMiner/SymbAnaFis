use crate::Expr;
use crate::core::BodyFn;
use crate::core::ExprKind as AstKind;
use rustc_hash::FxHashMap;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;

/// Macro to define a simplification rule with minimal boilerplate
///
/// Supports 4 forms:
/// - Basic: `rule!(Name, "name", priority, Category, &[ExprKind::...], |expr, ctx| { ... })`
/// - With targets: `rule!(Name, "name", priority, Category, &[ExprKind::...], targets: &["fn"], |expr, ctx| { ... })`
/// - With `alters_domain`: `rule!(Name, "name", priority, Category, &[ExprKind::...], alters_domain: true, |expr, ctx| { ... })`
/// - Both: `rule!(Name, "name", priority, Category, &[ExprKind::...], alters_domain: true, targets: &["fn"], |expr, ctx| { ... })`
macro_rules! rule {
    // Basic form
    ($name:ident, $rule_name:expr, $priority:expr, $category:ident, $applies_to:expr, $logic:expr) => {
        pub struct $name;
        impl Rule for $name {
            fn name(&self) -> &'static str {
                $rule_name
            }
            fn priority(&self) -> i32 {
                $priority
            }
            fn category(&self) -> RuleCategory {
                RuleCategory::$category
            }
            fn applies_to(&self) -> &'static [ExprKind] {
                $applies_to
            }
            fn apply(&self, expr: &Arc<Expr>, context: &RuleContext) -> Option<Arc<Expr>> {
                let _ = context;
                ($logic)(expr.as_ref(), context).map(Arc::new)
            }
        }
    };
    // With targets
    ($name:ident, $rule_name:expr, $priority:expr, $category:ident, $applies_to:expr, targets: $targets:expr, $logic:expr) => {
        pub struct $name;
        impl Rule for $name {
            fn name(&self) -> &'static str {
                $rule_name
            }
            fn priority(&self) -> i32 {
                $priority
            }
            fn category(&self) -> RuleCategory {
                RuleCategory::$category
            }
            fn applies_to(&self) -> &'static [ExprKind] {
                $applies_to
            }
            fn target_functions(&self) -> Vec<u64> {
                $targets.to_vec()
            }
            fn apply(&self, expr: &Arc<Expr>, context: &RuleContext) -> Option<Arc<Expr>> {
                let _ = context;
                ($logic)(expr.as_ref(), context).map(Arc::new)
            }
        }
    };
    // With alters_domain
    ($name:ident, $rule_name:expr, $priority:expr, $category:ident, $applies_to:expr, alters_domain: $alters:expr, $logic:expr) => {
        pub struct $name;
        impl Rule for $name {
            fn name(&self) -> &'static str {
                $rule_name
            }
            fn priority(&self) -> i32 {
                $priority
            }
            fn category(&self) -> RuleCategory {
                RuleCategory::$category
            }
            fn alters_domain(&self) -> bool {
                $alters
            }
            fn applies_to(&self) -> &'static [ExprKind] {
                $applies_to
            }
            fn apply(&self, expr: &Arc<Expr>, context: &RuleContext) -> Option<Arc<Expr>> {
                let _ = context;
                ($logic)(expr.as_ref(), context).map(Arc::new)
            }
        }
    };
    // With alters_domain AND targets
    ($name:ident, $rule_name:expr, $priority:expr, $category:ident, $applies_to:expr, alters_domain: $alters:expr, targets: $targets:expr, $logic:expr) => {
        pub struct $name;
        impl Rule for $name {
            fn name(&self) -> &'static str {
                $rule_name
            }
            fn priority(&self) -> i32 {
                $priority
            }
            fn category(&self) -> RuleCategory {
                RuleCategory::$category
            }
            fn alters_domain(&self) -> bool {
                $alters
            }
            fn applies_to(&self) -> &'static [ExprKind] {
                $applies_to
            }
            fn target_functions(&self) -> Vec<u64> {
                $targets.to_vec()
            }
            fn apply(&self, expr: &Arc<Expr>, context: &RuleContext) -> Option<Arc<Expr>> {
                let _ = context;
                ($logic)(expr.as_ref(), context).map(Arc::new)
            }
        }
    };
}

/// Macro to define a simplification rule that returns `Option<Arc<Expr>>` directly.
/// This avoids unnecessary wrapping when the result is already an Arc.
macro_rules! rule_arc {
    // Basic form
    ($name:ident, $rule_name:expr, $priority:expr, $category:ident, $applies_to:expr, $logic:expr) => {
        pub struct $name;
        impl Rule for $name {
            fn name(&self) -> &'static str {
                $rule_name
            }
            fn priority(&self) -> i32 {
                $priority
            }
            fn category(&self) -> RuleCategory {
                RuleCategory::$category
            }
            fn applies_to(&self) -> &'static [ExprKind] {
                $applies_to
            }
            fn apply(&self, expr: &Arc<Expr>, context: &RuleContext) -> Option<Arc<Expr>> {
                let _ = context;
                ($logic)(expr.as_ref(), context)
            }
        }
    };
    // With targets
    ($name:ident, $rule_name:expr, $priority:expr, $category:ident, $applies_to:expr, targets: $targets:expr, $logic:expr) => {
        pub struct $name;
        impl Rule for $name {
            fn name(&self) -> &'static str {
                $rule_name
            }
            fn priority(&self) -> i32 {
                $priority
            }
            fn category(&self) -> RuleCategory {
                RuleCategory::$category
            }
            fn applies_to(&self) -> &'static [ExprKind] {
                $applies_to
            }
            fn target_functions(&self) -> Vec<u64> {
                $targets.to_vec()
            }
            fn apply(&self, expr: &Arc<Expr>, context: &RuleContext) -> Option<Arc<Expr>> {
                let _ = context;
                ($logic)(expr.as_ref(), context)
            }
        }
    };
    // With alters_domain
    ($name:ident, $rule_name:expr, $priority:expr, $category:ident, $applies_to:expr, alters_domain: $alters:expr, $logic:expr) => {
        pub struct $name;
        impl Rule for $name {
            fn name(&self) -> &'static str {
                $rule_name
            }
            fn priority(&self) -> i32 {
                $priority
            }
            fn category(&self) -> RuleCategory {
                RuleCategory::$category
            }
            fn alters_domain(&self) -> bool {
                $alters
            }
            fn applies_to(&self) -> &'static [ExprKind] {
                $applies_to
            }
            fn apply(&self, expr: &Arc<Expr>, context: &RuleContext) -> Option<Arc<Expr>> {
                let _ = context;
                ($logic)(expr.as_ref(), context)
            }
        }
    };
    // With alters_domain AND targets
    ($name:ident, $rule_name:expr, $priority:expr, $category:ident, $applies_to:expr, alters_domain: $alters:expr, targets: $targets:expr, $logic:expr) => {
        pub struct $name;
        impl Rule for $name {
            fn name(&self) -> &'static str {
                $rule_name
            }
            fn priority(&self) -> i32 {
                $priority
            }
            fn category(&self) -> RuleCategory {
                RuleCategory::$category
            }
            fn alters_domain(&self) -> bool {
                $alters
            }
            fn applies_to(&self) -> &'static [ExprKind] {
                $applies_to
            }
            fn target_functions(&self) -> Vec<u64> {
                $targets.to_vec()
            }
            fn apply(&self, expr: &Arc<Expr>, context: &RuleContext) -> Option<Arc<Expr>> {
                let _ = context;
                ($logic)(expr.as_ref(), context)
            }
        }
    };
}

/// Macro for rules with helpers that return `Option<Arc<Expr>>` directly
/// This avoids unnecessary wrapping when the result is already an Arc.
macro_rules! rule_with_helpers_arc {
    // Basic form
    ($name:ident, $rule_name:expr, $priority:expr, $category:ident, $applies_to:expr, helpers: { $($helper:item)* }, $logic:expr) => {
        pub struct $name;
        impl Rule for $name {
            fn name(&self) -> &'static str { $rule_name }
            fn priority(&self) -> i32 { $priority }
            fn category(&self) -> RuleCategory { RuleCategory::$category }
            fn applies_to(&self) -> &'static [ExprKind] { $applies_to }
            fn apply(&self, expr: &Arc<Expr>, context: &RuleContext) -> Option<Arc<Expr>> {
                $($helper)*
                let _ = context;
                ($logic)(expr.as_ref(), context)
            }
        }
    };
    // With alters_domain
    ($name:ident, $rule_name:expr, $priority:expr, $category:ident, $applies_to:expr, alters_domain: $alters:expr, helpers: { $($helper:item)* }, $logic:expr) => {
        pub struct $name;
        impl Rule for $name {
            fn name(&self) -> &'static str { $rule_name }
            fn priority(&self) -> i32 { $priority }
            fn category(&self) -> RuleCategory { RuleCategory::$category }
            fn alters_domain(&self) -> bool { $alters }
            fn applies_to(&self) -> &'static [ExprKind] { $applies_to }
            fn apply(&self, expr: &Arc<Expr>, context: &RuleContext) -> Option<Arc<Expr>> {
                $($helper)*
                let _ = context;
                ($logic)(expr.as_ref(), context)
            }
        }
    };
}

/// Macro for rules with helpers that wrap result in Arc
macro_rules! rule_with_helpers {
    // Basic form
    ($name:ident, $rule_name:expr, $priority:expr, $category:ident, $applies_to:expr, helpers: { $($helper:item)* }, $logic:expr) => {
        pub struct $name;
        impl Rule for $name {
            fn name(&self) -> &'static str { $rule_name }
            fn priority(&self) -> i32 { $priority }
            fn category(&self) -> RuleCategory { RuleCategory::$category }
            fn applies_to(&self) -> &'static [ExprKind] { $applies_to }
            fn apply(&self, expr: &Arc<Expr>, context: &RuleContext) -> Option<Arc<Expr>> {
                let _ = context;
                $($helper)*
                ($logic)(expr.as_ref(), context).map(Arc::new)
            }
        }
    };
    // With alters_domain
    ($name:ident, $rule_name:expr, $priority:expr, $category:ident, $applies_to:expr, alters_domain: $alters:expr, helpers: { $($helper:item)* }, $logic:expr) => {
        pub struct $name;
        impl Rule for $name {
            fn name(&self) -> &'static str { $rule_name }
            fn priority(&self) -> i32 { $priority }
            fn category(&self) -> RuleCategory { RuleCategory::$category }
            fn alters_domain(&self) -> bool { $alters }
            fn applies_to(&self) -> &'static [ExprKind] { $applies_to }
            fn apply(&self, expr: &Arc<Expr>, context: &RuleContext) -> Option<Arc<Expr>> {
                let _ = context;
                $($helper)*
                ($logic)(expr.as_ref(), context).map(Arc::new)
            }
        }
    };
}

/// Expression kind for fast rule filtering
/// Rules declare which expression kinds they can apply to
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ExprKind {
    /// Numeric literal
    Number,
    /// Symbolic variable
    Symbol,
    /// N-ary addition
    Sum,
    /// N-ary multiplication
    Product,
    /// Division
    Div,
    /// Power/exponentiation
    Pow,
    /// Any function call
    Function,
    /// Partial derivative expression
    Derivative,
    /// Polynomial (don't trigger Sum rules)
    Poly,
}

impl ExprKind {
    /// Get the kind of an expression (cheap O(1) operation)
    #[inline]
    pub const fn of(expr: &Expr) -> Self {
        match &expr.kind {
            AstKind::Number(_) => Self::Number,
            AstKind::Symbol(_) => Self::Symbol,
            AstKind::Sum(_) => Self::Sum,
            AstKind::Product(_) => Self::Product,
            AstKind::Div(_, _) => Self::Div,
            AstKind::Pow(_, _) => Self::Pow,
            AstKind::FunctionCall { .. } => Self::Function,
            AstKind::Derivative { .. } => Self::Derivative,
            AstKind::Poly(_) => Self::Poly, // Poly has its own rules, don't trigger Sum rules
        }
    }
}

/// Core trait for all simplification rules
pub trait Rule {
    /// Returns the unique name of this rule
    fn name(&self) -> &'static str;
    /// Returns the priority of this rule (higher = applied first)
    fn priority(&self) -> i32;
    #[allow(
        dead_code,
        reason = "Legacy for categorizzation maybe useful in the future"
    )]
    /// Returns the category of this rule
    fn category(&self) -> RuleCategory;

    /// Returns whether this rule alters the domain of the expression (e.g., by removing singularities)
    fn alters_domain(&self) -> bool {
        false
    }

    /// Which expression kinds this rule can apply to.
    /// Rules will ONLY be checked against expressions matching these kinds.
    /// Default: all kinds (for backwards compatibility during migration)
    fn applies_to(&self) -> &'static [ExprKind] {
        ALL_EXPR_KINDS
    }

    /// Optimized Dispatch: List of function names this rule targets.
    /// If non-empty, the rule is ONLY checked for `FunctionCall` nodes with these names.
    /// If empty, it is checked for ALL `FunctionCall` nodes (generic rules).
    fn target_functions(&self) -> Vec<u64> {
        Vec::new()
    }

    /// Cheap structural pre-check called on cache-miss, before `apply()`.
    /// Return `false` to skip this rule without caching the result.
    /// Implement this for rules whose `apply()` has an O(1) early-return condition —
    /// avoids building the call frame and cache insertion for non-matching expressions.
    /// Default: always run (no pre-check).
    fn can_apply(&self, _expr: &Arc<Expr>) -> bool {
        true
    }

    /// Apply this rule to an expression. Returns `Some(new_expr)` if transformation applied.
    /// Takes &`Arc<Expr>` for efficient sub-expression cloning (`Arc::clone` is cheap).
    fn apply(&self, expr: &Arc<Expr>, context: &RuleContext) -> Option<Arc<Expr>>;
}

/// Categories of simplification rules
#[allow(
    dead_code,
    reason = "Legacy for categorizzation maybe useful in the future"
)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum RuleCategory {
    /// Constant folding, identities
    Numeric,
    /// General algebraic rules
    Algebraic,
    /// Trigonometric identities and simplifications
    Trigonometric,
    /// Hyperbolic function rules
    Hyperbolic,
    /// Exponential and logarithmic rules
    Exponential,
    /// Root and radical simplifications
    Root,
}

/// All expression kinds - used as default for rules
pub const ALL_EXPR_KINDS: &[ExprKind] = &[
    ExprKind::Number,
    ExprKind::Symbol,
    ExprKind::Sum,
    ExprKind::Product,
    ExprKind::Div,
    ExprKind::Pow,
    ExprKind::Function,
    ExprKind::Derivative,
    ExprKind::Poly,
];

/// Context passed to rules during application
/// Uses `Arc<HashSet>` for cheap cloning (context is cloned per-node)
#[derive(Clone, Default)]
pub struct RuleContext {
    /// Current recursion depth in the expression tree
    pub depth: usize,
    /// Whether to apply only domain-safe transformations
    pub domain_safe: bool,
    /// Custom function body definitions
    pub custom_bodies: Arc<FxHashMap<u64, BodyFn>>,
}

impl Debug for RuleContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("RuleContext")
            .field("depth", &self.depth)
            .field("domain_safe", &self.domain_safe)
            .field(
                "custom_bodies",
                &format!("<{} functions>", self.custom_bodies.len()),
            )
            .finish()
    }
}

impl RuleContext {
    /// Set depth by mutable reference (avoids clone)
    #[inline]
    pub const fn set_depth(&mut self, depth: usize) {
        self.depth = depth;
    }

    /// Sets the custom function bodies for this context.
    pub fn with_custom_bodies(mut self, custom_bodies: FxHashMap<u64, BodyFn>) -> Self {
        self.custom_bodies = Arc::new(custom_bodies);
        self
    }
}
