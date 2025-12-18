//! Core simplification engine with rule-based architecture
//!
//! Implements bottom-up tree traversal, rule application with memoization,
//! cycle detection, and configurable limits (iterations, depth, timeout).

use super::rules::{ExprKind, RuleContext, RuleRegistry};
use crate::{Expr, ExprKind as AstKind};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Efficiently extract owned Expr from Arc - avoids clone if reference count is 1
#[inline]
fn unwrap_or_clone(rc: Arc<Expr>) -> Expr {
    Arc::try_unwrap(rc).unwrap_or_else(|rc| (*rc).clone())
}

/// Check if tracing is enabled via environment variable (cached)
fn trace_enabled() -> bool {
    static TRACE: OnceLock<bool> = OnceLock::new();
    *TRACE.get_or_init(|| {
        std::env::var("SYMB_TRACE")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false)
    })
}

/// Global rule registry singleton - built once, reused across all simplifications
fn global_registry() -> &'static RuleRegistry {
    static REGISTRY: OnceLock<RuleRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut registry = RuleRegistry::new();
        registry.load_all_rules();
        registry.order_by_dependencies();
        registry
    })
}

/// Main simplification engine with rule-based architecture
pub(crate) struct Simplifier {
    rule_caches: HashMap<String, HashMap<u64, Option<Arc<Expr>>>>, // Per-rule memoization with Arc for cheap cloning
    max_iterations: usize,
    max_depth: usize,
    timeout: Option<Duration>, // Wall-clock timeout to prevent hangs
    context: RuleContext,
    domain_safe: bool,
}

impl Default for Simplifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Simplifier {
    pub fn new() -> Self {
        // Use global registry instead of rebuilding each time
        Self {
            rule_caches: HashMap::new(),
            max_iterations: 1000,
            max_depth: 50,
            timeout: None, // No timeout by default
            context: RuleContext::default(),
            domain_safe: false,
        }
    }

    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    pub fn with_domain_safe(mut self, domain_safe: bool) -> Self {
        self.domain_safe = domain_safe;
        self
    }

    pub fn with_variables(mut self, variables: HashSet<String>) -> Self {
        self.context = self.context.with_variables(variables);
        self
    }

    pub fn with_fixed_vars(mut self, fixed_vars: HashSet<String>) -> Self {
        self.context = self.context.with_fixed_vars(fixed_vars);
        self
    }

    /// Main simplification entry point
    pub fn simplify(&mut self, expr: Expr) -> Expr {
        let mut current = Arc::new(expr);
        let mut iterations = 0;
        // Use normalized expressions for cycle detection (handles Sub vs Add(-1*x) equivalence)
        let mut seen_expressions: HashSet<Expr> = HashSet::new();
        let start_time = Instant::now();

        loop {
            // Check timeout first
            if let Some(timeout) = self.timeout
                && start_time.elapsed() > timeout
            {
                eprintln!("Warning: Simplification timed out after {:?}", timeout);
                break;
            }

            if iterations >= self.max_iterations {
                eprintln!(
                    "Warning: Simplification exceeded maximum iterations ({})",
                    self.max_iterations
                );
                break;
            }

            let original = current.clone();
            current = self.apply_rules_bottom_up(current, 0);

            if trace_enabled() {
                eprintln!(
                    "[DEBUG] Iteration {}: {} -> {}",
                    iterations, original, current
                );
            }

            // Use structural equality to check if expression changed
            if *current == *original {
                break; // No changes
            }

            // After a full pass of all rules, check if we've seen this result before
            // Use normalized form for proper cycle detection (handles Sub vs Add(-1*x) equivalence)
            let normalized = crate::simplification::helpers::normalize_for_comparison(&current);
            if seen_expressions.contains(&normalized) {
                // We're in a cycle - stop here with the current result
                if trace_enabled() {
                    eprintln!("[DEBUG] Cycle detected, stopping");
                }
                break;
            }
            // Add AFTER checking, so first iteration's result doesn't trigger false positive
            seen_expressions.insert(normalized);

            iterations += 1;
        }

        // Unwrap Arc if we're the only holder, otherwise clone
        Arc::try_unwrap(current).unwrap_or_else(|rc| (*rc).clone())
    }

