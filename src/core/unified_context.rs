//! Unified Context for `symb_anafis` operations
//!
//! Provides a single `Context` type that combines:
//! - Isolated symbol registry (for parsing hints)
//! - User-defined functions with optional body and partial derivatives
//!
//! # Example
//! ```
//! use symb_anafis::{Context, UserFunction, Expr};
//!
//! let ctx = Context::new()
//!     .with_symbol("x")
//!     .with_symbol("y")
//!     .with_symbol("R")  // Multi-character symbol for parsing
//!     .with_function("f", UserFunction::new(1..=1)
//!         .body(|args| (*args[0]).clone().pow(2.0) + 1.0)
//!         .partial(0, |args| 2.0 * (*args[0]).clone()).expect("valid arg"));
//!
//! let x = ctx.symb("x");
//! let expr = x.pow(2.0);
//! ```

use super::symbol::{InternedSymbol, NEXT_SYMBOL_ID, Symbol};
use crate::Expr;
use std::collections::{HashMap, HashSet};
use std::ops::RangeInclusive;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

// =============================================================================
// Type Aliases for Thread-Safe Function Types
// =============================================================================

/// Thread-safe symbolic body function for user-defined functions.
/// Takes argument expressions (as `Arc<Expr>`) and returns the function body as an expression.
pub type BodyFn = Arc<dyn Fn(&[Arc<Expr>]) -> Expr + Send + Sync>;

/// Thread-safe partial derivative function.
/// Takes argument expressions (as `Arc<Expr>`) and returns the partial derivative expression.
pub type PartialFn = Arc<dyn Fn(&[Arc<Expr>]) -> Expr + Send + Sync>;

// =============================================================================
// UserFunction
// =============================================================================

/// A user-defined function with optional body expression and partial derivatives.
///
/// Functions can be registered with just a name (for parsing), or with full
/// body/partial definitions for symbolic evaluation and differentiation.
///
/// # Example
/// ```
/// use symb_anafis::{UserFunction, Expr};
///
/// // Function with body and partial derivative - all using Expr!
/// // f(x) = x^2 + 1, so f'(x) = 2x
/// let f = UserFunction::new(1..=1)
///     .body(|args| (*args[0]).clone().pow(2.0) + 1.0)
///     .partial(0, |args| 2.0 * (*args[0]).clone()).expect("valid arg");
///
/// // Function with just name (for parsing)
/// let g = UserFunction::new(2..=2);  // Parser recognizes g(x, y)
/// ```
#[derive(Clone)]
pub struct UserFunction {
    /// Arity range (min..=max arguments)
    pub(crate) arity: RangeInclusive<usize>,
    /// Optional symbolic body function (returns Expr given arg expressions)
    pub(crate) body: Option<BodyFn>,
    /// Partial derivatives by argument index (∂f/∂arg\[i\])
    pub(crate) partials: HashMap<usize, PartialFn>,
}

impl Default for UserFunction {
    fn default() -> Self {
        Self {
            arity: 0..=usize::MAX, // Default: any arity
            body: None,
            partials: HashMap::new(),
        }
    }
}

impl UserFunction {
    /// Create a new user function with the specified arity range.
    ///
    /// # Example
    /// ```
    /// use symb_anafis::UserFunction;
    /// let f = UserFunction::new(1..=2);  // 1 or 2 arguments
    /// ```
    #[must_use] 
    pub fn new(arity: RangeInclusive<usize>) -> Self {
        Self {
            arity,
            body: None,
            partials: HashMap::new(),
        }
    }

