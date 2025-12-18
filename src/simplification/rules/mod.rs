//! Rule infrastructure for the simplification engine
//!
//! Provides macros (`rule!`, `rule_with_helpers!`), traits (`Rule`), and registry
//! for defining and organizing simplification rules by category and priority.

use crate::Expr;
use crate::ExprKind as AstKind;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Macro to define a simplification rule with minimal boilerplate
///
/// Basic usage:
/// ```ignore
/// rule!(RuleName, "rule_name", priority, Category, &[ExprKind::Type], |expr, context| { logic })
/// ```
///
/// With alters_domain:
/// ```ignore
/// rule!(RuleName, "rule_name", priority, Category, &[ExprKind::Type], alters_domain: true, |expr, context| { logic })
/// ```
///
/// With helper functions (use rule_with_helpers! instead for complex cases)
#[macro_export]
macro_rules! rule {
    // Basic form without alters_domain
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
                // Suppress unused variable warning when context isn't used
                let _ = context;
                // Adapter: closure works with &Expr, we wrap result in Arc
                ($logic)(expr.as_ref(), context).map(std::sync::Arc::new)
            }
        }
    };

    // Form with alters_domain
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
                // Suppress unused variable warning when context isn't used
                let _ = context;
                // Adapter: closure works with &Expr, we wrap result in Arc
                ($logic)(expr.as_ref(), context).map(std::sync::Arc::new)
            }
        }
    };
}

/// Macro for rules that need helper functions defined inside apply()
/// The helpers block is inserted at the start of apply()
///
/// Usage:
/// ```ignore
/// rule_with_helpers!(RuleName, "rule_name", priority, Category, &[ExprKind::Type],
///     helpers: {
///         fn my_helper(x: &Expr) -> bool { ... }
///     },
///     |expr, context| { logic using my_helper }
/// )
/// ```
#[macro_export]
macro_rules! rule_with_helpers {
    ($name:ident, $rule_name:expr, $priority:expr, $category:ident, $applies_to:expr,
     helpers: { $($helper:item)* },
     $logic:expr) => {
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

            fn apply(&self, expr: &std::sync::Arc<Expr>, context: &RuleContext) -> Option<std::sync::Arc<Expr>> {
                // Suppress unused variable warning when context isn't used
                let _ = context;

                // Helper functions
                $($helper)*

                // Adapter: closure works with &Expr, we wrap result in Arc
                ($logic)(expr.as_ref(), context).map(std::sync::Arc::new)
            }
        }
    };

    // With alters_domain
    ($name:ident, $rule_name:expr, $priority:expr, $category:ident, $applies_to:expr,
     alters_domain: $alters:expr,
     helpers: { $($helper:item)* },
     $logic:expr) => {
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

            fn apply(&self, expr: &std::sync::Arc<Expr>, context: &RuleContext) -> Option<std::sync::Arc<Expr>> {
                // Suppress unused variable warning when context isn't used
                let _ = context;

                // Helper functions
                $($helper)*

                // Adapter: closure works with &Expr, we wrap result in Arc
                ($logic)(expr.as_ref(), context).map(std::sync::Arc::new)
            }
        }
    };
}

/// Expression kind for fast rule filtering
/// Rules declare which expression kinds they can apply to
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum ExprKind {
    Number,
    Symbol,
    Sum,     // N-ary addition
    Product, // N-ary multiplication
    Div,
    Pow,
    Function,   // Any function call
    Derivative, // Partial derivative expression
}

impl ExprKind {
    /// Get the kind of an expression (cheap O(1) operation)
    #[inline]
    pub fn of(expr: &Expr) -> Self {
        match &expr.kind {
            AstKind::Number(_) => ExprKind::Number,
            AstKind::Symbol(_) => ExprKind::Symbol,
            AstKind::Sum(_) => ExprKind::Sum,
            AstKind::Product(_) => ExprKind::Product,
            AstKind::Div(_, _) => ExprKind::Div,
            AstKind::Pow(_, _) => ExprKind::Pow,
            AstKind::FunctionCall { .. } => ExprKind::Function,
            AstKind::Derivative { .. } => ExprKind::Derivative,
        }
    }
}

