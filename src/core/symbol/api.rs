//! Unified API for the `symbol` module.
//!
//! This file provides both public API (for library users) and crate-internal API
//! (for other modules within the crate). The visibility is controlled per-item.

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::sync::Arc;

use slotmap::{DefaultKey, Key};

// ============================================================================
// Crate-internal API (pub - for other modules in the crate)
// ============================================================================

/// Internal interned symbol type for crate-wide use.
pub use super::logic::InternedSymbol;

/// Internal registry functions for crate-wide use.
pub use super::logic::{key_from_id, lookup_by_id, symb_interned, symb_new_isolated};

// ============================================================================
// Public API (re-exported to crate surface and library users)
// ============================================================================

/// Errors that can occur during symbol operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolError {
    /// Attempted to create a symbol with a name that's already registered.
    DuplicateName(String),
    /// Attempted to get a symbol that doesn't exist.
    NotFound(String),
}

impl Display for SymbolError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::DuplicateName(name) => {
                write!(
                    f,
                    "Symbol '{name}' is already registered. Use symb_get() to retrieve it."
                )
            }
            Self::NotFound(name) => {
                write!(
                    f,
                    "Symbol '{name}' not found. Use symb() to create it first."
                )
            }
        }
    }
}

impl Error for SymbolError {}

// ============================================================================
// Public re-exports
// ============================================================================

/// Public registry functions for library users.
pub use super::logic::{
    clear_symbols, remove_symbol, symb, symb_anon, symb_get, symb_new, symbol_count, symbol_exists,
    symbol_names,
};

use crate::Expr;

// ============================================================================
// Symbol
// ============================================================================

/// Type-safe, `Copy` handle to an interned symbol.
///
/// Symbols are interned — each unique name exists exactly once in memory, and
/// all handles share the same ID for O(1) equality comparisons.
///
/// ```
/// use symb_anafis::symb;
/// let a = symb("symbol_doc_a");
/// let expr = a + a;  // Copy — no clone needed
/// assert!(format!("{}", expr).contains("symbol_doc_a"));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Symbol(pub(crate) DefaultKey);

impl Symbol {
    /// Create a new anonymous symbol (unique ID, no string name).
    #[must_use]
    pub fn anon() -> Self {
        symb_anon()
    }

    /// Reconstruct a Symbol from a previously obtained ID.
    #[inline]
    #[must_use]
    pub fn from_id(id: u64) -> Self {
        Self(key_from_id(id))
    }

    /// The symbol's unique integer ID.
    #[inline]
    #[must_use]
    pub fn id(&self) -> u64 {
        self.0.data().as_ffi()
    }

    /// The symbol's internal slotmap key.
    #[inline]
    #[must_use]
    pub const fn key(&self) -> DefaultKey {
        self.0
    }

    /// The symbol's name, or `None` for anonymous symbols.
    ///
    /// Prefer [`name_arc`](Self::name_arc) in hot paths to avoid allocation.
    #[must_use]
    pub fn name(&self) -> Option<String> {
        self.name_arc().map(|arc| arc.to_string())
    }

    /// The symbol's name as an `Arc<str>` (no allocation on subsequent calls).
    #[must_use]
    pub fn name_arc(&self) -> Option<Arc<str>> {
        lookup_by_id(self.id()).and_then(|s| s.name_arc())
    }

    /// Convert to an `Expr`.
    #[must_use]
    pub fn to_expr(&self) -> Expr {
        lookup_by_id(self.id()).map_or_else(
            || Expr::from_interned(InternedSymbol::new_anon_with_key(self.0)),
            Expr::from_interned,
        )
    }
}
