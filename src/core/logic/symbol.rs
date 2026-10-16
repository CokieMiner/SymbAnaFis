//! Copy symbol handles and public registry operations.

#![expect(
    clippy::self_named_module_files,
    reason = "the type owner keeps fields private while its descendants implement operations; mod.rs files contain plumbing only"
)]

use core::marker::PhantomData;

use alloc::{string::String, sync::Arc, vec::Vec};

use super::{Context, Expr, SYMBOLS, SymbolData, SymbolError};

/// A globally unique Copy identity borrowed from its metadata owner.
///
/// Local handles borrow a [`Context`] or an owning [`Expr`]. They cannot outlive
/// that owner. Expressions built from them own their metadata and can escape the
/// context. Global handles have the `'static` lifetime and permanent metadata.
/// Name removal preserves handles within their lifetime. IDs are never reused,
/// and the global registry has no bulk-reset operation.
/// IDs occupy one native word and are unique within this running process;
/// they are not portable serialized identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Symbol<'owner>(usize, PhantomData<&'owner SymbolData>);

impl Symbol<'static> {
    /// Allocate a distinct anonymous symbol.
    ///
    /// # Panics
    /// Panics if symbol IDs are exhausted.
    #[must_use]
    pub fn anon() -> Self {
        SYMBOLS
            .anonymous()
            .expect("anonymous symbol allocation failed")
            .permanent_symbol()
    }

    /// Reconstruct a global handle after validating its permanent metadata.
    /// Use [`Context::symbol_from_id`] to resolve a locally owned identity.
    ///
    /// # Errors
    /// Returns an invalid-ID error for unknown or local identities, including
    /// local identities still retained by a context or expression.
    ///
    /// ```
    /// use symb_anafis::{Symbol, SymbolError};
    /// let x = Symbol::intern("checked_symbol_example");
    /// assert_eq!(Symbol::from_id(x.id())?, x);
    /// assert_eq!(Symbol::from_id(0), Err(SymbolError::InvalidId(0)));
    /// # Ok::<(), SymbolError>(())
    /// ```
    pub fn from_id(id: usize) -> Result<Self, SymbolError> {
        SymbolData::lookup_permanent(id)
    }

    /// Get or create a globally named symbol.
    ///
    /// # Panics
    /// Panics if symbol IDs are exhausted.
    #[must_use]
    pub fn intern(name: &str) -> Self {
        SYMBOLS
            .intern(name)
            .expect("symbol registration failed")
            .permanent_symbol()
    }

    /// Register a new global name.
    ///
    /// # Errors
    /// Returns a duplicate-name or ID-exhaustion error.
    pub fn register(name: &str) -> Result<Self, SymbolError> {
        SYMBOLS.register(name).map(|data| data.permanent_symbol())
    }

    /// Look up an existing global name without creating it.
    ///
    /// # Errors
    /// Returns a missing-name error.
    pub fn get(name: &str) -> Result<Self, SymbolError> {
        SYMBOLS.get(name)
    }

    /// Test whether a global name is bound.
    #[must_use]
    pub fn exists(name: &str) -> bool {
        SYMBOLS.get(name).is_ok()
    }

    /// Remove a global name binding. Existing handles remain usable; registering
    /// the name again allocates a distinct identity.
    pub fn remove(name: &str) -> bool {
        SYMBOLS.remove(name)
    }

    /// Count current global registrations, including anonymous symbols.
    #[must_use]
    pub fn count() -> usize {
        SYMBOLS.len()
    }

    /// List global names in unspecified order.
    #[must_use]
    pub fn names() -> Vec<String> {
        SYMBOLS.names()
    }
}

impl Symbol<'_> {
    /// Convert to an owning canonical expression. Conversion remains valid after
    /// name removal, while the borrowed owner lives. The resulting expression
    /// may outlive that owner.
    ///
    /// ```
    /// use symb_anafis::Context;
    /// let expression = {
    ///     let context = Context::new();
    ///     let x = context.symb("retained_context_symbol");
    ///     x + 1
    /// };
    /// assert!(expression.contains_var("retained_context_symbol"));
    /// ```
    ///
    /// Local Copy handles cannot escape their context:
    ///
    /// ```compile_fail
    /// use symb_anafis::Context;
    /// let x = {
    ///     let context = Context::new();
    ///     context.symb("x")
    /// };
    /// let expression = x + 1;
    /// ```
    #[must_use]
    pub fn to_expr(&self) -> Expr {
        SymbolData::for_symbol(*self).to_expr()
    }

    /// Remove this identity's global or anonymous registration while retaining
    /// its metadata. Context bindings are removed through their context manager.
    pub fn unregister(self) -> bool {
        SYMBOLS.remove_id(self.0)
    }

    /// Copy the name, or return `None` for an anonymous symbol.
    #[must_use]
    pub fn name(&self) -> Option<String> {
        self.name_arc().map(|name| name.as_ref().into())
    }

    /// Share the registry's name allocation.
    #[must_use]
    pub fn name_arc(&self) -> Option<Arc<str>> {
        SymbolData::for_symbol(*self).name_arc()
    }

    /// Get the stable identity.
    #[must_use]
    pub const fn id(&self) -> usize {
        self.0
    }
}

impl SymbolData {
    /// Borrow a Copy handle from metadata retained by an expression or guard.
    pub const fn symbol(&self) -> Symbol<'_> {
        Symbol(self.id(), PhantomData)
    }

    /// Construct a global handle. Sealed callers use only metadata published
    /// permanently by the global allocation or checked reconstruction paths.
    pub const fn permanent_symbol(&self) -> Symbol<'static> {
        Symbol(self.id(), PhantomData)
    }

    /// Bind a handle to a context that retains this identity in its owner map.
    /// Only context methods call this after allocation or owner-map lookup;
    /// name removal cannot remove the owner-map entry.
    pub const fn scoped_symbol<'owner>(&self, _owner: &'owner Context) -> Symbol<'owner> {
        Symbol(self.id(), PhantomData)
    }
}

mod operators;
