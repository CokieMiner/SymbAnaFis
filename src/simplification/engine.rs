//! Core simplification engine with rule-based architecture
//!
//! Implements bottom-up tree traversal, rule application with memoization,
//! cycle detection, and configurable limits (iterations, depth).

use super::rules::{ExprKind, RuleContext, RuleRegistry};
use crate::{Expr, ExprKind as AstKind};
use rustc_hash::FxHashMap;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

// =============================================================================
// HASH-KEYED CACHE - Avoids Arc cloning on cache lookups
// =============================================================================

/// A cache entry that stores the original expression and its cached result.
/// Used to handle hash collisions via structural equality comparison.
struct CacheEntry {
    /// The original expression (for collision detection)
    key_expr: Arc<Expr>,
    /// The cached result: Some(transformed) or None (no transformation)
    result: Option<Arc<Expr>>,
}

/// Hash-keyed cache for rule results.
///
/// Uses the expression's pre-computed structural hash as the primary key,
/// with a small vector to handle collisions. This avoids Arc cloning on
/// every cache lookup (the hot path), only cloning when inserting new entries.
///
/// Performance characteristics:
/// - Lookup (cache hit, no collision): O(1) hash lookup, 0 Arc clones
/// - Lookup (cache hit, with collision): O(k) where k = collision chain length
/// - Insert: 1 Arc clone for `key_expr`, 0-1 for result
struct HashKeyedCache {
    /// Maps expression hash -> collision chain of (expr, result) pairs
    /// In practice, collision chains are almost always length 1.
    entries: FxHashMap<u64, Vec<CacheEntry>>,
    /// Total number of cached entries (across all chains)
    len: usize,
}

impl HashKeyedCache {
    /// Creates a new empty cache.
    fn new() -> Self {
        Self {
            entries: FxHashMap::default(),
            len: 0,
        }
    }

    /// Look up a cached result by expression.
    /// Returns Some(&result) if found, None if not cached.
    ///
    /// This is the hot path - avoids Arc cloning by using hash lookup first.
    #[inline]
    fn get(&self, expr: &Arc<Expr>) -> Option<&Option<Arc<Expr>>> {
        let hash = expr.structural_hash();
        if let Some(chain) = self.entries.get(&hash) {
            // Check collision chain - usually length 1
            for entry in chain {
                // Compare by pointer first (very fast), then structural equality
                if Arc::ptr_eq(&entry.key_expr, expr) || *entry.key_expr == **expr {
                    return Some(&entry.result);
                }
            }
        }
        None
    }

    /// Insert a cache entry.
    /// Only clones the Arc when actually inserting (not on lookups).
    #[inline]
    fn insert(&mut self, expr: Arc<Expr>, result: Option<Arc<Expr>>) {
        let hash = expr.structural_hash();
        let chain = self.entries.entry(hash).or_default();

        // Check if already exists (update in place)
        for entry in chain.iter_mut() {
            if Arc::ptr_eq(&entry.key_expr, &expr) || *entry.key_expr == *expr {
                entry.result = result;
                return;
            }
        }

        // New entry
        chain.push(CacheEntry {
            key_expr: expr,
            result,
        });
        self.len += 1;
    }

    #[inline]
    /// Returns the total number of cached entries.
    /// Returns the total number of cached entries.
    const fn len(&self) -> usize {
        self.len
    }

    /// Clears all cached entries.
    fn clear(&mut self) {
        self.entries.clear();
        self.len = 0;
    }
}

