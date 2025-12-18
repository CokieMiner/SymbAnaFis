//! Abstract Syntax Tree for mathematical expressions
//!
//! N-ary Sum/Product architecture for efficient simplification.
//! Phase-specific epoch tracking for skip-if-already-processed optimization.

use std::cmp::Ordering as CmpOrdering;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::symbol::{InternedSymbol, get_or_intern};

/// Type alias for custom evaluation functions map
pub type CustomEvalMap =
    std::collections::HashMap<String, std::sync::Arc<dyn Fn(&[f64]) -> Option<f64> + Send + Sync>>;

// =============================================================================
// EPOCHS - Global counters for phase-specific simplification tracking
// =============================================================================

/// Global epochs for each simplification phase.
/// Aligned with priority tiers from simplification/README.md:
///   100:   Numeric       - constant folding
///   85-95: Expansion     - expand, distribute, flatten
///   70-84: Cancellation  - identities, cancel common factors  
///   40-69: Consolidation - combine terms, factor, compact
///   1-39:  Canonicalization - sort, normalize display
pub mod epochs {
    use std::sync::atomic::{AtomicU64, Ordering};

    pub static NUMERIC: AtomicU64 = AtomicU64::new(0);
    pub static EXPANSION: AtomicU64 = AtomicU64::new(0);
    pub static CANCELLATION: AtomicU64 = AtomicU64::new(0);
    pub static CONSOLIDATION: AtomicU64 = AtomicU64::new(0);
    pub static CANONICALIZATION: AtomicU64 = AtomicU64::new(0);

    /// Invalidate all caches (new rules added or forced re-simplification)
    #[allow(dead_code)]
    pub fn invalidate_all() {
        NUMERIC.fetch_add(1, Ordering::Relaxed);
        EXPANSION.fetch_add(1, Ordering::Relaxed);
        CANCELLATION.fetch_add(1, Ordering::Relaxed);
        CONSOLIDATION.fetch_add(1, Ordering::Relaxed);
        CANONICALIZATION.fetch_add(1, Ordering::Relaxed);
    }
}

// =============================================================================
// EXPRESSION FLAGS - Phase-specific epoch tracking
// =============================================================================

/// Phase-specific expression flags for fine-grained simplification tracking.
/// Each flag tracks when that phase was last completed on this expression.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExprFlags {
    /// Priority 100: numeric simplification (constant folding)
    pub numeric_at: Option<u64>,
    /// Priority 85-95: expansion, distribution, flattening
    pub expansion_at: Option<u64>,
    /// Priority 70-84: identities, cancellation
    pub cancellation_at: Option<u64>,
    /// Priority 40-69: combine terms, factor, compact
    pub consolidation_at: Option<u64>,
    /// Priority 1-39: sorting, normalization
    pub canonical_at: Option<u64>,
}

impl ExprFlags {
    /// Check if numeric phase (priority 100) needs to run
    #[inline]
    pub fn needs_numeric(&self) -> bool {
        self.numeric_at != Some(epochs::NUMERIC.load(Ordering::Relaxed))
    }

    /// Check if expansion phase (priority 85-95) needs to run
    #[inline]
    pub fn needs_expansion(&self) -> bool {
        self.expansion_at != Some(epochs::EXPANSION.load(Ordering::Relaxed))
    }

    /// Check if cancellation phase (priority 70-84) needs to run
    #[inline]
    pub fn needs_cancellation(&self) -> bool {
        self.cancellation_at != Some(epochs::CANCELLATION.load(Ordering::Relaxed))
    }

    /// Check if consolidation phase (priority 40-69) needs to run
    #[inline]
    pub fn needs_consolidation(&self) -> bool {
        self.consolidation_at != Some(epochs::CONSOLIDATION.load(Ordering::Relaxed))
    }

    /// Check if canonicalization phase (priority 1-39) needs to run
    #[inline]
    pub fn needs_canonicalization(&self) -> bool {
        self.canonical_at != Some(epochs::CANONICALIZATION.load(Ordering::Relaxed))
    }

    /// Check if fully simplified (all phases current)
    #[inline]
    pub fn is_simplified(&self) -> bool {
        !self.needs_numeric()
            && !self.needs_expansion()
            && !self.needs_cancellation()
            && !self.needs_consolidation()
            && !self.needs_canonicalization()
    }

    /// Mark a specific phase as completed at current epoch
    pub fn mark_numeric(&mut self) {
        self.numeric_at = Some(epochs::NUMERIC.load(Ordering::Relaxed));
    }

    pub fn mark_expansion(&mut self) {
        self.expansion_at = Some(epochs::EXPANSION.load(Ordering::Relaxed));
    }

    pub fn mark_cancellation(&mut self) {
        self.cancellation_at = Some(epochs::CANCELLATION.load(Ordering::Relaxed));
    }

    pub fn mark_consolidation(&mut self) {
        self.consolidation_at = Some(epochs::CONSOLIDATION.load(Ordering::Relaxed));
    }

