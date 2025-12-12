//! Python bindings for symb_anafis using PyO3
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
//!
//! # Available Functions
//! - `diff(formula, var, fixed_vars?, custom_functions?)` - Differentiate string formula
//! - `simplify(formula, fixed_vars?, custom_functions?)` - Simplify string formula
//! - `parse(formula, fixed_vars?, custom_functions?)` - Parse formula to string

#[cfg(feature = "parallel")]
use crate::parallel::{self, ExprInput, Value, VarInput};
use crate::uncertainty::{CovEntry, CovarianceMatrix};
use crate::{Expr as RustExpr, builder, symb};
use pyo3::prelude::*;
use std::collections::HashSet;

/// Wrapper for Rust Expr to expose to Python
#[pyclass(unsendable, name = "Expr")]
#[derive(Clone)]
struct PyExpr(RustExpr);

#[pymethods]
impl PyExpr {
    /// Create a symbolic expression from a string
    #[new]
    fn new(name: &str) -> Self {
        PyExpr(symb(name).into())
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("Expr({})", self.0)
    }

    // Arithmetic operators
    fn __add__(&self, other: &PyExpr) -> PyExpr {
        PyExpr(self.0.clone() + other.0.clone())
    }

    fn __sub__(&self, other: &PyExpr) -> PyExpr {
        PyExpr(self.0.clone() - other.0.clone())
    }

    fn __mul__(&self, other: &PyExpr) -> PyExpr {
        PyExpr(self.0.clone() * other.0.clone())
    }

    fn __truediv__(&self, other: &PyExpr) -> PyExpr {
        PyExpr(self.0.clone() / other.0.clone())
    }

