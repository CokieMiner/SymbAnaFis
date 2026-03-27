use super::logic::{
    evaluate_str as do_evaluate_str, gradient as do_gradient, gradient_str as do_gradient_str,
    hessian as do_hessian, hessian_str as do_hessian_str, jacobian as do_jacobian,
    jacobian_str as do_jacobian_str,
};
use crate::core::{DiffError, Expr, Symbol};

/// Compute the gradient of an expression with respect to multiple variables.
///
/// # Errors
/// Returns `DiffError` if differentiation fails for any variable.
pub fn gradient(expr: &Expr, vars: &[&Symbol]) -> Result<Vec<Expr>, DiffError> {
    do_gradient(expr, vars)
}

/// Compute the Hessian matrix of an expression.
///
/// # Errors
/// Returns `DiffError` if any second partial derivative fails.
pub fn hessian(expr: &Expr, vars: &[&Symbol]) -> Result<Vec<Vec<Expr>>, DiffError> {
    do_hessian(expr, vars)
}

/// Compute the Jacobian matrix of a vector of expressions.
///
/// # Errors
/// Returns `DiffError` if any partial derivative fails.
pub fn jacobian(exprs: &[Expr], vars: &[&Symbol]) -> Result<Vec<Vec<Expr>>, DiffError> {
    do_jacobian(exprs, vars)
}

/// Compute gradient from a formula string.
///
/// # Errors
/// Returns `DiffError` if parsing or differentiation fails.
pub fn gradient_str(formula: &str, vars: &[&str]) -> Result<Vec<String>, DiffError> {
    do_gradient_str(formula, vars)
}

/// Compute Hessian matrix from a formula string.
///
/// # Errors
/// Returns `DiffError` if parsing or differentiation fails.
pub fn hessian_str(formula: &str, vars: &[&str]) -> Result<Vec<Vec<String>>, DiffError> {
    do_hessian_str(formula, vars)
}

/// Compute Jacobian matrix from formula strings.
///
/// # Errors
/// Returns `DiffError` if parsing or differentiation fails.
pub fn jacobian_str(formulas: &[&str], vars: &[&str]) -> Result<Vec<Vec<String>>, DiffError> {
    do_jacobian_str(formulas, vars)
}

/// Evaluate a formula string with given variable values.
///
/// Performs partial evaluation and returns the simplified expression string.
///
/// # Errors
/// Returns `DiffError` if the formula cannot be parsed.
pub fn evaluate_str(formula: &str, vars: &[(&str, f64)]) -> Result<String, DiffError> {
    do_evaluate_str(formula, vars)
}
