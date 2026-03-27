//! Unified API for core helpers module.
//!
//! Items are re-exported with the correct visibility:
//! - Public API items (part of `lib.rs` surface) -> `pub use`
//! - Crate-internal items (used by sibling modules) -> `pub(crate) use`

// ============================================================================
// Error types — public API
// ============================================================================

pub use super::logic::{DiffError, Span};

// ============================================================================
// Known symbol IDs — re-export the logic submodule.
// ============================================================================

pub use super::logic::known_symbols;

// ============================================================================
// Expression view — public API for pattern matching on expression structure
// ============================================================================

pub use super::logic::ExprView;

// ============================================================================
// Math scalar helpers — re-export the logic submodule.
// ============================================================================

pub use super::logic::traits;
