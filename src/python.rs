//! Python bindings for symb_anafis using PyO3

use pyo3::prelude::*;
use std::collections::HashSet;

/// Differentiate a mathematical expression symbolically.
///
/// Args:
///     formula: Mathematical expression to differentiate (e.g., "x^2 + sin(x)")
///     var: Variable to differentiate with respect to (e.g., "x")
///     fixed_vars: Optional list of symbols that are constants (e.g., ["a", "b"])
///     custom_functions: Optional list of user-defined function names (e.g., ["f", "g"])
///
/// Returns:
///     The derivative as a simplified string expression.
///
/// Example:
///     >>> import symb_anafis
///     >>> symb_anafis.diff("x^2 + sin(x)", "x")
///     '2 * x + cos(x)'
///     >>> symb_anafis.diff("a * x^2", "x", fixed_vars=["a"])
///     '2 * a * x'
#[pyfunction]
#[pyo3(signature = (formula, var, fixed_vars=None, custom_functions=None))]
fn diff(
    formula: &str,
    var: &str,
    fixed_vars: Option<Vec<String>>,
    custom_functions: Option<Vec<String>>,
) -> PyResult<String> {
    let fixed = fixed_vars.map(|v| v.iter().map(|s| s.to_string()).collect());
    let custom = custom_functions.map(|v| v.iter().map(|s| s.to_string()).collect());
    
    crate::diff(
        formula.to_string(),
        var.to_string(),
        fixed.as_ref().map(|v: &Vec<String>| v.as_slice()),
        custom.as_ref().map(|v: &Vec<String>| v.as_slice()),
    )
    .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
}

/// Simplify a mathematical expression.
///
/// Args:
///     formula: Mathematical expression to simplify (e.g., "x + x + x")
///     fixed_vars: Optional list of symbols that are constants
///     custom_functions: Optional list of user-defined function names
///
/// Returns:
///     The simplified expression as a string.
///
/// Example:
///     >>> import symb_anafis
///     >>> symb_anafis.simplify("x + x + x")
///     '3 * x'
///     >>> symb_anafis.simplify("sin(x)^2 + cos(x)^2")
///     '1'
#[pyfunction]
#[pyo3(signature = (formula, fixed_vars=None, custom_functions=None))]
fn simplify(
    formula: &str,
    fixed_vars: Option<Vec<String>>,
    custom_functions: Option<Vec<String>>,
) -> PyResult<String> {
    let fixed = fixed_vars.map(|v| v.iter().map(|s| s.to_string()).collect());
    let custom = custom_functions.map(|v| v.iter().map(|s| s.to_string()).collect());
    
    crate::simplify(
        formula.to_string(),
        fixed.as_ref().map(|v: &Vec<String>| v.as_slice()),
        custom.as_ref().map(|v: &Vec<String>| v.as_slice()),
    )
    .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
}

/// Parse a mathematical expression and return its string representation.
///
/// Args:
///     formula: Mathematical expression to parse
///     fixed_vars: Optional list of symbols that are constants
///     custom_functions: Optional list of user-defined function names
///
/// Returns:
///     The parsed expression as a normalized string.
#[pyfunction]
#[pyo3(signature = (formula, fixed_vars=None, custom_functions=None))]
fn parse(
    formula: &str,
    fixed_vars: Option<Vec<String>>,
    custom_functions: Option<Vec<String>>,
) -> PyResult<String> {
    let fixed: HashSet<String> = fixed_vars
        .map(|v| v.into_iter().collect())
        .unwrap_or_default();
    let custom: HashSet<String> = custom_functions
        .map(|v| v.into_iter().collect())
        .unwrap_or_default();
    
    crate::parse(formula, &fixed, &custom)
        .map(|expr| expr.to_string())
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
}

/// Fast symbolic differentiation library.
///
/// SymbAnaFis is a high-performance symbolic mathematics library written in Rust,
/// providing fast differentiation and simplification of mathematical expressions.
///
/// Functions:
///     diff(formula, var, fixed_vars=None, custom_functions=None) -> str
///         Differentiate an expression with respect to a variable.
///     
///     simplify(formula, fixed_vars=None, custom_functions=None) -> str
///         Simplify a mathematical expression.
///     
///     parse(formula, fixed_vars=None, custom_functions=None) -> str
///         Parse and normalize a mathematical expression.
///
/// Example:
///     >>> import symb_anafis
///     >>> symb_anafis.diff("x^3 + 2*x^2 + x", "x")
///     '3 * x^2 + 4 * x + 1'
///     >>> symb_anafis.simplify("sin(x)^2 + cos(x)^2")
///     '1'
#[pymodule]
fn symb_anafis(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(diff, m)?)?;
    m.add_function(wrap_pyfunction!(simplify, m)?)?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
