//! Isolated symbol ownership and shared function-name registration.

#![expect(
    clippy::self_named_module_files,
    reason = "the type owner keeps fields private while its descendants implement operations; mod.rs files contain plumbing only"
)]

use alloc::sync::Arc;

use crate::collections::HashMap;

use super::{
    FunctionDefinition, FunctionError, FunctionName, IdSequence, Mutex, Symbol, SymbolData,
    SymbolError, UserFunction,
};

#[derive(Debug, Default)]
struct Bindings {
    symbols: HashMap<Arc<str>, Arc<SymbolData>>,
    owned_symbols: HashMap<usize, Arc<SymbolData>>,
    functions: HashMap<Arc<str>, FunctionName>,
}

/// An isolated symbol namespace and a shared set of function-name bindings.
///
/// Clones share bindings. Distinct contexts always allocate distinct variable
/// and function identities, including when a global name already exists.
/// Local Copy symbols borrow a context; shared ownership retains all issued
/// identities even after names are removed through another clone. Dropping the
/// final context owner releases unused metadata. Expressions retain the metadata
/// they reference and may outlive the context. [`Self::reset`] detaches this
/// context into a fresh namespace without invalidating another clone's handles.
/// Name removal retains every issued symbol identity until the namespace's
/// final owner is released. Repeated creation and removal therefore accumulates
/// metadata; use `reset` or a short-lived context to release unused identities.
/// Registered function handles own their immutable definitions. Removing a name
/// or dropping the context preserves definitions retained by handles or expressions.
#[derive(Clone)]
pub struct Context {
    id: usize,
    bindings: Arc<Mutex<Bindings>>,
}

mod methods;

#[cfg(test)]
mod tests;
