//! Python bindings for standalone functions
//!
//! This module provides the main API functions like `diff`, `simplify`, `parse`, etc.

use crate::Expr as RustExpr;
use crate::core::symbol::Symbol as RustSymbol;
use crate::{diff as rust_diff, simplify as rust_simplify};
use pyo3::prelude::*;
use std::collections::{HashMap, HashSet};

/// Differentiate a formula with respect to a variable
// PyO3 requires owned types; clippy suggestion to use references is invalid here
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 requires owned types for function arguments"
)]
#[pyfunction]
#[pyo3(signature = (formula, var, known_symbols=None, custom_functions=None))]
pub fn diff(
    formula: &str,
    var: &str,
    known_symbols: Option<Vec<String>>,
    custom_functions: Option<Vec<String>>,
) -> PyResult<String> {
    let known_strs: Vec<&str> = known_symbols
        .as_ref()
        .map(|v| v.iter().map(std::string::String::as_str).collect())
        .unwrap_or_default();
    let custom_strs: Option<Vec<&str>> = custom_functions
        .as_ref()
        .map(|v| v.iter().map(std::string::String::as_str).collect());

    rust_diff(formula, var, &known_strs, custom_strs.as_deref()).map_err(Into::into)
}

/// Simplify a mathematical expression string.
// PyO3 requires owned types; clippy suggestion to use references is invalid here
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 requires owned types for function arguments"
)]
#[pyfunction]
#[pyo3(signature = (formula, known_symbols=None, custom_functions=None))]
pub fn simplify(
    formula: &str,
    known_symbols: Option<Vec<String>>,
    custom_functions: Option<Vec<String>>,
) -> PyResult<String> {
    let known_strs: Vec<&str> = known_symbols
        .as_ref()
        .map(|v| v.iter().map(std::string::String::as_str).collect())
        .unwrap_or_default();
    let custom_strs: Option<Vec<&str>> = custom_functions
        .as_ref()
        .map(|v| v.iter().map(std::string::String::as_str).collect());

    rust_simplify(formula, &known_strs, custom_strs.as_deref()).map_err(Into::into)
}

/// Parse a mathematical expression and return the expression object.
// PyO3 requires owned types; clippy suggestion to use references is invalid here
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 requires owned types for function arguments"
)]
#[pyfunction]
#[pyo3(signature = (formula, known_symbols=None, custom_functions=None))]
pub fn parse(
    formula: &str,
    known_symbols: Option<Vec<String>>,
    custom_functions: Option<Vec<String>>,
) -> PyResult<super::expr::PyExpr> {
    let known: HashSet<String> = known_symbols
        .map(|v| v.into_iter().collect())
        .unwrap_or_default();
    let custom: HashSet<String> = custom_functions
        .map(|v| v.into_iter().collect())
        .unwrap_or_default();

    crate::parser::parse(formula, &known, &custom, None)
        .map(super::expr::PyExpr)
        .map_err(Into::into)
}

/// Compute the gradient of a scalar Expr.
// PyO3 requires owned types; clippy suggestion to use references is invalid here
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 requires owned types for function arguments"
)]
#[pyfunction]
pub fn gradient(
    expr: super::expr::PyExpr,
    vars: Vec<String>,
) -> PyResult<Vec<super::expr::PyExpr>> {
    let symbols: Vec<RustSymbol> = vars.iter().map(|s| crate::symb(s)).collect();
    let sym_refs: Vec<&RustSymbol> = symbols.iter().collect();

    let res = crate::gradient(&expr.0, &sym_refs).map_err(PyErr::from)?;
    Ok(res.into_iter().map(super::expr::PyExpr).collect())
}

/// Compute the gradient of a scalar expression string.
// PyO3 requires owned types; clippy suggestion to use references is invalid here
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 requires owned types for function arguments"
)]
#[pyfunction]
pub fn gradient_str(formula: &str, vars: Vec<String>) -> PyResult<Vec<String>> {
    let var_strs: Vec<&str> = vars.iter().map(std::string::String::as_str).collect();
    crate::gradient_str(formula, &var_strs).map_err(Into::into)
}

/// Compute the Hessian matrix of a scalar Expr.
// PyO3 requires owned types; clippy suggestion to use references is invalid here
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 requires owned types for function arguments"
)]
#[pyfunction]
pub fn hessian(
    expr: super::expr::PyExpr,
    vars: Vec<String>,
) -> PyResult<Vec<Vec<super::expr::PyExpr>>> {
    let symbols: Vec<RustSymbol> = vars.iter().map(|s| crate::symb(s)).collect();
    let sym_refs: Vec<&RustSymbol> = symbols.iter().collect();

    let res = crate::hessian(&expr.0, &sym_refs).map_err(PyErr::from)?;
    Ok(res
        .into_iter()
        .map(|row| row.into_iter().map(super::expr::PyExpr).collect())
        .collect())
}

/// Compute the Hessian matrix of a scalar expression string.
// PyO3 requires owned types; clippy suggestion to use references is invalid here
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 requires owned types for function arguments"
)]
#[pyfunction]
pub fn hessian_str(formula: &str, vars: Vec<String>) -> PyResult<Vec<Vec<String>>> {
    let var_strs: Vec<&str> = vars.iter().map(std::string::String::as_str).collect();
    crate::hessian_str(formula, &var_strs).map_err(Into::into)
}

