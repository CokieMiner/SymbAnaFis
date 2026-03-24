use super::core::{ALL_EXPR_KINDS, ExprKind, Rule};
use super::{algebraic, exponential, hyperbolic, numeric, root, trigonometric};
use rustc_hash::FxHashMap;
use std::sync::Arc;

/// Rule Registry for dynamic loading and dependency management
pub struct RuleRegistry {
    /// All loaded rules in priority order
    pub(crate) rules: Vec<Arc<dyn Rule + Send + Sync>>,
    /// Rules indexed by expression kind for fast lookup
    rules_by_kind: FxHashMap<ExprKind, Vec<Arc<dyn Rule + Send + Sync>>>,
    /// Rules indexed by function name ID (u64) for O(1) dispatch
    /// Maps function symbol ID -> List of rules that target it SPECIFICALLY
    rules_by_func: FxHashMap<u64, Vec<Arc<dyn Rule + Send + Sync>>>,
    /// Generic function rules that must run for ALL functions
    generic_func_rules: Vec<Arc<dyn Rule + Send + Sync>>,
}

impl RuleRegistry {
    /// Creates a new empty rule registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            rules_by_kind: FxHashMap::default(),
            rules_by_func: FxHashMap::default(),
            generic_func_rules: Vec::new(),
        }
    }

    /// Loads all available simplification rules into the registry.
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
    #[must_use]
    pub fn get_rules_for_kind(&self, kind: ExprKind) -> &[Arc<dyn Rule + Send + Sync>] {
        self.rules_by_kind
            .get(&kind)
            .map_or(&[], std::vec::Vec::as_slice)
    }

    #[inline]
    #[must_use]
    /// Gets rules that specifically target the given function ID.
    pub fn get_specific_func_rules(&self, func_id: u64) -> &[Arc<dyn Rule + Send + Sync>] {
        self.rules_by_func
            .get(&func_id)
            .map_or(&[], std::vec::Vec::as_slice)
    }

    #[inline]
    #[must_use]
    /// Gets generic function rules that apply to all functions.
    pub fn get_generic_func_rules(&self) -> &[Arc<dyn Rule + Send + Sync>] {
        &self.generic_func_rules
    }
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}
