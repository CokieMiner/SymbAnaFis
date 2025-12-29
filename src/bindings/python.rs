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
//! - `Context` - Isolated symbol and function registry
//!
//! # Available Functions
//! - `diff(formula, var, known_symbols?, custom_functions?)` - Differentiate string formula
//! - `simplify(formula, known_symbols?, custom_functions?)` - Simplify string formula
//! - `parse(formula, known_symbols?, custom_functions?)` - Parse formula to string

#[cfg(feature = "parallel")]
use crate::bindings::eval_f64::eval_f64 as rust_eval_f64;
use crate::core::evaluator::CompiledEvaluator;
use crate::core::symbol::Symbol as RustSymbol;
use crate::core::unified_context::Context as RustContext;
#[cfg(feature = "parallel")]
use crate::parallel::{self, ExprInput, Value, VarInput};
use crate::uncertainty::{CovEntry, CovarianceMatrix};
use crate::{
    Dual, Expr as RustExpr, api::builder, clear_symbols, remove_symbol, symb, symb_get, symb_new,
    symbol_count, symbol_exists, symbol_names,
};
use num_traits::Float;
use pyo3::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;

/// Type alias for complex partial derivative function type to improve readability
type PartialDerivativeFn = Arc<dyn Fn(&[Arc<RustExpr>]) -> RustExpr + Send + Sync>;

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
            return Ok(PyExpr(self.0.clone().pow(expr.0)));
        }
        // Try as float
        if let Ok(n) = other.extract::<f64>() {
            return Ok(PyExpr(self.0.clone().pow(n)));
        }
        // Try as int
        if let Ok(n) = other.extract::<i64>() {
            return Ok(PyExpr(self.0.clone().pow(n as f64)));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "pow() argument must be Expr, int, or float",
        ))
    }

    // Reverse power: 2 ** x where x is Expr
    fn __rpow__(&self, other: &Bound<'_, PyAny>, _modulo: Option<Py<PyAny>>) -> PyResult<PyExpr> {
        // other ** self (other is the base, self is the exponent)
        if let Ok(n) = other.extract::<f64>() {
            return Ok(PyExpr(RustExpr::number(n).pow(self.0.clone())));
        }
        if let Ok(n) = other.extract::<i64>() {
            return Ok(PyExpr(RustExpr::number(n as f64).pow(self.0.clone())));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "rpow() base must be int or float",
        ))
    }

    fn __neg__(&self) -> PyExpr {
        PyExpr(RustExpr::number(0.0) - self.0.clone())
    }

    fn __radd__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        if let Ok(n) = other.extract::<f64>() {
            return Ok(PyExpr(RustExpr::number(n) + self.0.clone()));
        }
        if let Ok(n) = other.extract::<i64>() {
            return Ok(PyExpr(RustExpr::number(n as f64) + self.0.clone()));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be int or float",
        ))
    }

    fn __rsub__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        if let Ok(n) = other.extract::<f64>() {
            return Ok(PyExpr(RustExpr::number(n) - self.0.clone()));
        }
        if let Ok(n) = other.extract::<i64>() {
            return Ok(PyExpr(RustExpr::number(n as f64) - self.0.clone()));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be int or float",
        ))
    }

    fn __rmul__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        if let Ok(n) = other.extract::<f64>() {
            return Ok(PyExpr(RustExpr::number(n) * self.0.clone()));
        }
        if let Ok(n) = other.extract::<i64>() {
            return Ok(PyExpr(RustExpr::number(n as f64) * self.0.clone()));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be int or float",
        ))
    }

    fn __rtruediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        if let Ok(n) = other.extract::<f64>() {
            return Ok(PyExpr(RustExpr::number(n) / self.0.clone()));
        }
        if let Ok(n) = other.extract::<i64>() {
            return Ok(PyExpr(RustExpr::number(n as f64) / self.0.clone()));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be int or float",
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
    /// Logarithm with the specified base: log(self, base) → log(base, self)
    /// For natural logarithm, use ln() instead
    fn log(&self, base: f64) -> PyExpr {
        PyExpr(self.0.clone().log(base))
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
    fn signum(&self) -> PyExpr {
        PyExpr(self.0.clone().signum())
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
    fn tetragamma(&self) -> PyExpr {
        PyExpr(self.0.clone().tetragamma())
    }
    fn floor(&self) -> PyExpr {
        PyExpr(self.0.clone().floor())
    }
    fn ceil(&self) -> PyExpr {
        PyExpr(self.0.clone().ceil())
    }
    fn round(&self) -> PyExpr {
        PyExpr(self.0.clone().round())
    }
    fn elliptic_k(&self) -> PyExpr {
        PyExpr(self.0.clone().elliptic_k())
    }
    fn elliptic_e(&self) -> PyExpr {
        PyExpr(self.0.clone().elliptic_e())
    }
    fn exp_polar(&self) -> PyExpr {
        PyExpr(self.0.clone().exp_polar())
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
        PyExpr(self.0.clone().pow(exp))
    }

    // Multi-argument functions
    /// Two-argument arctangent: atan2(y, x) = angle to point (x, y)
    fn atan2(&self, x: &PyExpr) -> PyExpr {
        PyExpr(RustExpr::func_multi(
            "atan2",
            vec![self.0.clone(), x.0.clone()],
        ))
    }

    /// Hermite polynomial H_n(self)
    fn hermite(&self, n: i32) -> PyExpr {
        PyExpr(RustExpr::func_multi(
            "hermite",
            vec![RustExpr::number(n as f64), self.0.clone()],
        ))
    }

    /// Associated Legendre polynomial P_l^m(self)
    fn assoc_legendre(&self, l: i32, m: i32) -> PyExpr {
        PyExpr(RustExpr::func_multi(
            "assoc_legendre",
            vec![
                RustExpr::number(l as f64),
                RustExpr::number(m as f64),
                self.0.clone(),
            ],
        ))
    }

    /// Spherical harmonic Y_l^m(theta, phi) where self is theta
    fn spherical_harmonic(&self, l: i32, m: i32, phi: &PyExpr) -> PyExpr {
        PyExpr(RustExpr::func_multi(
            "spherical_harmonic",
            vec![
                RustExpr::number(l as f64),
                RustExpr::number(m as f64),
                self.0.clone(),
                phi.0.clone(),
            ],
        ))
    }

    /// Alternative spherical harmonic notation Y_l^m(theta, phi)
    fn ynm(&self, l: i32, m: i32, phi: &PyExpr) -> PyExpr {
        PyExpr(RustExpr::func_multi(
            "ynm",
            vec![
                RustExpr::number(l as f64),
                RustExpr::number(m as f64),
                self.0.clone(),
                phi.0.clone(),
            ],
        ))
    }

    /// Derivative of Riemann zeta function: zeta^(n)(self)
    fn zeta_deriv(&self, n: i32) -> PyExpr {
        PyExpr(RustExpr::func_multi(
            "zeta_deriv",
            vec![RustExpr::number(n as f64), self.0.clone()],
        ))
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

    /// Check if expression is a raw symbol
    fn is_symbol(&self) -> bool {
        matches!(self.0.kind, crate::ExprKind::Symbol(_))
    }

    /// Check if expression is a constant number
    fn is_number(&self) -> bool {
        matches!(self.0.kind, crate::ExprKind::Number(_))
    }

    /// Check if expression is effectively zero
    fn is_zero(&self) -> bool {
        self.0.is_zero_num()
    }

    /// Check if expression is effectively one
    fn is_one(&self) -> bool {
        self.0.is_one_num()
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
        PyExpr(self.0.evaluate(&var_map, &std::collections::HashMap::new()))
    }

    /// Differentiate this expression
    fn diff(&self, var: &str) -> PyResult<PyExpr> {
        let sym = crate::symb(var);
        crate::Diff::new()
            .differentiate(self.0.clone(), &sym)
            .map(PyExpr)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
    }

    /// Simplify this expression
    fn simplify(&self) -> PyResult<PyExpr> {
        crate::Simplify::new()
            .simplify(self.0.clone())
            .map(PyExpr)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
    }
}

/// Wrapper for Rust Dual number for automatic differentiation
#[pyclass(name = "Dual")]
#[derive(Clone, Copy)]
struct PyDual(Dual<f64>);

#[pymethods]
impl PyDual {
    /// Create a new dual number
    ///
    /// Args:
    ///     val: The real value component
    ///     eps: The infinitesimal derivative component
    ///
    /// Returns:
    ///     A new Dual number representing val + eps*ε
    #[new]
    fn new(val: f64, eps: f64) -> Self {
        PyDual(Dual::new(val, eps))
    }

    /// Create a constant dual number (derivative = 0)
    ///
    /// Args:
    ///     val: The constant value
    ///
    /// Returns:
    ///     A Dual number representing val + 0*ε
    #[staticmethod]
    fn constant(val: f64) -> Self {
        PyDual(Dual::constant(val))
    }

    /// Get the real value component
    #[getter]
    fn val(&self) -> f64 {
        self.0.val
    }

    /// Get the infinitesimal derivative component
    #[getter]
    fn eps(&self) -> f64 {
        self.0.eps
    }

    fn __str__(&self) -> String {
        // Show the actual values honestly - floating point artifacts and all
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("Dual({}, {})", self.0.val, self.0.eps)
    }

    // Arithmetic operators
    fn __add__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyDual> {
        if let Ok(dual) = other.extract::<PyDual>() {
            return Ok(PyDual(self.0 + dual.0));
        }
        if let Ok(n) = other.extract::<f64>() {
            return Ok(PyDual(self.0 + Dual::constant(n)));
        }
        if let Ok(n) = other.extract::<i32>() {
            return Ok(PyDual(self.0 + Dual::constant(n as f64)));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be Dual, int, or float",
        ))
    }

    fn __sub__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyDual> {
        if let Ok(dual) = other.extract::<PyDual>() {
            return Ok(PyDual(self.0 - dual.0));
        }
        if let Ok(n) = other.extract::<f64>() {
            return Ok(PyDual(self.0 - Dual::constant(n)));
        }
        if let Ok(n) = other.extract::<i32>() {
            return Ok(PyDual(self.0 - Dual::constant(n as f64)));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be Dual, int, or float",
        ))
    }

    fn __mul__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyDual> {
        if let Ok(dual) = other.extract::<PyDual>() {
            return Ok(PyDual(self.0 * dual.0));
        }
        if let Ok(n) = other.extract::<f64>() {
            return Ok(PyDual(self.0 * Dual::constant(n)));
        }
        if let Ok(n) = other.extract::<i32>() {
            return Ok(PyDual(self.0 * Dual::constant(n as f64)));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be Dual, int, or float",
        ))
    }

    fn __truediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyDual> {
        if let Ok(dual) = other.extract::<PyDual>() {
            return Ok(PyDual(self.0 / dual.0));
        }
        if let Ok(n) = other.extract::<f64>() {
            return Ok(PyDual(self.0 / Dual::constant(n)));
        }
        if let Ok(n) = other.extract::<i32>() {
            return Ok(PyDual(self.0 / Dual::constant(n as f64)));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be Dual, int, or float",
        ))
    }

    fn __neg__(&self) -> PyDual {
        PyDual(-self.0)
    }

    // Reverse operations for mixed types
    fn __radd__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyDual> {
        if let Ok(n) = other.extract::<f64>() {
            return Ok(PyDual(Dual::constant(n) + self.0));
        }
        if let Ok(n) = other.extract::<i32>() {
            return Ok(PyDual(Dual::constant(n as f64) + self.0));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be int or float",
        ))
    }

    fn __rsub__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyDual> {
        if let Ok(n) = other.extract::<f64>() {
            return Ok(PyDual(Dual::constant(n) - self.0));
        }
        if let Ok(n) = other.extract::<i32>() {
            return Ok(PyDual(Dual::constant(n as f64) - self.0));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be int or float",
        ))
    }

    fn __rmul__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyDual> {
        if let Ok(n) = other.extract::<f64>() {
            return Ok(PyDual(Dual::constant(n) * self.0));
        }
        if let Ok(n) = other.extract::<i32>() {
            return Ok(PyDual(Dual::constant(n as f64) * self.0));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be int or float",
        ))
    }

    fn __rtruediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyDual> {
        if let Ok(n) = other.extract::<f64>() {
            return Ok(PyDual(Dual::constant(n) / self.0));
        }
        if let Ok(n) = other.extract::<i32>() {
            return Ok(PyDual(Dual::constant(n as f64) / self.0));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be int or float",
        ))
    }

    fn __pow__(&self, other: &Bound<'_, PyAny>, _modulo: Option<Py<PyAny>>) -> PyResult<PyDual> {
        if let Ok(dual) = other.extract::<PyDual>() {
            return Ok(PyDual(self.0.powf(dual.0)));
        }
        if let Ok(n) = other.extract::<f64>() {
            return Ok(PyDual(self.0.powf(Dual::constant(n))));
        }
        if let Ok(n) = other.extract::<i32>() {
            return Ok(PyDual(self.0.powi(n)));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Exponent must be Dual, int, or float",
        ))
    }

    fn __rpow__(&self, other: &Bound<'_, PyAny>, _modulo: Option<Py<PyAny>>) -> PyResult<PyDual> {
        // other ** self (other is the base, self is the exponent)
        if let Ok(n) = other.extract::<f64>() {
            return Ok(PyDual(Dual::constant(n).powf(self.0)));
        }
        if let Ok(n) = other.extract::<i32>() {
            return Ok(PyDual(Dual::constant(n as f64).powf(self.0)));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Base must be int or float",
        ))
    }

    // Mathematical functions
    fn sin(&self) -> PyDual {
        PyDual(self.0.sin())
    }

    fn cos(&self) -> PyDual {
        PyDual(self.0.cos())
    }

    fn tan(&self) -> PyDual {
        PyDual(self.0.tan())
    }

    fn asin(&self) -> PyDual {
        PyDual(self.0.asin())
    }

    fn acos(&self) -> PyDual {
        PyDual(self.0.acos())
    }

    fn atan(&self) -> PyDual {
        PyDual(self.0.atan())
    }

    fn sinh(&self) -> PyDual {
        PyDual(self.0.sinh())
    }

    fn cosh(&self) -> PyDual {
        PyDual(self.0.cosh())
    }

    fn tanh(&self) -> PyDual {
        PyDual(self.0.tanh())
    }

    fn exp(&self) -> PyDual {
        PyDual(self.0.exp())
    }

    fn exp2(&self) -> PyDual {
        PyDual(self.0.exp2())
    }

    fn ln(&self) -> PyDual {
        PyDual(self.0.ln())
    }

    fn log2(&self) -> PyDual {
        PyDual(self.0.log2())
    }

    fn log10(&self) -> PyDual {
        PyDual(self.0.log10())
    }

    fn sqrt(&self) -> PyDual {
        PyDual(self.0.sqrt())
    }

    fn cbrt(&self) -> PyDual {
        PyDual(self.0.cbrt())
    }

    fn abs(&self) -> PyDual {
        PyDual(self.0.abs())
    }

    fn powf(&self, other: &PyDual) -> PyDual {
        PyDual(self.0.powf(other.0))
    }

    fn powi(&self, n: i32) -> PyDual {
        PyDual(self.0.powi(n))
    }

    // Inverse hyperbolic functions
    fn asinh(&self) -> PyDual {
        PyDual(self.0.asinh())
    }

    fn acosh(&self) -> PyDual {
        PyDual(self.0.acosh())
    }

    fn atanh(&self) -> PyDual {
        PyDual(self.0.atanh())
    }

    // Special functions
    /// Error function
    fn erf(&self) -> PyDual {
        PyDual(self.0.erf())
    }

    /// Complementary error function
    fn erfc(&self) -> PyDual {
        PyDual(self.0.erfc())
    }

    /// Gamma function
    fn gamma(&self) -> PyResult<PyDual> {
        self.0.gamma().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Gamma function undefined for this value",
            )
        })
    }

    /// Digamma function (logarithmic derivative of gamma)
    fn digamma(&self) -> PyResult<PyDual> {
        self.0.digamma().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Digamma function undefined for this value",
            )
        })
    }

    /// Trigamma function
    fn trigamma(&self) -> PyResult<PyDual> {
        self.0.trigamma().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Trigamma function undefined for this value",
            )
        })
    }

    /// Polygamma function
    fn polygamma(&self, n: i32) -> PyResult<PyDual> {
        self.0.polygamma(n).map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Polygamma function undefined for this value",
            )
        })
    }

    /// Riemann zeta function
    fn zeta(&self) -> PyResult<PyDual> {
        self.0.zeta().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Zeta function undefined for this value",
            )
        })
    }

    /// Lambert W function
    fn lambert_w(&self) -> PyResult<PyDual> {
        self.0.lambert_w().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Lambert W function undefined for this value",
            )
        })
    }

    /// Bessel function of the first kind
    fn bessel_j(&self, n: i32) -> PyResult<PyDual> {
        self.0.bessel_j(n).map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Bessel J function undefined for this value",
            )
        })
    }

    /// Sinc function: sin(x)/x
    fn sinc(&self) -> PyDual {
        PyDual(self.0.sinc())
    }

    /// Sign function
    fn sign(&self) -> PyDual {
        PyDual(self.0.sign())
    }

    /// Elliptic integral of the first kind
    fn elliptic_k(&self) -> PyResult<PyDual> {
        self.0.elliptic_k().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Elliptic K function undefined for this value",
            )
        })
    }

    /// Elliptic integral of the second kind
    fn elliptic_e(&self) -> PyResult<PyDual> {
        self.0.elliptic_e().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Elliptic E function undefined for this value",
            )
        })
    }

    /// Beta function: B(a, b) = Γ(a)Γ(b)/Γ(a+b)
    fn beta(&self, b: &PyDual) -> PyResult<PyDual> {
        self.0.beta(b.0).map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Beta function undefined for these values",
            )
        })
    }

    // Additional Float trait methods
    fn signum(&self) -> PyDual {
        PyDual(self.0.signum())
    }

    fn floor(&self) -> PyDual {
        PyDual(self.0.floor())
    }

    fn ceil(&self) -> PyDual {
        PyDual(self.0.ceil())
    }

    fn round(&self) -> PyDual {
        PyDual(self.0.round())
    }

    fn trunc(&self) -> PyDual {
        PyDual(self.0.trunc())
    }

    fn fract(&self) -> PyDual {
        PyDual(self.0.fract())
    }

    fn recip(&self) -> PyDual {
        PyDual(self.0.recip())
    }

    fn exp_m1(&self) -> PyDual {
        PyDual(self.0.exp_m1())
    }

    fn ln_1p(&self) -> PyDual {
        PyDual(self.0.ln_1p())
    }

    fn hypot(&self, other: &PyDual) -> PyDual {
        PyDual(self.0.hypot(other.0))
    }

    fn atan2(&self, other: &PyDual) -> PyDual {
        PyDual(self.0.atan2(other.0))
    }

    fn log(&self, base: &PyDual) -> PyDual {
        PyDual(self.0.log(base.0))
    }
}

