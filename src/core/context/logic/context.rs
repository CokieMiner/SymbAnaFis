//! Implementation details for `Context` and `UserFunction`.
use std::collections::HashSet;
use std::ops::RangeInclusive;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use rustc_hash::FxHashMap;

use crate::Expr;
use crate::core::symbol::{InternedSymbol, Symbol, symb_interned};

use crate::core::context::api_crate::{BodyFn, PartialFn};

// =============================================================================
// UserFunction
// =============================================================================

/// A user-defined function with optional body expression and partial derivatives.
///
/// ```
/// use symb_anafis::{UserFunction, Expr};
///
/// // f(x) = x² + 1, f'(x) = 2x
/// let f = UserFunction::new(1..=1)
///     .body(|args| (*args[0]).clone().pow(2.0) + 1.0)
///     .partial(0, |args| 2.0 * (*args[0]).clone()).expect("valid arg");
/// ```
#[derive(Clone)]
pub struct UserFunction {
    pub(crate) arity: RangeInclusive<usize>,
    pub(crate) body: Option<BodyFn>,
    pub(crate) partials: FxHashMap<usize, PartialFn>,
}

impl Default for UserFunction {
    fn default() -> Self {
        Self {
            arity: 0..=usize::MAX,
            body: None,
            partials: FxHashMap::default(),
        }
    }
}

impl UserFunction {
    /// Create a new user function with the given arity range.
    ///
    /// ```
    /// use symb_anafis::UserFunction;
    /// let f = UserFunction::new(1..=2);
    /// ```
    #[must_use]
    pub fn new(arity: RangeInclusive<usize>) -> Self {
        Self {
            arity,
            body: None,
            partials: FxHashMap::default(),
        }
    }

    /// Create a variadic function (accepts any number of arguments).
    #[must_use]
    pub fn any_arity() -> Self {
        Self::new(0..=usize::MAX)
    }

    /// Set the symbolic body function.
    ///
    /// ```
    /// use symb_anafis::{UserFunction, Expr};
    /// let f = UserFunction::new(1..=1).body(|args| (*args[0]).clone().pow(2.0) + 1.0);
    /// ```
    #[must_use]
    pub fn body<F>(mut self, f: F) -> Self
    where
        F: Fn(&[Arc<Expr>]) -> Expr + Send + Sync + 'static,
    {
        self.body = Some(Arc::new(f));
        self
    }

    /// Set the body using a pre-wrapped `BodyFn` Arc (for FFI/Python bindings).
    #[must_use]
    pub fn body_arc(mut self, f: BodyFn) -> Self {
        self.body = Some(f);
        self
    }

    /// Add a partial derivative for argument at index `i`.
    ///
    /// # Errors
    /// Returns `DiffError::InvalidPartialIndex` if `arg_idx` exceeds the maximum arity.
    pub fn partial<F>(mut self, arg_idx: usize, f: F) -> Result<Self, crate::DiffError>
    where
        F: Fn(&[Arc<Expr>]) -> Expr + Send + Sync + 'static,
    {
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

    /// Add a partial using a pre-wrapped `PartialFn` Arc (for FFI/Python bindings).
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

    /// Returns `true` if this function has a body expression defined.
    #[inline]
    #[must_use]
    pub fn has_body(&self) -> bool {
        self.body.is_some()
    }

    /// Returns `true` if this function has a partial for the given argument index.
    #[inline]
    #[must_use]
    pub fn has_partial(&self, arg_idx: usize) -> bool {
        self.partials.contains_key(&arg_idx)
    }

    /// Returns `true` if `n` is within this function's arity range.
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

static NEXT_CONTEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Default)]
struct ContextInner {
    symbols: FxHashMap<String, InternedSymbol>,
}

