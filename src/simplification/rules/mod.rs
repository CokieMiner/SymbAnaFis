use crate::Expr;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Core trait for all simplification rules
pub trait Rule {
    fn name(&self) -> &'static str;
    fn priority(&self) -> i32;
    fn category(&self) -> RuleCategory;
    fn dependencies(&self) -> Vec<&'static str> { Vec::new() } // New: dependencies for ordering
    fn alters_domain(&self) -> bool { false } // Tag for rules that may change the domain of validity
    fn apply(&self, expr: &Expr, context: &RuleContext) -> Option<Expr>;
}

/// Categories of simplification rules
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RuleCategory {
    Numeric,      // Constant folding, identities
    Algebraic,    // General algebraic rules
    Trigonometric,
    Hyperbolic,
    Exponential,
    Root,
}

/// Priority ranges for different types of operations:
/// - 100-199: Expansion rules (distribute, expand powers, etc.)
/// - 50-99: Identity/Cancellation rules (x/x=1, x-x=0, etc.)
/// - 1-49: Compression/Consolidation rules (combine terms, factor, etc.)

/// Context passed to rules during application
#[derive(Clone, Debug)]
pub struct RuleContext {
    pub depth: usize,
    pub parent: Option<Expr>,
    pub variables: HashSet<String>,
    pub domain_safe: bool,
}

impl Default for RuleContext {
    fn default() -> Self {
        Self {
            depth: 0,
            parent: None,
            variables: HashSet::new(),
            domain_safe: false,
        }
    }
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
}

impl RuleRegistry {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
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
        self.rules.sort_by_key(|r| (
            match r.category() {
                RuleCategory::Numeric => 0,
                RuleCategory::Algebraic => 1,
                RuleCategory::Trigonometric => 2,
                RuleCategory::Hyperbolic => 3,
                RuleCategory::Exponential => 4,
                RuleCategory::Root => 5,
            },
            -r.priority()  // Negative for descending order
        ));
    }

    /// Order rules using topological sort based on dependencies
    pub fn order_by_dependencies(&mut self) {
        // Build graph: rule name -> (rule, dependencies)
        let mut graph: HashMap<String, (usize, Vec<String>)> = HashMap::new();
        for (i, rule) in self.rules.iter().enumerate() {
            graph.insert(rule.name().to_string(), (i, rule.dependencies().iter().map(|s| s.to_string()).collect()));
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
            if !visited.contains(name) {
                if let Err(e) = dfs(name, &graph, &mut visited, &mut visiting, &mut order) {
                    eprintln!("Warning: {}", e);
                    // Fallback to priority sort
                    self.rules.sort_by(|a, b| b.priority().cmp(&a.priority()));
                    return;
                }
            }
        }

        // Reorder rules
        let mut ordered = Vec::new();
        for &idx in order.iter().rev() {
            ordered.push(self.rules[idx].clone());
        }
        self.rules = ordered;

        // Finally, sort by priority descending to ensure high-priority rules run first
        self.rules.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    pub fn get_rules(&self) -> &Vec<Rc<dyn Rule>> {
        &self.rules
    }
}