/// Builder for differentiation operations
#[pyclass(name = "Diff")]
struct PyDiff {
    inner: builder::Diff,
    known_symbols: Vec<String>,
}

#[pymethods]
impl PyDiff {
    #[new]
    fn new() -> Self {
        PyDiff {
            inner: builder::Diff::new(),
            known_symbols: Vec::new(),
        }
    }

    fn domain_safe(mut self_: PyRefMut<'_, Self>, safe: bool) -> PyRefMut<'_, Self> {
        self_.inner = self_.inner.clone().domain_safe(safe);
        self_
    }

    fn fixed_var(mut self_: PyRefMut<'_, Self>, var: String) -> PyRefMut<'_, Self> {
        self_.known_symbols.push(var);
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

    fn with_context<'a>(
        mut self_: PyRefMut<'a, Self>,
        context: &'a PyContext,
    ) -> PyRefMut<'a, Self> {
        self_.inner = self_.inner.clone().with_context(&context.inner);
        self_
    }

    /// Register a user-defined function with a partial derivative callback.
    ///
    /// The callback receives the argument expressions and should return the partial derivative
    /// of the function with respect to its first argument.
    ///
    /// Example:
    /// ```python
    /// def my_partial(args):
    ///     # For f(u), return ∂f/∂u as an Expr
    ///     # args[0] is the first argument expression
    ///     return 3 * args[0] ** 2  # e.g., ∂f/∂u = 3u²
    ///
    /// diff = Diff().user_fn("my_func", 1, my_partial)
    /// ```
    fn user_fn(
        mut self_: PyRefMut<'_, Self>,
        name: String,
        arity: usize,
        partial_callback: Py<PyAny>,
    ) -> PyResult<PyRefMut<'_, Self>> {
        use crate::core::unified_context::UserFunction;
        use std::sync::Arc;

        // Create a partial derivative function that calls the Python callback
        let partial_fn: PartialDerivativeFn = Arc::new(move |args: &[Arc<RustExpr>]| -> RustExpr {
            Python::attach(|py| {
                // Convert args to Python list of PyExpr
                let py_args: Vec<PyExpr> = args.iter().map(|a| PyExpr((**a).clone())).collect();
                let py_list = match py_args.into_pyobject(py) {
                    Ok(list) => list,
                    Err(_) => return RustExpr::number(0.0),
                };

                let result = partial_callback.call1(py, (py_list,));

                match result {
                    Ok(res) => {
                        if let Ok(py_expr) = res.extract::<PyExpr>(py) {
                            py_expr.0
                        } else {
                            RustExpr::number(0.0)
                        }
                    }
                    Err(_) => RustExpr::number(0.0),
                }
            })
        });

        let user_fn = UserFunction::new(arity..=arity)
            .partial_arc(0, partial_fn)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))?;
        self_.inner = self_.inner.clone().user_fn(name, user_fn);
        Ok(self_)
    }

    fn diff_str(&self, formula: &str, var: &str) -> PyResult<String> {
        let known_symbols: Vec<&str> = self.known_symbols.iter().map(|s| s.as_str()).collect();
        self.inner
            .diff_str(formula, var, &known_symbols)
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
    known_symbols: Vec<String>,
}

