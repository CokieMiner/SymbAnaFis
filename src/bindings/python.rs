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

#[cfg(feature = "parallel")]
use crate::bindings::eval_f64::eval_f64 as rust_eval_f64;
use crate::core::evaluator::CompiledEvaluator;
use crate::core::symbol::Symbol as RustSymbol;
use crate::core::unified_context::{BodyFn, Context as RustContext};
#[cfg(feature = "parallel")]
use crate::parallel::{self, ExprInput, Value, VarInput};
use crate::uncertainty::{CovEntry, CovarianceMatrix};
use crate::{
    Dual, Expr as RustExpr, api::builder, clear_symbols, remove_symbol, symb, symb_get, symb_new,
    symbol_count, symbol_exists, symbol_names,
};
use num_traits::Float;
use numpy::{PyArray1, PyArray2, PyArrayMethods, PyReadonlyArray1};
use pyo3::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Type alias for complex partial derivative function type to improve readability
type PartialDerivativeFn = Arc<dyn Fn(&[Arc<RustExpr>]) -> RustExpr + Send + Sync>;

// Basic helper for hybrid NumPy/List support
enum DataInput<'py> {
    Array(PyReadonlyArray1<'py, f64>),
    List(Vec<f64>),
}

impl DataInput<'_> {
    fn as_slice(&self) -> PyResult<&[f64]> {
        match self {
            DataInput::Array(arr) => arr.as_slice().map_err(|_| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "All input arrays must be C-contiguous and f64",
                )
            }),
            DataInput::List(vec) => Ok(vec.as_slice()),
        }
    }

    fn len(&self) -> PyResult<usize> {
        match self {
            DataInput::Array(arr) => arr
                .len()
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}"))), // Handle Result<usize>? Wait, len()? is ok.
            DataInput::List(vec) => Ok(vec.len()),
        }
    }
}

fn extract_data_input<'py>(obj: &Bound<'py, PyAny>) -> PyResult<DataInput<'py>> {
    if let Ok(arr) = obj.extract::<PyReadonlyArray1<f64>>() {
        return Ok(DataInput::Array(arr));
    }
    if let Ok(vec) = obj.extract::<Vec<f64>>() {
        return Ok(DataInput::List(vec));
    }
    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
        "Expected NumPy array (float64) or list of floats",
    ))
}

// =============================================================================
// Domain Validation Helpers
// =============================================================================
// These functions check if a numeric argument is in the domain of special functions.
// They are called when the expression is a numeric constant to provide early error detection.

/// Check if a value is at a gamma/digamma/trigamma/tetragamma pole (non-positive integer)
fn is_gamma_pole(n: f64) -> bool {
    n <= 0.0 && (n - n.round()).abs() < 1e-10
}

/// Check if a value is at the zeta pole (s = 1)
fn is_zeta_pole(s: f64) -> bool {
    (s - 1.0).abs() < 1e-10
}

/// Check if a value is outside Lambert W domain (x < -1/e ≈ -0.367879)
fn is_lambert_w_domain_error(x: f64) -> bool {
    x < -std::f64::consts::E.recip() - 1e-10
}

/// Check if a value is outside Bessel Y/K domain (x <= 0)
fn is_bessel_yk_domain_error(x: f64) -> bool {
    x <= 0.0
}

/// Check if a value is outside elliptic K domain (|k| >= 1)
fn is_elliptic_k_domain_error(k: f64) -> bool {
    k.abs() >= 1.0
}

/// Check if a value is outside elliptic E domain (|k| > 1)
fn is_elliptic_e_domain_error(k: f64) -> bool {
    k.abs() > 1.0
}

/// Check if n is invalid for Hermite polynomial (n < 0)
fn is_hermite_domain_error(n: f64) -> bool {
    n < 0.0 || (n - n.round()).abs() > 1e-10
}

/// Check if x is outside Associated Legendre domain (|x| > 1)
fn is_assoc_legendre_x_domain_error(x: f64) -> bool {
    x.abs() > 1.0 + 1e-10
}

/// Check if l is invalid for Associated Legendre (l < 0)
fn is_legendre_l_domain_error(l: f64) -> bool {
    l < 0.0 || (l - l.round()).abs() > 1e-10
}

/// Check if |m| > l for Associated Legendre/Spherical Harmonic
const fn is_legendre_m_domain_error(l: f64, m: f64) -> bool {
    let l_int = l.round() as i32;
    let m_int = m.round() as i32;
    m_int.abs() > l_int
}

/// Check if polygamma order n is invalid (n < 0)
fn is_polygamma_order_domain_error(n: f64) -> bool {
    n < 0.0 || (n - n.round()).abs() > 1e-10
}

/// Check if base is invalid for logarithm (base <= 0 or base == 1)
fn is_log_base_domain_error(base: f64) -> bool {
    base <= 0.0 || (base - 1.0).abs() < 1e-10
}

/// Check if value is invalid for logarithm (x <= 0)
fn is_log_value_domain_error(x: f64) -> bool {
    x <= 0.0
}

/// Helper to get numeric value from expression if it's a constant
const fn get_numeric_value(expr: &RustExpr) -> Option<f64> {
    if let crate::ExprKind::Number(n) = &expr.kind {
        Some(*n)
    } else {
        None
    }
}

/// Helper to extract a Python value (int, float, Symbol, Expr, or string) to a Rust Expr
fn extract_to_expr(value: &Bound<'_, PyAny>) -> PyResult<RustExpr> {
    if let Ok(expr) = value.extract::<PyExpr>() {
        return Ok(expr.0);
    }
    if let Ok(sym) = value.extract::<PySymbol>() {
        return Ok(sym.0.to_expr());
    }
    if let Ok(n) = value.extract::<f64>() {
        return Ok(RustExpr::number(n));
    }
    if let Ok(n) = value.extract::<i64>() {
        // i64->f64 intentional: Python integers map naturally to floats
        #[allow(clippy::cast_precision_loss)]
        return Ok(RustExpr::number(n as f64));
    }
    if let Ok(s) = value.extract::<String>() {
        // Strings are strictly treated as symbols in this constructor path.
        // If the user wants to parse a formula, they should use the global parse() function.
        return Ok(symb(&s).into());
    }
    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
        "Argument must be Expr, Symbol, int, float, or string",
    ))
}

/// Wrapper for Rust Expr to expose to Python
#[pyclass(unsendable, name = "Expr")]
#[derive(Clone)]
struct PyExpr(RustExpr);