    /// Set the symbolic body function.
    ///
    /// The body function receives argument expressions and returns an Expr.
    /// This is used for both symbolic manipulation and numeric evaluation.
    ///
    /// # Example
    /// ```
    /// use symb_anafis::{UserFunction, Expr};
    /// // f(x) = x^2 + 1
    /// let f = UserFunction::new(1..=1)
    ///     .body(|args| (*args[0]).clone().pow(2.0) + 1.0);
    /// ```
    #[must_use]
    pub fn body<F>(mut self, f: F) -> Self
    where
        F: Fn(&[Arc<Expr>]) -> Expr + Send + Sync + 'static,
    {
        self.body = Some(Arc::new(f));
        self
    }

    /// Set the body using a pre-wrapped Arc (for FFI/Python bindings).
    #[must_use]
    pub fn body_arc(mut self, f: BodyFn) -> Self {
        self.body = Some(f);
        self
    }

    /// Add a partial derivative for argument at index `i`.
    ///
    /// The function receives all argument expressions as `Arc<Expr>` and returns `∂f/∂arg[i]`.
    ///
    /// # Returns
    /// - `Ok(Self)` if successful
    /// - `Err(DiffError)` if `arg_idx` exceeds the maximum arity
    ///
    /// # Example
    /// ```
    /// use symb_anafis::{UserFunction, Expr};
    ///
    /// // f(x) = x², so ∂f/∂x = 2x
    /// let f = UserFunction::new(1..=1)
    ///     .partial(0, |args| Expr::mul_expr(Expr::number(2.0), (*args[0]).clone()))
    ///     .expect("valid arg index");
    /// ```
    ///
    /// # Errors
    /// Returns `DiffError::InvalidPartialIndex` if `arg_idx` exceeds the maximum arity.
    pub fn partial<F>(mut self, arg_idx: usize, f: F) -> Result<Self, crate::DiffError>
    where
        F: Fn(&[Arc<Expr>]) -> Expr + Send + Sync + 'static,
    {
        // Validate that arg_idx is within bounds (skip check for unbounded arity)
        let max_arity = *self.arity.end();
        if max_arity != usize::MAX && arg_idx >= max_arity {
            return Err(crate::DiffError::InvalidPartialIndex {
                index: arg_idx,
                max_arity,
            });
        }
        self.partials.insert(arg_idx, Arc::new(f));
        Ok(self)
    }

    /// Add a partial derivative using a pre-wrapped Arc (for FFI/Python bindings).
    ///
    /// # Returns
    /// - `Ok(Self)` if successful
    /// - `Err(DiffError)` if `arg_idx` exceeds the maximum arity
    ///
    /// # Errors
    /// Returns `DiffError::InvalidPartialIndex` if `arg_idx` exceeds the maximum arity.
    pub fn partial_arc(mut self, arg_idx: usize, f: PartialFn) -> Result<Self, crate::DiffError> {
        let max_arity = *self.arity.end();
        if max_arity != usize::MAX && arg_idx >= max_arity {
            return Err(crate::DiffError::InvalidPartialIndex {
                index: arg_idx,
                max_arity,
            });
        }
        self.partials.insert(arg_idx, f);
        Ok(self)
    }

    /// Check if this function has a body expression defined.
    #[inline]
    #[must_use] 
    pub fn has_body(&self) -> bool {
        self.body.is_some()
    }

    /// Check if this function has a partial derivative for the given argument.
    #[inline]
    #[must_use] 
    pub fn has_partial(&self, arg_idx: usize) -> bool {
        self.partials.contains_key(&arg_idx)
    }

    /// Check if the given argument count is valid for this function.
    #[inline]
    #[must_use] 
    pub fn accepts_arity(&self, n: usize) -> bool {
        self.arity.contains(&n)
    }
}

impl std::fmt::Debug for UserFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserFunction")
            .field("arity", &self.arity)
            .field("has_body", &self.body.is_some())
            .field("partials", &self.partials.keys().collect::<Vec<_>>())
            .finish()
    }
}

// =============================================================================
// Context
// =============================================================================