#[pymethods]
impl PySimplify {
    #[new]
    fn new() -> Self {
        PySimplify {
            inner: builder::Simplify::new(),
            known_symbols: Vec::new(),
        }
    }

    fn domain_safe(mut self_: PyRefMut<'_, Self>, safe: bool) -> PyRefMut<'_, Self> {
        self_.inner = self_.inner.clone().domain_safe(safe);
        self_
    }

    fn fixed_var(mut self_: PyRefMut<'_, Self>, var: String) -> PyRefMut<'_, Self> {
        self_.known_symbols.push(var);
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

    fn with_context<'a>(
        mut self_: PyRefMut<'a, Self>,
        context: &'a PyContext,
    ) -> PyRefMut<'a, Self> {
        self_.inner = self_.inner.clone().with_context(&context.inner);
        self_
    }

    fn simplify(&self, expr: &PyExpr) -> PyResult<PyExpr> {
        self.inner
            .simplify(expr.0.clone())
            .map(PyExpr)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
    }

    fn simplify_str(&self, formula: &str) -> PyResult<String> {
        let known_symbols: Vec<&str> = self.known_symbols.iter().map(|s| s.as_str()).collect();
        self.inner
            .simplify_str(formula, &known_symbols)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
    }
}

/// Differentiate a mathematical expression symbolically.
#[pyfunction]
#[pyo3(signature = (formula, var, known_symbols=None, custom_functions=None))]
fn diff(
    formula: &str,
    var: &str,
    known_symbols: Option<Vec<String>>,
    custom_functions: Option<Vec<String>>,
) -> PyResult<String> {
    let known_strs: Option<Vec<&str>> = known_symbols
        .as_ref()
        .map(|v| v.iter().map(|s| s.as_str()).collect());
    let custom_strs: Option<Vec<&str>> = custom_functions
        .as_ref()
        .map(|v| v.iter().map(|s| s.as_str()).collect());

    crate::diff(
        formula,
        var,
        known_strs.as_deref().unwrap_or(&[]),
        custom_strs.as_deref(),
    )
    .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
}

/// Simplify a mathematical expression.
#[pyfunction]
#[pyo3(signature = (formula, known_symbols=None, custom_functions=None))]
fn simplify(
    formula: &str,
    known_symbols: Option<Vec<String>>,
    custom_functions: Option<Vec<String>>,
) -> PyResult<String> {
    let known_strs: Option<Vec<&str>> = known_symbols
        .as_ref()
        .map(|v| v.iter().map(|s| s.as_str()).collect());
    let custom_strs: Option<Vec<&str>> = custom_functions
        .as_ref()
        .map(|v| v.iter().map(|s| s.as_str()).collect());

    crate::simplify(
        formula,
        known_strs.as_deref().unwrap_or(&[]),
        custom_strs.as_deref(),
    )
    .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
}

/// Parse a mathematical expression and return its string representation.
#[pyfunction]
#[pyo3(signature = (formula, known_symbols=None, custom_functions=None))]
fn parse(
    formula: &str,
    known_symbols: Option<Vec<String>>,
    custom_functions: Option<Vec<String>>,
) -> PyResult<String> {
    let known: HashSet<String> = known_symbols
        .map(|v| v.into_iter().collect())
        .unwrap_or_default();
    let custom: HashSet<String> = custom_functions
        .map(|v| v.into_iter().collect())
        .unwrap_or_default();

    crate::parser::parse(formula, &known, &custom, None)
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
#[pyo3(name = "uncertainty_propagation", signature = (formula, variables, variances=None))]
fn uncertainty_propagation_py(
    formula: &str,
    variables: Vec<String>,
    variances: Option<Vec<f64>>,
) -> PyResult<String> {
    let expr = crate::parser::parse(formula, &HashSet::new(), &HashSet::new(), None)
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
#[pyo3(name = "relative_uncertainty", signature = (formula, variables, variances=None))]
fn relative_uncertainty_py(
    formula: &str,
    variables: Vec<String>,
    variances: Option<Vec<f64>>,
) -> PyResult<String> {
    let expr = crate::parser::parse(formula, &HashSet::new(), &HashSet::new(), None)
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
#[pyo3(name = "evaluate_parallel")]
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

// ============================================================================
// Symbol Management
// ============================================================================

/// Wrapper for Rust Symbol
#[pyclass(unsendable, name = "Symbol")]
#[derive(Clone)]
struct PySymbol(RustSymbol);

#[pymethods]
impl PySymbol {
    #[new]
    fn new(name: &str) -> Self {
        PySymbol(symb(name))
    }

    fn __str__(&self) -> String {
        self.0.name().unwrap_or_default()
    }

    fn __repr__(&self) -> String {
        format!("Symbol(\"{}\")", self.0.name().unwrap_or_default())
    }

    /// Get the symbol name
    fn name(&self) -> Option<String> {
        self.0.name()
    }

    /// Get the symbol's unique ID
    fn id(&self) -> u64 {
        self.0.id()
    }

    /// Convert to an expression
    fn to_expr(&self) -> PyExpr {
        PyExpr(self.0.to_expr())
    }

    // Arithmetic with other symbols/exprs
    fn __add__(&self, other: &PyExpr) -> PyExpr {
        PyExpr(self.0.to_expr() + other.0.clone())
    }

    fn __mul__(&self, other: &PyExpr) -> PyExpr {
        PyExpr(self.0.to_expr() * other.0.clone())
    }

    fn __sub__(&self, other: &PyExpr) -> PyExpr {
        PyExpr(self.0.to_expr() - other.0.clone())
    }

    fn __truediv__(&self, other: &PyExpr) -> PyExpr {
        PyExpr(self.0.to_expr() / other.0.clone())
    }
}

/// Create or get a symbol by name
#[pyfunction]
#[pyo3(name = "symb")]
fn py_symb(name: &str) -> PySymbol {
    PySymbol(symb(name))
}

/// Create a new symbol (fails if already exists)
#[pyfunction]
#[pyo3(name = "symb_new")]
fn py_symb_new(name: &str) -> PyResult<PySymbol> {
    symb_new(name)
        .map(PySymbol)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
}

/// Get an existing symbol (fails if not found)
#[pyfunction]
#[pyo3(name = "symb_get")]
fn py_symb_get(name: &str) -> PyResult<PySymbol> {
    symb_get(name)
        .map(PySymbol)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
}

/// Check if a symbol exists
#[pyfunction]
#[pyo3(name = "symbol_exists")]
fn py_symbol_exists(name: &str) -> bool {
    symbol_exists(name)
}

/// Get count of symbols in global context
#[pyfunction]
#[pyo3(name = "symbol_count")]
fn py_symbol_count() -> usize {
    symbol_count()
}

/// Get all symbol names in global context
#[pyfunction]
#[pyo3(name = "symbol_names")]
fn py_symbol_names() -> Vec<String> {
    symbol_names()
}

/// Remove a symbol from global context
#[pyfunction]
#[pyo3(name = "remove_symbol")]
fn py_remove_symbol(name: &str) -> bool {
    remove_symbol(name)
}

/// Clear all symbols from global context
#[pyfunction]
#[pyo3(name = "clear_symbols")]
fn py_clear_symbols() {
    clear_symbols()
}

// ============================================================================
// CompiledEvaluator for fast numeric evaluation
// ============================================================================

/// Compiled expression evaluator for fast numeric evaluation
#[pyclass(unsendable, name = "CompiledEvaluator")]
struct PyCompiledEvaluator {
    evaluator: CompiledEvaluator,
}

#[pymethods]
impl PyCompiledEvaluator {
    /// Compile an expression with specified parameter order
    #[new]
    #[pyo3(signature = (expr, params=None))]
    fn new(expr: &PyExpr, params: Option<Vec<String>>) -> PyResult<Self> {
        let param_refs: Vec<&str>;
        let evaluator = if let Some(p) = &params {
            param_refs = p.iter().map(|s| s.as_str()).collect();
            CompiledEvaluator::compile(&expr.0, &param_refs)
        } else {
            CompiledEvaluator::compile_auto(&expr.0)
        };

        evaluator
            .map(|e| PyCompiledEvaluator { evaluator: e })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
    }

    /// Evaluate at a single point
    fn evaluate(&self, params: Vec<f64>) -> f64 {
        self.evaluator.evaluate(&params)
    }

    /// Batch evaluate at multiple points (columnar data)
    /// columns[var_idx][point_idx] -> f64
    fn eval_batch(&self, columns: Vec<Vec<f64>>) -> PyResult<Vec<f64>> {
        let n_points = if columns.is_empty() {
            1
        } else {
            columns[0].len()
        };
        let col_refs: Vec<&[f64]> = columns.iter().map(|c| c.as_slice()).collect();
        let mut output = vec![0.0; n_points];
        self.evaluator
            .eval_batch(&col_refs, &mut output)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))?;
        Ok(output)
    }

    /// Get parameter names in order
    fn param_names(&self) -> Vec<String> {
        self.evaluator.param_names().to_vec()
    }

    /// Get number of parameters
    fn param_count(&self) -> usize {
        self.evaluator.param_count()
    }

    /// Get number of bytecode instructions
    fn instruction_count(&self) -> usize {
        self.evaluator.instruction_count()
    }

    /// Get required stack size
    fn stack_size(&self) -> usize {
        self.evaluator.stack_size()
    }
}

// ============================================================================
// FunctionContext for custom function registration
// ============================================================================

/// Custom function context for tracking custom function names.
///
/// This tracks custom function names for parsing. For full custom function
/// support with evaluation and derivatives, use the Diff.custom_derivative API.
#[pyclass(unsendable, name = "FunctionContext")]
struct PyFunctionContext {
    /// Set of registered custom function names
    functions: std::collections::HashSet<String>,
}

#[pymethods]
impl PyFunctionContext {
    #[new]
    fn new() -> Self {
        PyFunctionContext {
            functions: std::collections::HashSet::new(),
        }
    }

    /// Register a custom function name (for parsing support).
    ///
    /// For actual evaluation/differentiation, use Diff.custom_derivative().
    ///
    /// Args:
    ///     name: Function name to register
    fn register(&mut self, name: String) {
        self.functions.insert(name);
    }

    /// Check if a function is registered
    fn contains(&self, name: &str) -> bool {
        self.functions.contains(name)
    }

    /// Get all registered function names
    fn names(&self) -> Vec<String> {
        self.functions.iter().cloned().collect()
    }

    /// Get the number of registered functions
    fn len(&self) -> usize {
        self.functions.len()
    }

    /// Check if context is empty
    fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }

    /// Clear all functions
    fn clear(&mut self) {
        self.functions.clear()
    }
}

// ============================================================================
/// Unified Context for namespace isolation and function registry.
///
/// Use this to create isolated environments for symbols and functions.
///
/// Example:
///     ctx = Context()
///     x = ctx.symb("x")  # Creates 'x' in this context only
#[pyclass(unsendable, name = "Context")]
struct PyContext {
    inner: RustContext,
}

#[pymethods]
impl PyContext {
    /// Create a new empty context
    #[new]
    fn new() -> Self {
        PyContext {
            inner: RustContext::new(),
        }
    }

