//! Core types for symbolic mathematics
//!
//! This module contains the fundamental types:
//! - `Expr` / `ExprKind` - Expression AST
//! - `Symbol` / `InternedSymbol` - Symbol system
//! - `Polynomial` - Polynomial representation
//! - `DiffError` - Error types
//! - `CompiledEvaluator` - Fast bytecode-based evaluation
//! - Display formatting (to_string, to_latex, to_unicode)
//! - Visitor pattern for AST traversal

mod display; // Display implementations for Expr
pub(crate) mod error; // Error types (DiffError, Span)
pub(crate) mod evaluator; // Compiled evaluator for fast numeric evaluation
pub(crate) mod expr; // Expression AST (Expr, ExprKind)
pub(crate) mod known_symbols; // Well-known symbol IDs (pi, e, etc.)
pub(crate) mod poly; // Polynomial representation
pub(crate) mod symbol; // Symbol interning system
pub(crate) mod traits; // Common traits
pub mod visitor; // Public visitor pattern for AST traversal

// Public re-exports (for external API)
pub use error::{DiffError, Span};
pub use evaluator::{CompileError, CompiledEvaluator};
pub use expr::{Expr, ExprKind};
pub use symbol::{
    Symbol, SymbolContext, SymbolError, clear_symbols, global_context, remove_symbol, symb,
    symb_get, symb_new, symbol_count, symbol_exists, symbol_names,
};
