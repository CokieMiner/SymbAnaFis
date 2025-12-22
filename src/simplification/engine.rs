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

/// Default cache capacity per rule before clearing (10K entries)
const DEFAULT_CACHE_CAPACITY: usize = 10_000;

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
    /// Per-rule caches - cleared when exceeding capacity to bound memory
    /// Uses Arc<str> keys to avoid String allocation on each lookup
    rule_caches: HashMap<Arc<str>, HashMap<u64, Option<Arc<Expr>>>>,
    cache_capacity: usize,
    max_iterations: usize,
    max_depth: usize,
    timeout: Option<Duration>,
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
            cache_capacity: DEFAULT_CACHE_CAPACITY,
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
        // Set domain_safe on context once (apply_rules_to_node will only update depth)
        self.context.domain_safe = self.domain_safe;

        let mut current = Arc::new(expr);
        let mut iterations = 0;
        // Use expression id (structural hash) for cheap cycle detection
        // This avoids storing full expression clones and expensive normalization
        let mut seen_hashes: HashSet<u64> = HashSet::new();
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

            // After a full pass of all rules, check if we've seen this hash before
            // The id field is a structural hash - cycle means we've seen exact same structure
            let fingerprint = current.id;
            if seen_hashes.contains(&fingerprint) {
                // We're in a cycle - stop here with the current result
                if trace_enabled() {
                    eprintln!("[DEBUG] Cycle detected, stopping");
                }
                break;
            }
            // Add AFTER checking, so first iteration's result doesn't trigger false positive
            seen_hashes.insert(fingerprint);

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
                let mut simplified_terms: Option<Vec<Arc<Expr>>> = None;

                for (i, term) in terms.iter().enumerate() {
                    let simplified = self.apply_rules_bottom_up(term.clone(), depth + 1);
                    if !Arc::ptr_eq(&simplified, term) && simplified_terms.is_none() {
                        let mut v = Vec::with_capacity(terms.len());
                        v.extend(terms[..i].iter().cloned());
                        simplified_terms = Some(v);
                    }
                    if let Some(ref mut v) = simplified_terms {
                        v.push(simplified);
                    }
                }

                if let Some(v) = simplified_terms {
                    let new_expr = Arc::new(Expr::sum_from_arcs(v));
                    self.apply_rules_to_node(new_expr, depth)
                } else {
                    self.apply_rules_to_node(expr, depth)
                }
            }

            // N-ary Product - simplify all factors
            AstKind::Product(factors) => {
                let mut simplified_factors: Option<Vec<Arc<Expr>>> = None;

                for (i, factor) in factors.iter().enumerate() {
                    let simplified = self.apply_rules_bottom_up(factor.clone(), depth + 1);
                    if !Arc::ptr_eq(&simplified, factor) && simplified_factors.is_none() {
                        let mut v = Vec::with_capacity(factors.len());
                        v.extend(factors[..i].iter().cloned());
                        simplified_factors = Some(v);
                    }
                    if let Some(ref mut v) = simplified_factors {
                        v.push(simplified);
                    }
                }

                if let Some(v) = simplified_factors {
                    let new_expr = Arc::new(Expr::product_from_arcs(v));
                    self.apply_rules_to_node(new_expr, depth)
                } else {
                    self.apply_rules_to_node(expr, depth)
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
                let mut simplified_args: Option<Vec<Arc<Expr>>> = None;

                for (i, arg) in args.iter().enumerate() {
                    let simplified = self.apply_rules_bottom_up(arg.clone(), depth + 1);
                    if !Arc::ptr_eq(&simplified, arg) && simplified_args.is_none() {
                        let mut v = Vec::with_capacity(args.len());
                        v.extend(args[..i].iter().cloned());
                        simplified_args = Some(v);
                    }
                    if let Some(ref mut v) = simplified_args {
                        v.push(simplified);
                    }
                }

                if let Some(v) = simplified_args {
                    let new_expr = Arc::new(Expr::func_multi_from_arcs(name, v));
                    self.apply_rules_to_node(new_expr, depth)
                } else {
                    self.apply_rules_to_node(expr, depth)
                }
            }
            _ => self.apply_rules_to_node(expr, depth),
        }
    }

    /// Apply all applicable rules to a single node in dependency order
    fn apply_rules_to_node(&mut self, mut current: Arc<Expr>, depth: usize) -> Arc<Expr> {
        // Update depth in-place (context.domain_safe is already set in simplify())
        self.context.set_depth(depth);

        // Get the expression kind once and only check rules that apply to it
        let kind = ExprKind::of(current.as_ref());
        let applicable_rules = global_registry().get_rules_for_kind(kind);

        for rule in applicable_rules {
            if self.context.domain_safe && rule.alters_domain() {
                continue;
            }

            let rule_name = rule.name();

            // Check per-rule cache using expression ID as key (fast)
            let cache_key = current.id;

            if let Some(cache) = self.rule_caches.get(rule_name)
                && let Some(cached_result) = cache.get(&cache_key) {
                    if let Some(new_expr) = cached_result {
                        current = Arc::clone(new_expr); // Cheap Arc clone!
                    }
                    // cached_result is Some or None, either way we skip rule application
                    continue;
                }

            // Apply rule - pass &Arc<Expr>, get Option<Arc<Expr>>
            let original_id = current.id;

            // Get or create cache for this rule (Arc<str> avoids allocation on hit)
            let cache = self.rule_caches.entry(Arc::from(rule_name)).or_default();

            // Bound memory: clear if exceeding capacity (simple, fast eviction)
            if cache.len() >= self.cache_capacity {
                cache.clear();
            }

            if let Some(new_expr) = rule.apply(&current, &self.context) {
                if trace_enabled() {
                    eprintln!("[TRACE] {} : {} => {}", rule_name, current, new_expr);
                }
                cache.insert(original_id, Some(Arc::clone(&new_expr)));
                current = new_expr;
            } else {
                cache.insert(original_id, None);
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
        .with_max_depth(50)
        .with_variables(variables)
        .with_fixed_vars(fixed_vars);
    simplifier.simplify(expr)
}
