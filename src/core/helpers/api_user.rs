//! Public API surface for core helpers.

pub use super::logic::error::{DiffError, Span};

/// Common visitor infrastructure
pub mod visitor {
    pub use super::super::logic::visitor::*;
}