    /// Apply rules bottom-up through the expression tree
    fn apply_rules_bottom_up(&mut self, expr: Arc<Expr>, depth: usize) -> Arc<Expr> {
        if depth > self.max_depth {
            return expr;
        }

        match &expr.kind {
            // N-ary Sum - simplify all terms
            AstKind::Sum(terms) => {
                let simplified_terms: Vec<Arc<Expr>> = terms
                    .iter()
                    .map(|t| self.apply_rules_bottom_up(t.clone(), depth + 1))
                    .collect();

                // Check if any term changed
                let changed = simplified_terms
                    .iter()
                    .zip(terms.iter())
                    .any(|(new, old)| !Arc::ptr_eq(new, old));

                if !changed {
                    self.apply_rules_to_node(expr, depth)
                } else {
                    let new_expr = Arc::new(Expr::sum_from_arcs(simplified_terms));
                    self.apply_rules_to_node(new_expr, depth)
                }
            }

            // N-ary Product - simplify all factors
            AstKind::Product(factors) => {
                let simplified_factors: Vec<Arc<Expr>> = factors
                    .iter()
                    .map(|f| self.apply_rules_bottom_up(f.clone(), depth + 1))
                    .collect();

                let changed = simplified_factors
                    .iter()
                    .zip(factors.iter())
                    .any(|(new, old)| !Arc::ptr_eq(new, old));

                if !changed {
                    self.apply_rules_to_node(expr, depth)
                } else {
                    let new_expr = Arc::new(Expr::product_from_arcs(simplified_factors));
                    self.apply_rules_to_node(new_expr, depth)
                }
            }

            AstKind::Div(u, v) => {
                let u_simplified = self.apply_rules_bottom_up(u.clone(), depth + 1);
                let v_simplified = self.apply_rules_bottom_up(v.clone(), depth + 1);

                if Arc::ptr_eq(&u_simplified, u) && Arc::ptr_eq(&v_simplified, v) {
                    self.apply_rules_to_node(expr, depth)
                } else {
                    let new_expr = Arc::new(Expr::div_from_arcs(u_simplified, v_simplified));
                    self.apply_rules_to_node(new_expr, depth)
                }
            }
            AstKind::Pow(u, v) => {
                let u_simplified = self.apply_rules_bottom_up(u.clone(), depth + 1);
                let v_simplified = self.apply_rules_bottom_up(v.clone(), depth + 1);

                if Arc::ptr_eq(&u_simplified, u) && Arc::ptr_eq(&v_simplified, v) {
                    self.apply_rules_to_node(expr, depth)
                } else {
                    let new_expr = Arc::new(Expr::pow_from_arcs(u_simplified, v_simplified));
                    self.apply_rules_to_node(new_expr, depth)
                }
            }
            AstKind::FunctionCall { name, args } => {
                let args_simplified: Vec<Arc<Expr>> = args
                    .iter()
                    .map(|arg| self.apply_rules_bottom_up(Arc::new(arg.clone()), depth + 1))
                    .collect();

                // Check if any arg changed
                let changed = args_simplified
                    .iter()
                    .zip(args.iter())
                    .any(|(new, old)| new.as_ref() != old);

                if !changed {
                    self.apply_rules_to_node(expr, depth)
                } else {
                    let new_expr = Arc::new(Expr::new(AstKind::FunctionCall {
                        name: name.clone(),
                        args: args_simplified.into_iter().map(unwrap_or_clone).collect(),
                    }));
                    self.apply_rules_to_node(new_expr, depth)
                }
            }
            _ => self.apply_rules_to_node(expr, depth),
        }
    }

    /// Apply all applicable rules to a single node in dependency order
    fn apply_rules_to_node(&mut self, mut current: Arc<Expr>, depth: usize) -> Arc<Expr> {
        let context = self
            .context
            .clone()
            .with_depth(depth)
            .with_domain_safe(self.domain_safe);

        // Get the expression kind once and only check rules that apply to it
        let kind = ExprKind::of(current.as_ref());
        let applicable_rules = global_registry().get_rules_for_kind(kind);

        for rule in applicable_rules {
            if context.domain_safe && rule.alters_domain() {
                continue;
            }

            let rule_name = rule.name();

            // Check per-rule cache using expression ID as key (fast)
            let cache_key = current.id;
            if let Some(cache) = self.rule_caches.get(rule_name)
                && let Some(cached_result) = cache.get(&cache_key)
            {
                if let Some(new_expr) = cached_result {
                    current = Arc::clone(new_expr); // Cheap Arc clone!
                    continue;
                } else {
                    continue; // Cached as "no change"
                }
            }

            // Apply rule - pass &Arc<Expr>, get Option<Arc<Expr>>
            let original_id = current.id;
            if let Some(new_expr) = rule.apply(&current, &context) {
                if trace_enabled() {
                    eprintln!("[TRACE] {} : {} => {}", rule_name, current, new_expr);
                }

                // Cache the transformation - Arc clone is cheap!
                self.rule_caches
                    .entry(rule_name.to_string())
                    .or_default()
                    .insert(original_id, Some(Arc::clone(&new_expr)));

                current = new_expr;
            } else {
                // Cache as "no change"
                self.rule_caches
                    .entry(rule_name.to_string())
                    .or_default()
                    .insert(original_id, None);
            }
        }

        current
    }
}

/// Convenience function with user-specified fixed variables
pub(crate) fn simplify_expr_with_fixed_vars(expr: Expr, fixed_vars: HashSet<String>) -> Expr {
    let variables = expr.variables();
    // Skip verification for performance - just simplify directly
    let mut simplifier = Simplifier::new()
        .with_max_iterations(1000)
        .with_max_depth(20)
        .with_variables(variables)
        .with_fixed_vars(fixed_vars);
    simplifier.simplify(expr)
}