#[pymethods]
impl PyExpr {
    /// Create a symbolic expression or numeric constant
    #[new]
    fn new(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self(extract_to_expr(value)?))
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("Expr({})", self.0)
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        if let Ok(other_expr) = other.extract::<Self>() {
            return self.0 == other_expr.0;
        }
        if let Ok(other_sym) = other.extract::<PySymbol>() {
            return self.0 == other_sym.0.to_expr();
        }
        false
    }

    fn __hash__(&self) -> isize {
        // Simple hash based on string representation or kind
        // For better performance, we'd need a stable hash in Rust Expr
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut s = DefaultHasher::new();
        self.0.to_string().hash(&mut s);
        // u64->isize: Python __hash__ requires isize, wrap is expected
        #[allow(clippy::cast_possible_wrap)]
        {
            s.finish() as isize
        }
    }

    fn __float__(&self) -> PyResult<f64> {
        if let crate::ExprKind::Number(n) = &self.0.kind {
            Ok(*n)
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "Cannot convert non-numeric expression '{}' to float",
                self.0
            )))
        }
    }

    // Arithmetic operators
    fn __add__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        let other_expr = extract_to_expr(other)?;
        Ok(Self(self.0.clone() + other_expr))
    }

    fn __sub__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        let other_expr = extract_to_expr(other)?;
        Ok(Self(self.0.clone() - other_expr))
    }

    fn __mul__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        let other_expr = extract_to_expr(other)?;
        Ok(Self(self.0.clone() * other_expr))
    }

    fn __truediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        let other_expr = extract_to_expr(other)?;
        Ok(Self(self.0.clone() / other_expr))
    }

    fn __pow__(&self, other: &Bound<'_, PyAny>, _modulo: Option<Py<PyAny>>) -> PyResult<Self> {
        let other_expr = extract_to_expr(other)?;
        Ok(Self(self.0.clone().pow(other_expr)))
    }

    // Reverse power: 2 ** x where x is Expr
    fn __rpow__(&self, other: &Bound<'_, PyAny>, _modulo: Option<Py<PyAny>>) -> PyResult<Self> {
        // other ** self (other is the base, self is the exponent)
        if let Ok(n) = other.extract::<f64>() {
            return Ok(Self(RustExpr::number(n).pow(self.0.clone())));
        }
        if let Ok(n) = other.extract::<i64>() {
            // i64->f64 intentional: Python integers map naturally to floats
            #[allow(clippy::cast_precision_loss)]
            return Ok(Self(RustExpr::number(n as f64).pow(self.0.clone())));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "rpow() base must be int or float",
        ))
    }

    fn __neg__(&self) -> Self {
        Self(RustExpr::number(0.0) - self.0.clone())
    }

    fn __radd__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(n) = other.extract::<f64>() {
            return Ok(Self(RustExpr::number(n) + self.0.clone()));
        }
        if let Ok(n) = other.extract::<i64>() {
            // i64->f64 intentional: Python integers map naturally to floats
            #[allow(clippy::cast_precision_loss)]
            return Ok(Self(RustExpr::number(n as f64) + self.0.clone()));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be int or float",
        ))
    }

    fn __rsub__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(n) = other.extract::<f64>() {
            return Ok(Self(RustExpr::number(n) - self.0.clone()));
        }
        if let Ok(n) = other.extract::<i64>() {
            // i64->f64 intentional: Python integers map naturally to floats
            #[allow(clippy::cast_precision_loss)]
            return Ok(Self(RustExpr::number(n as f64) - self.0.clone()));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be int or float",
        ))
    }

    fn __rmul__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(n) = other.extract::<f64>() {
            return Ok(Self(RustExpr::number(n) * self.0.clone()));
        }
        if let Ok(n) = other.extract::<i64>() {
            // i64->f64 intentional: Python integers map naturally to floats
            #[allow(clippy::cast_precision_loss)]
            return Ok(Self(RustExpr::number(n as f64) * self.0.clone()));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be int or float",
        ))
    }

    fn __rtruediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(n) = other.extract::<f64>() {
            return Ok(Self(RustExpr::number(n) / self.0.clone()));
        }
        if let Ok(n) = other.extract::<i64>() {
            // i64->f64 intentional: Python integers map naturally to floats
            #[allow(clippy::cast_precision_loss)]
            return Ok(Self(RustExpr::number(n as f64) / self.0.clone()));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be int or float",
        ))
    }
    // Functions
    fn sin(&self) -> Self {
        Self(self.0.clone().sin())
    }
    fn cos(&self) -> Self {
        Self(self.0.clone().cos())
    }
    fn tan(&self) -> Self {
        Self(self.0.clone().tan())
    }
    fn cot(&self) -> Self {
        Self(self.0.clone().cot())
    }
    fn sec(&self) -> Self {
        Self(self.0.clone().sec())
    }
    fn csc(&self) -> Self {
        Self(self.0.clone().csc())
    }

    fn asin(&self) -> Self {
        Self(self.0.clone().asin())
    }
    fn acos(&self) -> Self {
        Self(self.0.clone().acos())
    }
    fn atan(&self) -> Self {
        Self(self.0.clone().atan())
    }
    fn acot(&self) -> Self {
        Self(self.0.clone().acot())
    }
    fn asec(&self) -> Self {
        Self(self.0.clone().asec())
    }
    fn acsc(&self) -> Self {
        Self(self.0.clone().acsc())
    }

    fn sinh(&self) -> Self {
        Self(self.0.clone().sinh())
    }
    fn cosh(&self) -> Self {
        Self(self.0.clone().cosh())
    }
    fn tanh(&self) -> Self {
        Self(self.0.clone().tanh())
    }
    fn coth(&self) -> Self {
        Self(self.0.clone().coth())
    }
    fn sech(&self) -> Self {
        Self(self.0.clone().sech())
    }
    fn csch(&self) -> Self {
        Self(self.0.clone().csch())
    }

    fn asinh(&self) -> Self {
        Self(self.0.clone().asinh())
    }
    fn acosh(&self) -> Self {
        Self(self.0.clone().acosh())
    }
    fn atanh(&self) -> Self {
        Self(self.0.clone().atanh())
    }
    fn acoth(&self) -> Self {
        Self(self.0.clone().acoth())
    }
    fn asech(&self) -> Self {
        Self(self.0.clone().asech())
    }
    fn acsch(&self) -> Self {
        Self(self.0.clone().acsch())
    }

    fn exp(&self) -> Self {
        Self(self.0.clone().exp())
    }
    fn ln(&self) -> Self {
        Self(self.0.clone().ln())
    }
    /// Logarithm with the specified base: log(self, base) → log(base, self)
    /// For natural logarithm, use `ln()` instead
    fn log(&self, base: &Bound<'_, PyAny>) -> PyResult<Self> {
        let base_expr = extract_to_expr(base)?;
        // Check base is valid
        if let Some(b) = get_numeric_value(&base_expr)
            && is_log_base_domain_error(b)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "log base {b} invalid: must be positive and not 1"
            )));
        }
        // Check value is valid
        if let Some(x) = get_numeric_value(&self.0)
            && is_log_value_domain_error(x)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "log({x}, x) undefined: x must be positive"
            )));
        }
        Ok(Self(self.0.clone().log(base_expr)))
    }
    fn log10(&self) -> Self {
        Self(self.0.clone().log10())
    }
    fn log2(&self) -> Self {
        Self(self.0.clone().log2())
    }

    fn sqrt(&self) -> Self {
        Self(self.0.clone().sqrt())
    }
    fn cbrt(&self) -> Self {
        Self(self.0.clone().cbrt())
    }

    fn abs(&self) -> Self {
        Self(self.0.clone().abs())
    }
    fn signum(&self) -> Self {
        Self(self.0.clone().signum())
    }
    fn sinc(&self) -> Self {
        Self(self.0.clone().sinc())
    }
    fn erf(&self) -> Self {
        Self(self.0.clone().erf())
    }
    fn erfc(&self) -> Self {
        Self(self.0.clone().erfc())
    }
    fn gamma(&self) -> PyResult<Self> {
        if let Some(n) = get_numeric_value(&self.0)
            && is_gamma_pole(n)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "gamma({n}) undefined: pole at non-positive integer"
            )));
        }
        Ok(Self(self.0.clone().gamma()))
    }
    fn digamma(&self) -> PyResult<Self> {
        if let Some(n) = get_numeric_value(&self.0)
            && is_gamma_pole(n)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "digamma({n}) undefined: pole at non-positive integer"
            )));
        }
        Ok(Self(self.0.clone().digamma()))
    }
    fn trigamma(&self) -> PyResult<Self> {
        if let Some(n) = get_numeric_value(&self.0)
            && is_gamma_pole(n)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "trigamma({n}) undefined: pole at non-positive integer"
            )));
        }
        Ok(Self(self.0.clone().trigamma()))
    }
    fn tetragamma(&self) -> PyResult<Self> {
        if let Some(n) = get_numeric_value(&self.0)
            && is_gamma_pole(n)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "tetragamma({n}) undefined: pole at non-positive integer"
            )));
        }
        Ok(Self(self.0.clone().tetragamma()))
    }
    fn floor(&self) -> Self {
        Self(self.0.clone().floor())
    }
    fn ceil(&self) -> Self {
        Self(self.0.clone().ceil())
    }
    fn round(&self) -> Self {
        Self(self.0.clone().round())
    }
    fn elliptic_k(&self) -> PyResult<Self> {
        if let Some(k) = get_numeric_value(&self.0)
            && is_elliptic_k_domain_error(k)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "elliptic_k({k}) undefined: |k| must be < 1"
            )));
        }
        Ok(Self(self.0.clone().elliptic_k()))
    }
    fn elliptic_e(&self) -> PyResult<Self> {
        if let Some(k) = get_numeric_value(&self.0)
            && is_elliptic_e_domain_error(k)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "elliptic_e({k}) undefined: |k| must be <= 1"
            )));
        }
        Ok(Self(self.0.clone().elliptic_e()))
    }
    fn exp_polar(&self) -> Self {
        Self(self.0.clone().exp_polar())
    }
    fn polygamma(&self, n: &Bound<'_, PyAny>) -> PyResult<Self> {
        let n_expr = extract_to_expr(n)?;
        // Check order n is valid
        if let Some(order) = get_numeric_value(&n_expr)
            && is_polygamma_order_domain_error(order)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "polygamma order {order} invalid: must be non-negative integer"
            )));
        }
        // Check x is not at a pole
        if let Some(x) = get_numeric_value(&self.0)
            && is_gamma_pole(x)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "polygamma(n, {x}) undefined: pole at non-positive integer"
            )));
        }
        Ok(Self(self.0.clone().polygamma(n_expr)))
    }
    fn beta(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        let other_expr = extract_to_expr(other)?;
        // Check both arguments are not at gamma poles
        if let Some(a) = get_numeric_value(&self.0)
            && is_gamma_pole(a)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "beta({a}, b) undefined: pole at non-positive integer"
            )));
        }
        if let Some(b) = get_numeric_value(&other_expr)
            && is_gamma_pole(b)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "beta(a, {b}) undefined: pole at non-positive integer"
            )));
        }
        Ok(Self(self.0.clone().beta(other_expr)))
    }
    fn zeta(&self) -> PyResult<Self> {
        if let Some(s) = get_numeric_value(&self.0)
            && is_zeta_pole(s)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "zeta(1) undefined: pole at s=1",
            ));
        }
        Ok(Self(self.0.clone().zeta()))
    }
    fn lambertw(&self) -> PyResult<Self> {
        if let Some(x) = get_numeric_value(&self.0)
            && is_lambert_w_domain_error(x)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "lambertw({x}) undefined: x must be >= -1/e"
            )));
        }
        Ok(Self(self.0.clone().lambertw()))
    }
    fn besselj(&self, n: &Bound<'_, PyAny>) -> PyResult<Self> {
        let n_expr = extract_to_expr(n)?;
        Ok(Self(self.0.clone().besselj(n_expr)))
    }
    fn bessely(&self, n: &Bound<'_, PyAny>) -> PyResult<Self> {
        let n_expr = extract_to_expr(n)?;
        // Y_n(x) requires x > 0
        if let Some(x) = get_numeric_value(&self.0)
            && is_bessel_yk_domain_error(x)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "bessely(n, {x}) undefined: x must be > 0"
            )));
        }
        Ok(Self(self.0.clone().bessely(n_expr)))
    }
    fn besseli(&self, n: &Bound<'_, PyAny>) -> PyResult<Self> {
        let n_expr = extract_to_expr(n)?;
        Ok(Self(self.0.clone().besseli(n_expr)))
    }
    fn besselk(&self, n: &Bound<'_, PyAny>) -> PyResult<Self> {
        let n_expr = extract_to_expr(n)?;
        // K_n(x) requires x > 0
        if let Some(x) = get_numeric_value(&self.0)
            && is_bessel_yk_domain_error(x)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "besselk(n, {x}) undefined: x must be > 0"
            )));
        }
        Ok(Self(self.0.clone().besselk(n_expr)))
    }

    fn pow(&self, exp: &Bound<'_, PyAny>) -> PyResult<Self> {
        let exp_expr = extract_to_expr(exp)?;
        Ok(Self(RustExpr::pow_static(self.0.clone(), exp_expr)))
    }

    // Multi-argument functions
    /// Two-argument arctangent: atan2(y, x) = angle to point (x, y)
    fn atan2(&self, x: &Bound<'_, PyAny>) -> PyResult<Self> {
        let x_expr = extract_to_expr(x)?;
        Ok(Self(RustExpr::func_multi(
            "atan2",
            vec![self.0.clone(), x_expr],
        )))
    }

    /// Hermite polynomial `H_n(self)`
    fn hermite(&self, n: &Bound<'_, PyAny>) -> PyResult<Self> {
        let n_expr = extract_to_expr(n)?;
        // Check n is valid (non-negative integer)
        if let Some(order) = get_numeric_value(&n_expr)
            && is_hermite_domain_error(order)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "hermite({order}, x) undefined: n must be non-negative integer"
            )));
        }
        Ok(Self(RustExpr::func_multi(
            "hermite",
            vec![n_expr, self.0.clone()],
        )))
    }

    /// Associated Legendre polynomial `P_l^m(self)`
    fn assoc_legendre(&self, l: &Bound<'_, PyAny>, m: &Bound<'_, PyAny>) -> PyResult<Self> {
        let l_expr = extract_to_expr(l)?;
        let m_expr = extract_to_expr(m)?;
        // Check domain: l >= 0, |m| <= l, |x| <= 1
        if let Some(l_val) = get_numeric_value(&l_expr) {
            if is_legendre_l_domain_error(l_val) {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "assoc_legendre({l_val}, m, x) undefined: l must be non-negative integer"
                )));
            }
            if let Some(m_val) = get_numeric_value(&m_expr)
                && is_legendre_m_domain_error(l_val, m_val)
            {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "assoc_legendre({l_val}, {m_val}, x) undefined: |m| must be <= l"
                )));
            }
        }
        if let Some(x) = get_numeric_value(&self.0)
            && is_assoc_legendre_x_domain_error(x)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "assoc_legendre(l, m, {x}) undefined: |x| must be <= 1"
            )));
        }
        Ok(Self(RustExpr::func_multi(
            "assoc_legendre",
            vec![l_expr, m_expr, self.0.clone()],
        )))
    }

    /// Spherical harmonic `Y_l^m(theta`, phi) where self is theta
    fn spherical_harmonic(
        &self,
        l: &Bound<'_, PyAny>,
        m: &Bound<'_, PyAny>,
        phi: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let l_expr = extract_to_expr(l)?;
        let m_expr = extract_to_expr(m)?;
        let phi_expr = extract_to_expr(phi)?;
        // Check domain: l >= 0, |m| <= l
        if let Some(l_val) = get_numeric_value(&l_expr) {
            if is_legendre_l_domain_error(l_val) {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "spherical_harmonic({l_val}, m, θ, φ) undefined: l must be non-negative integer"
                )));
            }
            if let Some(m_val) = get_numeric_value(&m_expr)
                && is_legendre_m_domain_error(l_val, m_val)
            {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "spherical_harmonic({l_val}, {m_val}, θ, φ) undefined: |m| must be <= l"
                )));
            }
        }
        Ok(Self(RustExpr::func_multi(
            "spherical_harmonic",
            vec![l_expr, m_expr, self.0.clone(), phi_expr],
        )))
    }

    /// Alternative spherical harmonic notation `Y_l^m(theta`, phi)
    fn ynm(
        &self,
        l: &Bound<'_, PyAny>,
        m: &Bound<'_, PyAny>,
        phi: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let l_expr = extract_to_expr(l)?;
        let m_expr = extract_to_expr(m)?;
        let phi_expr = extract_to_expr(phi)?;
        // Check domain: l >= 0, |m| <= l
        if let Some(l_val) = get_numeric_value(&l_expr) {
            if is_legendre_l_domain_error(l_val) {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "ynm({l_val}, m, θ, φ) undefined: l must be non-negative integer"
                )));
            }
            if let Some(m_val) = get_numeric_value(&m_expr)
                && is_legendre_m_domain_error(l_val, m_val)
            {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "ynm({l_val}, {m_val}, θ, φ) undefined: |m| must be <= l"
                )));
            }
        }
        Ok(Self(RustExpr::func_multi(
            "ynm",
            vec![l_expr, m_expr, self.0.clone(), phi_expr],
        )))
    }

    /// Derivative of Riemann zeta function: zeta^(n)(self)
    fn zeta_deriv(&self, n: &Bound<'_, PyAny>) -> PyResult<Self> {
        let n_expr = extract_to_expr(n)?;
        // Check zeta pole at s=1
        if let Some(s) = get_numeric_value(&self.0)
            && is_zeta_pole(s)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "zeta_deriv(n, 1) undefined: pole at s=1",
            ));
        }
        // Check n is non-negative
        if let Some(order) = get_numeric_value(&n_expr)
            && (order < 0.0 || (order - order.round()).abs() > 1e-10)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "zeta_deriv({order}, s) undefined: n must be non-negative integer"
            )));
        }
        Ok(Self(RustExpr::func_multi(
            "zeta_deriv",
            vec![n_expr, self.0.clone()],
        )))
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
    const fn is_symbol(&self) -> bool {
        matches!(self.0.kind, crate::ExprKind::Symbol(_))
    }

    /// Check if expression is a constant number
    const fn is_number(&self) -> bool {
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
    fn substitute(&self, var: &str, value: &Bound<'_, PyAny>) -> PyResult<Self> {
        let replacement = extract_to_expr(value)?;
        Ok(Self(self.0.substitute(var, &replacement)))
    }

    /// Evaluate the expression with given variable values
    ///
    /// Args:
    ///     vars: dict mapping variable names to float values
    ///
    /// Returns:
    ///     Evaluated expression (may be a number or symbolic if variables remain)
    // PyO3 requires owned HashMap for Python dict arguments
    #[allow(clippy::needless_pass_by_value)]
    fn evaluate(&self, vars: std::collections::HashMap<String, f64>) -> Self {
        let var_map: std::collections::HashMap<&str, f64> =
            vars.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        Self(self.0.evaluate(&var_map, &std::collections::HashMap::new()))
    }

    /// Differentiate this expression
    fn diff(&self, var: &str) -> PyResult<Self> {
        let sym = crate::symb(var);
        crate::Diff::new()
            .differentiate(&self.0, &sym)
            .map(PyExpr)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))
    }

    /// Simplify this expression
    fn simplify(&self) -> PyResult<Self> {
        crate::Simplify::new()
            .simplify(&self.0)
            .map(PyExpr)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))
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
    const fn new(val: f64, eps: f64) -> Self {
        Self(Dual::new(val, eps))
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
        Self(Dual::constant(val))
    }

    /// Get the real value component
    #[getter]
    const fn val(&self) -> f64 {
        self.0.val
    }

    /// Get the infinitesimal derivative component
    #[getter]
    const fn eps(&self) -> f64 {
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
    fn __add__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(dual) = other.extract::<Self>() {
            return Ok(Self(self.0 + dual.0));
        }
        if let Ok(n) = other.extract::<f64>() {
            return Ok(Self(self.0 + Dual::constant(n)));
        }
        if let Ok(n) = other.extract::<i32>() {
            return Ok(Self(self.0 + Dual::constant(f64::from(n))));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be Dual, int, or float",
        ))
    }

    fn __sub__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(dual) = other.extract::<Self>() {
            return Ok(Self(self.0 - dual.0));
        }
        if let Ok(n) = other.extract::<f64>() {
            return Ok(Self(self.0 - Dual::constant(n)));
        }
        if let Ok(n) = other.extract::<i32>() {
            return Ok(Self(self.0 - Dual::constant(f64::from(n))));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be Dual, int, or float",
        ))
    }

    fn __mul__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(dual) = other.extract::<Self>() {
            return Ok(Self(self.0 * dual.0));
        }
        if let Ok(n) = other.extract::<f64>() {
            return Ok(Self(self.0 * Dual::constant(n)));
        }
        if let Ok(n) = other.extract::<i32>() {
            return Ok(Self(self.0 * Dual::constant(f64::from(n))));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be Dual, int, or float",
        ))
    }

    fn __truediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(dual) = other.extract::<Self>() {
            return Ok(Self(self.0 / dual.0));
        }
        if let Ok(n) = other.extract::<f64>() {
            return Ok(Self(self.0 / Dual::constant(n)));
        }
        if let Ok(n) = other.extract::<i32>() {
            return Ok(Self(self.0 / Dual::constant(f64::from(n))));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be Dual, int, or float",
        ))
    }

    fn __neg__(&self) -> Self {
        Self(-self.0)
    }

    // Reverse operations for mixed types
    fn __radd__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(n) = other.extract::<f64>() {
            return Ok(Self(Dual::constant(n) + self.0));
        }
        if let Ok(n) = other.extract::<i32>() {
            return Ok(Self(Dual::constant(f64::from(n)) + self.0));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be int or float",
        ))
    }

    fn __rsub__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(n) = other.extract::<f64>() {
            return Ok(Self(Dual::constant(n) - self.0));
        }
        if let Ok(n) = other.extract::<i32>() {
            return Ok(Self(Dual::constant(f64::from(n)) - self.0));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be int or float",
        ))
    }

    fn __rmul__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(n) = other.extract::<f64>() {
            return Ok(Self(Dual::constant(n) * self.0));
        }
        if let Ok(n) = other.extract::<i32>() {
            return Ok(Self(Dual::constant(f64::from(n)) * self.0));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be int or float",
        ))
    }

    fn __rtruediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(n) = other.extract::<f64>() {
            return Ok(Self(Dual::constant(n) / self.0));
        }
        if let Ok(n) = other.extract::<i32>() {
            return Ok(Self(Dual::constant(f64::from(n)) / self.0));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be int or float",
        ))
    }

    fn __pow__(&self, other: &Bound<'_, PyAny>, _modulo: Option<Py<PyAny>>) -> PyResult<Self> {
        if let Ok(dual) = other.extract::<Self>() {
            return Ok(Self(self.0.powf(dual.0)));
        }
        if let Ok(n) = other.extract::<f64>() {
            return Ok(Self(self.0.powf(Dual::constant(n))));
        }
        if let Ok(n) = other.extract::<i32>() {
            return Ok(Self(self.0.powi(n)));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Exponent must be Dual, int, or float",
        ))
    }

    fn __rpow__(&self, other: &Bound<'_, PyAny>, _modulo: Option<Py<PyAny>>) -> PyResult<Self> {
        // other ** self (other is the base, self is the exponent)
        if let Ok(n) = other.extract::<f64>() {
            return Ok(Self(Dual::constant(n).powf(self.0)));
        }
        if let Ok(n) = other.extract::<i32>() {
            return Ok(Self(Dual::constant(f64::from(n)).powf(self.0)));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Base must be int or float",
        ))
    }

    // Mathematical functions
    fn sin(&self) -> Self {
        Self(self.0.sin())
    }

    fn cos(&self) -> Self {
        Self(self.0.cos())
    }

    fn tan(&self) -> Self {
        Self(self.0.tan())
    }

    fn asin(&self) -> Self {
        Self(self.0.asin())
    }

    fn acos(&self) -> Self {
        Self(self.0.acos())
    }

    fn atan(&self) -> Self {
        Self(self.0.atan())
    }

    fn sinh(&self) -> Self {
        Self(self.0.sinh())
    }

    fn cosh(&self) -> Self {
        Self(self.0.cosh())
    }

    fn tanh(&self) -> Self {
        Self(self.0.tanh())
    }

    fn exp(&self) -> Self {
        Self(self.0.exp())
    }

    fn exp2(&self) -> Self {
        Self(self.0.exp2())
    }

    fn ln(&self) -> Self {
        Self(self.0.ln())
    }

    fn log2(&self) -> Self {
        Self(self.0.log2())
    }

    fn log10(&self) -> Self {
        Self(self.0.log10())
    }

    fn sqrt(&self) -> Self {
        Self(self.0.sqrt())
    }

    fn cbrt(&self) -> Self {
        Self(self.0.cbrt())
    }

    fn abs(&self) -> Self {
        Self(self.0.abs())
    }

    fn powf(&self, other: &Self) -> Self {
        Self(self.0.powf(other.0))
    }

    fn powi(&self, n: i32) -> Self {
        Self(self.0.powi(n))
    }

    // Inverse hyperbolic functions
    fn asinh(&self) -> Self {
        Self(self.0.asinh())
    }

    fn acosh(&self) -> Self {
        Self(self.0.acosh())
    }

    fn atanh(&self) -> Self {
        Self(self.0.atanh())
    }

    // Special functions
    /// Error function
    fn erf(&self) -> Self {
        Self(self.0.erf())
    }

    /// Complementary error function
    fn erfc(&self) -> Self {
        Self(self.0.erfc())
    }

    /// Gamma function
    fn gamma(&self) -> PyResult<Self> {
        self.0.gamma().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Gamma function undefined for this value",
            )
        })
    }

    /// Digamma function (logarithmic derivative of gamma)
    fn digamma(&self) -> PyResult<Self> {
        self.0.digamma().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Digamma function undefined for this value",
            )
        })
    }

    /// Trigamma function
    fn trigamma(&self) -> PyResult<Self> {
        self.0.trigamma().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Trigamma function undefined for this value",
            )
        })
    }

    /// Polygamma function
    fn polygamma(&self, n: i32) -> PyResult<Self> {
        self.0.polygamma(n).map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Polygamma function undefined for this value",
            )
        })
    }

    /// Riemann zeta function
    fn zeta(&self) -> PyResult<Self> {
        self.0.zeta().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Zeta function undefined for this value",
            )
        })
    }

    /// Lambert W function
    fn lambert_w(&self) -> PyResult<Self> {
        self.0.lambert_w().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Lambert W function undefined for this value",
            )
        })
    }

    /// Bessel function of the first kind
    fn bessel_j(&self, n: i32) -> PyResult<Self> {
        self.0.bessel_j(n).map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Bessel J function undefined for this value",
            )
        })
    }

    /// Sinc function: sin(x)/x
    fn sinc(&self) -> Self {
        Self(self.0.sinc())
    }

    /// Sign function
    fn sign(&self) -> Self {
        Self(self.0.sign())
    }

    /// Elliptic integral of the first kind
    fn elliptic_k(&self) -> PyResult<Self> {
        self.0.elliptic_k().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Elliptic K function undefined for this value",
            )
        })
    }

    /// Elliptic integral of the second kind
    fn elliptic_e(&self) -> PyResult<Self> {
        self.0.elliptic_e().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Elliptic E function undefined for this value",
            )
        })
    }

    /// Beta function: B(a, b) = Γ(a)Γ(b)/Γ(a+b)
    fn beta(&self, b: &Self) -> PyResult<Self> {
        self.0.beta(b.0).map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Beta function undefined for these values",
            )
        })
    }

    // Additional Float trait methods
    fn signum(&self) -> Self {
        Self(self.0.signum())
    }

    fn floor(&self) -> Self {
        Self(self.0.floor())
    }

    fn ceil(&self) -> Self {
        Self(self.0.ceil())
    }

    fn round(&self) -> Self {
        Self(self.0.round())
    }

    fn trunc(&self) -> Self {
        Self(self.0.trunc())
    }

    fn fract(&self) -> Self {
        Self(self.0.fract())
    }

    fn recip(&self) -> Self {
        Self(self.0.recip())
    }

    fn exp_m1(&self) -> Self {
        Self(self.0.exp_m1())
    }

    fn ln_1p(&self) -> Self {
        Self(self.0.ln_1p())
    }

    fn hypot(&self, other: &Self) -> Self {
        Self(self.0.hypot(other.0))
    }

    fn atan2(&self, other: &Self) -> Self {
        Self(self.0.atan2(other.0))
    }

    fn log(&self, base: &Self) -> Self {
        Self(self.0.log(base.0))
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
        Self {
            inner: builder::Diff::new(),
        }
    }

    fn domain_safe(mut self_: PyRefMut<'_, Self>, safe: bool) -> PyRefMut<'_, Self> {
        self_.inner = self_.inner.clone().domain_safe(safe);
        self_
    }

    fn fixed_var<'a>(
        mut self_: PyRefMut<'a, Self>,
        var: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'a, Self>> {
        if let Ok(s) = var.extract::<String>() {
            self_.inner = self_.inner.clone().fixed_var(&s);
        } else if let Ok(sym) = var.extract::<PySymbol>() {
            self_.inner = self_.inner.clone().fixed_var(&sym.0);
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "fixed_var requires a string or Symbol object.",
            ));
        }
        Ok(self_)
    }

    fn fixed_vars<'a>(
        mut self_: PyRefMut<'a, Self>,
        vars: Vec<Bound<'_, PyAny>>,
    ) -> PyResult<PyRefMut<'a, Self>> {
        for var in vars {
            if let Ok(s) = var.extract::<String>() {
                self_.inner = self_.inner.clone().fixed_var(&s);
            } else if let Ok(sym) = var.extract::<PySymbol>() {
                self_.inner = self_.inner.clone().fixed_var(&sym.0);
            } else {
                return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                    "fixed_vars requires a list of strings or Symbol objects.",
                ));
            }
        }
        Ok(self_)
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
    /// Register a custom function with optional body and partial derivatives.
    ///
    /// Args:
    ///     name: Function name
    ///     arity: Number of arguments
    ///     body_callback: Optional Function(list of exprs) -> expr (for evaluation)
    ///     partials: Optional list of functions for derivatives [∂f/∂x0, ∂f/∂x1, ...]
    ///
    /// Example:
    /// ```python
    /// # f(x, y) with body and two partials
    /// diff = Diff().user_fn("my_func", 2, my_body, [partial_x, partial_y])
    /// ```
    #[pyo3(signature = (name, arity, body_callback=None, partials=None))]
    fn user_fn(
        mut self_: PyRefMut<'_, Self>,
        name: String,
        arity: usize,
        body_callback: Option<Py<PyAny>>,
        partials: Option<Vec<Py<PyAny>>>,
    ) -> PyResult<PyRefMut<'_, Self>> {
        use crate::core::unified_context::UserFunction;
        use std::sync::Arc;

        let mut user_fn = UserFunction::new(arity..=arity);

        // Handle optional body function
        if let Some(callback) = body_callback {
            let body_fn: BodyFn = Arc::new(move |args: &[Arc<RustExpr>]| -> RustExpr {
                Python::attach(|py| {
                    let py_args: Vec<PyExpr> = args.iter().map(|a| PyExpr((**a).clone())).collect();
                    let Ok(py_list) = py_args.into_pyobject(py) else {
                        return RustExpr::number(0.0);
                    };

                    let result = callback.call1(py, (py_list,));

                    result.map_or_else(
                        |_| RustExpr::number(0.0),
                        |res| {
                            res.extract::<PyExpr>(py)
                                .map_or_else(|_| RustExpr::number(0.0), |py_expr| py_expr.0)
                        },
                    )
                })
            });
            user_fn = user_fn.body_arc(body_fn);
        }

        // Handle optional list of partial derivatives
        if let Some(callbacks) = partials {
            for (i, callback) in callbacks.into_iter().enumerate() {
                let partial_fn: PartialDerivativeFn =
                    Arc::new(move |args: &[Arc<RustExpr>]| -> RustExpr {
                        Python::attach(|py| {
                            let py_args: Vec<PyExpr> =
                                args.iter().map(|a| PyExpr((**a).clone())).collect();
                            let Ok(py_list) = py_args.into_pyobject(py) else {
                                return RustExpr::number(0.0);
                            };

                            let result = callback.call1(py, (py_list,));

                            result.map_or_else(
                                |_| RustExpr::number(0.0),
                                |res| {
                                    res.extract::<PyExpr>(py)
                                        .map_or_else(|_| RustExpr::number(0.0), |py_expr| py_expr.0)
                                },
                            )
                        })
                    });

                user_fn = user_fn.partial_arc(i, partial_fn).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}"))
                })?;
            }
        }

        self_.inner = self_.inner.clone().user_fn(name, user_fn);
        Ok(self_)
    }

    fn diff_str(&self, formula: &str, var: &str) -> PyResult<String> {
        self.inner
            .diff_str(formula, var, &[])
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))
    }

    fn differentiate(&self, expr: &Bound<'_, PyAny>, var: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let rust_expr = extract_to_expr(expr)?;
        let sym = if let Ok(var_str) = var.extract::<String>() {
            crate::symb(&var_str)
        } else if let Ok(var_sym) = var.extract::<PySymbol>() {
            var_sym.0
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Variable must be a string or Symbol",
            ));
        };

        self.inner
            .differentiate(&rust_expr, &sym)
            .map(PyExpr)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))
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
        Self {
            inner: builder::Simplify::new(),
        }
    }

    fn domain_safe(mut self_: PyRefMut<'_, Self>, safe: bool) -> PyRefMut<'_, Self> {
        self_.inner = self_.inner.clone().domain_safe(safe);
        self_
    }

    fn fixed_var<'a>(
        mut self_: PyRefMut<'a, Self>,
        var: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'a, Self>> {
        if let Ok(s) = var.extract::<String>() {
            self_.inner = self_.inner.clone().fixed_var(&s);
        } else if let Ok(sym) = var.extract::<PySymbol>() {
            self_.inner = self_.inner.clone().fixed_var(&sym.0);
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "fixed_var requires a string or Symbol object.",
            ));
        }
        Ok(self_)
    }

    fn fixed_vars<'a>(
        mut self_: PyRefMut<'a, Self>,
        vars: Vec<Bound<'_, PyAny>>,
    ) -> PyResult<PyRefMut<'a, Self>> {
        for var in vars {
            if let Ok(s) = var.extract::<String>() {
                self_.inner = self_.inner.clone().fixed_var(&s);
            } else if let Ok(sym) = var.extract::<PySymbol>() {
                self_.inner = self_.inner.clone().fixed_var(&sym.0);
            } else {
                return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                    "fixed_vars requires a list of strings or Symbol objects.",
                ));
            }
        }
        Ok(self_)
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

    fn simplify(&self, expr: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let rust_expr = extract_to_expr(expr)?;
        self.inner
            .simplify(&rust_expr)
            .map(PyExpr)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))
    }

    fn simplify_str(&self, formula: &str) -> PyResult<String> {
        self.inner
            .simplify_str(formula, &[])
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))
    }
}

