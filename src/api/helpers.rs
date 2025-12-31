//! Multi-variable differentiation helpers
//!
//! Provides gradient, hessian, and jacobian computation functions
//! for both Expr-based and String-based APIs.

use crate::{Diff, DiffError, Expr, Symbol, parser};
use std::collections::HashSet;

// ===== Internal Helpers =====

/// Empty context for parsing without custom functions or fixed variables
#[inline]
fn empty_context() -> (HashSet<String>, HashSet<String>) {
    (HashSet::new(), HashSet::new())
}

/// Helper to extract variable names from Symbol slices
#[inline]
fn extract_var_names(vars: &[&Symbol]) -> Vec<String> {
    vars.iter().filter_map(|s| s.name()).collect()
}

/// Helper to convert variable names to &str slices
#[inline]
fn var_names_to_str_refs(var_names: &[String]) -> Vec<&str> {
    var_names.iter().map(|s| s.as_str()).collect()
}

/// Internal gradient implementation using &str variable names
/// This is the core implementation - public APIs wrap this
fn gradient_internal(expr: &Expr, vars: &[&str]) -> Result<Vec<Expr>, DiffError> {
    let diff = Diff::new();
    vars.iter()
        .map(|var| diff.differentiate_by_name(expr, var))
        .collect()
}

/// Internal hessian implementation using &str variable names
fn hessian_internal(expr: &Expr, vars: &[&str]) -> Result<Vec<Vec<Expr>>, DiffError> {
    let diff = Diff::new();
    let grad = gradient_internal(expr, vars)?;

    grad.iter()
        .map(|partial| {
            vars.iter()
                .map(|var| diff.differentiate_by_name(partial, var))
                .collect::<Result<Vec<_>, _>>()
        })
        .collect()
}

/// Internal jacobian implementation using &str variable names
fn jacobian_internal(exprs: &[Expr], vars: &[&str]) -> Result<Vec<Vec<Expr>>, DiffError> {
    exprs
        .iter()
        .map(|expr| gradient_internal(expr, vars))
        .collect()
}

// ===== Public Symbol-based API =====

/// Compute the gradient of an expression with respect to multiple variables
/// Returns a vector of partial derivatives [∂f/∂x₁, ∂f/∂x₂, ...]
///
/// # Example
/// ```
/// use symb_anafis::{symb, gradient};
/// let x = symb("gradient_x");
/// let y = symb("gradient_y");
/// let expr = x.pow(2.0) + y.pow(2.0);
/// let grad = gradient(&expr, &[&x, &y]).unwrap();
/// assert_eq!(grad.len(), 2);
/// ```
pub fn gradient(expr: &Expr, vars: &[&Symbol]) -> Result<Vec<Expr>, DiffError> {
    let var_names = extract_var_names(vars);
    let var_refs = var_names_to_str_refs(&var_names);
    gradient_internal(expr, &var_refs)
}

/// Compute the Hessian matrix of an expression
/// Returns a 2D vector of second partial derivatives
/// H\[i\]\[j\] = ∂²f/∂xᵢ∂xⱼ
///
/// # Example
/// ```
/// use symb_anafis::{symb, hessian};
/// let x = symb("hessian_x");
/// let y = symb("hessian_y");
/// let expr = x.pow(2.0) * &y;
/// let hess = hessian(&expr, &[&x, &y]).unwrap();
/// assert_eq!(hess.len(), 2);
/// assert_eq!(hess[0].len(), 2);
/// ```
pub fn hessian(expr: &Expr, vars: &[&Symbol]) -> Result<Vec<Vec<Expr>>, DiffError> {
    let var_names = extract_var_names(vars);
    let var_refs = var_names_to_str_refs(&var_names);
    hessian_internal(expr, &var_refs)
}

/// Compute the Jacobian matrix of a vector of expressions
/// Returns a 2D vector where `J[i][j]` = ∂fᵢ/∂xⱼ
///
/// # Example
/// ```
/// use symb_anafis::{symb, jacobian};
/// let x = symb("jacobian_x");
/// let y = symb("jacobian_y");
/// let f1 = x.pow(2.0) + &y;
/// let f2 = &x * &y;
/// let jac = jacobian(&[f1, f2], &[&x, &y]).unwrap();
/// assert_eq!(jac.len(), 2);
/// assert_eq!(jac[0].len(), 2);
/// ```
pub fn jacobian(exprs: &[Expr], vars: &[&Symbol]) -> Result<Vec<Vec<Expr>>, DiffError> {
    let var_names: Vec<String> = vars.iter().filter_map(|s| s.name()).collect();
    let var_refs: Vec<&str> = var_names.iter().map(|s| s.as_str()).collect();
    jacobian_internal(exprs, &var_refs)
}

// ===== String-based API =====