    fn __pow__(&self, other: &Bound<'_, PyAny>, _modulo: Option<Py<PyAny>>) -> PyResult<PyExpr> {
        // Try to extract as PyExpr first
        if let Ok(expr) = other.extract::<PyExpr>() {
            return Ok(PyExpr(self.0.clone().pow_of(expr.0)));
        }
        // Try as float
        if let Ok(n) = other.extract::<f64>() {
            return Ok(PyExpr(self.0.clone().pow_of(n)));
        }
        // Try as int
        if let Ok(n) = other.extract::<i64>() {
            return Ok(PyExpr(self.0.clone().pow_of(n as f64)));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "pow() argument must be Expr, int, or float",
        ))
    }

    // Reverse power: 2 ** x where x is Expr
    fn __rpow__(&self, other: &Bound<'_, PyAny>, _modulo: Option<Py<PyAny>>) -> PyResult<PyExpr> {
        // other ** self (other is the base, self is the exponent)
        if let Ok(n) = other.extract::<f64>() {
            return Ok(PyExpr(RustExpr::number(n).pow_of(self.0.clone())));
        }
        if let Ok(n) = other.extract::<i64>() {
            return Ok(PyExpr(RustExpr::number(n as f64).pow_of(self.0.clone())));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "rpow() base must be int or float",
        ))
    }
    // Functions
    fn sin(&self) -> PyExpr {
        PyExpr(self.0.clone().sin())
    }
    fn cos(&self) -> PyExpr {
        PyExpr(self.0.clone().cos())
    }
    fn tan(&self) -> PyExpr {
        PyExpr(self.0.clone().tan())
    }
    fn cot(&self) -> PyExpr {
        PyExpr(self.0.clone().cot())
    }
    fn sec(&self) -> PyExpr {
        PyExpr(self.0.clone().sec())
    }
    fn csc(&self) -> PyExpr {
        PyExpr(self.0.clone().csc())
    }

    fn asin(&self) -> PyExpr {
        PyExpr(self.0.clone().asin())
    }
    fn acos(&self) -> PyExpr {
        PyExpr(self.0.clone().acos())
    }
    fn atan(&self) -> PyExpr {
        PyExpr(self.0.clone().atan())
    }
    fn acot(&self) -> PyExpr {
        PyExpr(self.0.clone().acot())
    }
    fn asec(&self) -> PyExpr {
        PyExpr(self.0.clone().asec())
    }
    fn acsc(&self) -> PyExpr {
        PyExpr(self.0.clone().acsc())
    }

    fn sinh(&self) -> PyExpr {
        PyExpr(self.0.clone().sinh())
    }
    fn cosh(&self) -> PyExpr {
        PyExpr(self.0.clone().cosh())
    }
    fn tanh(&self) -> PyExpr {
        PyExpr(self.0.clone().tanh())
    }
    fn coth(&self) -> PyExpr {
        PyExpr(self.0.clone().coth())
    }
    fn sech(&self) -> PyExpr {
        PyExpr(self.0.clone().sech())
    }
    fn csch(&self) -> PyExpr {
        PyExpr(self.0.clone().csch())
    }

    fn asinh(&self) -> PyExpr {
        PyExpr(self.0.clone().asinh())
    }
    fn acosh(&self) -> PyExpr {
        PyExpr(self.0.clone().acosh())
    }
    fn atanh(&self) -> PyExpr {
        PyExpr(self.0.clone().atanh())
    }
    fn acoth(&self) -> PyExpr {
        PyExpr(self.0.clone().acoth())
    }
    fn asech(&self) -> PyExpr {
        PyExpr(self.0.clone().asech())
    }
    fn acsch(&self) -> PyExpr {
        PyExpr(self.0.clone().acsch())
    }

    fn exp(&self) -> PyExpr {
        PyExpr(self.0.clone().exp())
    }
    fn ln(&self) -> PyExpr {
        PyExpr(self.0.clone().ln())
    }
    fn log(&self) -> PyExpr {
        PyExpr(self.0.clone().log())
    }
    fn log10(&self) -> PyExpr {
        PyExpr(self.0.clone().log10())
    }
    fn log2(&self) -> PyExpr {
        PyExpr(self.0.clone().log2())
    }

    fn sqrt(&self) -> PyExpr {
        PyExpr(self.0.clone().sqrt())
    }
    fn cbrt(&self) -> PyExpr {
        PyExpr(self.0.clone().cbrt())
    }

    fn abs(&self) -> PyExpr {
        PyExpr(self.0.clone().abs())
    }
    fn sign(&self) -> PyExpr {
        PyExpr(self.0.clone().sign())
    }
    fn sinc(&self) -> PyExpr {
        PyExpr(self.0.clone().sinc())
    }
    fn erf(&self) -> PyExpr {
        PyExpr(self.0.clone().erf())
    }
    fn erfc(&self) -> PyExpr {
        PyExpr(self.0.clone().erfc())
    }
    fn gamma(&self) -> PyExpr {
        PyExpr(self.0.clone().gamma())
    }
    fn digamma(&self) -> PyExpr {
        PyExpr(self.0.clone().digamma())
    }
    fn trigamma(&self) -> PyExpr {
        PyExpr(self.0.clone().trigamma())
    }
    fn polygamma(&self, n: f64) -> PyExpr {
        PyExpr(self.0.clone().polygamma(n))
    }
    fn beta(&self, other: &PyExpr) -> PyExpr {
        PyExpr(self.0.clone().beta(other.0.clone()))
    }
    fn zeta(&self) -> PyExpr {
        PyExpr(self.0.clone().zeta())
    }
    fn lambertw(&self) -> PyExpr {
        PyExpr(self.0.clone().lambertw())
    }
    fn besselj(&self, n: f64) -> PyExpr {
        PyExpr(self.0.clone().besselj(n))
    }
    fn bessely(&self, n: f64) -> PyExpr {
        PyExpr(self.0.clone().bessely(n))
    }
    fn besseli(&self, n: f64) -> PyExpr {
        PyExpr(self.0.clone().besseli(n))
    }
    fn besselk(&self, n: f64) -> PyExpr {
        PyExpr(self.0.clone().besselk(n))
    }

    fn pow(&self, exp: f64) -> PyExpr {
        PyExpr(self.0.clone().pow_of(exp))
    }

    // Output formats
    /// Convert expression to LaTeX string
    fn to_latex(&self) -> String {
        self.0.to_latex()
    }

    /// Convert expression to Unicode string (with Greek symbols, superscripts)
    fn to_unicode(&self) -> String {
        self.0.to_unicode()
    }

    // Expression info
    /// Get the number of nodes in the expression tree
    fn node_count(&self) -> usize {
        self.0.node_count()
    }

    /// Get the maximum depth of the expression tree
    fn max_depth(&self) -> usize {
        self.0.max_depth()
    }

    /// Substitute a variable with a numeric value or another expression
    #[pyo3(signature = (var, value))]
    fn substitute(&self, var: &str, value: &PyExpr) -> PyExpr {
        PyExpr(self.0.substitute(var, &value.0))
    }

    /// Evaluate the expression with given variable values
    ///
    /// Args:
    ///     vars: dict mapping variable names to float values
    ///
    /// Returns:
    ///     Evaluated expression (may be a number or symbolic if variables remain)
    fn evaluate(&self, vars: std::collections::HashMap<String, f64>) -> PyExpr {
        let var_map: std::collections::HashMap<&str, f64> =
            vars.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        PyExpr(self.0.evaluate(&var_map))
    }
}