/// Differentiate a mathematical expression string symbolically.
///
/// Args:
///     formula: Mathematical expression string to differentiate
///     var: Variable to differentiate with respect to
///     `known_symbols`: Optional list of multi-character symbols
///     `custom_functions`: Optional list of user-defined function names
///
/// Returns:
///     The derivative as a string
///
/// For Expr input, use `Diff().differentiate()` instead.
// PyO3 requires owned types for Python function arguments
#[allow(clippy::needless_pass_by_value)]
#[pyfunction]
#[pyo3(signature = (formula, var, known_symbols=None, custom_functions=None))]
fn diff(
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

    crate::diff(formula, var, &known_strs, custom_strs.as_deref())
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))
}

/// Simplify a mathematical expression string.
///
/// Args:
///     formula: Mathematical expression string to simplify
///     `known_symbols`: Optional list of multi-character symbols
///     `custom_functions`: Optional list of user-defined function names
///
/// Returns:
///     The simplified expression as a string
///
/// For Expr input, use `Simplify().simplify()` instead.
// PyO3 requires owned types for Python function arguments
#[allow(clippy::needless_pass_by_value)]
#[pyfunction]
#[pyo3(signature = (formula, known_symbols=None, custom_functions=None))]
fn simplify(
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

    crate::simplify(formula, &known_strs, custom_strs.as_deref())
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))
}