/// Unified context for all `symb_anafis` operations.
///
/// Combines an isolated symbol registry (for parsing hints) and user-defined
/// functions (with optional body and partial derivatives).
///
/// ```
/// use symb_anafis::{Context, UserFunction};
///
/// let ctx = Context::new()
///     .with_symbol("x")
///     .with_symbol("alpha")
///     .with_function("f", UserFunction::new(1..=1).body(|args| (*args[0]).clone().sin()));
/// ```
#[derive(Clone)]
pub struct Context {
    id: u64,
    inner: Arc<RwLock<ContextInner>>,
    user_functions: FxHashMap<u64, UserFunction>,
    fn_name_to_id: FxHashMap<String, u64>,
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
            user_functions: FxHashMap::default(),
            fn_name_to_id: FxHashMap::default(),
        }
    }

    /// Get this context's unique ID.
    #[inline]
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    // =========================================================================
    // Symbol registration
    // =========================================================================

    /// Register a symbol name (builder pattern).
    #[must_use]
    pub fn with_symbol(self, name: &str) -> Self {
        self.register_symbol(name);
        self
    }

    /// Register multiple symbols (builder pattern).
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

    /// Get or create a symbol in this context.
    ///
    /// # Panics
    /// Panics if the internal lock is poisoned.
    #[must_use]
    pub fn symb(&self, name: &str) -> Symbol {
        let mut inner = self.inner.write().expect("Context lock poisoned");
        if let Some(existing) = inner.symbols.get(name) {
            return Symbol::from_id(existing.id());
        }
        let symbol = crate::core::symbol::symb_get(name)
            .unwrap_or_else(|_| crate::core::symbol::symb_new_isolated(name));
        let interned = crate::core::symbol::lookup_by_id(symbol.id())
            .expect("Symbol just created should exist");
        inner.symbols.insert(name.to_owned(), interned);
        symbol
    }

    fn register_symbol(&self, name: &str) {
        #[allow(clippy::let_underscore_must_use, reason = "Side-effect only")]
        let _ = self.symb(name);
    }

    /// Check if a symbol is registered in this context.
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

    /// Get a symbol by name, or `None` if not registered.
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

    /// List all registered symbol names.
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

    /// Symbol names as a `HashSet` (for parser compatibility).
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
    // User function registration
    // =========================================================================

    /// Register a user-defined function (builder pattern).
    #[must_use]
    pub fn with_function(mut self, name: &str, func: UserFunction) -> Self {
        let id = symb_interned(name).id();
        self.user_functions.insert(id, func);
        self.fn_name_to_id.insert(name.to_owned(), id);
        self
    }

    /// Register a function name only (for parsing, no eval/partial).
    #[must_use]
    pub fn with_function_name(mut self, name: &str) -> Self {
        let id = symb_interned(name).id();
        if let std::collections::hash_map::Entry::Vacant(e) = self.user_functions.entry(id) {
            e.insert(UserFunction::new(0..=usize::MAX));
            self.fn_name_to_id.insert(name.to_owned(), id);
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
        let id = symb_interned(name).id();
        self.user_functions.get(&id)
    }

    /// Check if a function is registered.
    #[inline]
    #[must_use]
    pub fn has_function(&self, name: &str) -> bool {
        let id = symb_interned(name).id();
        self.user_functions.contains_key(&id)
    }

    /// Get all registered function names.
    #[must_use]
    pub fn function_names(&self) -> HashSet<String> {
        self.fn_name_to_id.keys().cloned().collect()
    }

    /// Get the name → ID mapping for user functions.
    #[must_use]
    pub const fn fn_name_to_id(&self) -> &FxHashMap<String, u64> {
        &self.fn_name_to_id
    }

    /// Get the body function for a user function by name.
    #[inline]
    #[must_use]
    pub fn get_body(&self, name: &str) -> Option<&BodyFn> {
        let id = symb_interned(name).id();
        self.user_functions.get(&id).and_then(|f| f.body.as_ref())
    }

    /// Get the body function by symbol ID.
    #[inline]
    #[must_use]
    pub fn get_body_by_id(&self, id: u64) -> Option<&BodyFn> {
        self.user_functions.get(&id).and_then(|f| f.body.as_ref())
    }

    /// Get a partial derivative function by name and argument index.
    #[inline]
    #[must_use]
    pub fn get_partial(&self, name: &str, arg_idx: usize) -> Option<&PartialFn> {
        let id = symb_interned(name).id();
        self.user_functions
            .get(&id)
            .and_then(|f| f.partials.get(&arg_idx))
    }

    /// Get a user function by symbol ID.
    #[inline]
    #[must_use]
    pub fn get_user_fn_by_id(&self, id: u64) -> Option<&UserFunction> {
        self.user_functions.get(&id)
    }

    /// Returns `true` if any registered function has a body that can be expanded.
    #[inline]
    #[must_use]
    pub fn has_expandable_functions(&self) -> bool {
        self.user_functions.values().any(|f| f.body.is_some())
    }

    // =========================================================================
    // Removal / clearing
    // =========================================================================

    /// Remove a symbol. Returns `true` if it was present.
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

    /// Remove a user function. Returns `true` if it was present.
    pub fn remove_function(&mut self, name: &str) -> bool {
        if let Some(id) = self.fn_name_to_id.remove(name) {
            self.user_functions.remove(&id).is_some()
        } else {
            false
        }
    }

    /// Clear all symbols.
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

    /// Clear all user functions.
    pub fn clear_functions(&mut self) {
        self.user_functions.clear();
        self.fn_name_to_id.clear();
    }

    /// Clear everything.
    pub fn clear_all(&mut self) {
        self.clear_symbols();
        self.clear_functions();
    }

    /// Number of symbols registered in this context.
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

    /// Returns `true` if no symbols and no functions are registered.
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
                &self.fn_name_to_id.keys().collect::<Vec<_>>(),
            )
            .finish_non_exhaustive()
    }
}
