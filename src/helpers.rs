//! Multi-variable differentiation helpers
//!
//! Provides gradient, hessian, and jacobian computation functions
//! for both Expr-based and String-based APIs.

use crate::{Diff, DiffError, Expr, Symbol, parser};
use std::collections::HashSet;

// ===== Internal Helpers =====

/// Empty context for parsing without custom functions or fixed variables
fn empty_context() -> (HashSet<String>, HashSet<String>) {
    (HashSet::new(), HashSet::new())
}

/// Internal gradient implementation using &str variable names
/// This is the core implementation - public APIs wrap this
fn gradient_internal(expr: &Expr, vars: &[&str]) -> Vec<Expr> {
    let diff = Diff::new();
    vars.iter()
        .map(|var| {
            // Use differentiate_by_name to avoid creating Symbol from &str
            diff.differentiate_by_name(expr.clone(), var)
                .unwrap_or_else(|_| Expr::number(0.0))
        })
        .collect()
}

/// Internal hessian implementation using &str variable names
fn hessian_internal(expr: &Expr, vars: &[&str]) -> Vec<Vec<Expr>> {
    let diff = Diff::new();
    let grad = gradient_internal(expr, vars);

    grad.iter()
        .map(|partial| {
            vars.iter()
                .map(|var| {
                    // Use differentiate_by_name to avoid creating Symbol from &str
                    diff.differentiate_by_name(partial.clone(), var)
                        .unwrap_or_else(|_| Expr::number(0.0))
                })
                .collect()
        })
        .collect()
}

/// Internal jacobian implementation using &str variable names
fn jacobian_internal(exprs: &[Expr], vars: &[&str]) -> Vec<Vec<Expr>> {
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
/// ```ignore
/// let x = symb("x");
/// let y = symb("y");
/// let expr = x.pow(2.0) + y.pow(2.0);
/// let grad = gradient(&expr, &[&x, &y]);
/// // grad = [2*x, 2*y]
/// ```
pub fn gradient(expr: &Expr, vars: &[&Symbol]) -> Vec<Expr> {
    let var_names: Vec<&str> = vars.iter().filter_map(|s| s.name()).collect();
    gradient_internal(expr, &var_names)
}

/// Compute the Hessian matrix of an expression
/// Returns a 2D vector of second partial derivatives
/// H\[i\]\[j\] = ∂²f/∂xᵢ∂xⱼ
///
/// # Example
/// ```ignore
/// let x = symb("x");
/// let y = symb("y");
/// let expr = x.pow(2.0) * &y;
/// let hess = hessian(&expr, &[&x, &y]);
/// // hess = [[2*y, 2*x], [2*x, 0]]
/// ```
pub fn hessian(expr: &Expr, vars: &[&Symbol]) -> Vec<Vec<Expr>> {
    let var_names: Vec<&str> = vars.iter().filter_map(|s| s.name()).collect();
    hessian_internal(expr, &var_names)
}

/// Compute the Jacobian matrix of a vector of expressions
/// Returns a 2D vector where `J[i][j]` = ∂fᵢ/∂xⱼ
///
/// # Example
/// ```ignore
/// let x = symb("x");
/// let y = symb("y");
/// let f1 = x.pow(2.0) + &y;
/// let f2 = &x * &y;
/// let jac = jacobian(&[f1, f2], &[&x, &y]);
/// // jac = [[2*x, 1], [y, x]]
/// ```
pub fn jacobian(exprs: &[Expr], vars: &[&Symbol]) -> Vec<Vec<Expr>> {
    let var_names: Vec<&str> = vars.iter().filter_map(|s| s.name()).collect();
    jacobian_internal(exprs, &var_names)
}

// ===== String-based API =====

/// Compute gradient from a formula string
///
/// # Example
/// ```ignore
/// let grad = gradient_str("x^2 + y^2", &["x", "y"])?;
/// // grad = ["2x", "2y"]
/// ```
pub fn gradient_str(formula: &str, vars: &[&str]) -> Result<Vec<String>, DiffError> {
    let (fixed_vars, custom_fns) = empty_context();
    let expr = parser::parse(formula, &fixed_vars, &custom_fns, None)?;

    // Call internal directly - no Symbol conversion needed!
    let grad = gradient_internal(&expr, vars);
    Ok(grad.iter().map(|e| e.to_string()).collect())
}

/// Compute Hessian matrix from a formula string
///
/// # Example
/// ```ignore
/// let hess = hessian_str("x^2 * y", &["x", "y"])?;
/// // hess = [["2y", "2x"], ["2x", "0"]]
/// ```
pub fn hessian_str(formula: &str, vars: &[&str]) -> Result<Vec<Vec<String>>, DiffError> {
    let (fixed_vars, custom_fns) = empty_context();
    let expr = parser::parse(formula, &fixed_vars, &custom_fns, None)?;

    // Call internal directly - no Symbol conversion needed!
    let hess = hessian_internal(&expr, vars);
    Ok(hess
        .iter()
        .map(|row| row.iter().map(|e| e.to_string()).collect())
        .collect())
}

/// Compute Jacobian matrix from formula strings
///
/// # Example
/// ```ignore
/// let jac = jacobian_str(&["x^2 + y", "x * y"], &["x", "y"])?;
/// // jac = [["2x", "1"], ["y", "x"]]
/// ```
pub fn jacobian_str(formulas: &[&str], vars: &[&str]) -> Result<Vec<Vec<String>>, DiffError> {
    let (fixed_vars, custom_fns) = empty_context();

    let exprs: Vec<Expr> = formulas
        .iter()
        .map(|f| parser::parse(f, &fixed_vars, &custom_fns, None))
        .collect::<Result<Vec<_>, _>>()?;

    // Call internal directly - no Symbol conversion needed!
    let jac = jacobian_internal(&exprs, vars);
    Ok(jac
        .iter()
        .map(|row| row.iter().map(|e| e.to_string()).collect())
        .collect())
}

/// Evaluate a formula string with given variable values
/// Performs partial evaluation - returns simplified expression string
///
/// # Example
/// ```ignore
/// let result = evaluate_str("x * y + 1", &[("x", 3.0)])?;
/// // result = "3y + 1"
///
/// let result = evaluate_str("x * y + 1", &[("x", 3.0), ("y", 2.0)])?;
/// // result = "7"
/// ```
pub fn evaluate_str(formula: &str, vars: &[(&str, f64)]) -> Result<String, DiffError> {
    let (fixed_vars, custom_fns) = empty_context();
    let expr = parser::parse(formula, &fixed_vars, &custom_fns, None)?;

    let var_map: std::collections::HashMap<&str, f64> = vars.iter().cloned().collect();
    let result = expr.evaluate(&var_map);
    Ok(result.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gradient() {
        let grad = gradient_str("x^2 + y^2", &["x", "y"]).unwrap();
        assert_eq!(grad.len(), 2);
        assert_eq!(grad[0], "2x");
        assert_eq!(grad[1], "2y");
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
        assert_eq!(jac[0][0], "2x");
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
