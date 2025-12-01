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

mod ast;
mod differentiation;
mod display;
mod error;
mod parser;
mod simplification;

#[cfg(test)]
mod tests;

// Re-export key types for easier usage
pub use ast::Expr;
pub use error::DiffError;
pub use parser::parse;

use std::collections::HashSet;
use std::env;

/// Get the maximum allowed AST depth from environment variable or default
fn max_depth() -> usize {
    env::var("SYMB_ANAFIS_MAX_DEPTH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100)
}

/// Get the maximum allowed AST node count from environment variable or default
fn max_nodes() -> usize {
    env::var("SYMB_ANAFIS_MAX_NODES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10_000)
}

/// Get domain safety setting from environment variable or default (false)
/// Set SYMB_ANAFIS_DOMAIN_SAFETY=true to skip domain-altering simplification rules
fn domain_safety() -> bool {
    env::var("SYMB_ANAFIS_DOMAIN_SAFETY")
        .ok()
        .map(|s| s.to_lowercase() == "true" || s == "1")
        .unwrap_or(false)
}

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
/// ```ignore
///
/// let result = diff(
///     "a * sin(x)".to_string(),
///     "x".to_string(),
///     Some(&["a".to_string()]),
///     None
/// );
/// assert!(result.is_ok());
/// ```
pub fn diff(
    formula: String,
    var_to_diff: String,
    fixed_vars: Option<&[String]>,
    custom_functions: Option<&[String]>,
) -> Result<String, DiffError> {
    // Step 1: Convert to HashSets for O(1) lookups
    let fixed_set: HashSet<String> = fixed_vars.unwrap_or(&[]).iter().cloned().collect();

    let custom_funcs: HashSet<String> = custom_functions.unwrap_or(&[]).iter().cloned().collect();

    // Step 2: Validate parameters
    if fixed_set.contains(&var_to_diff) {
        return Err(DiffError::VariableInBothFixedAndDiff {
            var: var_to_diff.clone(),
        });
    }

    // Check for name collisions
    for name in &fixed_set {
        if custom_funcs.contains(name) {
            return Err(DiffError::NameCollision { name: name.clone() });
        }
    }

    // Step 3: Parse the formula into AST
    let ast = parser::parse(&formula, &fixed_set, &custom_funcs)?;

    // Step 4: Check safety limits
    if ast.max_depth() > max_depth() {
        return Err(DiffError::MaxDepthExceeded);
    }
    if ast.node_count() > max_nodes() {
        return Err(DiffError::MaxNodesExceeded);
    }

    // Step 5: Differentiate
    let derivative = ast.derive(&var_to_diff, &fixed_set);

    // Step 6: Simplify with configured domain safety
    let simplified = if domain_safety() {
        simplification::simplify_domain_safe(derivative)
    } else {
        simplification::simplify(derivative)
    };

    // Step 7: Convert to string
    Ok(format!("{}", simplified))
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
/// ```ignore
/// let result = simplify(
///     "x^2 + 2*x + 1".to_string(),
///     None, // No fixed variables
///     None  // No custom functions
/// ).unwrap();
///
/// println!("Simplified: {}", result);
/// // Output: (x + 1)^2
/// ```
pub fn simplify(
    formula: String,
    fixed_vars: Option<&[String]>,
    custom_functions: Option<&[String]>,
) -> Result<String, DiffError> {
    // Step 1: Convert to HashSets for O(1) lookups
    let fixed_set: HashSet<String> = fixed_vars.unwrap_or(&[]).iter().cloned().collect();
    let custom_funcs: HashSet<String> = custom_functions.unwrap_or(&[]).iter().cloned().collect();

    // Check for name collisions
    for name in &fixed_set {
        if custom_funcs.contains(name) {
            return Err(DiffError::NameCollision { name: name.clone() });
        }
    }

    // Step 2: Parse the formula into AST
    let ast = parser::parse(&formula, &fixed_set, &custom_funcs)?;

    // Step 3: Check safety limits
    if ast.max_depth() > max_depth() {
        return Err(DiffError::MaxDepthExceeded);
    }
    if ast.node_count() > max_nodes() {
        return Err(DiffError::MaxNodesExceeded);
    }

    // Step 4: Simplify with configured domain safety
    let simplified = if domain_safety() {
        simplification::simplify_domain_safe(ast)
    } else {
        simplification::simplify(ast)
    };

    // Step 5: Convert to string
    Ok(format!("{}", simplified))
}
