//! Python module registration and API surface mapping.

use super::{
    PyCompiledEvaluator, PyContext, PyDiff, PyDual, PyExpr, PyExprView, PyFunctionContext,
    PySimplify, PySymbol, diff, evaluate, evaluate_str, gradient, gradient_str, hessian,
    hessian_str, jacobian, jacobian_str, parse, py_clear_symbols, py_remove_symbol, py_symb,
    py_symb_get, py_symb_new, py_symbol_count, py_symbol_exists, py_symbol_names,
    relative_uncertainty_py, simplify, uncertainty_propagation_py,
};
#[cfg(feature = "parallel")]
use super::{eval_f64, evaluate_parallel};
use pyo3::prelude::*;

/// Python module for symbolic mathematics
#[pymodule]
fn symb_anafis(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add classes
    m.add_class::<PyExpr>()?;
    m.add_class::<PySymbol>()?;
    m.add_class::<PyCompiledEvaluator>()?;
    m.add_class::<PyContext>()?;
    m.add_class::<PyFunctionContext>()?;
    m.add_class::<PyDual>()?;
    m.add_class::<PyDiff>()?;
    m.add_class::<PySimplify>()?;
    m.add_class::<PyExprView>()?;

    // Add functions
    m.add_function(wrap_pyfunction!(diff, m)?)?;
    m.add_function(wrap_pyfunction!(simplify, m)?)?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(gradient, m)?)?;
    m.add_function(wrap_pyfunction!(hessian, m)?)?;
    m.add_function(wrap_pyfunction!(jacobian, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate, m)?)?;
    m.add_function(wrap_pyfunction!(gradient_str, m)?)?;
    m.add_function(wrap_pyfunction!(hessian_str, m)?)?;
    m.add_function(wrap_pyfunction!(jacobian_str, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_str, m)?)?;
    m.add_function(wrap_pyfunction!(uncertainty_propagation_py, m)?)?;
    m.add_function(wrap_pyfunction!(relative_uncertainty_py, m)?)?;

    // Add parallel evaluation
    #[cfg(feature = "parallel")]
    m.add_function(wrap_pyfunction!(evaluate_parallel, m)?)?;
    #[cfg(feature = "parallel")]
    m.add_function(wrap_pyfunction!(eval_f64, m)?)?;

    // Add symbol management
    m.add_function(wrap_pyfunction!(py_symb, m)?)?;
    m.add_function(wrap_pyfunction!(py_symb_new, m)?)?;
    m.add_function(wrap_pyfunction!(py_symb_get, m)?)?;
    m.add_function(wrap_pyfunction!(py_remove_symbol, m)?)?;

    // Add utilities
    m.add_function(wrap_pyfunction!(py_clear_symbols, m)?)?;
    m.add_function(wrap_pyfunction!(py_symbol_count, m)?)?;
    m.add_function(wrap_pyfunction!(py_symbol_names, m)?)?;
    m.add_function(wrap_pyfunction!(py_symbol_exists, m)?)?;

    // Version
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    Ok(())
}
