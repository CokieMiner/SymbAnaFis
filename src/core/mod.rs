//! Core types for symbolic mathematics
//!
//! This module contains the fundamental public types:
//!
//! ## Expression Types
//! - [`Expr`] - The main expression type (AST node)
//! - [`ExprKind`] - Enum of expression variants (Number, Symbol, Add, etc.)
//! - [`Span`] - Source location information for error messages
//!
//! ## Symbol System
//! - [`Symbol`] - Lightweight, copyable symbol reference
//!
//! ## Evaluation
//! - [`CompiledEvaluator`] - Fast bytecode-based numeric evaluation
//!
//! ## Error Handling
//! - [`DiffError`] - Error type for differentiation and parsing
//! - [`SymbolError`] - Error type for symbol operations
//!
//! ## AST Traversal
//! - [`visitor`] module - Visitor pattern for walking the AST
//!
//! ## Display Formatting
//! Expressions support multiple output formats via methods:
//! - `to_string()` - Standard mathematical notation
//! - `to_latex()` - LaTeX formatted output
//! - `to_unicode()` - Unicode mathematical symbols

// ============================================================================
// Internal Modules
// ============================================================================

pub mod display; // Display implementations for Expr
pub mod error; // Error types (DiffError, Span)
pub mod evaluator; // Compiled evaluator for fast numeric evaluation
pub mod expr; // Expression AST (Expr, ExprKind)
pub mod known_symbols; // Well-known symbol IDs (pi, e, etc.)
pub mod poly; // Polynomial representation (internal)
pub mod symbol; // Symbol interning system
pub mod traits; // Common traits (MathScalar, epsilon helpers)
pub mod unified_context; // Unified context for symbols and functions

// ============================================================================
// Public Modules
// ============================================================================

pub mod visitor; // Public visitor pattern for AST traversal

// ============================================================================
// Public Re-exports
// ============================================================================

// --- Error Types ---
pub use error::{DiffError, Span};
pub use symbol::SymbolError;

// --- Expression Types ---
pub use evaluator::CompiledEvaluator;
pub use expr::{Expr, ExprKind};

// --- Symbol Management ---
pub use symbol::{
    ArcExprExt, Symbol, clear_symbols, remove_symbol, symb, symb_get, symb_new, symbol_count,
    symbol_exists, symbol_names,
};