/// Builder for differentiation operations
#[pyclass(name = "Diff")]
struct PyDiff {
    inner: builder::Diff,
}

#[pymethods]
impl PyDiff {
    #[new]
    fn new() -> Self {
        PyDiff {
            inner: builder::Diff::new(),
        }
    }

    fn domain_safe(mut self_: PyRefMut<'_, Self>, safe: bool) -> PyRefMut<'_, Self> {
        self_.inner = self_.inner.clone().domain_safe(safe);
        self_
    }

    fn fixed_var(mut self_: PyRefMut<'_, Self>, var: String) -> PyRefMut<'_, Self> {
        let sym = crate::symb(&var);
        self_.inner = self_.inner.clone().fixed_var(&sym);
        self_
    }

    fn max_depth(mut self_: PyRefMut<'_, Self>, depth: usize) -> PyRefMut<'_, Self> {
        self_.inner = self_.inner.clone().max_depth(depth);
        self_
    }

    fn max_nodes(mut self_: PyRefMut<'_, Self>, nodes: usize) -> PyRefMut<'_, Self> {
        self_.inner = self_.inner.clone().max_nodes(nodes);
        self_
    }

    fn custom_derivative(
        mut self_: PyRefMut<'_, Self>,
        name: String,
        callback: Py<PyAny>,
    ) -> PyResult<PyRefMut<'_, Self>> {
        // We need to wrap the Python callback into a Rust CustomDerivativeFn
        // This closure needs to be Send + Sync, so we wrap PyObject appropriately
        // Actually, Python objects are not Send, so we need to be careful.
        // But for single-threaded usage (GIL), we can use Python::with_gil inside the closure?
        // No, `dyn Fn` must be Send+Sync to be stored in `Diff` (because Diff is used in potential threads?).
        // `Diff` implementation uses `Arc<dyn Fn ... + Send + Sync>`.
        // To make Python callback Send+Sync, we can use a wrapper that acquires GIL.

        let callback_fn = move |inner: &RustExpr, var: &str, inner_prime: &RustExpr| -> RustExpr {
            Python::attach(|py| {
                // Convert Rust types to Python objects using into_pyobject
                // Handle conversion errors gracefully instead of panicking
                let py_inner = match PyExpr(inner.clone()).into_pyobject(py) {
                    Ok(obj) => obj,
                    Err(_) => return RustExpr::number(0.0),
                };
                let py_var = match var.into_pyobject(py) {
                    Ok(obj) => obj,
                    Err(_) => return RustExpr::number(0.0),
                };
                let py_inner_prime = match PyExpr(inner_prime.clone()).into_pyobject(py) {
                    Ok(obj) => obj,
                    Err(_) => return RustExpr::number(0.0),
                };

                let result = callback.call1(py, (py_inner, py_var, py_inner_prime));

                match result {
                    Ok(res) => {
                        if let Ok(py_expr) = res.extract::<PyExpr>(py) {
                            py_expr.0
                        } else {
                            // Fallback
                            RustExpr::number(0.0)
                        }
                    }
                    Err(_) => RustExpr::number(0.0), // Ignore python errors inside derivation
                }
            })
        };

        self_.inner = self_.inner.clone().custom_derivative(name, callback_fn);
        Ok(self_)
    }

    fn diff_str(&self, formula: &str, var: &str) -> PyResult<String> {
        self.inner
            .diff_str(formula, var)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
    }

    fn differentiate(&self, expr: &PyExpr, var: &str) -> PyResult<PyExpr> {
        let sym = crate::symb(var);
        self.inner
            .differentiate(expr.0.clone(), &sym)
            .map(PyExpr)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
    }
}

/// Builder for simplification operations
#[pyclass(name = "Simplify")]
struct PySimplify {
    inner: builder::Simplify,
}

#[pymethods]
impl PySimplify {
    #[new]
    fn new() -> Self {
        PySimplify {
            inner: builder::Simplify::new(),
        }
    }

    fn domain_safe(mut self_: PyRefMut<'_, Self>, safe: bool) -> PyRefMut<'_, Self> {
        self_.inner = self_.inner.clone().domain_safe(safe);
        self_
    }

    fn fixed_var(mut self_: PyRefMut<'_, Self>, var: String) -> PyRefMut<'_, Self> {
        let sym = crate::symb(&var);
        self_.inner = self_.inner.clone().fixed_var(&sym);
        self_
    }

    fn max_depth(mut self_: PyRefMut<'_, Self>, depth: usize) -> PyRefMut<'_, Self> {
        self_.inner = self_.inner.clone().max_depth(depth);
        self_
    }

    fn max_nodes(mut self_: PyRefMut<'_, Self>, nodes: usize) -> PyRefMut<'_, Self> {
        self_.inner = self_.inner.clone().max_nodes(nodes);
        self_
    }

    fn simplify(&self, expr: &PyExpr) -> PyResult<PyExpr> {
        self.inner
            .simplify(expr.0.clone())
            .map(PyExpr)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
    }

    fn simplify_str(&self, formula: &str) -> PyResult<String> {
        self.inner
            .simplify_str(formula)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
    }
}

/// Differentiate a mathematical expression symbolically.
#[pyfunction]
#[pyo3(signature = (formula, var, fixed_vars=None, custom_functions=None))]
fn diff(
    formula: &str,
    var: &str,
    fixed_vars: Option<Vec<String>>,
    custom_functions: Option<Vec<String>>,
) -> PyResult<String> {
    let fixed_strs: Option<Vec<&str>> = fixed_vars
        .as_ref()
        .map(|v| v.iter().map(|s| s.as_str()).collect());
    let custom_strs: Option<Vec<&str>> = custom_functions
        .as_ref()
        .map(|v| v.iter().map(|s| s.as_str()).collect());

    crate::diff(formula, var, fixed_strs.as_deref(), custom_strs.as_deref())
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
}

/// Simplify a mathematical expression.
#[pyfunction]
#[pyo3(signature = (formula, fixed_vars=None, custom_functions=None))]
fn simplify(
    formula: &str,
    fixed_vars: Option<Vec<String>>,
    custom_functions: Option<Vec<String>>,
) -> PyResult<String> {
    let fixed_strs: Option<Vec<&str>> = fixed_vars
        .as_ref()
        .map(|v| v.iter().map(|s| s.as_str()).collect());
    let custom_strs: Option<Vec<&str>> = custom_functions
        .as_ref()
        .map(|v| v.iter().map(|s| s.as_str()).collect());

    crate::simplify(formula, fixed_strs.as_deref(), custom_strs.as_deref())
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
}

/// Parse a mathematical expression and return its string representation.
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

/// Compute the gradient of a scalar expression.
///
/// Args:
///     formula: String formula to differentiate
///     vars: List of variable names to differentiate with respect to
///
/// Returns:
///     List of partial derivative strings [∂f/∂x₁, ∂f/∂x₂, ...]
#[pyfunction]
fn gradient(formula: &str, vars: Vec<String>) -> PyResult<Vec<String>> {
    let var_strs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
    crate::gradient_str(formula, &var_strs)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
}

/// Compute the Hessian matrix of a scalar expression.
///
/// Args:
///     formula: String formula to differentiate twice
///     vars: List of variable names
///
/// Returns:
///     2D list of second partial derivatives [[∂²f/∂x₁², ∂²f/∂x₁∂x₂, ...], ...]
#[pyfunction]
fn hessian(formula: &str, vars: Vec<String>) -> PyResult<Vec<Vec<String>>> {
    let var_strs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
    crate::hessian_str(formula, &var_strs)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
}

/// Compute the Jacobian matrix of a vector function.
///
/// Args:
///     formulas: List of string formulas (vector function)
///     vars: List of variable names
///
/// Returns:
///     2D list where J[i][j] = ∂fᵢ/∂xⱼ
#[pyfunction]
fn jacobian(formulas: Vec<String>, vars: Vec<String>) -> PyResult<Vec<Vec<String>>> {
    let formula_strs: Vec<&str> = formulas.iter().map(|s| s.as_str()).collect();
    let var_strs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
    crate::jacobian_str(&formula_strs, &var_strs)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
}

/// Evaluate a string expression with given variable values.
///
/// Args:
///     formula: Expression string
///     vars: List of (name, value) tuples
///
/// Returns:
///     Evaluated expression as string
#[pyfunction]
fn evaluate(formula: &str, vars: Vec<(String, f64)>) -> PyResult<String> {
    let var_tuples: Vec<(&str, f64)> = vars.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    crate::evaluate_str(formula, &var_tuples)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
}

/// Compute uncertainty propagation for an expression.
///
/// Args:
///     formula: Expression string or Expr object
///     variables: List of variable names to propagate uncertainty for
///     variances: Optional list of variance values (σ²) for each variable.
///                If None, uses symbolic variances σ_x², σ_y², etc.
///
/// Returns:
///     String representation of the uncertainty expression σ_f
///
/// Example:
///     >>> uncertainty_propagation("x + y", ["x", "y"])
///     "sqrt(sigma_x^2 + sigma_y^2)"
///     >>> uncertainty_propagation("x * y", ["x", "y"], [0.01, 0.04])
///     "sqrt(0.04*x^2 + 0.01*y^2)"
#[pyfunction]
#[pyo3(signature = (formula, variables, variances=None))]
fn uncertainty_propagation_py(
    formula: &str,
    variables: Vec<String>,
    variances: Option<Vec<f64>>,
) -> PyResult<String> {
    let expr = crate::parser::parse(formula, &HashSet::new(), &HashSet::new())
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))?;

    let var_strs: Vec<&str> = variables.iter().map(|s| s.as_str()).collect();

    let cov = variances
        .map(|vars| CovarianceMatrix::diagonal(vars.into_iter().map(CovEntry::Num).collect()));

    crate::uncertainty_propagation(&expr, &var_strs, cov.as_ref())
        .map(|e| e.to_string())
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
}

/// Compute relative uncertainty for an expression.
///
/// Args:
///     formula: Expression string
///     variables: List of variable names
///     variances: Optional list of variance values (σ²) for each variable
///
/// Returns:
///     String representation of σ_f / |f|
#[pyfunction]
#[pyo3(signature = (formula, variables, variances=None))]
fn relative_uncertainty_py(
    formula: &str,
    variables: Vec<String>,
    variances: Option<Vec<f64>>,
) -> PyResult<String> {
    let expr = crate::parser::parse(formula, &HashSet::new(), &HashSet::new())
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))?;

    let var_strs: Vec<&str> = variables.iter().map(|s| s.as_str()).collect();

    let cov = variances
        .map(|vars| CovarianceMatrix::diagonal(vars.into_iter().map(CovEntry::Num).collect()));

    crate::relative_uncertainty(&expr, &var_strs, cov.as_ref())
        .map(|e| e.to_string())
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
}

/// Parallel evaluation of multiple expressions at multiple points.
///
/// Args:
///     expressions: List of expression strings
///     variables: List of variable name lists, one per expression
///     values: 3D list of values: [expr_idx][var_idx][point_idx]
///             Use None for a value to keep it symbolic (SKIP)
///
/// Returns:
///     2D list of result strings: [expr_idx][point_idx]
///
/// Example:
///     >>> evaluate_parallel(
///     ...     ["x^2", "x + y"],
///     ...     [["x"], ["x", "y"]],
///     ...     [[[1.0, 2.0, 3.0]], [[1.0, 2.0], [3.0, 4.0]]]
///     ... )
///     [["1", "4", "9"], ["4", "6"]]
#[cfg(feature = "parallel")]
#[pyfunction]
fn evaluate_parallel_py(
    expressions: Vec<String>,
    variables: Vec<Vec<String>>,
    values: Vec<Vec<Vec<Option<f64>>>>,
) -> PyResult<Vec<Vec<String>>> {
    let exprs: Vec<ExprInput> = expressions.into_iter().map(ExprInput::from).collect();

    let vars: Vec<Vec<VarInput>> = variables
        .into_iter()
        .map(|vs| vs.into_iter().map(VarInput::from).collect())
        .collect();

    let vals: Vec<Vec<Vec<Value>>> = values
        .into_iter()
        .map(|expr_vals| {
            expr_vals
                .into_iter()
                .map(|var_vals| {
                    var_vals
                        .into_iter()
                        .map(|v| match v {
                            Some(n) => Value::Num(n),
                            None => Value::Skip,
                        })
                        .collect()
                })
                .collect()
        })
        .collect();

    parallel::evaluate_parallel(exprs, vars, vals)
        .map(|results| {
            results
                .into_iter()
                .map(|expr_results| expr_results.into_iter().map(|r| r.to_string()).collect())
                .collect()
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
}

#[pymodule]
fn symb_anafis(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyExpr>()?;
    m.add_class::<PyDiff>()?;
    m.add_class::<PySimplify>()?;
    m.add_function(wrap_pyfunction!(diff, m)?)?;
    m.add_function(wrap_pyfunction!(simplify, m)?)?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(gradient, m)?)?;
    m.add_function(wrap_pyfunction!(hessian, m)?)?;
    m.add_function(wrap_pyfunction!(jacobian, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate, m)?)?;
    m.add_function(wrap_pyfunction!(uncertainty_propagation_py, m)?)?;
    m.add_function(wrap_pyfunction!(relative_uncertainty_py, m)?)?;
    #[cfg(feature = "parallel")]
    m.add_function(wrap_pyfunction!(evaluate_parallel_py, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