    pub fn mark_canonicalization(&mut self) {
        self.canonical_at = Some(epochs::CANONICALIZATION.load(Ordering::Relaxed));
    }

    /// Mark all phases as completed (fully simplified)
    pub fn mark_all_simplified(&mut self) {
        self.mark_numeric();
        self.mark_expansion();
        self.mark_cancellation();
        self.mark_consolidation();
        self.mark_canonicalization();
    }
}

// =============================================================================
// EXPRESSION ID COUNTER
// =============================================================================

static EXPR_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_id() -> u64 {
    EXPR_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

// =============================================================================
// EXPR - The main expression type
// =============================================================================

#[derive(Debug, Clone)]
pub struct Expr {
    /// Unique ID for debugging and caching (not used in equality comparisons)
    pub id: u64,
    /// The kind of expression (structure)
    pub kind: ExprKind,
    /// Phase-specific simplification tracking
    pub flags: ExprFlags,
}

impl Deref for Expr {
    type Target = ExprKind;
    fn deref(&self) -> &Self::Target {
        &self.kind
    }
}

// Structural equality based on KIND only
impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

impl Eq for Expr {}

impl std::hash::Hash for Expr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
    }
}

// =============================================================================
// EXPRKIND - N-ary Sum/Product architecture
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// Constant number (e.g., 3.14, 1e10)
    Number(f64),

    /// Variable or constant symbol (e.g., "x", "a", "pi")
    /// Uses InternedSymbol for O(1) equality comparisons
    Symbol(InternedSymbol),

    /// Function call (built-in or custom)
    /// Uses InternedSymbol for O(1) name comparisons
    FunctionCall {
        name: InternedSymbol,
        args: Vec<Expr>,
    },

    /// N-ary sum: a + b + c + ...
    /// Stored flat and sorted for canonical form.
    /// Subtraction is represented as: a - b = Sum([a, Product([-1, b])])
    Sum(Vec<Arc<Expr>>),

    /// N-ary product: a * b * c * ...
    /// Stored flat and sorted for canonical form.
    Product(Vec<Arc<Expr>>),

    /// Division (binary - not associative)
    Div(Arc<Expr>, Arc<Expr>),

    /// Exponentiation (binary - not associative)  
    Pow(Arc<Expr>, Arc<Expr>),

    /// Partial derivative notation: ∂^order/∂var^order of inner expression
    Derivative {
        inner: Arc<Expr>,
        var: String,
        order: u32,
    },
}

// =============================================================================
// EXPR CONSTRUCTORS AND METHODS
// =============================================================================

impl Expr {
    /// Create a new expression with fresh ID and default flags
    pub fn new(kind: ExprKind) -> Self {
        Expr {
            id: next_id(),
            kind,
            flags: ExprFlags::default(),
        }
    }

    /// Create a new expression with specific flags
    pub fn with_flags(kind: ExprKind, flags: ExprFlags) -> Self {
        Expr {
            id: next_id(),
            kind,
            flags,
        }
    }

    // -------------------------------------------------------------------------
    // Accessor methods
    // -------------------------------------------------------------------------