    /// Create or get a symbol in this context
    fn symb(&self, name: &str) -> PySymbol {
        PySymbol(self.inner.symb(name))
    }

    /// Create a new symbol (fails if name already exists in this context)
    fn symb_new(&self, name: &str) -> PyResult<PySymbol> {
        if self.inner.contains_symbol(name) {
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Symbol '{}' already exists in this context",
                name
            )))
        } else {
            Ok(PySymbol(self.inner.symb(name)))
        }
    }

    /// Get an existing symbol by name (returns None if not found)
    fn get_symbol(&self, name: &str) -> Option<PySymbol> {
        self.inner.get_symbol(name).map(PySymbol)
    }

    /// Check if a symbol exists in this context
    fn contains_symbol(&self, name: &str) -> bool {
        self.inner.contains_symbol(name)
    }

    /// Get the number of symbols in this context
    fn symbol_count(&self) -> usize {
        self.inner.symbol_names().len()
    }

    /// Check if context is empty
    fn is_empty(&self) -> bool {
        self.inner.symbol_names().is_empty()
    }

    /// Get all symbol names in this context
    fn symbol_names(&self) -> Vec<String> {
        self.inner.symbol_names()
    }

    /// Get the context's unique ID
    fn id(&self) -> u64 {
        self.inner.id()
    }

    /// Remove a symbol from the context
    fn remove_symbol(&mut self, name: &str) -> bool {
        self.inner.remove_symbol(name)
    }

    /// Clear all symbols and functions from the context
    fn clear_all(&mut self) {
        self.inner.clear_all()
    }

    /// Register a user function with a partial derivative callback
    fn with_function(
        mut self_: PyRefMut<'_, Self>,
        name: String,
        arity: usize,
        callback: Py<PyAny>,
    ) -> PyResult<PyRefMut<'_, Self>> {
        use crate::core::unified_context::UserFunction;
        use std::sync::Arc;

        let partial_fn: PartialDerivativeFn = Arc::new(move |args: &[Arc<RustExpr>]| -> RustExpr {
            Python::attach(|py| {
                let py_args: Vec<PyExpr> = args.iter().map(|a| PyExpr((**a).clone())).collect();
                let py_list = match py_args.into_pyobject(py) {
                    Ok(list) => list,
                    Err(_) => return RustExpr::number(0.0),
                };

                let result = callback.call1(py, (py_list,));

                match result {
                    Ok(res) => {
                        if let Ok(py_expr) = res.extract::<PyExpr>(py) {
                            py_expr.0
                        } else {
                            RustExpr::number(0.0)
                        }
                    }
                    Err(_) => RustExpr::number(0.0),
                }
            })
        });

        let user_fn = UserFunction::new(arity..=arity)
            .partial_arc(0, partial_fn)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))?;

        self_.inner = self_.inner.clone().with_function(&name, user_fn);
        Ok(self_)
    }

    fn __repr__(&self) -> String {
        format!(
            "Context(id={}, symbols={})",
            self.inner.id(),
            self.inner.symbol_names().len()
        )
    }
}

