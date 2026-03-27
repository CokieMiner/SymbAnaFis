//! Internal implementation details for the context module.

pub mod context;

// Staircase re-exports — Public API items (exported by lib.rs)
pub use context::{Context, UserFunction};

pub use super::PartialFn;

#[cfg(test)]
mod tests;