    /// Check if expression is a constant number and return its value
    pub fn as_number(&self) -> Option<f64> {
        match &self.kind {
            ExprKind::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Check if this expression is the number zero (with tolerance)
    #[inline]
    pub fn is_zero_num(&self) -> bool {
        self.as_number().is_some_and(crate::traits::is_zero)
    }

    /// Check if this expression is the number one (with tolerance)
    #[inline]
    pub fn is_one_num(&self) -> bool {
        self.as_number().is_some_and(crate::traits::is_one)
    }

    /// Check if this expression is the number negative one (with tolerance)
    #[inline]
    pub fn is_neg_one_num(&self) -> bool {
        self.as_number().is_some_and(crate::traits::is_neg_one)
    }

    // -------------------------------------------------------------------------
    // Basic constructors
    // -------------------------------------------------------------------------

    /// Create a number expression
    pub fn number(n: f64) -> Self {
        Expr::new(ExprKind::Number(n))
    }

    /// Create a symbol expression (auto-interned)
    pub fn symbol(s: impl AsRef<str>) -> Self {
        Expr::new(ExprKind::Symbol(get_or_intern(s.as_ref())))
    }

    /// Create from an already-interned symbol
    pub(crate) fn from_interned(interned: InternedSymbol) -> Self {
        Expr::new(ExprKind::Symbol(interned))
    }

    // -------------------------------------------------------------------------
    // N-ary Sum constructor (smart - flattens and sorts)
    // -------------------------------------------------------------------------

    /// Create a sum expression from terms.
    /// Automatically flattens nested sums and sorts for canonical form.
    pub fn sum(terms: Vec<Expr>) -> Self {
        if terms.is_empty() {
            return Expr::number(0.0);
        }
        if terms.len() == 1 {
            return terms.into_iter().next().unwrap();
        }

        let mut flat: Vec<Arc<Expr>> = Vec::with_capacity(terms.len());

        for t in terms {
            match t.kind {
                ExprKind::Sum(inner) => flat.extend(inner),
                _ => flat.push(Arc::new(t)),
            }
        }

        if flat.len() == 1 {
            return Arc::try_unwrap(flat.pop().unwrap()).unwrap_or_else(|arc| (*arc).clone());
        }

        // Sort for canonical form
        flat.sort_by(|a, b| expr_cmp(a, b));

        Expr::new(ExprKind::Sum(flat))
    }

    /// Create sum from Arc terms (flattens and sorts for canonical form)
    pub fn sum_from_arcs(terms: Vec<Arc<Expr>>) -> Self {
        if terms.is_empty() {
            return Expr::number(0.0);
        }
        if terms.len() == 1 {
            return Arc::try_unwrap(terms.into_iter().next().unwrap())
                .unwrap_or_else(|arc| (*arc).clone());
        }

        // Flatten nested sums
        let mut flat: Vec<Arc<Expr>> = Vec::with_capacity(terms.len());
        for t in terms {
            match &t.kind {
                ExprKind::Sum(inner) => flat.extend(inner.clone()),
                _ => flat.push(t),
            }
        }

        if flat.len() == 1 {
            return Arc::try_unwrap(flat.pop().unwrap()).unwrap_or_else(|arc| (*arc).clone());
        }

        // Sort for canonical form
        flat.sort_by(|a, b| expr_cmp(a, b));
        Expr::new(ExprKind::Sum(flat))
    }

    // -------------------------------------------------------------------------
    // N-ary Product constructor (smart - flattens and sorts)
    // -------------------------------------------------------------------------

    /// Create a product expression from factors.
    /// Automatically flattens nested products and sorts for canonical form.
    pub fn product(factors: Vec<Expr>) -> Self {
        if factors.is_empty() {
            return Expr::number(1.0);
        }
        if factors.len() == 1 {
            return factors.into_iter().next().unwrap();
        }

        let mut flat: Vec<Arc<Expr>> = Vec::with_capacity(factors.len());

        for f in factors {
            match f.kind {
                ExprKind::Product(inner) => flat.extend(inner),
                _ => flat.push(Arc::new(f)),
            }
        }

        if flat.len() == 1 {
            return Arc::try_unwrap(flat.pop().unwrap()).unwrap_or_else(|arc| (*arc).clone());
        }

        // Sort for canonical form
        flat.sort_by(|a, b| expr_cmp(a, b));

        Expr::new(ExprKind::Product(flat))
    }

    /// Create product from Arc factors (flattens and sorts for canonical form)
    pub fn product_from_arcs(factors: Vec<Arc<Expr>>) -> Self {
        if factors.is_empty() {
            return Expr::number(1.0);
        }
        if factors.len() == 1 {
            return Arc::try_unwrap(factors.into_iter().next().unwrap())
                .unwrap_or_else(|arc| (*arc).clone());
        }

        // Flatten nested products
        let mut flat: Vec<Arc<Expr>> = Vec::with_capacity(factors.len());
        for f in factors {
            match &f.kind {
                ExprKind::Product(inner) => flat.extend(inner.clone()),
                _ => flat.push(f),
            }
        }

        if flat.len() == 1 {
            return Arc::try_unwrap(flat.pop().unwrap()).unwrap_or_else(|arc| (*arc).clone());
        }

        // Sort for canonical form
        flat.sort_by(|a, b| expr_cmp(a, b));
        Expr::new(ExprKind::Product(flat))
    }

    // -------------------------------------------------------------------------
    // Binary operation constructors (for legacy compatibility during migration)
    // -------------------------------------------------------------------------

    /// Create addition: a + b → Sum([a, b])
    pub fn add_expr(left: Expr, right: Expr) -> Self {
        Expr::sum(vec![left, right])
    }

    /// Create subtraction: a - b → Sum([a, Product([-1, b])])
    pub fn sub_expr(left: Expr, right: Expr) -> Self {
        let neg_right = Expr::product(vec![Expr::number(-1.0), right]);
        Expr::sum(vec![left, neg_right])
    }

    /// Create multiplication: a * b → Product([a, b])
    pub fn mul_expr(left: Expr, right: Expr) -> Self {
        Expr::product(vec![left, right])
    }

    /// Create division
    pub fn div_expr(left: Expr, right: Expr) -> Self {
        Expr::new(ExprKind::Div(Arc::new(left), Arc::new(right)))
    }

    /// Create division from Arc operands (avoids cloning if Arc ref count is 1)
    pub fn div_from_arcs(left: Arc<Expr>, right: Arc<Expr>) -> Self {
        Expr::new(ExprKind::Div(left, right))
    }

    /// Create power expression
    pub fn pow(base: Expr, exponent: Expr) -> Self {
        Expr::new(ExprKind::Pow(Arc::new(base), Arc::new(exponent)))
    }

    /// Create power from Arc operands (avoids cloning if Arc ref count is 1)
    pub fn pow_from_arcs(base: Arc<Expr>, exponent: Arc<Expr>) -> Self {
        Expr::new(ExprKind::Pow(base, exponent))
    }

    /// Create a function call expression (single argument)
    pub fn func(name: impl AsRef<str>, content: Expr) -> Self {
        Expr::new(ExprKind::FunctionCall {
            name: get_or_intern(name.as_ref()),
            args: vec![content],
        })
    }

    /// Create a multi-argument function call
    pub fn func_multi(name: impl AsRef<str>, args: Vec<Expr>) -> Self {
        Expr::new(ExprKind::FunctionCall {
            name: get_or_intern(name.as_ref()),
            args,
        })
    }

    /// Create a function call with explicit arguments using array syntax
    pub fn call<const N: usize>(name: impl AsRef<str>, args: [Expr; N]) -> Self {
        Expr::func_multi(name, args.into())
    }

    /// Create a partial derivative expression
    pub fn derivative(inner: Expr, var: String, order: u32) -> Self {
        Expr::new(ExprKind::Derivative {
            inner: Arc::new(inner),
            var,
            order,
        })
    }

    // -------------------------------------------------------------------------
    // Negation helper
    // -------------------------------------------------------------------------

    /// Negate this expression: -x = Product([-1, x])
    pub fn negate(self) -> Self {
        Expr::product(vec![Expr::number(-1.0), self])
    }

    // -------------------------------------------------------------------------
    // Analysis methods
    // -------------------------------------------------------------------------

    /// Count the total number of nodes in the AST
    pub fn node_count(&self) -> usize {
        match &self.kind {
            ExprKind::Number(_) | ExprKind::Symbol(_) => 1,
            ExprKind::FunctionCall { args, .. } => {
                1 + args.iter().map(|a| a.node_count()).sum::<usize>()
            }
            ExprKind::Sum(terms) => 1 + terms.iter().map(|t| t.node_count()).sum::<usize>(),
            ExprKind::Product(factors) => 1 + factors.iter().map(|f| f.node_count()).sum::<usize>(),
            ExprKind::Div(l, r) | ExprKind::Pow(l, r) => 1 + l.node_count() + r.node_count(),
            ExprKind::Derivative { inner, .. } => 1 + inner.node_count(),
        }
    }

    /// Get the maximum nesting depth of the AST
    pub fn max_depth(&self) -> usize {
        match &self.kind {
            ExprKind::Number(_) | ExprKind::Symbol(_) => 1,
            ExprKind::FunctionCall { args, .. } => {
                1 + args.iter().map(|a| a.max_depth()).max().unwrap_or(0)
            }
            ExprKind::Sum(terms) => 1 + terms.iter().map(|t| t.max_depth()).max().unwrap_or(0),
            ExprKind::Product(factors) => {
                1 + factors.iter().map(|f| f.max_depth()).max().unwrap_or(0)
            }
            ExprKind::Div(l, r) | ExprKind::Pow(l, r) => 1 + l.max_depth().max(r.max_depth()),
            ExprKind::Derivative { inner, .. } => 1 + inner.max_depth(),
        }
    }

    /// Check if the expression contains a specific variable
    pub fn contains_var(&self, var: &str) -> bool {
        match &self.kind {
            ExprKind::Number(_) => false,
            ExprKind::Symbol(s) => s == var,
            ExprKind::FunctionCall { args, .. } => args.iter().any(|a| a.contains_var(var)),
            ExprKind::Sum(terms) => terms.iter().any(|t| t.contains_var(var)),
            ExprKind::Product(factors) => factors.iter().any(|f| f.contains_var(var)),
            ExprKind::Div(l, r) | ExprKind::Pow(l, r) => l.contains_var(var) || r.contains_var(var),
            ExprKind::Derivative { inner, var: v, .. } => v == var || inner.contains_var(var),
        }
    }

    /// Check if the expression contains any free variables
    pub fn has_free_variables(&self, excluded: &std::collections::HashSet<String>) -> bool {
        match &self.kind {
            ExprKind::Number(_) => false,
            ExprKind::Symbol(name) => !excluded.contains(name.as_ref()),
            ExprKind::Sum(terms) => terms.iter().any(|t| t.has_free_variables(excluded)),
            ExprKind::Product(factors) => factors.iter().any(|f| f.has_free_variables(excluded)),
            ExprKind::Div(l, r) | ExprKind::Pow(l, r) => {
                l.has_free_variables(excluded) || r.has_free_variables(excluded)
            }
            ExprKind::FunctionCall { args, .. } => {
                args.iter().any(|arg| arg.has_free_variables(excluded))
            }
            ExprKind::Derivative { inner, var, .. } => {
                !excluded.contains(var) || inner.has_free_variables(excluded)
            }
        }
    }

    /// Collect all variables in the expression
    pub fn variables(&self) -> std::collections::HashSet<String> {
        let mut vars = std::collections::HashSet::new();
        self.collect_variables(&mut vars);
        vars
    }

    fn collect_variables(&self, vars: &mut std::collections::HashSet<String>) {
        match &self.kind {
            ExprKind::Symbol(s) => {
                if let Some(name) = s.name() {
                    vars.insert(name.to_string());
                }
            }
            ExprKind::FunctionCall { args, .. } => {
                for arg in args {
                    arg.collect_variables(vars);
                }
            }
            ExprKind::Sum(terms) => {
                for t in terms {
                    t.collect_variables(vars);
                }
            }
            ExprKind::Product(factors) => {
                for f in factors {
                    f.collect_variables(vars);
                }
            }
            ExprKind::Div(l, r) | ExprKind::Pow(l, r) => {
                l.collect_variables(vars);
                r.collect_variables(vars);
            }
            ExprKind::Derivative { inner, var, .. } => {
                vars.insert(var.clone());
                inner.collect_variables(vars);
            }
            ExprKind::Number(_) => {}
        }
    }

    /// Create a deep clone (with new IDs)
    pub fn deep_clone(&self) -> Expr {
        match &self.kind {
            ExprKind::Number(n) => Expr::number(*n),
            ExprKind::Symbol(s) => Expr::from_interned(s.clone()),
            ExprKind::FunctionCall { name, args } => Expr::new(ExprKind::FunctionCall {
                name: name.clone(),
                args: args.iter().map(|arg| arg.deep_clone()).collect(),
            }),
            ExprKind::Sum(terms) => {
                let cloned: Vec<Arc<Expr>> = terms
                    .iter()
                    .map(|t| Arc::new(t.as_ref().deep_clone()))
                    .collect();
                Expr::new(ExprKind::Sum(cloned))
            }
            ExprKind::Product(factors) => {
                let cloned: Vec<Arc<Expr>> = factors
                    .iter()
                    .map(|f| Arc::new(f.as_ref().deep_clone()))
                    .collect();
                Expr::new(ExprKind::Product(cloned))
            }
            ExprKind::Div(a, b) => Expr::div_expr(a.as_ref().deep_clone(), b.as_ref().deep_clone()),
            ExprKind::Pow(a, b) => Expr::pow(a.as_ref().deep_clone(), b.as_ref().deep_clone()),
            ExprKind::Derivative { inner, var, order } => {
                Expr::derivative(inner.as_ref().deep_clone(), var.clone(), *order)
            }
        }
    }

    // -------------------------------------------------------------------------
    // Convenience methods
    // -------------------------------------------------------------------------

    /// Differentiate with respect to a variable
    pub fn diff(&self, var: &str) -> Result<Expr, crate::DiffError> {
        crate::Diff::new().differentiate(self.clone(), &crate::symb(var))
    }

    /// Simplify this expression
    pub fn simplified(&self) -> Result<Expr, crate::DiffError> {
        crate::Simplify::new().simplify(self.clone())
    }

    /// Fold over the expression tree (pre-order)
    pub fn fold<T, F>(&self, init: T, f: F) -> T
    where
        F: Fn(T, &Expr) -> T + Copy,
    {
        let acc = f(init, self);
        match &self.kind {
            ExprKind::Number(_) | ExprKind::Symbol(_) => acc,
            ExprKind::FunctionCall { args, .. } => args.iter().fold(acc, |a, arg| arg.fold(a, f)),
            ExprKind::Sum(terms) => terms.iter().fold(acc, |a, t| t.fold(a, f)),
            ExprKind::Product(factors) => factors.iter().fold(acc, |a, f_| f_.fold(a, f)),
            ExprKind::Div(l, r) | ExprKind::Pow(l, r) => {
                let acc = l.fold(acc, f);
                r.fold(acc, f)
            }
            ExprKind::Derivative { inner, .. } => inner.fold(acc, f),
        }
    }

    /// Transform the expression tree (post-order)
    pub fn map<F>(&self, f: F) -> Expr
    where
        F: Fn(&Expr) -> Expr + Copy,
    {
        let transformed = match &self.kind {
            ExprKind::Number(_) | ExprKind::Symbol(_) => self.clone(),
            ExprKind::FunctionCall { name, args } => Expr::new(ExprKind::FunctionCall {
                name: name.clone(),
                args: args.iter().map(|arg| arg.map(f)).collect(),
            }),
            ExprKind::Sum(terms) => {
                let mapped: Vec<Arc<Expr>> =
                    terms.iter().map(|t| Arc::new(t.as_ref().map(f))).collect();
                Expr::new(ExprKind::Sum(mapped))
            }
            ExprKind::Product(factors) => {
                let mapped: Vec<Arc<Expr>> = factors
                    .iter()
                    .map(|fac| Arc::new(fac.as_ref().map(f)))
                    .collect();
                Expr::new(ExprKind::Product(mapped))
            }
            ExprKind::Div(a, b) => Expr::div_expr(a.map(f), b.map(f)),
            ExprKind::Pow(a, b) => Expr::pow(a.map(f), b.map(f)),
            ExprKind::Derivative { inner, var, order } => {
                Expr::derivative(inner.map(f), var.clone(), *order)
            }
        };
        f(&transformed)
    }

    /// Substitute a variable with another expression
    pub fn substitute(&self, var: &str, replacement: &Expr) -> Expr {
        self.map(|node| {
            if let ExprKind::Symbol(s) = &node.kind
                && s == var
            {
                return replacement.clone();
            }
            node.clone()
        })
    }

    /// Evaluate expression with given variable values
    pub fn evaluate(&self, vars: &std::collections::HashMap<&str, f64>) -> Expr {
        self.evaluate_with_custom(vars, &std::collections::HashMap::new())
    }

    /// Evaluate with custom function evaluators
    pub fn evaluate_with_custom(
        &self,
        vars: &std::collections::HashMap<&str, f64>,
        custom_evals: &CustomEvalMap,
    ) -> Expr {
        match &self.kind {
            ExprKind::Number(n) => Expr::number(*n),
            ExprKind::Symbol(s) => {
                if let Some(name) = s.name()
                    && let Some(&val) = vars.get(name)
                {
                    return Expr::number(val);
                }
                self.clone()
            }
            ExprKind::FunctionCall { name, args } => {
                let eval_args: Vec<Expr> = args
                    .iter()
                    .map(|a| a.evaluate_with_custom(vars, custom_evals))
                    .collect();

                let numeric_args: Option<Vec<f64>> =
                    eval_args.iter().map(|e| e.as_number()).collect();

                if let Some(args_vec) = numeric_args {
                    if let Some(custom_eval) = custom_evals.get(name.as_str())
                        && let Some(result) = custom_eval(&args_vec)
                    {
                        return Expr::number(result);
                    }
                    if let Some(func_def) = crate::functions::registry::Registry::get(name.as_str())
                        && let Some(result) = (func_def.eval)(&args_vec)
                    {
                        return Expr::number(result);
                    }
                }

                Expr::new(ExprKind::FunctionCall {
                    name: name.clone(),
                    args: eval_args,
                })
            }
            ExprKind::Sum(terms) => {
                let eval_terms: Vec<Expr> = terms
                    .iter()
                    .map(|t| t.evaluate_with_custom(vars, custom_evals))
                    .collect();

                // Try to combine all numeric terms
                let (nums, others): (Vec<_>, Vec<_>) = eval_terms
                    .into_iter()
                    .partition(|e| matches!(e.kind, ExprKind::Number(_)));

                let num_sum: f64 = nums.iter().filter_map(|e| e.as_number()).sum();

                let mut result_terms: Vec<Expr> = others;
                if num_sum != 0.0 || result_terms.is_empty() {
                    result_terms.push(Expr::number(num_sum));
                }

                if result_terms.len() == 1 {
                    result_terms.pop().unwrap()
                } else {
                    Expr::sum(result_terms)
                }
            }
            ExprKind::Product(factors) => {
                let eval_factors: Vec<Expr> = factors
                    .iter()
                    .map(|f| f.evaluate_with_custom(vars, custom_evals))
                    .collect();

                // Try to combine all numeric factors
                let (nums, others): (Vec<_>, Vec<_>) = eval_factors
                    .into_iter()
                    .partition(|e| matches!(e.kind, ExprKind::Number(_)));

                let num_prod: f64 = nums.iter().filter_map(|e| e.as_number()).product();

                if num_prod == 0.0 {
                    return Expr::number(0.0);
                }

                let mut result_factors: Vec<Expr> = others;
                if num_prod != 1.0 || result_factors.is_empty() {
                    result_factors.insert(0, Expr::number(num_prod));
                }

                if result_factors.len() == 1 {
                    result_factors.pop().unwrap()
                } else {
                    Expr::product(result_factors)
                }
            }
            ExprKind::Div(a, b) => {
                let ea = a.evaluate_with_custom(vars, custom_evals);
                let eb = b.evaluate_with_custom(vars, custom_evals);
                match (&ea.kind, &eb.kind) {
                    (ExprKind::Number(x), ExprKind::Number(y)) if *y != 0.0 => Expr::number(x / y),
                    _ => Expr::div_expr(ea, eb),
                }
            }
            ExprKind::Pow(a, b) => {
                let ea = a.evaluate_with_custom(vars, custom_evals);
                let eb = b.evaluate_with_custom(vars, custom_evals);
                match (&ea.kind, &eb.kind) {
                    (ExprKind::Number(x), ExprKind::Number(y)) => Expr::number(x.powf(*y)),
                    _ => Expr::pow(ea, eb),
                }
            }
            ExprKind::Derivative { inner, var, order } => Expr::derivative(
                inner.evaluate_with_custom(vars, custom_evals),
                var.clone(),
                *order,
            ),
        }
    }
}

// =============================================================================
// CANONICAL ORDERING FOR EXPRESSIONS
// =============================================================================

/// Compare expressions for canonical ordering.
/// Order: Numbers < Symbols (alphabetical) < Functions < Sum < Product < Div < Pow
fn expr_cmp(a: &Expr, b: &Expr) -> CmpOrdering {
    use ExprKind::*;

    // 1. Numbers always come first
    if let (Number(x), Number(y)) = (&a.kind, &b.kind) {
        return x.partial_cmp(y).unwrap_or(CmpOrdering::Equal);
    }
    if matches!(a.kind, Number(_)) {
        return CmpOrdering::Less;
    }
    if matches!(b.kind, Number(_)) {
        return CmpOrdering::Greater;
    }

    // Helper: Extract sort key (Base, Exponent, Coefficient)
    // Returns: (Base, Exponent, Coefficient, IsAtomic)
    // Note: Exponent is Option<&Expr> (None means 1), Coefficient is f64
    fn extract_key(e: &Expr) -> (&Expr, Option<&Expr>, f64, bool) {
        match &e.kind {
            // Case: x^2 -> Base x, Exp 2, Coeff 1
            Pow(b, exp) => (b.as_ref(), Some(exp.as_ref()), 1.0, false),

            // Case: 2*x -> Base x, Exp 1, Coeff 2 (Only if Product starts with Number)
            Product(factors) if factors.len() == 2 => {
                if let Number(n) = &factors[0].kind {
                    (&factors[1], None, *n, false)
                } else {
                    (e, None, 1.0, true)
                }
            }
            // Case: x -> Base x, Exp 1, Coeff 1
            _ => (e, None, 1.0, true),
        }
    }

    let (base_a, exp_a, coeff_a, atomic_a) = extract_key(a);
    let (base_b, exp_b, coeff_b, atomic_b) = extract_key(b);

    // 2. If both are atomic (e.g., Symbol vs Symbol), use strict type sorting fallback
    // This prevents infinite recursion (comparing x vs x)
    if atomic_a && atomic_b {
        return expr_cmp_type_strict(a, b);
    }

    // 3. Compare Bases (Recursively)
    // Recursion is safe because at least one is composite (smaller depth)
    let base_cmp = expr_cmp(base_a, base_b);
    if base_cmp != CmpOrdering::Equal {
        return base_cmp;
    }

    // logic: 1 vs 2 -> Less
    // logic: 1 vs 1 -> Equal
    // logic: 2 vs 1 -> Greater

    // If one has explicit exponent and one implied 1:
    // x (1) vs x^2 (2) -> 1 < 2 -> Less
    match (exp_a, exp_b) {
        (Some(e_a), Some(e_b)) => {
            let exp_cmp = expr_cmp(e_a, e_b);
            if exp_cmp != CmpOrdering::Equal {
                return exp_cmp;
            }
        }
        (Some(e_a), None) => {
            // Compare expr e_a vs 1.0
            // Usually e_a > 1 (like 2, 3), but could be 0.5
            // Safer to compare full expr
            let one = Expr::number(1.0);
            let exp_cmp = expr_cmp(e_a, &one);
            if exp_cmp != CmpOrdering::Equal {
                return exp_cmp;
            }
        }
        (None, Some(e_b)) => {
            // Compare 1.0 vs e_b
            let one = Expr::number(1.0);
            let exp_cmp = expr_cmp(&one, e_b);
            if exp_cmp != CmpOrdering::Equal {
                return exp_cmp;
            }
        }
        (None, None) => {} // Both 1
    }

    // 5. Compare Coefficients (1 < 2)
    // x vs 2x -> 1 < 2 -> Less -> x, 2x
    coeff_a.partial_cmp(&coeff_b).unwrap_or(CmpOrdering::Equal)
}

// Fallback: Original strict type comparisons for atomic terms
fn expr_cmp_type_strict(a: &Expr, b: &Expr) -> CmpOrdering {
    use ExprKind::*;
    match (&a.kind, &b.kind) {
        (Symbol(x), Symbol(y)) => x.as_ref().cmp(y.as_ref()),
        (Symbol(_), _) => CmpOrdering::Less,
        (_, Symbol(_)) => CmpOrdering::Greater,

        (FunctionCall { name: n1, args: a1 }, FunctionCall { name: n2, args: a2 }) => {
            n1.cmp(n2).then_with(|| {
                for (x, y) in a1.iter().zip(a2.iter()) {
                    match expr_cmp(x, y) {
                        CmpOrdering::Equal => continue,
                        other => return other,
                    }
                }
                a1.len().cmp(&a2.len())
            })
        }
        (FunctionCall { .. }, _) => CmpOrdering::Less,
        (_, FunctionCall { .. }) => CmpOrdering::Greater,

        (Sum(t1), Sum(t2)) => t1.len().cmp(&t2.len()).then_with(|| {
            for (x, y) in t1.iter().zip(t2.iter()) {
                match expr_cmp(x, y) {
                    CmpOrdering::Equal => continue,
                    other => return other,
                }
            }
            CmpOrdering::Equal
        }),
        (Sum(_), _) => CmpOrdering::Less,
        (_, Sum(_)) => CmpOrdering::Greater,

        // Products are handled as atomics if they don't match the "Coeff * Rest" pattern
        (Product(f1), Product(f2)) => f1.len().cmp(&f2.len()).then_with(|| {
            for (x, y) in f1.iter().zip(f2.iter()) {
                match expr_cmp(x, y) {
                    CmpOrdering::Equal => continue,
                    other => return other,
                }
            }
            CmpOrdering::Equal
        }),
        (Product(_), _) => CmpOrdering::Less,
        (_, Product(_)) => CmpOrdering::Greater,

        (Div(l1, r1), Div(l2, r2)) => expr_cmp(l1, l2).then_with(|| expr_cmp(r1, r2)),
        (Div(_, _), _) => CmpOrdering::Less,
        (_, Div(_, _)) => CmpOrdering::Greater,

        (Pow(b1, e1), Pow(b2, e2)) => expr_cmp(b1, b2).then_with(|| expr_cmp(e1, e2)),
        (Pow(_, _), _) => CmpOrdering::Less,
        (_, Pow(_, _)) => CmpOrdering::Greater,

        (
            Derivative {
                inner: i1,
                var: v1,
                order: o1,
            },
            Derivative {
                inner: i2,
                var: v2,
                order: o2,
            },
        ) => v1
            .cmp(v2)
            .then_with(|| o1.cmp(o2))
            .then_with(|| expr_cmp(i1, i2)),

        _ => CmpOrdering::Equal, // Should be covered by match arms above but safe fallback
    }
}

// =============================================================================
// HASH FOR EXPRKIND
// =============================================================================

impl std::hash::Hash for ExprKind {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            ExprKind::Number(n) => n.to_bits().hash(state),
            ExprKind::Symbol(s) => s.hash(state),
            ExprKind::FunctionCall { name, args } => {
                name.hash(state);
                args.hash(state);
            }
            ExprKind::Sum(terms) => {
                terms.len().hash(state);
                for t in terms {
                    t.hash(state);
                }
            }
            ExprKind::Product(factors) => {
                factors.len().hash(state);
                for f in factors {
                    f.hash(state);
                }
            }
            ExprKind::Div(l, r) | ExprKind::Pow(l, r) => {
                l.hash(state);
                r.hash(state);
            }
            ExprKind::Derivative { inner, var, order } => {
                inner.hash(state);
                var.hash(state);
                order.hash(state);
            }
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum_flattening() {
        let x = Expr::symbol("x");
        let y = Expr::symbol("y");
        let z = Expr::symbol("z");

        // (x + y) + z should flatten to Sum([x, y, z])
        let inner = Expr::sum(vec![x.clone(), y.clone()]);
        let outer = Expr::sum(vec![inner, z.clone()]);

        match &outer.kind {
            ExprKind::Sum(terms) => assert_eq!(terms.len(), 3),
            _ => panic!("Expected Sum"),
        }
    }

    #[test]
    fn test_product_flattening() {
        let a = Expr::symbol("a");
        let b = Expr::symbol("b");
        let c = Expr::symbol("c");

        // (a * b) * c should flatten to Product([a, b, c])
        let inner = Expr::product(vec![a.clone(), b.clone()]);
        let outer = Expr::product(vec![inner, c.clone()]);

        match &outer.kind {
            ExprKind::Product(factors) => assert_eq!(factors.len(), 3),
            _ => panic!("Expected Product"),
        }
    }

    #[test]
    fn test_subtraction_as_sum() {
        let x = Expr::symbol("x");
        let y = Expr::symbol("y");

        // x - y = Sum([x, Product([-1, y])])
        let result = Expr::sub_expr(x.clone(), y.clone());

        match &result.kind {
            ExprKind::Sum(terms) => {
                assert_eq!(terms.len(), 2);
            }
            _ => panic!("Expected Sum from subtraction"),
        }
    }

    #[test]
    fn test_expr_flags() {
        let mut flags = ExprFlags::default();
        assert!(flags.needs_numeric());
        assert!(flags.needs_expansion());

        flags.mark_numeric();
        assert!(!flags.needs_numeric());
        assert!(flags.needs_expansion());

        flags.mark_all_simplified();
        assert!(flags.is_simplified());
    }

    #[test]
    fn test_epoch_invalidation() {
        let mut flags = ExprFlags::default();
        flags.mark_all_simplified();
        assert!(flags.is_simplified());

        // Simulate invalidation
        epochs::NUMERIC.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // After invalidation, should need simplification again
        assert!(!flags.is_simplified());
    }
}
