//! Public diagnostics grouped by the subsystem that defines their contract.
//!
//! Core operations use focused identity, definition, and transformation errors.
//! `DiffError` contains the diagnostics consumed by the parked mathematical
//! engines; it is not the error type for custom function registration.
//!
//! ```
//! use std::sync::Arc;
//! use symb_anafis::{Expr, FunctionError, UserFunction};
//! let result = UserFunction::new(1..=1).partial(1, Arc::new(|_| Expr::number(0)));
//! assert!(matches!(result, Err(FunctionError::InvalidPartialIndex {
//!     index: 1, max_arity: 1,
//! })));
//! ```

mod core;
mod diff;
mod parser;

pub use core::{FunctionError, RewriteError, SubstitutionError, SymbolError};
pub use diff::DiffError;
pub use parser::Span;

#[cfg(test)]
mod tests;
