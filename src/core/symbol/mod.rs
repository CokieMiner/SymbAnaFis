//! Symbol interning and ergonomic expression building.
//!
//! - [`api`] — unified API file (both public and crate-internal)
//! - [`logic`] — implementation details (not accessible outside this module)

mod api;
mod logic;

pub use api::*;
