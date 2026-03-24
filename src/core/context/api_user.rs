//! Public API for end-users of the `context` module.
//!
//! - [`Context`] — builder for symbol registries and user-defined functions
//! - [`UserFunction`] — definition of a custom function with optional body and partials

pub use super::logic::context::{Context, UserFunction};
