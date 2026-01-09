//! Core simplification engine with rule-based architecture
//!
//! Implements bottom-up tree traversal, rule application with memoization,
//! cycle detection, and configurable limits (iterations, depth, timeout).

use super::rules::{ExprKind, RuleContext, RuleRegistry};
use crate::{Expr, ExprKind as AstKind};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

/// Macro for trace logging (opt-in via `SYMB_TRACE=1` env var).
/// Silent by default - only outputs when explicitly requested.
macro_rules! trace_log {
    ($($arg:tt)*) => {
        if trace_enabled() {
            #[allow(clippy::print_stderr)]
            {
                eprintln!($($arg)*);
            }
        }
    };
}

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
pub struct Simplifier {
    /// Per-rule caches - cleared when exceeding capacity to bound memory
    /// Uses &'static str keys since rule names are guaranteed to be static
    rule_caches: HashMap<&'static str, HashMap<u64, Option<Arc<Expr>>>>,
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

    pub const fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    pub const fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    pub const fn with_domain_safe(mut self, domain_safe: bool) -> Self {
        self.domain_safe = domain_safe;
        self
    }

    pub fn with_variables(mut self, variables: HashSet<String>) -> Self {
        self.context = self.context.with_variables(variables);
        self
    }

    pub fn with_known_symbols(mut self, known_symbols: HashSet<String>) -> Self {
        self.context = self.context.with_known_symbols(known_symbols);
        self
    }

    pub fn with_custom_bodies(
        mut self,
        custom_bodies: HashMap<String, crate::core::unified_context::BodyFn>,
    ) -> Self {
        self.context = self.context.with_custom_bodies(custom_bodies);
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
                break;
            }

            if iterations >= self.max_iterations {
                break;
            }

            let original = Arc::clone(&current);
            current = self.apply_rules_bottom_up(current, 0);

            // Use structural equality to check if expression changed
            if *current == *original {
                break; // No changes
            }

            trace_log!("[DEBUG] Iteration {iterations}: {original} -> {current}");

            // After a full pass of all rules, check if we've seen this structural hash before
            // The hash field is a structural hash - same hash means structurally identical expression
            let fingerprint = current.hash;
            if seen_hashes.contains(&fingerprint) {
                // Cycle detected! Return the CURRENT (last) expression as it's the most canonicalized.
                // Canonicalization rules run last (low priority), so the latest iteration
                // has the most canonical form (e.g., sorted products).
                trace_log!("[DEBUG] Cycle detected, returning last (most canonical) form");
                return Arc::try_unwrap(current).unwrap_or_else(|rc| (*rc).clone());
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

        // Helper: map over children lazily, only allocating if something changed
        // Returns Some(new_vec) if any child changed, None otherwise
        let map_lazy = |items: &[Arc<Expr>], simplifier: &mut Self| -> Option<Vec<Arc<Expr>>> {
            let mut result: Option<Vec<Arc<Expr>>> = None;
            for (i, item) in items.iter().enumerate() {
                let simplified = simplifier.apply_rules_bottom_up(Arc::clone(item), depth + 1);
                if !Arc::ptr_eq(&simplified, item) && result.is_none() {
                    let mut v = Vec::with_capacity(items.len());
                    v.extend(items[..i].iter().cloned());
                    result = Some(v);
                }
                if let Some(ref mut v) = result {
                    v.push(simplified);
                }
            }
            result
        };

        match &expr.kind {
            // N-ary Sum - simplify all terms
            AstKind::Sum(terms) => {
                if let Some(v) = map_lazy(terms, self) {
                    let new_expr = Arc::new(Expr::sum_from_arcs(v));
                    self.apply_rules_to_node(new_expr, depth)
                } else {
                    self.apply_rules_to_node(expr, depth)
                }
            }

            // N-ary Product - simplify all factors
            AstKind::Product(factors) => {
                if let Some(v) = map_lazy(factors, self) {
                    let new_expr = Arc::new(Expr::product_from_arcs(v));
                    self.apply_rules_to_node(new_expr, depth)
                } else {
                    self.apply_rules_to_node(expr, depth)
                }
            }

            AstKind::Div(u, v) => {
                let u_simplified = self.apply_rules_bottom_up(Arc::clone(u), depth + 1);
                let v_simplified = self.apply_rules_bottom_up(Arc::clone(v), depth + 1);

                if Arc::ptr_eq(&u_simplified, u) && Arc::ptr_eq(&v_simplified, v) {
                    self.apply_rules_to_node(expr, depth)
                } else {
                    let new_expr = Arc::new(Expr::div_from_arcs(u_simplified, v_simplified));
                    self.apply_rules_to_node(new_expr, depth)
                }
            }
            AstKind::Pow(u, v) => {
                let u_simplified = self.apply_rules_bottom_up(Arc::clone(u), depth + 1);
                let v_simplified = self.apply_rules_bottom_up(Arc::clone(v), depth + 1);

                if Arc::ptr_eq(&u_simplified, u) && Arc::ptr_eq(&v_simplified, v) {
                    self.apply_rules_to_node(expr, depth)
                } else {
                    let new_expr = Arc::new(Expr::pow_from_arcs(u_simplified, v_simplified));
                    self.apply_rules_to_node(new_expr, depth)
                }
            }
            AstKind::FunctionCall { name, args } => {
                if let Some(v) = map_lazy(args, self) {
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

        // Helper macro to apply a rule and update current if successful
        // Returns true if a rule was applied (and we should continue/restart?)
        // Actually, the original logic didn't restart on success immediately,
        // it continued to next rule with new expression.
        // So we just update `current`.
        macro_rules! try_apply {
            ($rule:expr) => {
                if self.context.domain_safe && $rule.alters_domain() {
                    continue;
                }

                let rule_name = $rule.name();
                let original_id = current.id;

                // Check per-rule cache
                let cache = self.rule_caches.entry(rule_name).or_default();
                if let Some(res) = cache.get(&original_id) {
                    if let Some(new_expr) = res {
                        current = Arc::clone(new_expr);
                    }
                    // Cached result (Some or None), skip application
                    continue;
                }

                // Check memory limit validation for cache? (Moved inside to be safe)
                if cache.len() >= self.cache_capacity {
                    cache.clear();
                }

                if let Some(new_expr) = $rule.apply(&current, &self.context) {
                    trace_log!("[TRACE] {} : {} => {}", rule_name, current, new_expr);
                    cache.insert(original_id, Some(Arc::clone(&new_expr)));
                    current = new_expr;
                } else {
                    cache.insert(original_id, None);
                }
            };
        }

        if kind == ExprKind::Function {
            if let AstKind::FunctionCall { name, .. } = &current.kind {
                let registry = global_registry();
                let specific = registry.get_specific_func_rules(name.id());
                let generic = registry.get_generic_func_rules();

                for rule in specific {
                    try_apply!(rule);
                }
                for rule in generic {
                    try_apply!(rule);
                }
            } else {
                // Fallback (should not happen for kind=Function)
                for rule in global_registry().get_rules_for_kind(kind) {
                    try_apply!(rule);
                }
            }
        } else {
            for rule in global_registry().get_rules_for_kind(kind) {
                try_apply!(rule);
            }
        }

        current
    }
}
