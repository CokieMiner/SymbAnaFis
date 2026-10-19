//! Local symbol ownership and custom functions keyed by canonical handles.

use core::{
    fmt::{Debug, Formatter, Result as FmtResult},
    hash::Hash,
    mem::take,
};

use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};

use super::{
    Bindings, Context, FunctionDefinition, FunctionError, FunctionName, HashMap, IdSequence, Mutex,
    Symbol, SymbolData, SymbolError, UserFunction,
};

static NEXT_CONTEXT: IdSequence = IdSequence::new();

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for Context {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        // A caller's formatting writer may reenter the context. Retain a name
        // snapshot, then release the registry lock before writing any output.
        let (symbols, functions) = {
            let bindings = self.bindings.lock();
            (
                bindings.symbols.keys().cloned().collect::<Vec<_>>(),
                bindings.functions.keys().cloned().collect::<Vec<_>>(),
            )
        };
        formatter
            .debug_struct("Context")
            .field("id", &self.id)
            .field("symbols", &symbols)
            .field("functions", &functions)
            .finish_non_exhaustive()
    }
}

impl Context {
    /// Create a local namespace whose clones share symbol and function bindings.
    ///
    /// # Panics
    /// Panics if the representable context IDs are exhausted.
    #[must_use]
    pub fn new() -> Self {
        Self {
            id: NEXT_CONTEXT.next().expect("context IDs exhausted"),
            bindings: Arc::new(Mutex::new(Bindings::default())),
        }
    }

