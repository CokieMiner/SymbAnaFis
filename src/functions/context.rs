use super::registry::FunctionDefinition;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// An isolated function context for storing custom function definitions.
///
/// Unlike the global static `Registry`, this context can be instantiated per-user
/// or per-thread, allowing for complete isolation in multi-tenant environments.
///
/// # Example
/// ```
/// use symb_anafis::{FunctionContext, FunctionDefinition, Expr};
/// use std::sync::Arc;
///
/// let ctx = FunctionContext::new();
///
/// // Register a custom function: double(x) = 2*x
/// let double_fn = FunctionDefinition::new(
///     "double",
///     1..=1,
///     |args| Some(2.0 * args[0]),
///     |args, derivs| {
///         // d/dx[2*u] = 2 * u'
///         Expr::product(vec![Expr::number(2.0), derivs[0].clone()])
///     },
/// );
///
/// ctx.register("double", double_fn).unwrap();
/// assert!(ctx.len() == 1);
/// ```
#[derive(Clone, Default, Debug)]
pub struct FunctionContext {
    /// Thread-safe storage for function definitions.
    /// Maps Symbol ID (u64) -> FunctionDefinition.
    functions: Arc<RwLock<HashMap<u64, FunctionDefinition>>>,
}

impl FunctionContext {
    /// Create a new empty function context
    ///
    /// # Example
    /// ```
    /// use symb_anafis::FunctionContext;
    /// let ctx = FunctionContext::new();
    /// assert!(ctx.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            functions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a function is defined in this context by its symbol ID
    pub fn contains(&self, id: u64) -> bool {
        self.functions.read().unwrap().contains_key(&id)
    }

    /// Get a function definition by Symbol ID
    ///
    /// Checks local context first, then falls back to the global Registry.
    pub fn get(&self, id: u64) -> Option<FunctionDefinition> {
        // 1. Check local storage
        if let Some(def) = self.functions.read().unwrap().get(&id) {
            return Some(def.clone());
        }

        // 2. Fallback to global registry
        // We need to look up the InternedSymbol to query the Registry
        if let Some(sym) = crate::core::symbol::lookup_by_id(id)
            && let Some(def) = super::registry::Registry::get_by_symbol(&sym)
        {
            return Some(def.clone());
        }

        None
    }

    /// Register a new function definition.
    ///
    /// # Arguments
    /// * `name` - The name of the function (used to resolve symbol ID)
    /// * `def` - The function definition struct
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the function is already defined.
    ///
    /// # Example
    /// ```
    /// use symb_anafis::{FunctionContext, FunctionDefinition, Expr};
    ///
    /// let ctx = FunctionContext::new();
    ///
    /// let negate_fn = FunctionDefinition::new(
    ///     "negate",
    ///     1..=1,
    ///     |args| Some(-args[0]),
    ///     |_args, derivs| Expr::product(vec![Expr::number(-1.0), derivs[0].clone()]),
    /// );
    ///
    /// ctx.register("negate", negate_fn).unwrap();
    ///
    /// // Trying to register the same function again will fail
    /// let dup = FunctionDefinition::new("negate", 1..=1, |_| None, |_, _| Expr::number(0.0));
    /// assert!(ctx.register("negate", dup).is_err());
    /// ```
    pub fn register(&self, name: &str, def: FunctionDefinition) -> Result<(), String> {
        // Resolve symbol ID using the global symbol system
        let symbol = crate::core::symbol::symb(name);
        let id = symbol.id();

        let mut lock = self.functions.write().unwrap();
        if lock.contains_key(&id) {
            return Err(format!(
                "Function '{}' is already defined in this context",
                name
            ));
        }

        lock.insert(id, def);
        Ok(())
    }

    /// Clear all functions from this context
    pub fn clear(&self) {
        self.functions.write().unwrap().clear();
    }

    /// Get the number of functions in this context
    pub fn len(&self) -> usize {
        self.functions.read().unwrap().len()
    }

    /// Check if the context is empty
    pub fn is_empty(&self) -> bool {
        self.functions.read().unwrap().is_empty()
    }
}
