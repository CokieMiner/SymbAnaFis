//! Rule infrastructure for the simplification engine
//!
//! Provides macros (`rule!`, `rule_with_helpers!`), traits (`Rule`), and registry
//! for defining and organizing simplification rules by category and priority.

use crate::Expr;
use crate::ExprKind as AstKind;
use crate::core::unified_context::BodyFn;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Macro to define a simplification rule with minimal boilerplate
///
/// Supports 4 forms:
/// - Basic: `rule!(Name, "name", priority, Category, &[ExprKind::...], |expr, ctx| { ... })`
/// - With targets: `rule!(Name, "name", priority, Category, &[ExprKind::...], targets: &["fn"], |expr, ctx| { ... })`
/// - With `alters_domain`: `rule!(Name, "name", priority, Category, &[ExprKind::...], alters_domain: true, |expr, ctx| { ... })`
/// - Both: `rule!(Name, "name", priority, Category, &[ExprKind::...], alters_domain: true, targets: &["fn"], |expr, ctx| { ... })`
#[macro_export]
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
            fn apply(
                &self,
                expr: &std::sync::Arc<Expr>,
                context: &RuleContext,
            ) -> Option<std::sync::Arc<Expr>> {
                let _ = context;
                ($logic)(expr.as_ref(), context).map(std::sync::Arc::new)
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
            fn apply(
                &self,
                expr: &std::sync::Arc<Expr>,
                context: &RuleContext,
            ) -> Option<std::sync::Arc<Expr>> {
                let _ = context;
                ($logic)(expr.as_ref(), context).map(std::sync::Arc::new)
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
            fn apply(
                &self,
                expr: &std::sync::Arc<Expr>,
                context: &RuleContext,
            ) -> Option<std::sync::Arc<Expr>> {
                let _ = context;
                ($logic)(expr.as_ref(), context).map(std::sync::Arc::new)
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
            fn apply(
                &self,
                expr: &std::sync::Arc<Expr>,
                context: &RuleContext,
            ) -> Option<std::sync::Arc<Expr>> {
                let _ = context;
                ($logic)(expr.as_ref(), context).map(std::sync::Arc::new)
            }
        }
    };
}

/// Macro to define a simplification rule that returns `Option<Arc<Expr>>` directly.
/// This avoids unnecessary wrapping when the result is already an Arc.
#[macro_export]
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
            fn apply(
                &self,
                expr: &std::sync::Arc<Expr>,
                context: &RuleContext,
            ) -> Option<std::sync::Arc<Expr>> {
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
            fn apply(
                &self,
                expr: &std::sync::Arc<Expr>,
                context: &RuleContext,
            ) -> Option<std::sync::Arc<Expr>> {
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
            fn apply(
                &self,
                expr: &std::sync::Arc<Expr>,
                context: &RuleContext,
            ) -> Option<std::sync::Arc<Expr>> {
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
            fn apply(
                &self,
                expr: &std::sync::Arc<Expr>,
                context: &RuleContext,
            ) -> Option<std::sync::Arc<Expr>> {
                let _ = context;
                ($logic)(expr.as_ref(), context)
            }
        }
    };
}