/// Parse a mathematical expression and return the expression object.
#[pyfunction]
#[pyo3(signature = (formula, known_symbols=None, custom_functions=None))]
fn parse(
    formula: &str,
    known_symbols: Option<Vec<String>>,
    custom_functions: Option<Vec<String>>,
) -> PyResult<PyExpr> {
    let known: HashSet<String> = known_symbols
        .map(|v| v.into_iter().collect())
        .unwrap_or_default();
    let custom: HashSet<String> = custom_functions
        .map(|v| v.into_iter().collect())
        .unwrap_or_default();

    crate::parser::parse(formula, &known, &custom, None)
        .map(PyExpr)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))
}

/// Compute the gradient of a scalar Expr.
///
/// Args:
///     expr: Expr object to differentiate
///     vars: List of variable names to differentiate with respect to
///
/// Returns:
///     List of partial derivative Expr objects [∂f/∂x₁, ∂f/∂x₂, ...]
///
/// For string input, use `gradient_str()` instead.
// PyO3 requires owned types for Python function arguments
#[allow(clippy::needless_pass_by_value)]
#[pyfunction]
fn gradient(expr: PyExpr, vars: Vec<String>) -> PyResult<Vec<PyExpr>> {
    let symbols: Vec<RustSymbol> = vars.iter().map(|s| crate::symb(s)).collect();
    let sym_refs: Vec<&RustSymbol> = symbols.iter().collect();

    let res = crate::gradient(&expr.0, &sym_refs)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))?;
    Ok(res.into_iter().map(PyExpr).collect())
}

