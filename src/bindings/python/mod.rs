//! Python bindings for `symb_anafis` using `PyO3`
//!
//! # Installation
//! ```bash
//! pip install symb_anafis
//! ```
//!
//! # Quick Start
//! ```python
//! from symb_anafis import Expr, Diff, Simplify, diff, simplify
//!
//! # Create symbolic expressions
//! x = Expr("x")
//! y = Expr("y")
//! expr = x ** 2 + y
//!
//! # Differentiate
//! result = diff("x^2 + sin(x)", "x")
//! print(result)  # "2*x + cos(x)"
//!
//! # Use builder API for more control
//! d = Diff().fixed_var("a").domain_safe(True)
//! result = d.diff_str("x^2 + a*x", "x")  # "2*x + a"
//!
//! # Simplify expressions
//! result = simplify("x + x + 0")  # "2*x"
//! ```
//!
//! # Available Classes
//! - `Expr` - Symbolic expression wrapper
//! - `Diff` - Differentiation builder
//! - `Simplify` - Simplification builder
//! - `Context` - Isolated symbol and function registry
//!
//! # Available Functions
//! - `diff(formula, var, known_symbols?, custom_functions?)` - Differentiate string formula
//! - `simplify(formula, known_symbols?, custom_functions?)` - Simplify string formula
//! - `parse(formula, known_symbols?, custom_functions?)` - Parse formula to string

// Submodules
mod builder;
mod context;
mod dual;
mod error;
mod evaluator;
mod expr;
mod functions;
mod parallel;
mod symbol;
mod utilities;
mod visitor;

// Re-exports
pub use builder::*;
pub use context::*;
pub use dual::*;
pub use evaluator::*;
pub use expr::*;
pub use functions::*;
#[cfg(feature = "parallel")]
pub use parallel::*;
pub use symbol::*;
pub use utilities::*;
pub use visitor::*;

// Main PyO3 module declaration
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

    // Add visitor utilities
    m.add_function(wrap_pyfunction!(count_nodes, m)?)?;
    m.add_function(wrap_pyfunction!(collect_variables, m)?)?;

    // Version
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    Ok(())
}
