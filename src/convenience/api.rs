//! Public convenience APIs for common one-call operations.

use crate::{DiffError, Expr, Symbol};

/// Compute the gradient of an expression with respect to multiple variables.
///
/// # Errors
/// Returns `DiffError` if differentiation fails for any variable.
pub fn gradient(expr: &Expr, vars: &[&Symbol]) -> Result<Vec<Expr>, DiffError> {
    super::logic::calculus::gradient(expr, vars)
}

/// Compute the Hessian matrix of an expression.
///
/// # Errors
/// Returns `DiffError` if any second partial derivative fails.
pub fn hessian(expr: &Expr, vars: &[&Symbol]) -> Result<Vec<Vec<Expr>>, DiffError> {
    super::logic::calculus::hessian(expr, vars)
}

/// Compute the Jacobian matrix of a vector of expressions.
///
/// # Errors
/// Returns `DiffError` if any partial derivative fails.
pub fn jacobian(exprs: &[Expr], vars: &[&Symbol]) -> Result<Vec<Vec<Expr>>, DiffError> {
    super::logic::calculus::jacobian(exprs, vars)
}

/// Compute gradient from a formula string.
///
/// # Errors
/// Returns `DiffError` if parsing or differentiation fails.
pub fn gradient_str(formula: &str, vars: &[&str]) -> Result<Vec<String>, DiffError> {
    super::logic::calculus::gradient_str(formula, vars)
}

/// Compute Hessian matrix from a formula string.
///
/// # Errors
/// Returns `DiffError` if parsing or differentiation fails.
pub fn hessian_str(formula: &str, vars: &[&str]) -> Result<Vec<Vec<String>>, DiffError> {
    super::logic::calculus::hessian_str(formula, vars)
}

/// Compute Jacobian matrix from formula strings.
///
/// # Errors
/// Returns `DiffError` if parsing or differentiation fails.
pub fn jacobian_str(formulas: &[&str], vars: &[&str]) -> Result<Vec<Vec<String>>, DiffError> {
    super::logic::calculus::jacobian_str(formulas, vars)
}

/// Evaluate a formula string with given variable values.
///
/// Performs partial evaluation and returns the simplified expression string.
///
/// # Errors
/// Returns `DiffError` if the formula cannot be parsed.
pub fn evaluate_str(formula: &str, vars: &[(&str, f64)]) -> Result<String, DiffError> {
    super::logic::evaluation::evaluate_str(formula, vars)
}