/// Compute the gradient of a scalar expression string.
///
/// Args:
///     formula: String formula to differentiate
///     vars: List of variable names to differentiate with respect to
///
/// Returns:
///     List of partial derivative strings [∂f/∂x₁, ∂f/∂x₂, ...]
// PyO3 requires owned types for Python function arguments
#[allow(clippy::needless_pass_by_value)]
#[pyfunction]
fn gradient_str(formula: &str, vars: Vec<String>) -> PyResult<Vec<String>> {
    let var_strs: Vec<&str> = vars.iter().map(std::string::String::as_str).collect();
    crate::gradient_str(formula, &var_strs)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))
}

/// Compute the Hessian matrix of a scalar Expr.
///
/// Args:
///     expr: Expr object to differentiate twice
///     vars: List of variable names
///
/// Returns:
///     2D list of second partial derivative Expr objects
///
/// For string input, use `hessian_str()` instead.
// PyO3 requires owned types for Python function arguments
#[allow(clippy::needless_pass_by_value)]
#[pyfunction]
fn hessian(expr: PyExpr, vars: Vec<String>) -> PyResult<Vec<Vec<PyExpr>>> {
    let symbols: Vec<RustSymbol> = vars.iter().map(|s| crate::symb(s)).collect();
    let sym_refs: Vec<&RustSymbol> = symbols.iter().collect();

    let res = crate::hessian(&expr.0, &sym_refs)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))?;
    Ok(res
        .into_iter()
        .map(|row| row.into_iter().map(PyExpr).collect())
        .collect())
}

/// Compute the Hessian matrix of a scalar expression string.
///
/// Args:
///     formula: String formula to differentiate twice
///     vars: List of variable names
///
/// Returns:
///     2D list of second partial derivative strings
// PyO3 requires owned types for Python function arguments
#[allow(clippy::needless_pass_by_value)]
#[pyfunction]
fn hessian_str(formula: &str, vars: Vec<String>) -> PyResult<Vec<Vec<String>>> {
    let var_strs: Vec<&str> = vars.iter().map(std::string::String::as_str).collect();
    crate::hessian_str(formula, &var_strs)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))
}

/// Compute the Jacobian matrix of a vector of Expr objects.
///
/// Args:
///     exprs: List of Expr objects (vector function)
///     vars: List of variable names
///
/// Returns:
///     2D list where J[i][j] = ∂fᵢ/∂xⱼ as Expr objects
///
/// For string input, use `jacobian_str()` instead.
// PyO3 requires owned types for Python function arguments
#[allow(clippy::needless_pass_by_value)]
#[pyfunction]
fn jacobian(exprs: Vec<PyExpr>, vars: Vec<String>) -> PyResult<Vec<Vec<PyExpr>>> {
    let rust_exprs: Vec<RustExpr> = exprs.into_iter().map(|e| e.0).collect();
    let symbols: Vec<RustSymbol> = vars.iter().map(|s| crate::symb(s)).collect();
    let sym_refs: Vec<&RustSymbol> = symbols.iter().collect();

    let res = crate::jacobian(&rust_exprs, &sym_refs)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))?;
    Ok(res
        .into_iter()
        .map(|row| row.into_iter().map(PyExpr).collect())
        .collect())
}

/// Compute the Jacobian matrix of a vector function from strings.
///
/// Args:
///     formulas: List of string formulas (vector function)
///     vars: List of variable names
///
/// Returns:
///     2D list where J[i][j] = ∂fᵢ/∂xⱼ as strings
// PyO3 requires owned types for Python function arguments
#[allow(clippy::needless_pass_by_value)]
#[pyfunction]
fn jacobian_str(formulas: Vec<String>, vars: Vec<String>) -> PyResult<Vec<Vec<String>>> {
    let f_strs: Vec<&str> = formulas.iter().map(std::string::String::as_str).collect();
    let var_strs: Vec<&str> = vars.iter().map(std::string::String::as_str).collect();
    crate::jacobian_str(&f_strs, &var_strs)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))
}

/// Evaluate an Expr with given variable values.
///
/// Args:
///     expr: Expr object to evaluate
///     vars: List of (name, value) tuples
///
/// Returns:
///     Evaluated Expr (may be numeric or symbolic if variables remain)
///
/// For string input, use `evaluate_str()` instead.
// PyO3 requires owned types for Python function arguments
#[allow(clippy::needless_pass_by_value)]
#[pyfunction]
fn evaluate(expr: PyExpr, vars: Vec<(String, f64)>) -> PyExpr {
    let var_map: HashMap<&str, f64> = vars.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    let empty_funcs = HashMap::new();
    let res = expr.0.evaluate(&var_map, &empty_funcs);
    PyExpr(res)
}

/// Evaluate a string expression with given variable values.
///
/// Args:
///     formula: Expression string
///     vars: List of (name, value) tuples
///
/// Returns:
///     Evaluated expression as string
// PyO3 requires owned types for Python function arguments
#[allow(clippy::needless_pass_by_value)]
#[pyfunction]
fn evaluate_str(formula: &str, vars: Vec<(String, f64)>) -> PyResult<String> {
    let var_tuples: Vec<(&str, f64)> = vars.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    crate::evaluate_str(formula, &var_tuples)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))
}

