#![forbid(unsafe_code)]
//! Symbolic Differentiation Library
//!
//! A fast, focused Rust library for symbolic differentiation.
//!
//! # Features
//! - Context-aware parsing with fixed variables and custom functions
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
//! let result = diff("x^2", "x", None, None).unwrap();
//! assert_eq!(result, "2*x");
//! ```
//!
//! ## Type-safe API (new)
//! ```
//! use symb_anafis::{symb, Diff};
//! let x = symb("x");
//! let expr = x.pow(2.0) + x.sin();
//! let derivative = Diff::new().differentiate(expr, &x).unwrap();
//! // derivative is: 2*x + cos(x)
//! ```

// New module structure
mod api; // User-facing builders: Diff, Simplify, helpers
mod bindings; // External bindings (Python, parallel)
mod core; // Core types: Expr, Symbol, Polynomial, Error, Display, Visitor
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

// Re-export key types from core
pub use core::{CompileError, CompiledEvaluator};
pub use core::{DiffError, Span};
pub use core::{Expr, ExprKind};
pub use core::{
    InternedSymbol, Symbol, SymbolContext, SymbolError, clear_symbols, global_context,
    remove_symbol, symb, symb_get, symb_new, symbol_count, symbol_exists, symbol_names,
};

// Re-export API types
pub use api::{CustomDerivativeFn, CustomFn, Diff, Simplify};
pub use api::{evaluate_str, gradient, gradient_str, hessian, hessian_str, jacobian, jacobian_str};

// Re-export other public APIs
pub use parser::parse;
pub use simplification::simplify_expr;
pub use uncertainty::{CovEntry, CovarianceMatrix, relative_uncertainty, uncertainty_propagation};

// Conditional re-exports
#[cfg(feature = "parallel")]
pub use bindings::parallel;

/// Default maximum AST depth
pub const DEFAULT_MAX_DEPTH: usize = 100;
/// Default maximum AST node count
pub const DEFAULT_MAX_NODES: usize = 10_000;

/// Main API function for symbolic differentiation
///
/// # Arguments
/// * `formula` - Mathematical expression to differentiate (e.g., "x^2 + y()")
/// * `var_to_diff` - Variable to differentiate with respect to (e.g., "x")
/// * `fixed_vars` - Symbols that are constants (e.g., &["a", "b"])
/// * `custom_functions` - User-defined function names (e.g., &["y", "f"])
///
/// # Returns
/// The derivative as a string, or an error if parsing/differentiation fails
///
/// # Example
/// ```
/// use symb_anafis::diff;
/// let result = diff("a * sin(x)", "x", Some(&["a"]), None);
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap(), "a*cos(x)");
/// ```
///
/// # Note
/// For more control (domain_safe, max_depth, etc.), use the `Diff` builder:
/// ```
/// use symb_anafis::Diff;
/// let result = Diff::new().domain_safe(true).diff_str("x^2", "x").unwrap();
/// assert_eq!(result, "2*x");
/// ```
pub fn diff(
    formula: &str,
    var_to_diff: &str,
    fixed_vars: Option<&[&str]>,
    custom_functions: Option<&[&str]>,
) -> Result<String, DiffError> {
    let mut builder = Diff::new();

    if let Some(vars) = fixed_vars {
        builder = builder.fixed_vars_str(vars.iter().copied());
    }

    if let Some(funcs) = custom_functions {
        builder = funcs.iter().fold(builder, |b, f| b.custom_fn(*f));
    }

    builder
        .max_depth(DEFAULT_MAX_DEPTH)
        .max_nodes(DEFAULT_MAX_NODES)
        .diff_str(formula, var_to_diff)
}

/// Simplify a mathematical expression
///
/// # Arguments
/// * `formula` - Mathematical expression to simplify (e.g., "x^2 + 2*x + 1")
/// * `fixed_vars` - Symbols that are constants (e.g., &["a", "b"])
/// * `custom_functions` - User-defined function names (e.g., &["f", "g"])
///
/// # Returns
/// The simplified expression as a string, or an error if parsing/simplification fails
///
/// # Example
/// ```
/// use symb_anafis::simplify;
/// let result = simplify("x + x", None, None).unwrap();
/// assert_eq!(result, "2*x");
/// ```
///
/// # Note
/// For more control (domain_safe, max_depth, etc.), use the `Simplify` builder:
/// ```
/// use symb_anafis::Simplify;
/// let result = Simplify::new().domain_safe(true).simplify_str("2*x + 3*x").unwrap();
/// assert_eq!(result, "5*x");
/// ```
pub fn simplify(
    formula: &str,
    fixed_vars: Option<&[&str]>,
    custom_functions: Option<&[&str]>,
) -> Result<String, DiffError> {
    let mut builder = Simplify::new();

    if let Some(vars) = fixed_vars {
        builder = builder.fixed_vars_str(vars.iter().copied());
    }

    if let Some(funcs) = custom_functions {
        builder = funcs.iter().fold(builder, |b, f| b.custom_fn(*f));
    }

    builder
        .max_depth(DEFAULT_MAX_DEPTH)
        .max_nodes(DEFAULT_MAX_NODES)
        .simplify_str(formula)
}