/// Macro for trace logging (opt-in via `SYMB_TRACE=1` env var).
/// Silent by default - only outputs when explicitly requested.
macro_rules! trace_log {
    ($($arg:tt)*) => {
        if trace_enabled() {
            #[allow(clippy::print_stderr, reason = "Trace logging macro uses stderr for debug output")]
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
pub fn global_registry() -> &'static RuleRegistry {
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
    /// Per-rule caches using hash-keyed storage for O(1) lookups without Arc cloning.
    /// Uses &'static str keys since rule names are guaranteed to be static.
    rule_caches: FxHashMap<&'static str, HashKeyedCache>,
    /// Maximum number of entries per rule cache
    cache_capacity: usize,
    /// Maximum number of simplification iterations
    max_iterations: usize,
    /// Maximum depth for recursive simplifications
    max_depth: usize,
    /// Context containing variables, known symbols, and custom functions
    context: RuleContext,
    /// Whether to apply only domain-safe transformations
    domain_safe: bool,
}

impl Default for Simplifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Simplifier {
    /// Creates a new simplifier with default settings.
    pub fn new() -> Self {
        // Use global registry instead of rebuilding each time
        Self {
            rule_caches: FxHashMap::default(),
            cache_capacity: DEFAULT_CACHE_CAPACITY,
            max_iterations: 1000,
            max_depth: 50,
            context: RuleContext::default(),
            domain_safe: false,
        }
    }

    /// Sets the maximum number of simplification iterations.
    pub const fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Sets the maximum depth for recursive simplifications.
    pub const fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Enables or disables domain-safe transformations.
    pub const fn with_domain_safe(mut self, domain_safe: bool) -> Self {
        self.domain_safe = domain_safe;
        self
    }

    /// Sets custom function bodies.
    pub fn with_custom_bodies(
        mut self,
        custom_bodies: HashMap<u64, crate::core::unified_context::BodyFn>,
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
        // Use full expression storage for cycle detection to handle hash collisions safely.
        // `Expr` hash implementation uses the pre-computed hash, so this is still fast (O(1)),
        // but `HashSet` will verify structural equality on collision.
        let mut seen_exprs: HashSet<Arc<Expr>> = HashSet::new();

        loop {
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

            // Cycle detection: Check if we've seen this exact expression before.
            //
            // If we see the same expression twice, we're in a simplification cycle where
            // rules are undoing each other's transformations (e.g., a/b ↔ a*(1/b)).
            //
            // When a cycle is detected, we return the CURRENT (most recent) expression
            // because canonicalization rules (lowest priority) run last, making the
            // latest iteration the most canonical form (e.g., sorted products/sums).
            if seen_exprs.contains(&current) {
                trace_log!("[DEBUG] Cycle detected, returning last (most canonical) form");
                return Arc::try_unwrap(current).unwrap_or_else(|rc| (*rc).clone());
            }
            // Add AFTER checking to avoid false positive on first iteration
            seen_exprs.insert(Arc::clone(&current));

            iterations += 1;
        }

        // Unwrap Arc if we're the only holder, otherwise clone
        Arc::try_unwrap(current).unwrap_or_else(|rc| (*rc).clone())
    }

    /// Apply rules bottom-up through the expression tree
    ///
    /// This method traverses the expression tree depth-first, simplifying children before parents.
    /// For each node, it:
    /// 1. Recursively simplifies all child expressions
    /// 2. Rebuilds the node with simplified children (if any changed)
    /// 3. Applies all applicable rules to the rebuilt node
    ///
    /// The bottom-up approach ensures that rules work on already-simplified sub-expressions,
    /// reducing the need for complex pattern matching in individual rules.
    fn apply_rules_bottom_up(&mut self, expr: Arc<Expr>, depth: usize) -> Arc<Expr> {
        if depth > self.max_depth {
            return expr;
        }

        // Optimized helper: map over children lazily, only allocating if something changed.
        //
        // Performance Strategy:
        // - Uses `Arc::ptr_eq` to detect if simplification changed a child (O(1) pointer comparison)
        // - Defers allocation until first change detected (avoids Vec allocation for unchanged nodes)
        // - When change detected: copies already-processed unchanged items, then continues with changed items
        //
        // Returns `Some(new_vec)` if any child changed, `None` if all children unchanged.
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
        macro_rules! try_apply {
            ($rule:expr) => {
                if self.context.domain_safe && $rule.alters_domain() {
                    continue;
                }

                let rule_name = $rule.name();

                // Check per-rule cache (hash-keyed for zero Arc clones on lookup)
                let cache = self
                    .rule_caches
                    .entry(rule_name)
                    .or_insert_with(HashKeyedCache::new);
                if let Some(res) = cache.get(&current) {
                    if let Some(new_expr) = res {
                        current = Arc::clone(new_expr);
                    }
                    // Cached result (Some or None), skip application
                    continue;
                }

                // Check memory limit - clear if exceeded
                if cache.len() >= self.cache_capacity {
                    cache.clear();
                }

                if let Some(new_expr) = $rule.apply(&current, &self.context) {
                    trace_log!("[TRACE] {} : {} => {}", rule_name, current, new_expr);
                    cache.insert(Arc::clone(&current), Some(Arc::clone(&new_expr)));
                    current = new_expr;
                } else {
                    cache.insert(Arc::clone(&current), None);
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
