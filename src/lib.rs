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
pub use simplification::simplify;

use std::collections::HashSet;

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
    if ast.max_depth() > MAX_DEPTH {
        return Err(DiffError::MaxDepthExceeded);
    }
    if ast.node_count() > MAX_NODES {
        return Err(DiffError::MaxNodesExceeded);
    }

    // Step 5: Differentiate
    let derivative = ast.derive(&var_to_diff, &fixed_set);

    // Step 6: Simplify
    let simplified = simplification::simplify(derivative);

    // Step 7: Convert to string
    Ok(format!("{}", simplified))
}

// Constants for safety limits
const MAX_DEPTH: usize = 100;
const MAX_NODES: usize = 10_000;