/// Compute uncertainty propagation for an expression.
///
/// Args:
///     formula: Expression string or Expr object
///     variables: List of variable names to propagate uncertainty for
///     variances: Optional list of variance values (σ²) for each variable.
///                If None, uses symbolic variances `σ_x²`, `σ_y²`, etc.
///
/// Returns:
///     String representation of the uncertainty expression `σ_f`
///
/// Example:
///     >>> `uncertainty_propagation("x` + y", ["x", "y"])
///     "`sqrt(sigma_x^2` + `sigma_y^2`)"
///     >>> `uncertainty_propagation("x` * y", ["x", "y"], [0.01, 0.04])
///     "sqrt(0.04*x^2 + 0.01*y^2)"
// PyO3 requires owned types for Python function arguments
#[allow(clippy::needless_pass_by_value)]
#[pyfunction]
#[pyo3(name = "uncertainty_propagation", signature = (formula, variables, variances=None))]
fn uncertainty_propagation_py(
    formula: &Bound<'_, PyAny>,
    variables: Vec<String>,
    variances: Option<Vec<f64>>,
) -> PyResult<String> {
    // 1. Get the Expr (either parsed from string or extracted directly)
    let expr = if let Ok(s) = formula.extract::<String>() {
        crate::parser::parse(&s, &HashSet::new(), &HashSet::new(), None)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))?
    } else if let Ok(e) = formula.extract::<PyExpr>() {
        e.0
    } else {
        return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Argument `formula` must be str or Expr",
        ));
    };

    let var_strs: Vec<&str> = variables.iter().map(std::string::String::as_str).collect();

    let cov = variances
        .map(|vars| CovarianceMatrix::diagonal(vars.into_iter().map(CovEntry::Num).collect()));

    crate::uncertainty_propagation(&expr, &var_strs, cov.as_ref())
        .map(|e| e.to_string())
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))
}

/// Compute relative uncertainty for an expression.
///
/// Args:
///     formula: Expression string
///     variables: List of variable names
///     variances: Optional list of variance values (σ²) for each variable
///
/// Returns:
///     String representation of `σ_f` / |f|
// PyO3 requires owned types for Python function arguments
#[allow(clippy::needless_pass_by_value)]
#[pyfunction]
#[pyo3(name = "relative_uncertainty", signature = (formula, variables, variances=None))]
fn relative_uncertainty_py(
    formula: &Bound<'_, PyAny>,
    variables: Vec<String>,
    variances: Option<Vec<f64>>,
) -> PyResult<String> {
    // 1. Get the Expr
    let expr = if let Ok(s) = formula.extract::<String>() {
        crate::parser::parse(&s, &HashSet::new(), &HashSet::new(), None)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))?
    } else if let Ok(e) = formula.extract::<PyExpr>() {
        e.0
    } else {
        return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Argument `formula` must be str or Expr",
        ));
    };

    let var_strs: Vec<&str> = variables.iter().map(std::string::String::as_str).collect();

    let cov = variances
        .map(|vars| CovarianceMatrix::diagonal(vars.into_iter().map(CovEntry::Num).collect()));

    crate::relative_uncertainty(&expr, &var_strs, cov.as_ref())
        .map(|e| e.to_string())
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))
}

