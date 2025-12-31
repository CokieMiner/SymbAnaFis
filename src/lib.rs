#![deny(unsafe_code)]
//! Symbolic Differentiation Library
//!
//! A fast, focused Rust library for symbolic differentiation.
//!
//! # Features
//! - Context-aware parsing with known symbols and custom functions
//! - Extensible simplification framework
//! - Support for built-in functions (sin, cos, ln, exp)
//! - Implicit function handling
//! - Partial derivative notation
//! - **Type-safe expression building** with operator overloading
//! - **Builder pattern API** for differentiation and simplification
//!
//! # Usage Examples
//!
//! ## String-based API (original)
//! ```
//! use symb_anafis::diff;
//! let result = diff("x^2", "x", &[], None).unwrap();
//! assert_eq!(result, "2*x");
//! ```
//!
//! ## Type-safe API (new)
//! ```
//! use symb_anafis::{symb, Diff};
//! let x = symb("x");
//! let expr = x.pow(2.0) + x.sin();
//! let derivative = Diff::new().differentiate(&expr, &x).unwrap();
//! // derivative is: 2*x + cos(x)
//! ```

// New module structure
mod api; // User-facing builders: Diff, Simplify, helpers
mod bindings; // External bindings (Python, parallel)
mod core; // Core types: Expr, Symbol, Error, Display, Visitor, CompiledEvaluator
mod diff; // Differentiation engine

mod functions;
mod math;
mod parser;
mod simplification;
mod uncertainty;

// Re-export visitor at crate root for public API
pub use core::visitor;

#[cfg(test)]
mod tests;

// ============================================================================
// Public API Re-exports (organized by concept)
// ============================================================================

// --- Error Types ---
pub use core::{DiffError, Span, SymbolError};

// --- Expression Types ---
pub use core::CompiledEvaluator;
pub use core::Expr;
pub(crate) use core::ExprKind;

// --- Math Types ---
pub use core::traits::MathScalar;
pub use math::dual::Dual;

// --- Symbol Management ---
/// Symbol type and context for variable handling.
/// - `symb(name)` - Get or create a symbol (most common usage)
/// - `symb_new(name)` - Create a new symbol (errors if exists)
/// - `symb_get(name)` - Get existing symbol (errors if not found)
pub use core::{
    ArcExprExt, Symbol, clear_symbols, remove_symbol, symb, symb_get, symb_new, symbol_count,
    symbol_exists, symbol_names,
};

// --- Builder API ---
pub use api::{Diff, Simplify};

// --- Unified Context ---
pub use core::unified_context::{BodyFn, Context, PartialFn, UserFunction};

// --- Utility Functions ---
pub use api::{evaluate_str, gradient, gradient_str, hessian, hessian_str, jacobian, jacobian_str};
pub use parser::parse;
pub use uncertainty::{CovEntry, CovarianceMatrix, relative_uncertainty, uncertainty_propagation};

// Conditional re-exports
#[cfg(feature = "parallel")]
pub use bindings::eval_f64::eval_f64;
#[cfg(feature = "parallel")]
pub use bindings::parallel;

/// Default maximum AST depth.
/// This limit prevents stack overflow from deeply nested expressions.
pub const DEFAULT_MAX_DEPTH: usize = 100;

/// Default maximum AST node count.
/// This limit prevents memory exhaustion from extremely large expressions.
pub const DEFAULT_MAX_NODES: usize = 10_000;

/// Main API function for symbolic differentiation
///
/// # Arguments
/// * `formula` - Mathematical expression to differentiate (e.g., "x^2 + y()")
/// * `var_to_diff` - Variable to differentiate with respect to (e.g., "x")
/// * `known_symbols` - Known multi-character symbols for parsing (e.g., &["alpha", "beta"])
/// * `custom_functions` - Optional user-defined function names (e.g., Some(&[[["y", "f"]))
///
/// # Returns
/// The derivative as a string, or an error if parsing/differentiation fails
///
/// # Example
/// ```
/// use symb_anafis::diff;
/// let result = diff("alpha * sin(x)", "x", &["alpha"], None);
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap(), "alpha*cos(x)");
/// ```
///
/// # Default Limits
/// This function applies `DEFAULT_MAX_DEPTH` (100) and `DEFAULT_MAX_NODES` (10,000)
/// to prevent stack overflow and memory exhaustion.
///
/// # Note
/// For more control (domain_safe, max_depth, etc.), use the `Diff` builder:
/// ```
/// use symb_anafis::Diff;
/// let result = Diff::new().domain_safe(true).diff_str("x^2", "x", &[]).unwrap();
/// assert_eq!(result, "2*x");
/// ```
pub fn diff(
    formula: &str,
    var_to_diff: &str,
    known_symbols: &[&str],
    custom_functions: Option<&[&str]>,
) -> Result<String, DiffError> {
    let mut builder = Diff::new();

    if let Some(funcs) = custom_functions {
        builder = funcs.iter().fold(builder, |b, f| {
            b.user_fn(*f, UserFunction::new(0..=usize::MAX))
        });
    }

    builder
        .max_depth(DEFAULT_MAX_DEPTH)
        .max_nodes(DEFAULT_MAX_NODES)
        .diff_str(formula, var_to_diff, known_symbols)
}

/// Simplify a mathematical expression
///
/// # Arguments
/// * `formula` - Mathematical expression to simplify (e.g., "x^2 + 2*x + 1")
/// * `known_symbols` - Known multi-character symbols for parsing (e.g., &["alpha", "beta"])
/// * `custom_functions` - Optional user-defined function names (e.g., Some(&[[["f", "g"]))
///
/// # Returns
/// The simplified expression as a string, or an error if parsing/simplification fails
///
/// # Example
/// ```
/// use symb_anafis::simplify;
/// let result = simplify("x + x", &[], None).unwrap();
/// assert_eq!(result, "2*x");
/// ```
///
/// # Default Limits
/// This function applies `DEFAULT_MAX_DEPTH` (100) and `DEFAULT_MAX_NODES` (10,000)
/// to prevent stack overflow and memory exhaustion.
///
/// # Note
/// For more control (domain_safe, max_depth, etc.), use the `Simplify` builder:
/// ```
/// use symb_anafis::Simplify;
/// let result = Simplify::new().domain_safe(true).simplify_str("2*x + 3*x", &[]).unwrap();
/// assert_eq!(result, "5*x");
/// ```
pub fn simplify(
    formula: &str,
    known_symbols: &[&str],
    custom_functions: Option<&[&str]>,
) -> Result<String, DiffError> {
    let mut builder = Simplify::new();

    if let Some(funcs) = custom_functions {
        builder = funcs.iter().fold(builder, |b, f| {
            b.user_fn(*f, UserFunction::new(0..=usize::MAX))
        });
    }

    builder
        .max_depth(DEFAULT_MAX_DEPTH)
        .max_nodes(DEFAULT_MAX_NODES)
        .simplify_str(formula, known_symbols)
}