/// Global counter for context IDs
static NEXT_CONTEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Inner data for Context (behind `Arc<RwLock>`)
#[derive(Debug, Default)]
struct ContextInner {
    /// Isolated symbol registry: name -> `InternedSymbol`
    symbols: HashMap<String, InternedSymbol>,
}

/// Unified context for all `symb_anafis` operations.
///
/// Combines:
/// - **Isolated symbol registry** (for parsing hints - telling parser about multi-char symbols)
/// - **User-defined functions** (with optional body and partial derivatives)
///
/// # Example
/// ```
/// use symb_anafis::{Context, UserFunction, Expr};
///
/// let ctx = Context::new()
///     .with_symbol("x")
///     .with_symbol("alpha")  // Multi-char symbol for parsing
///     .with_function("f", UserFunction::new(1..=1)
///         .body(|args| (*args[0]).clone().sin()));
///
/// let x = ctx.symb("x");
/// let expr = x.pow(2.0) + Expr::func("f", x);
/// ```
#[derive(Clone)]
pub struct Context {
    /// Unique ID for this context
    id: u64,
    /// Thread-safe symbol registry (isolated per context)
    inner: Arc<RwLock<ContextInner>>,
    /// User-defined functions
    user_functions: HashMap<String, UserFunction>,
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self {
            id: NEXT_CONTEXT_ID.fetch_add(1, Ordering::Relaxed),
            inner: Arc::new(RwLock::new(ContextInner::default())),
            user_functions: HashMap::new(),
        }
    }

    /// Get the context's unique ID.
    #[inline]
    #[must_use] 
    pub const fn id(&self) -> u64 {
        self.id
    }

    // =========================================================================
    // Symbol Methods (for parsing hints)
    // =========================================================================

    /// Register a symbol name (builder pattern).
    ///
    /// This pre-registers a symbol in the context's isolated registry.
    /// Useful for telling the parser about multi-character variable names.
    #[must_use] 
    pub fn with_symbol(self, name: &str) -> Self {
        self.register_symbol(name);
        self
    }

    /// Add multiple symbols (builder pattern).
    #[must_use]
    pub fn with_symbols<I, S>(self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for name in names {
            self.register_symbol(name.as_ref());
        }
        self
    }

    /// Create or get a symbol in this context.
    ///
    /// If a symbol with this name already exists, returns it.
    /// Otherwise, creates a new symbol with a unique ID.
    ///
    /// # Panics
    /// Panics if the internal lock is poisoned (indicates a prior panic while holding the lock).
    #[must_use] 
    pub fn symb(&self, name: &str) -> Symbol {
        let mut inner = self.inner.write().expect("Context lock poisoned");

        if let Some(existing) = inner.symbols.get(name) {
            return Symbol::from_id(existing.id());
        }

        // Create new symbol with isolated ID
        let interned = InternedSymbol::new_named_for_context(name, &NEXT_SYMBOL_ID);
        let id = interned.id();

        // Register in global ID registry so Symbol::name() works
        crate::core::symbol::register_in_id_registry(id, interned.clone());

        // Save in local context
        inner.symbols.insert(name.to_string(), interned);
        drop(inner);

        Symbol::from_id(id)
    }

    /// Register a symbol (mutating version).
    fn register_symbol(&self, name: &str) {
        let _ = self.symb(name);
    }

    /// Check if a symbol exists in this context.
    ///
    /// # Panics
    /// Panics if the internal lock is poisoned.
    #[must_use] 
    pub fn contains_symbol(&self, name: &str) -> bool {
        self.inner
            .read()
            .expect("Context lock poisoned")
            .symbols
            .contains_key(name)
    }

    /// Get an existing symbol by name (returns None if not found).
    ///
    /// # Panics
    /// Panics if the internal lock is poisoned.
    #[must_use] 
    pub fn get_symbol(&self, name: &str) -> Option<Symbol> {
        self.inner
            .read()
            .expect("Context lock poisoned")
            .symbols
            .get(name)
            .map(|s| Symbol::from_id(s.id()))
    }

    /// List all symbol names in this context.
    ///
    /// # Panics
    /// Panics if the internal lock is poisoned.
    #[must_use] 
    pub fn symbol_names(&self) -> Vec<String> {
        self.inner
            .read()
            .expect("Context lock poisoned")
            .symbols
            .keys()
            .cloned()
            .collect()
    }

    /// Get symbol names as a `HashSet` (for parser compatibility).
    ///
    /// # Panics
    /// Panics if the internal lock is poisoned.
    #[must_use] 
    pub fn symbol_names_set(&self) -> HashSet<String> {
        self.inner
            .read()
            .expect("Context lock poisoned")
            .symbols
            .keys()
            .cloned()
            .collect()
    }

    // =========================================================================
    // User Function Methods
    // =========================================================================

    /// Register a user-defined function (builder pattern).
    #[must_use] 
    pub fn with_function(mut self, name: &str, func: UserFunction) -> Self {
        self.user_functions.insert(name.to_string(), func);
        self
    }

    /// Register a function name only (for parsing, no eval/partial).
    ///
    /// This allows the parser to recognize `f(x)` without requiring
    /// an evaluation or derivative definition.
    #[must_use] 
    pub fn with_function_name(mut self, name: &str) -> Self {
        if !self.user_functions.contains_key(name) {
            // Create a placeholder with default arity (any number of args)
            self.user_functions
                .insert(name.to_string(), UserFunction::new(0..=usize::MAX));
        }
        self
    }

    /// Register multiple function names (for parsing).
    #[must_use]
    pub fn with_function_names<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for name in names {
            self = self.with_function_name(name.as_ref());
        }
        self
    }

    /// Get a user function by name.
    #[inline]
    #[must_use] 
    pub fn get_user_fn(&self, name: &str) -> Option<&UserFunction> {
        self.user_functions.get(name)
    }

    /// Check if a function is registered.
    #[inline]
    #[must_use] 
    pub fn has_function(&self, name: &str) -> bool {
        self.user_functions.contains_key(name)
    }

    /// Get all registered function names.
    #[must_use] 
    pub fn function_names(&self) -> HashSet<String> {
        self.user_functions.keys().cloned().collect()
    }

    /// Get the body function for a user function.
    #[inline]
    #[must_use] 
    pub fn get_body(&self, name: &str) -> Option<&BodyFn> {
        self.user_functions.get(name).and_then(|f| f.body.as_ref())
    }

    /// Get the partial derivative function for a user function argument.
    #[inline]
    #[must_use] 
    pub fn get_partial(&self, name: &str, arg_idx: usize) -> Option<&PartialFn> {
        self.user_functions
            .get(name)
            .and_then(|f| f.partials.get(&arg_idx))
    }

    /// Get the body function by symbol ID (for `CompiledEvaluator`).
    ///
    /// Looks up the symbol name from the global registry, then finds the user function.
    #[inline]
    #[must_use] 
    pub fn get_body_by_id(&self, id: u64) -> Option<&BodyFn> {
        // Look up symbol name from global registry
        if let Some(sym) = crate::core::symbol::lookup_by_id(id)
            && let Some(name) = sym.name()
        {
            return self.get_body(name); // Removed needless borrow &name
        }
        None
    }

    /// Get a user function by symbol ID (for `CompiledEvaluator`).
    #[inline]
    #[must_use] 
    pub fn get_user_fn_by_id(&self, id: u64) -> Option<&UserFunction> {
        if let Some(sym) = crate::core::symbol::lookup_by_id(id)
            && let Some(name) = sym.name()
        {
            return self.user_functions.get(name); // Removed needless deref &*name
        }
        None
    }

    // =========================================================================
    // Remove/Clear Methods (for memory management)
    // =========================================================================

    /// Remove a specific symbol from this context.
    /// Returns true if the symbol was removed.
    ///
    /// # Panics
    /// Panics if the internal lock is poisoned.
    pub fn remove_symbol(&mut self, name: &str) -> bool {
        self.inner
            .write()
            .expect("Context lock poisoned")
            .symbols
            .remove(name)
            .is_some()
    }

    /// Remove a specific user-defined function from this context.
    /// Returns true if it was removed.
    pub fn remove_function(&mut self, name: &str) -> bool {
        self.user_functions.remove(name).is_some()
    }

    /// Clear all symbols from this context.
    ///
    /// # Panics
    /// Panics if the internal lock is poisoned.
    pub fn clear_symbols(&mut self) {
        self.inner
            .write()
            .expect("Context lock poisoned")
            .symbols
            .clear();
    }

    /// Clear all user-defined functions from this context.
    pub fn clear_functions(&mut self) {
        self.user_functions.clear();
    }

    /// Clear everything from this context (symbols, functions).
    pub fn clear_all(&mut self) {
        self.clear_symbols();
        self.clear_functions();
    }

    /// Get the number of symbols in this context.
    ///
    /// # Panics
    /// Panics if the internal lock is poisoned.
    #[must_use] 
    pub fn symbol_count(&self) -> usize {
        self.inner
            .read()
            .expect("Context lock poisoned")
            .symbols
            .len()
    }

    /// Check if context has no symbols and no functions.
    #[must_use] 
    pub fn is_empty(&self) -> bool {
        self.symbol_count() == 0 && self.user_functions.is_empty()
    }
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context")
            .field("id", &self.id)
            .field("symbols", &self.symbol_names())
            .field(
                "user_functions",
                &self.user_functions.keys().collect::<Vec<_>>(),
            )
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = Context::new();
        assert!(ctx.function_names().is_empty());
    }

    #[test]
    fn test_context_with_symbol() {
        let ctx = Context::new().with_symbol("x").with_symbol("y");
        assert!(ctx.contains_symbol("x"));
        assert!(ctx.contains_symbol("y"));
        assert!(!ctx.contains_symbol("z"));
    }

    #[test]
    fn test_context_symbol_isolation() {
        let ctx1 = Context::new().with_symbol("x");
        let ctx2 = Context::new().with_symbol("x");

        // Same name, different IDs (isolated)
        let x1 = ctx1.symb("x");
        let x2 = ctx2.symb("x");
        assert_ne!(x1.id(), x2.id());
    }

    #[test]
    fn test_context_user_function() {
        let ctx = Context::new().with_function(
            "f",
            UserFunction::new(1..=1).body(|args| 2.0 * (*args[0]).clone()),
        );

        assert!(ctx.has_function("f"));
        assert!(!ctx.has_function("g"));

        let f = ctx.get_user_fn("f").unwrap();
        assert!(f.has_body());
        assert!(f.accepts_arity(1));
        assert!(!f.accepts_arity(2));
    }

    #[test]
    fn test_context_function_name_only() {
        let ctx = Context::new().with_function_name("g");

        assert!(ctx.has_function("g"));
        let g = ctx.get_user_fn("g").unwrap();
        assert!(!g.has_body()); // No body defined
    }

    #[test]
    fn test_user_function_partial() {
        let f = UserFunction::new(2..=2)
            .partial(0, |_args| Expr::number(1.0))
            .expect("valid arg")
            .partial(1, |_args| Expr::number(2.0))
            .expect("valid arg");

        assert!(f.has_partial(0));
        assert!(f.has_partial(1));
        assert!(!f.has_partial(2));
    }

    #[test]
    fn test_context_clear() {
        let mut ctx = Context::new().with_symbol("x").with_function_name("f");

        assert!(ctx.contains_symbol("x"));
        assert!(ctx.has_function("f"));

        ctx.clear_all();

        assert!(!ctx.contains_symbol("x"));
        assert!(!ctx.has_function("f"));
    }
}