/// Parallel evaluation of multiple expressions at multiple points.
///
/// Args:
///     expressions: List of expression strings OR Expr objects
///     variables: List of variable name lists, one per expression
///     values: 3D list/array of values: [expr_idx][var_idx][point_idx]
///             Accepts Python lists or `NumPy` arrays (f64)
///             Use None for a value to keep it symbolic (SKIP)
///
/// Returns:
///     2D list of results: [expr_idx][point_idx]
///     - If input was str: returns float (numeric) or str (symbolic)
///     - If input was Expr: returns float (numeric) or Expr (symbolic)
///
/// Example:
///     >>> `evaluate_parallel`(
///     ...     `["x^2", "x + y"]`,
///     ...     `[["x"], ["x", "y"]]`,
///     ...     [[[1.0, 2.0, 3.0]], [[1.0, 2.0], [3.0, 4.0]]]
///     ... )
///     [[1.0, 4.0, 9.0], [4.0, 6.0]]
///
/// Note: Returns hybrid types based on input and result:
/// - Numeric results are always native Python floats
/// - Symbolic results preserve input type (str→str, Expr→Expr)
#[cfg(feature = "parallel")]
#[pyfunction]
#[pyo3(name = "evaluate_parallel")]
fn evaluate_parallel_py(
    py: Python<'_>,
    expressions: Vec<Bound<'_, PyAny>>,
    variables: Vec<Vec<String>>,
    values: Vec<Vec<Bound<'_, PyAny>>>,
) -> PyResult<Vec<Vec<Py<PyAny>>>> {
    // Track which expressions were Expr vs String for output type
    let mut was_expr: Vec<bool> = Vec::with_capacity(expressions.len());

    // Convert expressions to ExprInput, tracking input type
    let exprs: Vec<ExprInput> = expressions
        .into_iter()
        .map(|e| {
            if let Ok(py_expr) = e.extract::<PyExpr>() {
                was_expr.push(true);
                Ok(ExprInput::Parsed(py_expr.0))
            } else if let Ok(s) = e.extract::<String>() {
                was_expr.push(false);
                Ok(ExprInput::String(s))
            } else {
                Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                    "expressions must be str or Expr",
                ))
            }
        })
        .collect::<PyResult<Vec<_>>>()?;

    let vars: Vec<Vec<VarInput>> = variables
        .into_iter()
        .map(|vs| vs.into_iter().map(VarInput::from).collect())
        .collect();

    // Track if all values for each expression are numeric (no None/Skip values)
    let mut is_fully_numeric: Vec<bool> = Vec::with_capacity(values.len());

    // Convert values - supports both lists and NumPy arrays
    let converted_values: Vec<Vec<Vec<Value>>> = values
        .into_iter()
        .map(|expr_vals| {
            let mut expr_is_numeric = true;
            let converted: Vec<Vec<Value>> = expr_vals
                .into_iter()
                .map(|var_vals| {
                    // Try NumPy array first (zero-copy path)
                    if let Ok(arr) = var_vals.extract::<numpy::PyReadonlyArray1<f64>>()
                        && let Ok(slice) = arr.as_slice()
                    {
                        return slice.iter().map(|&n| Value::Num(n)).collect();
                    }
                    // Fallback to Python list with Option<f64>
                    if let Ok(list) = var_vals.extract::<Vec<Option<f64>>>() {
                        return list
                            .into_iter()
                            .map(|v| {
                                // Side-effect in else branch makes map_or_else unsuitable
                                #[allow(clippy::option_if_let_else)]
                                if let Some(n) = v {
                                    Value::Num(n)
                                } else {
                                    expr_is_numeric = false;
                                    Value::Skip
                                }
                            })
                            .collect();
                    }
                    // Try pure f64 list (no None values)
                    if let Ok(list) = var_vals.extract::<Vec<f64>>() {
                        return list.into_iter().map(Value::Num).collect();
                    }
                    // Empty fallback
                    expr_is_numeric = false;
                    vec![]
                })
                .collect();
            is_fully_numeric.push(expr_is_numeric);
            converted
        })
        .collect();

    // Use the hint-based version to skip double-scan
    parallel::evaluate_parallel_with_hint(exprs, vars, converted_values, Some(is_fully_numeric))
        .map(|results| {
            results
                .into_iter()
                .zip(was_expr.iter())
                .map(|(expr_results, &input_was_expr)| {
                    expr_results
                        .into_iter()
                        .map(|r| match r {
                            parallel::EvalResult::String(s) => {
                                // Input was string; Result::map_or_else is better than this pattern
                                #[allow(clippy::option_if_let_else)]
                                if let Ok(n) = s.parse::<f64>() {
                                    // Numeric result → float
                                    n.into_pyobject(py).unwrap().into_any().unbind()
                                } else {
                                    // Symbolic result → str
                                    s.into_pyobject(py).unwrap().into_any().unbind()
                                }
                            }
                            parallel::EvalResult::Expr(e) => {
                                if let crate::ExprKind::Number(n) = &e.kind {
                                    // Numeric result → float
                                    n.into_pyobject(py).unwrap().into_any().unbind()
                                } else if input_was_expr {
                                    // Symbolic result, input was Expr → Expr
                                    PyExpr(e).into_pyobject(py).unwrap().into_any().unbind()
                                } else {
                                    // Symbolic result, input was str → str
                                    e.to_string().into_pyobject(py).unwrap().into_any().unbind()
                                }
                            }
                        })
                        .collect()
                })
                .collect()
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))
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
        Self(symb(name))
    }

    fn __str__(&self) -> String {
        self.0.name().unwrap_or_default()
    }

    fn __repr__(&self) -> String {
        format!("Symbol(\"{}\")", self.0.name().unwrap_or_default())
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        if let Ok(other_sym) = other.extract::<Self>() {
            return self.0 == other_sym.0;
        }
        false
    }

    // u64->isize: Python __hash__ requires isize, wrap is expected
    #[allow(clippy::cast_possible_wrap)]
    const fn __hash__(&self) -> isize {
        self.0.id() as isize
    }

    /// Get the symbol name
    fn name(&self) -> Option<String> {
        self.0.name()
    }

    /// Get the symbol's unique ID
    const fn id(&self) -> u64 {
        self.0.id()
    }

    /// Convert to an expression
    fn to_expr(&self) -> PyExpr {
        PyExpr(self.0.to_expr())
    }

    // Arithmetic operators - accept Expr, Symbol, int, or float
    fn __add__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(self.0.to_expr() + other_expr))
    }

    fn __sub__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(self.0.to_expr() - other_expr))
    }

    fn __mul__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(self.0.to_expr() * other_expr))
    }

    fn __truediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(self.0.to_expr() / other_expr))
    }

    fn __pow__(
        &self,
        other: &Bound<'_, PyAny>,
        _modulo: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(RustExpr::pow_static(self.0.to_expr(), other_expr)))
    }

    fn __radd__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(other_expr + self.0.to_expr()))
    }

    fn __rsub__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(other_expr - self.0.to_expr()))
    }

    fn __rmul__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(other_expr * self.0.to_expr()))
    }

    fn __rtruediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(other_expr / self.0.to_expr()))
    }

    fn __rpow__(
        &self,
        other: &Bound<'_, PyAny>,
        _modulo: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(RustExpr::pow_static(other_expr, self.0.to_expr())))
    }

    fn __neg__(&self) -> PyExpr {
        PyExpr(RustExpr::number(0.0) - self.0.to_expr())
    }

    // Math function methods - convert Symbol to Expr and apply function
    fn sin(&self) -> PyExpr {
        PyExpr(self.0.sin())
    }
    fn cos(&self) -> PyExpr {
        PyExpr(self.0.cos())
    }
    fn tan(&self) -> PyExpr {
        PyExpr(self.0.tan())
    }
    fn cot(&self) -> PyExpr {
        PyExpr(self.0.cot())
    }
    fn sec(&self) -> PyExpr {
        PyExpr(self.0.sec())
    }
    fn csc(&self) -> PyExpr {
        PyExpr(self.0.csc())
    }

    fn asin(&self) -> PyExpr {
        PyExpr(self.0.asin())
    }
    fn acos(&self) -> PyExpr {
        PyExpr(self.0.acos())
    }
    fn atan(&self) -> PyExpr {
        PyExpr(self.0.atan())
    }

    fn sinh(&self) -> PyExpr {
        PyExpr(self.0.sinh())
    }
    fn cosh(&self) -> PyExpr {
        PyExpr(self.0.cosh())
    }
    fn tanh(&self) -> PyExpr {
        PyExpr(self.0.tanh())
    }

    fn asinh(&self) -> PyExpr {
        PyExpr(self.0.asinh())
    }
    fn acosh(&self) -> PyExpr {
        PyExpr(self.0.acosh())
    }
    fn atanh(&self) -> PyExpr {
        PyExpr(self.0.atanh())
    }

    fn exp(&self) -> PyExpr {
        PyExpr(self.0.exp())
    }
    fn ln(&self) -> PyExpr {
        PyExpr(self.0.ln())
    }
    fn log10(&self) -> PyExpr {
        PyExpr(self.0.log10())
    }
    fn log2(&self) -> PyExpr {
        PyExpr(self.0.log2())
    }

    fn sqrt(&self) -> PyExpr {
        PyExpr(self.0.sqrt())
    }
    fn cbrt(&self) -> PyExpr {
        PyExpr(self.0.cbrt())
    }
    fn abs(&self) -> PyExpr {
        PyExpr(self.0.abs())
    }

    fn floor(&self) -> PyExpr {
        PyExpr(self.0.floor())
    }
    fn ceil(&self) -> PyExpr {
        PyExpr(self.0.ceil())
    }
    fn round(&self) -> PyExpr {
        PyExpr(self.0.round())
    }

    fn erf(&self) -> PyExpr {
        PyExpr(self.0.erf())
    }
    fn erfc(&self) -> PyExpr {
        PyExpr(self.0.erfc())
    }
    fn gamma(&self) -> PyExpr {
        PyExpr(self.0.gamma())
    }

    // Additional inverse trig
    fn acot(&self) -> PyExpr {
        PyExpr(self.0.acot())
    }
    fn asec(&self) -> PyExpr {
        PyExpr(self.0.asec())
    }
    fn acsc(&self) -> PyExpr {
        PyExpr(self.0.acsc())
    }

    // Additional hyperbolic
    fn coth(&self) -> PyExpr {
        PyExpr(self.0.coth())
    }
    fn sech(&self) -> PyExpr {
        PyExpr(self.0.sech())
    }
    fn csch(&self) -> PyExpr {
        PyExpr(self.0.csch())
    }

    // Additional inverse hyperbolic
    fn acoth(&self) -> PyExpr {
        PyExpr(self.0.acoth())
    }
    fn asech(&self) -> PyExpr {
        PyExpr(self.0.asech())
    }
    fn acsch(&self) -> PyExpr {
        PyExpr(self.0.acsch())
    }

    // Additional special functions
    fn signum(&self) -> PyExpr {
        PyExpr(self.0.signum())
    }
    fn sinc(&self) -> PyExpr {
        PyExpr(self.0.sinc())
    }
    fn lambertw(&self) -> PyExpr {
        PyExpr(self.0.lambertw())
    }
    fn zeta(&self) -> PyExpr {
        PyExpr(self.0.zeta())
    }
    fn elliptic_k(&self) -> PyExpr {
        PyExpr(self.0.elliptic_k())
    }
    fn elliptic_e(&self) -> PyExpr {
        PyExpr(self.0.elliptic_e())
    }
    fn exp_polar(&self) -> PyExpr {
        PyExpr(self.0.exp_polar())
    }

    // Additional gamma family
    fn digamma(&self) -> PyExpr {
        PyExpr(self.0.digamma())
    }
    fn trigamma(&self) -> PyExpr {
        PyExpr(self.0.trigamma())
    }
    fn tetragamma(&self) -> PyExpr {
        PyExpr(self.0.tetragamma())
    }

    // Multi-argument functions
    /// Logarithm with arbitrary base: log(base, x)
    fn log(&self, base: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let base_expr = extract_to_expr(base)?;
        Ok(PyExpr(self.0.log(base_expr)))
    }

    /// Polygamma function: ψ^(n)(x)
    fn polygamma(&self, n: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let n_expr = extract_to_expr(n)?;
        Ok(PyExpr(self.0.polygamma(n_expr)))
    }

    /// Beta function: B(self, other)
    fn beta(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(self.0.beta(other_expr)))
    }

    /// Bessel function of the first kind: `J_n(x)`
    fn besselj(&self, n: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let n_expr = extract_to_expr(n)?;
        Ok(PyExpr(self.0.besselj(n_expr)))
    }

    /// Bessel function of the second kind: `Y_n(x)`
    fn bessely(&self, n: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let n_expr = extract_to_expr(n)?;
        Ok(PyExpr(self.0.bessely(n_expr)))
    }

    /// Modified Bessel function of the first kind: `I_n(x)`
    fn besseli(&self, n: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let n_expr = extract_to_expr(n)?;
        Ok(PyExpr(self.0.besseli(n_expr)))
    }

    /// Modified Bessel function of the second kind: `K_n(x)`
    fn besselk(&self, n: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let n_expr = extract_to_expr(n)?;
        Ok(PyExpr(self.0.besselk(n_expr)))
    }

    /// Two-argument arctangent: atan2(self, x) = angle to point (x, self)
    fn atan2(&self, x: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let x_expr = extract_to_expr(x)?;
        Ok(PyExpr(RustExpr::func_multi(
            "atan2",
            vec![self.0.to_expr(), x_expr],
        )))
    }

    /// Hermite polynomial `H_n(self)`
    fn hermite(&self, n: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let n_expr = extract_to_expr(n)?;
        Ok(PyExpr(RustExpr::func_multi(
            "hermite",
            vec![n_expr, self.0.to_expr()],
        )))
    }

    /// Associated Legendre polynomial `P_l^m(self)`
    fn assoc_legendre(&self, l: &Bound<'_, PyAny>, m: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let l_expr = extract_to_expr(l)?;
        let m_expr = extract_to_expr(m)?;
        Ok(PyExpr(RustExpr::func_multi(
            "assoc_legendre",
            vec![l_expr, m_expr, self.0.to_expr()],
        )))
    }

    /// Spherical harmonic `Y_l^m(theta`, phi) where self is theta
    fn spherical_harmonic(
        &self,
        l: &Bound<'_, PyAny>,
        m: &Bound<'_, PyAny>,
        phi: &Bound<'_, PyAny>,
    ) -> PyResult<PyExpr> {
        let l_expr = extract_to_expr(l)?;
        let m_expr = extract_to_expr(m)?;
        let phi_expr = extract_to_expr(phi)?;
        Ok(PyExpr(RustExpr::func_multi(
            "spherical_harmonic",
            vec![l_expr, m_expr, self.0.to_expr(), phi_expr],
        )))
    }

    /// Alternative spherical harmonic notation `Y_l^m(theta`, phi)
    fn ynm(
        &self,
        l: &Bound<'_, PyAny>,
        m: &Bound<'_, PyAny>,
        phi: &Bound<'_, PyAny>,
    ) -> PyResult<PyExpr> {
        let l_expr = extract_to_expr(l)?;
        let m_expr = extract_to_expr(m)?;
        let phi_expr = extract_to_expr(phi)?;
        Ok(PyExpr(RustExpr::func_multi(
            "ynm",
            vec![l_expr, m_expr, self.0.to_expr(), phi_expr],
        )))
    }

    /// Derivative of Riemann zeta function: zeta^(n)(self)
    fn zeta_deriv(&self, n: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let n_expr = extract_to_expr(n)?;
        Ok(PyExpr(RustExpr::func_multi(
            "zeta_deriv",
            vec![n_expr, self.0.to_expr()],
        )))
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
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))
}

