//! Public API surface for the `core` module.
//!
//! This file is the single source of truth for what `core` exposes to the rest
//! of the crate. Everything re-exported here is part of the stable public
//! interface; everything else is an implementation detail.

// --- Error types ---
pub use super::logic::error::{DiffError, Span};
pub use super::symbol::SymbolError;

// --- Expression types ---
pub use super::expr::{Expr, ExprKind};

// --- Visitor pattern ---
/// Expression visitor utilities
pub mod visitor {
    pub use super::super::logic::visitor::*;
}

// --- Symbol management ---
pub use super::symbol::{
    ArcExprExt, Symbol, clear_symbols, remove_symbol, symb, symb_get, symb_new, symbol_count,
    symbol_exists, symbol_names,
};
