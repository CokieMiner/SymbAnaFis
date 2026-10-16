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
    Bindings, BodyFn, Context, FunctionDefinition, FunctionError, FunctionName, HashMap, HashSet,
    IdSequence, Mutex, PartialFn, Symbol, SymbolData, SymbolError, UserFunction,
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
                bindings.function_names.keys().cloned().collect::<Vec<_>>(),
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
    /// Get the context identity, shared by clones.
    #[must_use]
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Get or create a locally bound symbol.
    ///
    /// # Panics
    /// Panics if IDs are exhausted.
    #[must_use]
    pub fn symb(&self, name: &str) -> Symbol<'_> {
        self.try_symb(name)
            .expect("context symbol registration failed")
    }

    /// Get or create a locally bound symbol without a global name binding.
    ///
    /// # Errors
    /// Returns ID exhaustion.
    pub fn try_symb(&self, name: &str) -> Result<Symbol<'_>, SymbolError> {
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

    /// Collect context variable names for set membership.
    #[must_use]
    pub fn symbol_names_set(&self) -> HashSet<String> {
        self.bindings
            .lock()
            .symbols
            .keys()
            .map(ToString::to_string)
            .collect()
    }

    /// List context variable names.
    #[must_use]
    pub fn symbol_names(&self) -> Vec<String> {
        self.bindings
            .lock()
            .symbols
            .keys()
            .map(ToString::to_string)
            .collect()
    }

    /// Register an unbound custom function name with an immutable arity.
    /// Remove an existing registration before reusing its name. Surviving handles
    /// retain the previous identity and argument contract.
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
        if bindings.function_names.contains_key(name) {
            drop(bindings);
            return Err(FunctionError::DuplicateName(name.into()));
        }
        let text: Arc<str> = Arc::from(name);
        let function = FunctionDefinition::local(Arc::clone(&text), arity);
        bindings.function_names.insert(text, function.clone());
        bindings.functions.insert(function.clone(), definition);
        drop(bindings);
        Ok(function)
    }

    /// Resolve the typed identity of a registered local function.
    #[must_use]
    pub fn get_function(&self, name: &str) -> Option<FunctionName> {
        self.bindings.lock().function_names.get(name).cloned()
    }

    /// Copy a local definition by name without creating missing names.
    #[must_use]
    pub fn get_user_fn(&self, name: &str) -> Option<UserFunction> {
        let bindings = self.bindings.lock();
        bindings
            .function_names
            .get(name)
            .and_then(|function| bindings.functions.get(function))
            .cloned()
    }

    /// Copy a local definition by its canonical identity.
    #[must_use]
    pub fn get_user_fn_for(&self, function: &FunctionName) -> Option<UserFunction> {
        self.bindings.lock().functions.get(function).cloned()
    }

    /// Test whether a local function is registered.
    #[must_use]
    pub fn has_function(&self, name: &str) -> bool {
        self.bindings.lock().function_names.contains_key(name)
    }

    /// List registered function names.
    #[must_use]
    pub fn function_names(&self) -> HashSet<String> {
        self.bindings
            .lock()
            .function_names
            .keys()
            .map(ToString::to_string)
            .collect()
    }

    /// Copy the name-to-function handle index.
    #[must_use]
    pub fn function_handles(&self) -> HashMap<String, FunctionName> {
        self.bindings
            .lock()
            .function_names
            .iter()
            .map(|(name, function)| (name.to_string(), function.clone()))
            .collect()
    }

    /// Share a function body by name without cloning its whole definition.
    #[must_use]
    pub fn get_body(&self, name: &str) -> Option<BodyFn> {
        let bindings = self.bindings.lock();
        bindings
            .function_names
            .get(name)
            .and_then(|function| bindings.functions.get(function))
            .and_then(UserFunction::body_fn)
    }

    /// Share a body by canonical function identity, with no name resolution.
    #[must_use]
    pub fn get_body_for(&self, function: &FunctionName) -> Option<BodyFn> {
        self.bindings
            .lock()
            .functions
            .get(function)
            .and_then(UserFunction::body_fn)
    }

    /// Share a partial derivative by function name and argument index.
    #[must_use]
    pub fn get_partial(&self, name: &str, index: usize) -> Option<PartialFn> {
        let bindings = self.bindings.lock();
        bindings
            .function_names
            .get(name)
            .and_then(|function| bindings.functions.get(function))
            .and_then(|definition| definition.partial_fn(index))
    }

    /// Share a partial derivative by canonical identity and argument index.
    #[must_use]
    pub fn get_partial_for(&self, function: &FunctionName, index: usize) -> Option<PartialFn> {
        self.bindings
            .lock()
            .functions
            .get(function)
            .and_then(|definition| definition.partial_fn(index))
    }

    /// Test whether any custom function has a symbolic body.
    #[must_use]
    pub fn has_expandable_functions(&self) -> bool {
        self.bindings
            .lock()
            .functions
            .values()
            .any(UserFunction::has_body)
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

    /// Remove a local function; surviving expressions retain its old identity.
    #[must_use]
    pub fn remove_function(&self, name: &str) -> bool {
        let removed = {
            let mut bindings = self.bindings.lock();
            let removed = bindings
                .function_names
                .remove(name)
                .and_then(|function| bindings.functions.remove(&function));
            Bindings::compact_map(&mut bindings.function_names);
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

    /// Remove all local custom definitions.
    pub fn clear_functions(&self) {
        let removed = {
            let mut bindings = self.bindings.lock();
            let functions = take(&mut bindings.functions);
            let names = take(&mut bindings.function_names);
            drop(bindings);
            (functions, names)
        };
        // Both callback captures and local function names are released outside
        // the lock. The replacement maps have zero retained capacity.
        drop(removed);
    }

    /// Remove all shared name bindings and definitions. Issued local identities
    /// remain owned by the namespace; use [`Self::reset`] to release ownership.
    pub fn clear_all(&self) {
        let removed = {
            let mut bindings = self.bindings.lock();
            let symbols = take(&mut bindings.symbols);
            let functions = take(&mut bindings.functions);
            let names = take(&mut bindings.function_names);
            drop(bindings);
            (symbols, functions, names)
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
    /// let expression = context.symb("x") + 1;
    /// context.reset();
    /// assert!(context.is_empty());
    /// assert!(expression.contains_var("x"));
    /// ```
    ///
    /// Rust prevents resetting the owner while a borrowed symbol is still used:
    ///
    /// ```compile_fail
    /// use symb_anafis::Context;
    /// let mut context = Context::new();
    /// let x = context.symb("x");
    /// context.reset();
    /// let expression = x + 1;
    /// ```
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Create a local namespace whose clones share bindings and definitions.
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

    /// Count live local symbol bindings.
    #[must_use]
    pub fn symbol_count(&self) -> usize {
        self.bindings.lock().symbols.len()
    }

    /// Test whether the context has no local symbols or definitions.
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