/// Core trait for all simplification rules
pub(crate) trait Rule {
    fn name(&self) -> &'static str;
    fn priority(&self) -> i32;
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

    /// Apply this rule to an expression. Returns Some(new_expr) if transformation applied.
    /// Takes &Arc<Expr> for efficient sub-expression cloning (Arc::clone is cheap).
    fn apply(&self, expr: &Arc<Expr>, context: &RuleContext) -> Option<Arc<Expr>>;
}

/// Categories of simplification rules
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RuleCategory {
    Numeric,   // Constant folding, identities
    Algebraic, // General algebraic rules
    Trigonometric,
    Hyperbolic,
    Exponential,
    Root,
}

/// All expression kinds - used as default for rules
pub(crate) const ALL_EXPR_KINDS: &[ExprKind] = &[
    ExprKind::Number,
    ExprKind::Symbol,
    ExprKind::Sum,
    ExprKind::Product,
    ExprKind::Div,
    ExprKind::Pow,
    ExprKind::Function,
    ExprKind::Derivative,
];

/// Priority ranges for different types of operations:
/// - 85-95: Expansion rules (distribute, expand powers, flatten nested structures)
/// - 70-84: Identity/Cancellation rules (x/x=1, x-x=0, x^a/x^b=x^(a-b), etc.)
/// - 40-69: Compression/Consolidation rules (combine terms, factor, compact a^n/b^n → (a/b)^n)
/// - 1-39: Canonicalization rules (sort terms, normalize display form)
///
/// Context passed to rules during application
/// Uses Arc<HashSet> for cheap cloning (context is cloned per-node)
#[derive(Clone, Debug, Default)]
pub(crate) struct RuleContext {
    pub depth: usize,
    pub variables: Arc<HashSet<String>>,
    pub fixed_vars: Arc<HashSet<String>>, // User-specified fixed variables (constants)
    pub domain_safe: bool,
}

impl RuleContext {
    pub fn with_depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    pub fn with_domain_safe(mut self, domain_safe: bool) -> Self {
        self.domain_safe = domain_safe;
        self
    }

    pub fn with_variables(mut self, variables: HashSet<String>) -> Self {
        self.variables = Arc::new(variables);
        self
    }

    pub fn with_fixed_vars(mut self, fixed_vars: HashSet<String>) -> Self {
        self.fixed_vars = Arc::new(fixed_vars);
        self
    }
}

/// Numeric simplification rules
pub(crate) mod numeric;

/// Algebraic simplification rules
pub(crate) mod algebraic;

/// Trigonometric simplification rules
pub(crate) mod trigonometric;

/// Exponential and logarithmic simplification rules
pub(crate) mod exponential;

/// Root simplification rules
pub(crate) mod root;

/// Hyperbolic function simplification rules
pub(crate) mod hyperbolic;

/// Rule Registry for dynamic loading and dependency management
pub(crate) struct RuleRegistry {
    pub(crate) rules: Vec<Arc<dyn Rule + Send + Sync>>,
    /// Rules indexed by expression kind for fast lookup
    rules_by_kind: HashMap<ExprKind, Vec<Arc<dyn Rule + Send + Sync>>>,
}

impl RuleRegistry {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            rules_by_kind: HashMap::new(),
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

        // Sort by category, then by priority (higher first)
        self.rules.sort_by_key(|r| {
            (
                match r.category() {
                    RuleCategory::Numeric => 0,
                    RuleCategory::Algebraic => 1,
                    RuleCategory::Trigonometric => 2,
                    RuleCategory::Hyperbolic => 3,
                    RuleCategory::Exponential => 4,
                    RuleCategory::Root => 5,
                },
                -r.priority(), // Negative for descending order
            )
        });
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
                    rules.push(rule.clone());
                }
            }
        }
    }

    /// Get only rules that apply to a specific expression kind
    #[inline]
    pub fn get_rules_for_kind(&self, kind: ExprKind) -> &[Arc<dyn Rule + Send + Sync>] {
        self.rules_by_kind
            .get(&kind)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}