    /// Get the context identity, shared by clones.
    #[must_use]
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Get or create a locally bound symbol without a global name binding.
    ///
    /// # Errors
    /// Returns [`SymbolError::IdExhausted`] when a new identity cannot be allocated.
    /// Existing names remain resolvable after exhaustion.
    pub fn symb(&self, name: &str) -> Result<Symbol<'_>, SymbolError> {
        let mut bindings = self.bindings.lock();
        if let Some(existing) = bindings.symbols.get(name) {
            return Ok(existing.scoped_symbol(self));
        }
        let text: Arc<str> = Arc::from(name);
        let data = SymbolData::local(Arc::clone(&text))?;
        let symbol = data.scoped_symbol(self);
        bindings.owned_symbols.insert(data.id(), Arc::clone(&data));
        bindings.symbols.insert(text, data);
        drop(bindings);
        Ok(symbol)
    }

    /// Query a live context binding without creating it.
    #[must_use]
    pub fn get_symbol(&self, name: &str) -> Option<Symbol<'_>> {
        self.bindings
            .lock()
            .symbols
            .get(name)
            .map(|data| data.scoped_symbol(self))
    }

    /// Resolve a raw local ID while borrowing its owning context. Removed name
    /// bindings remain resolvable until this namespace's final owner is dropped.
    ///
    /// # Errors
    /// Returns an invalid-ID error for an ID not owned by this context, including
    /// global IDs and IDs from an earlier reset of this context.
    pub fn symbol_from_id(&self, id: usize) -> Result<Symbol<'_>, SymbolError> {
        self.bindings
            .lock()
            .owned_symbols
            .get(&id)
            .map(|data| data.scoped_symbol(self))
            .ok_or(SymbolError::InvalidId(id))
    }

    /// Test whether a context symbol is registered.
    #[must_use]
    pub fn contains_symbol(&self, name: &str) -> bool {
        self.bindings.lock().symbols.contains_key(name)
    }

    /// Snapshot context variable names in lexicographic order.
    #[must_use]
    pub fn symbol_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self
            .bindings
            .lock()
            .symbols
            .keys()
            .map(ToString::to_string)
            .collect();
        names.sort_unstable();
        names
    }

    /// Register an unbound custom function name with an immutable definition.
    /// Remove an existing registration before reusing its name. Surviving handles
    /// retain the previous identity, argument contract, body, and partials.
    ///
    /// # Errors
    /// Returns [`FunctionError::DuplicateName`] if this context already binds the
    /// name or the name resolves to a builtin, including its aliases.
    /// Returns [`FunctionError::InvalidArityRange`] for an empty argument range.
    pub fn register_function(
        &self,
        name: &str,
        definition: UserFunction,
    ) -> Result<FunctionName, FunctionError> {
        if FunctionName::builtin(name).is_some() {
            return Err(FunctionError::DuplicateName(name.into()));
        }
        let arity = definition.arity();
        if arity.is_empty() {
            return Err(FunctionError::InvalidArityRange {
                minimum: *arity.start(),
                maximum: *arity.end(),
            });
        }
        let mut bindings = self.bindings.lock();
        if bindings.functions.contains_key(name) {
            drop(bindings);
            return Err(FunctionError::DuplicateName(name.into()));
        }
        let text: Arc<str> = Arc::from(name);
        let function = FunctionDefinition::local(Arc::clone(&text), definition);
        bindings.functions.insert(text, function.clone());
        drop(bindings);
        Ok(function)
    }

    /// Resolve the typed identity of a registered local function.
    #[must_use]
    pub fn get_function(&self, name: &str) -> Option<FunctionName> {
        self.bindings.lock().functions.get(name).cloned()
    }

    /// Test whether a local function is registered.
    #[must_use]
    pub fn has_function(&self, name: &str) -> bool {
        self.bindings.lock().functions.contains_key(name)
    }

    /// Snapshot registered function handles in lexicographic name order.
    /// Handles retain their complete definitions after removal from this context.
    #[must_use]
    pub fn functions(&self) -> Vec<FunctionName> {
        let mut functions: Vec<_> = self.bindings.lock().functions.values().cloned().collect();
        functions.sort_unstable();
        functions
    }

    /// Test whether any custom function has a symbolic body.
    #[must_use]
    pub fn has_expandable_functions(&self) -> bool {
        self.bindings
            .lock()
            .functions
            .values()
            .any(|function| function.definition().is_some_and(UserFunction::has_body))
    }

    /// Remove a local name binding while keeping issued Copy handles valid.
    #[must_use]
    pub fn remove_symbol(&self, name: &str) -> bool {
        let removed = {
            let mut bindings = self.bindings.lock();
            let removed = bindings.symbols.remove(name);
            Bindings::compact_map(&mut bindings.symbols);
            removed
        };
        // Symbol metadata may retire through its final owner. Release it after
        // the context lock so its destructor can safely reenter the registry.
        let present = removed.is_some();
        drop(removed);
        present
    }

    /// Remove a function-name binding; surviving handles retain its definition.
    #[must_use]
    pub fn remove_function(&self, name: &str) -> bool {
        let removed = {
            let mut bindings = self.bindings.lock();
            let removed = bindings.functions.remove(name);
            Bindings::compact_map(&mut bindings.functions);
            removed
        };
        // User callback captures can have arbitrary destructors. Release the
        // removed definition after unlocking so they can reenter this context.
        let present = removed.is_some();
        drop(removed);
        present
    }

    /// Remove local symbol bindings while preserving all issued identities.
    pub fn clear_symbols(&self) {
        let removed = take(&mut self.bindings.lock().symbols);
        drop(removed);
    }

    /// Remove all function-name bindings. Referenced definitions remain alive.
    pub fn clear_functions(&self) {
        let removed = take(&mut self.bindings.lock().functions);
        drop(removed);
    }

    /// Remove all shared name bindings. Issued symbol identities remain owned
    /// by the namespace; use [`Self::reset`] to release ownership. Function
    /// handles and expressions retain their own definitions.
    pub fn clear_all(&self) {
        let removed = {
            let mut bindings = self.bindings.lock();
            let symbols = take(&mut bindings.symbols);
            let functions = take(&mut bindings.functions);
            drop(bindings);
            (symbols, functions)
        };
        drop(removed);
    }

    /// Release this context's namespace ownership and start a fresh namespace.
    /// Other clones retain the old namespace. Unused local metadata is reclaimed
    /// after its final context owner is released; surviving expressions retain
    /// the metadata they use. Callback captures are released without a held lock.
    ///
    /// # Panics
    /// Panics if context IDs are exhausted, before changing this context.
    ///
    /// ```
    /// use symb_anafis::Context;
    /// let mut context = Context::new();
    /// let expression = context.symb("x")? + 1;
    /// context.reset();
    /// assert!(context.is_empty());
    /// assert!(expression.contains_var("x"));
    /// # Ok::<(), symb_anafis::SymbolError>(())
    /// ```
    ///
    /// Rust prevents resetting the owner while a borrowed symbol is still used:
    ///
    /// ```compile_fail
    /// use symb_anafis::Context;
    /// let mut context = Context::new();
    /// let x = context.symb("x").expect("symbol allocation");
    /// context.reset();
    /// let expression = x + 1;
    /// ```
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Count live local symbol bindings.
    #[must_use]
    pub fn symbol_count(&self) -> usize {
        self.bindings.lock().symbols.len()
    }

    /// Test whether the context has no local symbol or function-name bindings.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let bindings = self.bindings.lock();
        bindings.symbols.is_empty() && bindings.functions.is_empty()
    }
}

impl Bindings {
    fn compact_map<Key: Eq + Hash, Value>(map: &mut HashMap<Key, Value>) {
        if map.is_empty() {
            *map = HashMap::new();
        } else if map.capacity() > map.len().saturating_mul(4).max(64) {
            map.shrink_to_fit();
        }
    }
}
