//! Never-reused identities, global registries, and local context ownership.

use super::{
    Expr, FixedState, Function, FunctionDefinition, FunctionError, Interner, Mutex, SymbolError,
    UserFunction,
};

mod context;
mod registry;
mod symbol;

pub use context::Context;
#[cfg(test)]
pub use registry::SymbolRegistry;
pub use registry::{IdSequence, SYMBOLS, SymbolData};
pub use symbol::Symbol;
