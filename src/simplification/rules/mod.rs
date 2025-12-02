use crate::Expr;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Expression kind for fast rule filtering
/// Rules declare which expression kinds they can apply to
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ExprKind {
    Number,
    Symbol,
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Function, // Any function call
}

impl ExprKind {
    /// Get the kind of an expression (cheap O(1) operation)
    #[inline]
    pub fn of(expr: &Expr) -> Self {
        match expr {
            Expr::Number(_) => ExprKind::Number,
            Expr::Symbol(_) => ExprKind::Symbol,
            Expr::Add(_, _) => ExprKind::Add,
            Expr::Sub(_, _) => ExprKind::Sub,
            Expr::Mul(_, _) => ExprKind::Mul,
            Expr::Div(_, _) => ExprKind::Div,
            Expr::Pow(_, _) => ExprKind::Pow,
            Expr::FunctionCall { .. } => ExprKind::Function,
        }
    }
}

/// Core trait for all simplification rules
pub trait Rule {
    fn name(&self) -> &'static str;
    fn priority(&self) -> i32;
    fn category(&self) -> RuleCategory;
    fn dependencies(&self) -> Vec<&'static str> {
        Vec::new()
    }
    fn alters_domain(&self) -> bool {
        false
    }
    
    /// Which expression kinds this rule can apply to.
    /// Rules will ONLY be checked against expressions matching these kinds.
    /// Default: all kinds (for backwards compatibility during migration)
    fn applies_to(&self) -> &'static [ExprKind] {
        &[
            ExprKind::Number,
            ExprKind::Symbol,
            ExprKind::Add,
            ExprKind::Sub,
            ExprKind::Mul,
            ExprKind::Div,
            ExprKind::Pow,
            ExprKind::Function,
        ]
    }
    
    fn apply(&self, expr: &Expr, context: &RuleContext) -> Option<Expr>;
}

/// Categories of simplification rules
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RuleCategory {
    Numeric,   // Constant folding, identities
    Algebraic, // General algebraic rules
    Trigonometric,
    Hyperbolic,
    Exponential,
    Root,
}

/// Priority ranges for different types of operations:
/// - 100-199: Expansion rules (distribute, expand powers, etc.)
/// - 50-99: Identity/Cancellation rules (x/x=1, x-x=0, etc.)
/// - 1-49: Compression/Consolidation rules (combine terms, factor, etc.)
///
/// Context passed to rules during application
#[derive(Clone, Debug, Default)]
pub struct RuleContext {
    pub depth: usize,
    pub parent: Option<Expr>,
    pub variables: HashSet<String>,
    pub fixed_vars: HashSet<String>, // User-specified fixed variables (constants)
    pub domain_safe: bool,
}

impl RuleContext {
    pub fn with_depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    pub fn with_parent(mut self, parent: Expr) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn with_domain_safe(mut self, domain_safe: bool) -> Self {
        self.domain_safe = domain_safe;
        self
    }

    pub fn with_variables(mut self, variables: HashSet<String>) -> Self {
        self.variables = variables;
        self
    }

    pub fn with_fixed_vars(mut self, fixed_vars: HashSet<String>) -> Self {
        self.fixed_vars = fixed_vars;
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
    pub(crate) rules: Vec<Rc<dyn Rule>>,
    /// Rules indexed by expression kind for fast lookup
    rules_by_kind: HashMap<ExprKind, Vec<Rc<dyn Rule>>>,
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

    /// Order rules using topological sort based on dependencies
    pub fn order_by_dependencies(&mut self) {
        // Build graph: rule name -> (rule, dependencies)
        let mut graph: HashMap<String, (usize, Vec<String>)> = HashMap::new();
        for (i, rule) in self.rules.iter().enumerate() {
            graph.insert(
                rule.name().to_string(),
                (
                    i,
                    rule.dependencies().iter().map(|s| s.to_string()).collect(),
                ),
            );
        }

        // Topological sort
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();
        let mut order = Vec::new();

        fn dfs(
            node: &String,
            graph: &HashMap<String, (usize, Vec<String>)>,
            visited: &mut HashSet<String>,
            visiting: &mut HashSet<String>,
            order: &mut Vec<usize>,
        ) -> Result<(), String> {
            if visiting.contains(node) {
                return Err(format!("Cycle detected involving rule: {}", node));
            }
            if visited.contains(node) {
                return Ok(());
            }
            visiting.insert(node.clone());
            if let Some((_, deps)) = graph.get(node) {
                for dep in deps {
                    dfs(dep, graph, visited, visiting, order)?;
                }
            }
            visiting.remove(node);
            visited.insert(node.clone());
            order.push(graph[node].0);
            Ok(())
        }

        for name in graph.keys() {
            if !visited.contains(name)
                && let Err(e) = dfs(name, &graph, &mut visited, &mut visiting, &mut order)
            {
                eprintln!("Warning: {}", e);
                // Fallback to priority sort
                self.rules.sort_by_key(|b| std::cmp::Reverse(b.priority()));
                self.build_kind_index();
                return;
            }
        }

        // Reorder rules
        let mut ordered = Vec::new();
        for &idx in order.iter().rev() {
            ordered.push(self.rules[idx].clone());
        }
        self.rules = ordered;

        // Finally, sort by priority descending to ensure high-priority rules run first
        self.rules.sort_by_key(|b| std::cmp::Reverse(b.priority()));
        
        // Build the kind index after final ordering
        self.build_kind_index();
    }
    
    /// Build the index of rules by expression kind
    fn build_kind_index(&mut self) {
        self.rules_by_kind.clear();
        
        // Initialize all kinds
        for kind in [
            ExprKind::Number,
            ExprKind::Symbol,
            ExprKind::Add,
            ExprKind::Sub,
            ExprKind::Mul,
            ExprKind::Div,
            ExprKind::Pow,
            ExprKind::Function,
        ] {
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
    pub fn get_rules_for_kind(&self, kind: ExprKind) -> &[Rc<dyn Rule>] {
        self.rules_by_kind.get(&kind).map(|v| v.as_slice()).unwrap_or(&[])
    }
}