// ============================================================================
// Visitor utilities for AST traversal
// ============================================================================

use crate::visitor::{NodeCounter, VariableCollector, walk_expr};

/// Count the number of nodes in an expression tree.
///
/// Args:
///     expr: Expression to count nodes in
///
/// Returns:
///     Number of nodes (symbols, numbers, operators, functions)
#[pyfunction]
fn count_nodes(expr: &PyExpr) -> usize {
    let mut counter = NodeCounter::default();
    walk_expr(&expr.0, &mut counter);
    counter.count
}

/// Collect all unique variable names in an expression.
///
/// Args:
///     expr: Expression to collect variables from
///
/// Returns:
///     Set of variable name strings
#[pyfunction]
fn collect_variables(expr: &PyExpr) -> std::collections::HashSet<String> {
    let mut collector = VariableCollector::default();
    walk_expr(&expr.0, &mut collector);
    collector.variables
}

// ============================================================================
// High-performance parallel batch evaluation (eval_f64)
// ============================================================================

/// High-performance parallel batch evaluation for multiple expressions.
///
/// This uses SIMD and parallel processing for maximum performance.
/// Best for pure numeric workloads with many data points.
///
/// Args:
///     expressions: List of Expr objects
///     var_names: List of variable name lists, one per expression
///     data: 3D list of values: data[expr_idx][var_idx] = [values...]
///
/// Returns:
///     2D list of results: result[expr_idx][point_idx]
#[cfg(feature = "parallel")]
#[pyfunction]
#[pyo3(name = "eval_f64")]
fn eval_f64_py(
    expressions: Vec<PyExpr>,
    var_names: Vec<Vec<String>>,
    data: Vec<Vec<Vec<f64>>>,
) -> PyResult<Vec<Vec<f64>>> {
    let expr_refs: Vec<&RustExpr> = expressions.iter().map(|e| &e.0).collect();

    // Convert var_names to the required format
    let var_refs: Vec<Vec<&str>> = var_names
        .iter()
        .map(|vs| vs.iter().map(|s| s.as_str()).collect())
        .collect();
    let var_slice_refs: Vec<&[&str]> = var_refs.iter().map(|v| v.as_slice()).collect();

    // Convert data to the required format
    let data_refs: Vec<Vec<&[f64]>> = data
        .iter()
        .map(|expr_data| expr_data.iter().map(|col| col.as_slice()).collect())
        .collect();
    let data_slice_refs: Vec<&[&[f64]]> = data_refs.iter().map(|d| d.as_slice()).collect();

    rust_eval_f64(&expr_refs, &var_slice_refs, &data_slice_refs)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
}

