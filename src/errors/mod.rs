//! Public diagnostics grouped by the subsystem that defines their contract.
//!
//! Core operations use focused identity, definition, and transformation errors.
//! Rewriting, text parsing, and numerical evaluation expose their own
//! operation-specific diagnostics without sharing a catch-all error type.
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
mod evaluator;
mod parser;
mod rewrite;
mod simplification;
mod uncertainty;

pub use core::{FunctionError, SubstitutionError, SymbolError};
pub use evaluator::EvalError;
pub use parser::{ParseError, Span};
pub use rewrite::RewriteError;
pub use simplification::SimplifyError;
pub use uncertainty::UncertaintyError;

#[cfg(test)]
mod tests;