/// Compute the Jacobian matrix of a vector of Expr objects.
// PyO3 requires owned types; clippy suggestion to use references is invalid here
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 requires owned types for function arguments"
)]
#[pyfunction]
pub fn jacobian(
    exprs: Vec<super::expr::PyExpr>,
    vars: Vec<String>,
) -> PyResult<Vec<Vec<super::expr::PyExpr>>> {
    let rust_exprs: Vec<RustExpr> = exprs.into_iter().map(|e| e.0).collect();
    let symbols: Vec<RustSymbol> = vars.iter().map(|s| crate::symb(s)).collect();
    let sym_refs: Vec<&RustSymbol> = symbols.iter().collect();

    let res = crate::jacobian(&rust_exprs, &sym_refs).map_err(PyErr::from)?;
    Ok(res
        .into_iter()
        .map(|row| row.into_iter().map(super::expr::PyExpr).collect())
        .collect())
}

/// Compute the Jacobian matrix of a vector function from strings.
// PyO3 requires owned types; clippy suggestion to use references is invalid here
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 requires owned types for function arguments"
)]
#[pyfunction]
pub fn jacobian_str(formulas: Vec<String>, vars: Vec<String>) -> PyResult<Vec<Vec<String>>> {
    let f_strs: Vec<&str> = formulas.iter().map(std::string::String::as_str).collect();
    let var_strs: Vec<&str> = vars.iter().map(std::string::String::as_str).collect();
    crate::jacobian_str(&f_strs, &var_strs).map_err(Into::into)
}

/// Evaluate an Expr with given variable values.
// PyO3 requires owned types; clippy suggestion to use references is invalid here
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 requires owned types for function arguments"
)]
#[pyfunction]
pub fn evaluate(expr: super::expr::PyExpr, vars: Vec<(String, f64)>) -> super::expr::PyExpr {
    let var_map: HashMap<&str, f64> = vars.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    let empty_funcs = HashMap::new();
    let res = expr.0.evaluate(&var_map, &empty_funcs);
    super::expr::PyExpr(res)
}

/// Evaluate a string expression with given variable values.
// PyO3 requires owned types; clippy suggestion to use references is invalid here
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 requires owned types for function arguments"
)]
#[pyfunction]
pub fn evaluate_str(formula: &str, vars: Vec<(String, f64)>) -> PyResult<String> {
    let var_tuples: Vec<(&str, f64)> = vars.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    crate::evaluate_str(formula, &var_tuples).map_err(Into::into)
}

/// Compute uncertainty propagation for an expression.
// PyO3 requires owned types; clippy suggestion to use references is invalid here
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 requires owned types for function arguments"
)]
#[pyfunction]
#[pyo3(name = "uncertainty_propagation", signature = (formula, variables, variances=None))]
pub fn uncertainty_propagation_py(
    formula: &Bound<'_, PyAny>,
    variables: Vec<String>,
    variances: Option<Vec<f64>>,
) -> PyResult<String> {
    // 1. Get the Expr (either parsed from string or extracted directly)
    let expr = if let Ok(s) = formula.extract::<String>() {
        crate::parser::parse(&s, &HashSet::new(), &HashSet::new(), None).map_err(PyErr::from)?
    } else if let Ok(e) = formula.extract::<super::expr::PyExpr>() {
        e.0
    } else {
        return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Argument `formula` must be str or Expr",
        ));
    };

    let var_strs: Vec<&str> = variables.iter().map(std::string::String::as_str).collect();

    let cov = variances.map(|vars| {
        crate::CovarianceMatrix::diagonal(vars.into_iter().map(crate::CovEntry::Num).collect())
    });

    crate::uncertainty_propagation(&expr, &var_strs, cov.as_ref())
        .map(|e| e.to_string())
        .map_err(Into::into)
}

/// Compute relative uncertainty for an expression.
// PyO3 requires owned types; clippy suggestion to use references is invalid here
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 requires owned types for function arguments"
)]
#[pyfunction]
#[pyo3(name = "relative_uncertainty", signature = (formula, variables, variances=None))]
pub fn relative_uncertainty_py(
    formula: &Bound<'_, PyAny>,
    variables: Vec<String>,
    variances: Option<Vec<f64>>,
) -> PyResult<String> {
    // 1. Get the Expr
    let expr = if let Ok(s) = formula.extract::<String>() {
        crate::parser::parse(&s, &HashSet::new(), &HashSet::new(), None).map_err(PyErr::from)?
    } else if let Ok(e) = formula.extract::<super::expr::PyExpr>() {
        e.0
    } else {
        return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Argument `formula` must be str or Expr",
        ));
    };

    let var_strs: Vec<&str> = variables.iter().map(std::string::String::as_str).collect();

    let cov = variances.map(|vars| {
        crate::CovarianceMatrix::diagonal(vars.into_iter().map(crate::CovEntry::Num).collect())
    });

    crate::relative_uncertainty(&expr, &var_strs, cov.as_ref())
        .map(|e| e.to_string())
        .map_err(Into::into)
}