/// Helper to parse a formula string with empty context
#[inline]
fn parse_formula(formula: &str) -> Result<Expr, DiffError> {
    let (fixed_vars, custom_fns) = empty_context();
    parser::parse(formula, &fixed_vars, &custom_fns, None)
}

/// Compute gradient from a formula string
///
/// # Example
/// ```
/// use symb_anafis::gradient_str;
/// let grad = gradient_str("x^2 + y^2", &["x", "y"]).unwrap();
/// assert_eq!(grad.len(), 2);
/// assert_eq!(grad[0], "2*x");
/// ```
pub fn gradient_str(formula: &str, vars: &[&str]) -> Result<Vec<String>, DiffError> {
    let expr = parse_formula(formula)?;
    let grad = gradient_internal(&expr, vars)?;
    Ok(grad.iter().map(|e| e.to_string()).collect())
}

/// Compute Hessian matrix from a formula string
///
/// # Example
/// ```
/// use symb_anafis::hessian_str;
/// let hess = hessian_str("x^2 + y^2", &["x", "y"]).unwrap();
/// assert_eq!(hess.len(), 2);
/// assert_eq!(hess[0][0], "2");
/// ```
pub fn hessian_str(formula: &str, vars: &[&str]) -> Result<Vec<Vec<String>>, DiffError> {
    let expr = parse_formula(formula)?;
    let hess = hessian_internal(&expr, vars)?;
    Ok(hess
        .iter()
        .map(|row| row.iter().map(|e| e.to_string()).collect())
        .collect())
}

/// Helper to parse multiple formula strings with empty context
#[inline]
fn parse_formulas(formulas: &[&str]) -> Result<Vec<Expr>, DiffError> {
    let (fixed_vars, custom_fns) = empty_context();
    formulas
        .iter()
        .map(|f| parser::parse(f, &fixed_vars, &custom_fns, None))
        .collect()
}

/// Compute Jacobian matrix from formula strings
///
/// # Example
/// ```
/// use symb_anafis::jacobian_str;
/// let jac = jacobian_str(&["x^2 + y", "x * y"], &["x", "y"]).unwrap();
/// assert_eq!(jac.len(), 2);
/// assert_eq!(jac[0][0], "2*x");
/// ```
pub fn jacobian_str(formulas: &[&str], vars: &[&str]) -> Result<Vec<Vec<String>>, DiffError> {
    let exprs = parse_formulas(formulas)?;
    let jac = jacobian_internal(&exprs, vars)?;
    Ok(jac
        .iter()
        .map(|row| row.iter().map(|e| e.to_string()).collect())
        .collect())
}

/// Evaluate a formula string with given variable values
/// Performs partial evaluation - returns simplified expression string
///
/// # Example
/// ```
/// use symb_anafis::evaluate_str;
/// let result = evaluate_str("x * y", &[("x", 3.0), ("y", 2.0)]).unwrap();
/// assert_eq!(result, "6");
///
/// // Partial evaluation:
/// let result = evaluate_str("x * y", &[("x", 3.0)]).unwrap();
/// assert!(result.contains("3") && result.contains("y"));
/// ```
pub fn evaluate_str(formula: &str, vars: &[(&str, f64)]) -> Result<String, DiffError> {
    let (fixed_vars, custom_fns) = empty_context();
    let expr = parser::parse(formula, &fixed_vars, &custom_fns, None)?;

    let var_map: std::collections::HashMap<&str, f64> = vars.iter().cloned().collect();
    let result = expr.evaluate(&var_map, &std::collections::HashMap::new());
    Ok(result.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gradient() {
        let grad = gradient_str("x^2 + y^2", &["x", "y"]).unwrap();
        assert_eq!(grad.len(), 2);
        assert_eq!(grad[0], "2*x");
        assert_eq!(grad[1], "2*y");
    }

    #[test]
    fn test_hessian() {
        let hess = hessian_str("x^2 + y^2", &["x", "y"]).unwrap();
        assert_eq!(hess.len(), 2);
        assert_eq!(hess[0].len(), 2);
        assert_eq!(hess[0][0], "2");
        assert_eq!(hess[1][1], "2");
    }

    #[test]
    fn test_jacobian() {
        let jac = jacobian_str(&["x^2", "x * y"], &["x", "y"]).unwrap();
        assert_eq!(jac.len(), 2);
        assert_eq!(jac[0][0], "2*x");
        assert_eq!(jac[1][0], "y");
    }

    #[test]
    fn test_evaluate_str_partial() {
        let result = evaluate_str("x * y", &[("x", 3.0)]).unwrap();
        assert!(result.contains("3") && result.contains("y"));
    }

    #[test]
    fn test_evaluate_str_full() {
        let result = evaluate_str("x * y", &[("x", 3.0), ("y", 2.0)]).unwrap();
        assert_eq!(result, "6");
    }
}
