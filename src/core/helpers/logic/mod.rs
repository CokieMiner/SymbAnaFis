//! Logic layer for the `core` root module.

pub mod error;
pub mod known_symbols;
pub mod traits;
pub mod view;

// Staircase re-exports: public API items → bare pub use; crate-internal → pub(crate) use
pub use error::{DiffError, Span};
pub use view::ExprView;

#[cfg(test)]
mod tests;
