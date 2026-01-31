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

// ============================================================================
// Module Declarations (organized by layer)
// ============================================================================

// Core infrastructure
mod core; // Core types: Expr, Symbol, Error, Display, Visitor
mod parser; // String-to-AST parsing

// Computation engines
mod diff; // Differentiation engine
mod evaluator; // Compiled evaluator
mod simplification; // Expression simplification

// Function and math support
mod functions; // Function definitions and registry
mod math; // Mathematical function implementations
mod uncertainty; // Uncertainty propagation

// User-facing APIs
mod api; // Builder APIs: Diff, Simplify, helpers
mod bindings; // External bindings (Python, parallel)

// ============================================================================
// Test Module
// ============================================================================

#[cfg(test)]
#[allow(missing_docs)]
#[allow(clippy::pedantic, clippy::nursery, clippy::restriction)]
#[allow(clippy::cast_possible_truncation, clippy::float_cmp)]
#[allow(clippy::print_stdout, clippy::unwrap_used)] // Standard in tests
mod tests;

// ============================================================================
// Public API Re-exports (organized by category)
// ============================================================================

// --- Core Types ---
pub use core::traits::MathScalar;
pub use core::{DiffError, Expr, Span, Symbol, SymbolError};
pub use evaluator::{CompiledEvaluator, ToParamName};
pub use math::dual::Dual;

// --- Symbol Management ---
pub use core::{
    ArcExprExt, clear_symbols, remove_symbol, symb, symb_get, symb_new, symbol_count,
    symbol_exists, symbol_names,
};

// --- Context and Functions ---
pub use core::unified_context::{BodyFn, Context, PartialFn, UserFunction};
pub use parser::parse;

// --- Builder APIs ---
pub use api::{Diff, Simplify};

// --- Utility Functions ---
pub use api::{evaluate_str, gradient, gradient_str, hessian, hessian_str, jacobian, jacobian_str};
pub use uncertainty::{CovEntry, CovarianceMatrix, relative_uncertainty, uncertainty_propagation};

// --- Visitor Pattern (for advanced users) ---
pub use core::visitor;

// --- Feature-gated APIs ---
#[cfg(feature = "parallel")]
pub use bindings::eval_f64::eval_f64;
#[cfg(feature = "parallel")]
pub use bindings::parallel;

// ============================================================================
// Constants
// ============================================================================

/// Default maximum AST depth.
/// This limit prevents stack overflow from deeply nested expressions.
pub const DEFAULT_MAX_DEPTH: usize = 100;

/// Default maximum AST node count.
/// This limit prevents memory exhaustion from extremely large expressions.
pub const DEFAULT_MAX_NODES: usize = 10_000;

// ============================================================================
// Convenience Functions
// ============================================================================

/// Main API function for symbolic differentiation
///
/// # Arguments
/// * `formula` - Mathematical expression to differentiate (e.g., "x^2 + `y()`")
/// * `var_to_diff` - Variable to differentiate with respect to (e.g., "x")
/// * `known_symbols` - Known multi-character symbols for parsing (e.g., `&["alpha", "beta"]`)
/// * `custom_functions` - Optional user-defined function names (e.g., `Some(&["y", "f"])`)
///
/// # Returns
/// The derivative as a string, or an error if parsing/differentiation fails
///
/// # Errors
/// Returns `DiffError` if:
/// - The formula cannot be parsed (syntax error)
/// - The variable to differentiate is not found
/// - An unsupported operation is encountered
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
/// For more control (`domain_safe`, `max_depth`, etc.), use the `Diff` builder:
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
        builder = funcs
            .iter()
            .fold(builder, |b, f| b.user_fn(*f, UserFunction::any_arity()));
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
/// * `known_symbols` - Known multi-character symbols for parsing (e.g., `&["alpha", "beta"]`)
/// * `custom_functions` - Optional user-defined function names (e.g., `Some(&["f", "g"])`)
///
/// # Returns
/// The simplified expression as a string, or an error if parsing/simplification fails
///
/// # Errors
/// Returns `DiffError` if:
/// - The formula cannot be parsed (syntax error)
/// - An unsupported operation is encountered during simplification
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
/// For more control (`domain_safe`, `max_depth`, etc.), use the `Simplify` builder:
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
        builder = funcs
            .iter()
            .fold(builder, |b, f| b.user_fn(*f, UserFunction::any_arity()));
    }

    builder
        .max_depth(DEFAULT_MAX_DEPTH)
        .max_nodes(DEFAULT_MAX_NODES)
        .simplify_str(formula, known_symbols)
}

// ============================================================================
// Internal Re-exports (for crate use only)
// ============================================================================

// ExprKind is NOT re-exported at the crate root to encourage use of Expr constructors
// (Expr::sum, Expr::product, etc.) instead of direct ExprKind construction.
// It IS still public via `use symb_anafis::core::ExprKind` for pattern matching.
// This pub(crate) re-export is only for internal crate usage.
pub(crate) use core::ExprKind;