/// Get an existing symbol (fails if not found)
#[pyfunction]
#[pyo3(name = "symb_get")]
fn py_symb_get(name: &str) -> PyResult<PySymbol> {
    symb_get(name)
        .map(PySymbol)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))
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
    clear_symbols();
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
    /// Compile an expression with specified parameter order and optional context.
    // PyO3 requires owned types; if-let pattern clearer than map_or_else here
    #[allow(clippy::needless_pass_by_value, clippy::option_if_let_else)]
    #[new]
    #[pyo3(signature = (expr, params=None, context=None))]
    fn new(
        expr: &PyExpr,
        params: Option<Vec<String>>,
        context: Option<&PyContext>,
    ) -> PyResult<Self> {
        let param_refs: Vec<&str>;
        let rust_context = context.map(|c| &c.inner);

        let evaluator = if let Some(p) = &params {
            param_refs = p.iter().map(std::string::String::as_str).collect();
            CompiledEvaluator::compile(&expr.0, &param_refs, rust_context)
        } else {
            CompiledEvaluator::compile_auto(&expr.0, rust_context)
        };

        evaluator
            .map(|e| Self { evaluator: e })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))
    }

    /// Evaluate at a single point
    /// Evaluate at a single point (Zero-Copy `NumPy` or List)
    // PyO3 requires Bound<'_, PyAny> by value for flexible input types
    #[allow(clippy::needless_pass_by_value)]
    fn evaluate(&self, input: Bound<'_, PyAny>) -> PyResult<f64> {
        let data = extract_data_input(&input)?;
        let slice = data.as_slice()?;
        Ok(self.evaluator.evaluate(slice))
    }

    /// Batch evaluate at multiple points (columnar data)
    /// columns[`var_idx`][point_idx] -> f64
    ///
    /// Accepts `NumPy` arrays (Zero-Copy) or Python Lists (fallback).
    // PyO3 requires owned Vec for Python list arguments
    #[allow(clippy::needless_pass_by_value)]
    fn eval_batch<'py>(
        &self,
        py: Python<'py>,
        columns: Vec<Bound<'py, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let inputs: Vec<DataInput> = columns
            .iter()
            .map(|c| extract_data_input(c))
            .collect::<PyResult<Vec<_>>>()?;

        let n_points = if inputs.is_empty() {
            1
        } else {
            inputs[0].len()?
        };

        // Check if any input is a NumPy array to decide output format
        let use_numpy = inputs.iter().any(|d| matches!(d, DataInput::Array(_)));

        // Zero-copy access (or slice from vec)
        let col_refs: Vec<&[f64]> = inputs
            .iter()
            .map(|c| c.as_slice())
            .collect::<PyResult<Vec<_>>>()?;

        let mut output = vec![0.0; n_points];
        self.evaluator
            .eval_batch(&col_refs, &mut output, None)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))?;

        if use_numpy {
            // Return NumPy array (this copies the result, but input was zero-copy)
            Ok(PyArray1::from_vec(py, output).into_any().unbind())
        } else {
            // Return standard Python list
            Ok(output.into_pyobject(py).unwrap().into_any().unbind())
        }
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
    const fn stack_size(&self) -> usize {
        self.evaluator.stack_size()
    }
}

// ============================================================================
// FunctionContext for custom function registration
// ============================================================================

/// Custom function context for tracking custom function names.
///
/// This tracks custom function names for parsing. For full custom function
/// support with evaluation and derivatives, use the `Diff.custom_derivative` API.
#[pyclass(unsendable, name = "FunctionContext")]
struct PyFunctionContext {
    /// Set of registered custom function names
    functions: std::collections::HashSet<String>,
}

#[pymethods]
impl PyFunctionContext {
    #[new]
    fn new() -> Self {
        Self {
            functions: std::collections::HashSet::new(),
        }
    }

    /// Register a custom function name (for parsing support).
    ///
    /// For actual evaluation/differentiation, use `Diff.custom_derivative()`.
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
        self.functions.clear();
    }
}

// ============================================================================
/// Unified Context for namespace isolation and function registry.
///
/// Use this to create isolated environments for symbols and functions.
///
/// Example:
///     ctx = `Context()`
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
        Self {
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
                "Symbol '{name}' already exists in this context"
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
    const fn id(&self) -> u64 {
        self.inner.id()
    }

    /// Remove a symbol from the context
    fn remove_symbol(&mut self, name: &str) -> bool {
        self.inner.remove_symbol(name)
    }

    /// Clear all symbols and functions from the context
    fn clear_all(&mut self) {
        self.inner.clear_all();
    }

    /// Register a user function with optional body and partial derivatives.
    // PyO3 requires owned String for name parameter
    #[allow(clippy::needless_pass_by_value)]
    #[pyo3(signature = (name, arity, body_callback=None, partials=None))]
    fn with_function(
        mut self_: PyRefMut<'_, Self>,
        name: String,
        arity: usize,
        body_callback: Option<Py<PyAny>>,
        partials: Option<Vec<Py<PyAny>>>,
    ) -> PyResult<PyRefMut<'_, Self>> {
        use crate::core::unified_context::UserFunction;
        use std::sync::Arc;

        let mut user_fn = UserFunction::new(arity..=arity);

        // Handle optional body function
        if let Some(callback) = body_callback {
            let body_fn: BodyFn = Arc::new(move |args: &[Arc<RustExpr>]| -> RustExpr {
                Python::attach(|py| {
                    let py_args: Vec<PyExpr> = args.iter().map(|a| PyExpr((**a).clone())).collect();
                    let Ok(py_list) = py_args.into_pyobject(py) else {
                        return RustExpr::number(0.0);
                    };

                    let result = callback.call1(py, (py_list,));

                    result.map_or_else(
                        |_| RustExpr::number(0.0),
                        |res| {
                            res.extract::<PyExpr>(py)
                                .map_or_else(|_| RustExpr::number(0.0), |py_expr| py_expr.0)
                        },
                    )
                })
            });
            user_fn = user_fn.body_arc(body_fn);
        }

        // Handle optional list of partial derivatives
        if let Some(callbacks) = partials {
            for (i, callback) in callbacks.into_iter().enumerate() {
                let partial_fn: PartialDerivativeFn =
                    Arc::new(move |args: &[Arc<RustExpr>]| -> RustExpr {
                        Python::attach(|py| {
                            let py_args: Vec<PyExpr> =
                                args.iter().map(|a| PyExpr((**a).clone())).collect();
                            let Ok(py_list) = py_args.into_pyobject(py) else {
                                return RustExpr::number(0.0);
                            };

                            let result = callback.call1(py, (py_list,));

                            result.map_or_else(
                                |_| RustExpr::number(0.0),
                                |res| {
                                    res.extract::<PyExpr>(py)
                                        .map_or_else(|_| RustExpr::number(0.0), |py_expr| py_expr.0)
                                },
                            )
                        })
                    });

                user_fn = user_fn.partial_arc(i, partial_fn).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}"))
                })?;
            }
        }

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
///     `var_names`: List of variable name lists, one per expression
///     data: 3D list of values: data[expr_idx][var_idx] = [values...]
///
/// Returns:
///     2D list of results: result[expr_idx][point_idx]
#[cfg(feature = "parallel")]
// PyO3 requires owned Vec for Python list arguments
#[allow(clippy::needless_pass_by_value)]
#[pyfunction]
#[pyo3(name = "eval_f64")]
fn eval_f64_py<'py>(
    py: Python<'py>,
    expressions: Vec<PyExpr>,
    var_names: Vec<Vec<String>>,
    data: Vec<Vec<Bound<'py, PyAny>>>,
) -> PyResult<Py<PyAny>> {
    let expr_refs: Vec<&RustExpr> = expressions.iter().map(|e| &e.0).collect();

    // Convert var_names to the required format
    let var_refs: Vec<Vec<&str>> = var_names
        .iter()
        .map(|vs| vs.iter().map(std::string::String::as_str).collect())
        .collect();
    let var_slice_refs: Vec<&[&str]> = var_refs.iter().map(std::vec::Vec::as_slice).collect();

    // Convert data to the required format (Zero-Copy or List)
    // We need to keep DataInput alive to hold the borrows/ownership
    let data_inputs: Vec<Vec<DataInput>> = data
        .iter()
        .map(|expr_data| {
            expr_data
                .iter()
                .map(|col| extract_data_input(col))
                .collect::<PyResult<Vec<_>>>()
        })
        .collect::<PyResult<Vec<_>>>()?;

    // Check if any input is a NumPy array to decide output format
    let use_numpy = data_inputs
        .iter()
        .flat_map(|row| row.iter())
        .any(|d| matches!(d, DataInput::Array(_)));

    let data_refs: Vec<Vec<&[f64]>> = data_inputs
        .iter()
        .map(|expr_data| {
            expr_data
                .iter()
                .map(|col| col.as_slice())
                .collect::<PyResult<Vec<_>>>()
        })
        .collect::<PyResult<Vec<_>>>()?;
    let data_slice_refs: Vec<&[&[f64]]> = data_refs.iter().map(std::vec::Vec::as_slice).collect();

    let results = rust_eval_f64(&expr_refs, &var_slice_refs, &data_slice_refs)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}")))?;

    if use_numpy {
        // Convert Vec<Vec<f64>> to 2D NumPy array
        // Dimensions: [n_exprs, n_points]
        if results.is_empty() {
            return Ok(PyArray2::<f64>::zeros(py, [0, 0], false)
                .into_any()
                .unbind());
        }

        let n_exprs = results.len();
        let n_points = results[0].len();

        // Flatten the results into a single contiguous vector
        let flat_results: Vec<f64> = results.into_iter().flatten().collect();

        // Create the 2D array from the valid vector
        let arr = PyArray1::from_vec(py, flat_results)
            .reshape([n_exprs, n_points])
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Failed to reshape output: {e:?}"
                ))
            })?;

        Ok(arr.into_any().unbind())
    } else {
        // Return standard Python list of lists
        Ok(results.into_pyobject(py).unwrap().into_any().unbind())
    }
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

    // Core functions (string API)
    m.add_function(wrap_pyfunction!(diff, m)?)?;
    m.add_function(wrap_pyfunction!(simplify, m)?)?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_str, m)?)?;

    // Multi-variable calculus (Expr API)
    m.add_function(wrap_pyfunction!(gradient, m)?)?;
    m.add_function(wrap_pyfunction!(hessian, m)?)?;
    m.add_function(wrap_pyfunction!(jacobian, m)?)?;

    // Multi-variable calculus (string API)
    m.add_function(wrap_pyfunction!(gradient_str, m)?)?;
    m.add_function(wrap_pyfunction!(hessian_str, m)?)?;
    m.add_function(wrap_pyfunction!(jacobian_str, m)?)?;

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