/// Macro for rules with helpers that return `Option<Arc<Expr>>` directly
#[macro_export]
macro_rules! rule_with_helpers_arc {
    // Basic form
    ($name:ident, $rule_name:expr, $priority:expr, $category:ident, $applies_to:expr, helpers: { $($helper:item)* }, $logic:expr) => {
        pub struct $name;
        impl Rule for $name {
            fn name(&self) -> &'static str { $rule_name }
            fn priority(&self) -> i32 { $priority }
            fn category(&self) -> RuleCategory { RuleCategory::$category }
            fn applies_to(&self) -> &'static [ExprKind] { $applies_to }
            fn apply(&self, expr: &std::sync::Arc<Expr>, context: &RuleContext) -> Option<std::sync::Arc<Expr>> {
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
            fn apply(&self, expr: &std::sync::Arc<Expr>, context: &RuleContext) -> Option<std::sync::Arc<Expr>> {
                $($helper)*
                let _ = context;
                ($logic)(expr.as_ref(), context)
            }
        }
    };
}

/// Macro for rules with helpers that wrap result in Arc
#[macro_export]
macro_rules! rule_with_helpers {
    // Basic form
    ($name:ident, $rule_name:expr, $priority:expr, $category:ident, $applies_to:expr, helpers: { $($helper:item)* }, $logic:expr) => {
        pub struct $name;
        impl Rule for $name {
            fn name(&self) -> &'static str { $rule_name }
            fn priority(&self) -> i32 { $priority }
            fn category(&self) -> RuleCategory { RuleCategory::$category }
            fn applies_to(&self) -> &'static [ExprKind] { $applies_to }
            fn apply(&self, expr: &std::sync::Arc<Expr>, context: &RuleContext) -> Option<std::sync::Arc<Expr>> {
                let _ = context;
                $($helper)*
                ($logic)(expr.as_ref(), context).map(std::sync::Arc::new)
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
            fn apply(&self, expr: &std::sync::Arc<Expr>, context: &RuleContext) -> Option<std::sync::Arc<Expr>> {
                let _ = context;
                $($helper)*
                ($logic)(expr.as_ref(), context).map(std::sync::Arc::new)
            }
        }
    };
}

/// Expression kind for fast rule filtering
/// Rules declare which expression kinds they can apply to
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ExprKind {
    Number,
    Symbol,
    Sum,     // N-ary addition
    Product, // N-ary multiplication
    Div,
    Pow,
    Function,   // Any function call
    Derivative, // Partial derivative expression
    Poly,       // Polynomial (don't trigger Sum rules)
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
    fn name(&self) -> &'static str;
    fn priority(&self) -> i32;
    #[allow(dead_code)]
    fn category(&self) -> RuleCategory;

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

    /// Apply this rule to an expression. Returns `Some(new_expr)` if transformation applied.
    /// Takes &`Arc<Expr>` for efficient sub-expression cloning (`Arc::clone` is cheap).
    fn apply(&self, expr: &Arc<Expr>, context: &RuleContext) -> Option<Arc<Expr>>;
}

/// Categories of simplification rules
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum RuleCategory {
    Numeric,   // Constant folding, identities
    Algebraic, // General algebraic rules
    Trigonometric,
    Hyperbolic,
    Exponential,
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

/// Priority ranges for different types of operations:
/// - 85-95: Expansion rules (distribute, expand powers, flatten nested structures)
/// - 70-84: Identity/Cancellation rules (x/x=1, x-x=0, x^a/x^b=x^(a-b), etc.)
/// - 40-69: Compression/Consolidation rules (combine terms, factor, compact a^n/b^n → (a/b)^n)
/// - 1-39: Canonicalization rules (sort terms, normalize display form)
///
/// Context passed to rules during application
/// Uses `Arc<HashSet>` for cheap cloning (context is cloned per-node)
#[derive(Clone, Default)]
pub struct RuleContext {
    pub depth: usize,
    pub variables: Arc<HashSet<String>>,
    pub known_symbols: Arc<HashSet<String>>, // User-specified known symbols (parsing hints)
    pub domain_safe: bool,
    pub custom_bodies: Arc<HashMap<String, BodyFn>>, // Custom function body definitions
}

impl std::fmt::Debug for RuleContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuleContext")
            .field("depth", &self.depth)
            .field("variables", &self.variables)
            .field("known_symbols", &self.known_symbols)
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

    pub fn with_variables(mut self, variables: HashSet<String>) -> Self {
        self.variables = Arc::new(variables);
        self
    }

    pub fn with_known_symbols(mut self, known_symbols: HashSet<String>) -> Self {
        self.known_symbols = Arc::new(known_symbols);
        self
    }

    pub fn with_custom_bodies(mut self, custom_bodies: HashMap<String, BodyFn>) -> Self {
        self.custom_bodies = Arc::new(custom_bodies);
        self
    }
}

/// Numeric simplification rules
pub mod numeric;

/// Algebraic simplification rules
pub mod algebraic;

/// Trigonometric simplification rules
pub mod trigonometric;

/// Exponential and logarithmic simplification rules
pub mod exponential;

/// Root simplification rules
pub mod root;

/// Hyperbolic function simplification rules
pub mod hyperbolic;

/// Rule Registry for dynamic loading and dependency management
pub struct RuleRegistry {
    pub(crate) rules: Vec<Arc<dyn Rule + Send + Sync>>,
    /// Rules indexed by expression kind for fast lookup
    rules_by_kind: HashMap<ExprKind, Vec<Arc<dyn Rule + Send + Sync>>>,
    /// Rules indexed by function name ID (u64) for O(1) dispatch
    /// Maps function symbol ID -> List of rules that target it SPECIFICALLY
    rules_by_func: HashMap<u64, Vec<Arc<dyn Rule + Send + Sync>>>,
    /// Generic function rules that must run for ALL functions
    generic_func_rules: Vec<Arc<dyn Rule + Send + Sync>>,
}

impl RuleRegistry {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            rules_by_kind: HashMap::new(),
            rules_by_func: HashMap::new(),
            generic_func_rules: Vec::new(),
        }
    }

    pub fn load_all_rules(&mut self) {
        // Load rules from each category
        self.rules.extend(numeric::get_numeric_rules());
        self.rules.extend(algebraic::get_algebraic_rules());
        self.rules.extend(trigonometric::get_trigonometric_rules());
        self.rules.extend(exponential::get_exponential_rules());
        self.rules.extend(root::get_root_rules());
        self.rules.extend(hyperbolic::get_hyperbolic_rules());
        // Note: Rules are sorted by priority in order_by_dependencies()
    }

    /// Build the kind index after ordering rules
    pub fn order_by_dependencies(&mut self) {
        // Sort by priority descending (higher priority runs first)
        // Rules are processed by ExprKind separately, so category order doesn't matter
        self.rules.sort_by_key(|r| std::cmp::Reverse(r.priority()));

        self.build_kind_index();
    }

    /// Build the index of rules by expression kind
    fn build_kind_index(&mut self) {
        self.rules_by_kind.clear();

        // Initialize all kinds
        for &kind in ALL_EXPR_KINDS {
            self.rules_by_kind.insert(kind, Vec::new());
        }

        // Index each rule by the kinds it applies to
        for rule in &self.rules {
            for &kind in rule.applies_to() {
                if let Some(rules) = self.rules_by_kind.get_mut(&kind) {
                    rules.push(Arc::clone(rule));
                }

                // Special indexing for Function kind
                if kind == ExprKind::Function {
                    let targets = rule.target_functions();
                    if targets.is_empty() {
                        self.generic_func_rules.push(Arc::clone(rule));
                    } else {
                        for fid in targets {
                            self.rules_by_func
                                .entry(fid)
                                .or_default()
                                .push(Arc::clone(rule));
                        }
                    }
                }
            }
        }

        // Ensure generic rules are also sorted? They are processed in insertion order which is priority sorted.
    }

    /// Get only rules that apply to a specific expression kind
    #[inline]
    pub fn get_rules_for_kind(&self, kind: ExprKind) -> &[Arc<dyn Rule + Send + Sync>] {
        self.rules_by_kind
            .get(&kind)
            .map_or(&[], std::vec::Vec::as_slice)
    }

    #[inline]
    pub fn get_specific_func_rules(&self, func_id: u64) -> &[Arc<dyn Rule + Send + Sync>] {
        self.rules_by_func
            .get(&func_id)
            .map_or(&[], std::vec::Vec::as_slice)
    }

    #[inline]
    pub fn get_generic_func_rules(&self) -> &[Arc<dyn Rule + Send + Sync>] {
        &self.generic_func_rules
    }
}