#[pymodule]
fn symb_anafis(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Core types
    m.add_class::<PyExpr>()?;
    m.add_class::<PySymbol>()?;
    m.add_class::<PyContext>()?;
    m.add_class::<PyCompiledEvaluator>()?;
    m.add_class::<PyFunctionContext>()?;
    m.add_class::<PyDiff>()?;
    m.add_class::<PySimplify>()?;
    m.add_class::<PyDual>()?;

    // Core functions
    m.add_function(wrap_pyfunction!(diff, m)?)?;
    m.add_function(wrap_pyfunction!(simplify, m)?)?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate, m)?)?;

    // Multi-variable calculus
    m.add_function(wrap_pyfunction!(gradient, m)?)?;
    m.add_function(wrap_pyfunction!(hessian, m)?)?;
    m.add_function(wrap_pyfunction!(jacobian, m)?)?;

    // Uncertainty propagation
    m.add_function(wrap_pyfunction!(uncertainty_propagation_py, m)?)?;
    m.add_function(wrap_pyfunction!(relative_uncertainty_py, m)?)?;

    // Symbol management
    m.add_function(wrap_pyfunction!(py_symb, m)?)?;
    m.add_function(wrap_pyfunction!(py_symb_new, m)?)?;
    m.add_function(wrap_pyfunction!(py_symb_get, m)?)?;
    m.add_function(wrap_pyfunction!(py_symbol_exists, m)?)?;
    m.add_function(wrap_pyfunction!(py_symbol_count, m)?)?;
    m.add_function(wrap_pyfunction!(py_symbol_names, m)?)?;
    m.add_function(wrap_pyfunction!(py_remove_symbol, m)?)?;
    m.add_function(wrap_pyfunction!(py_clear_symbols, m)?)?;

    // Visitor utilities
    m.add_function(wrap_pyfunction!(count_nodes, m)?)?;
    m.add_function(wrap_pyfunction!(collect_variables, m)?)?;

    // Parallel evaluation (feature-gated)
    #[cfg(feature = "parallel")]
    m.add_function(wrap_pyfunction!(evaluate_parallel_py, m)?)?;
    #[cfg(feature = "parallel")]
    m.add_function(wrap_pyfunction!(eval_f64_py, m)?)?;

    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
