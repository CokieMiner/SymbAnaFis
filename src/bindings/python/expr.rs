//! Python bindings for symbolic expressions
//!
//! This module provides the `PyExpr` class which wraps Rust's `Expr` type
//! for Python interoperability.

// Allow: PyO3 from_py_object macro generates FromPyObject with short lifetime names
#![allow(
    clippy::single_char_lifetime_names,
    reason = "PyO3 from_py_object macro generates lifetimes"
)]

use crate::Expr as RustExpr;
use crate::core::traits::EPSILON;
use pyo3::prelude::*;

/// Python wrapper for symbolic expressions
#[pyclass(unsendable, name = "Expr", from_py_object)]
#[derive(Clone)]
pub struct PyExpr(pub RustExpr);

#[pymethods]
impl PyExpr {
    /// Create a symbolic expression or numeric constant
    #[new]
    fn new(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self(extract_to_expr(value)?))
    }

    /// String representation of the expression
    fn __str__(&self) -> String {
        self.0.to_string()
    }

    /// Python repr of the expression
    fn __repr__(&self) -> String {
        format!("Expr({})", self.0)
    }

    /// Equality comparison
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        if let Ok(other_expr) = other.extract::<Self>() {
            return self.0 == other_expr.0;
        }
        if let Ok(other_sym) = other.extract::<super::symbol::PySymbol>() {
            return self.0 == other_sym.0.to_expr();
        }
        false
    }

    /// Hash function for use in sets and dictionaries
    fn __hash__(&self) -> isize {
        // Simple hash based on string representation or kind
        // For better performance, we'd need a stable hash in Rust Expr
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut s = DefaultHasher::new();
        self.0.to_string().hash(&mut s);
        #[allow(
            clippy::cast_possible_wrap,
            reason = "Python __hash__ requires isize, wrapping is expected"
        )]
        {
            // u64->isize: Python __hash__ requires isize, allow truncation
            let hash = s.finish();
            #[allow(
                clippy::cast_possible_truncation,
                reason = "Python __hash__ requires isize, truncation is expected"
            )]
            // Python __hash__ requires isize, truncation is expected
            let res = hash as isize;
            res
        }
    }

    /// Convert to float if the expression is a numeric constant
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
    /// Addition operator
    fn __add__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        let other_expr = extract_to_expr(other)?;
        Ok(Self(self.0.clone() + other_expr))
    }

    /// Subtraction operator
    fn __sub__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        let other_expr = extract_to_expr(other)?;
        Ok(Self(self.0.clone() - other_expr))
    }

    /// Multiplication operator
    fn __mul__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        let other_expr = extract_to_expr(other)?;
        Ok(Self(self.0.clone() * other_expr))
    }

    /// Division operator
    fn __truediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        let other_expr = extract_to_expr(other)?;
        Ok(Self(self.0.clone() / other_expr))
    }

    /// Power operator
    fn __pow__(&self, other: &Bound<'_, PyAny>, _modulo: Option<Py<PyAny>>) -> PyResult<Self> {
        let other_expr = extract_to_expr(other)?;
        Ok(Self(self.0.clone().pow(other_expr)))
    }

    // Reverse power: 2 ** x where x is Expr
    /// Reverse power operator
    fn __rpow__(&self, other: &Bound<'_, PyAny>, _modulo: Option<Py<PyAny>>) -> PyResult<Self> {
        // other ** self (other is the base, self is the exponent)
        if let Ok(n) = other.extract::<f64>() {
            return Ok(Self(RustExpr::number(n).pow(self.0.clone())));
        }
        if let Ok(n) = other.extract::<i64>() {
            // i64->f64 intentional: Python integers map naturally to floats
            #[allow(
                clippy::cast_precision_loss,
                reason = "i64→f64: Python integers map naturally to floats"
            )]
            // i64→f64: Python integers map naturally to floats
            return Ok(Self(RustExpr::number(n as f64).pow(self.0.clone())));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "rpow() base must be int or float",
        ))
    }

    /// Negation operator
    fn __neg__(&self) -> Self {
        Self(RustExpr::number(0.0) - self.0.clone())
    }

    /// Reverse addition operator
    fn __radd__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(n) = other.extract::<f64>() {
            return Ok(Self(RustExpr::number(n) + self.0.clone()));
        }
        if let Ok(n) = other.extract::<i64>() {
            // i64->f64 intentional: Python integers map naturally to floats
            #[allow(
                clippy::cast_precision_loss,
                reason = "i64→f64: Python integers map naturally to floats"
            )]
            // i64→f64: Python integers map naturally to floats
            return Ok(Self(RustExpr::number(n as f64) + self.0.clone()));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be int or float",
        ))
    }

    /// Reverse subtraction operator
    fn __rsub__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(n) = other.extract::<f64>() {
            return Ok(Self(RustExpr::number(n) - self.0.clone()));
        }
        if let Ok(n) = other.extract::<i64>() {
            // i64->f64 intentional: Python integers map naturally to floats
            #[allow(
                clippy::cast_precision_loss,
                reason = "i64→f64: Python integers map naturally to floats"
            )]
            // i64→f64: Python integers map naturally to floats
            return Ok(Self(RustExpr::number(n as f64) - self.0.clone()));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be int or float",
        ))
    }

    /// Reverse multiplication operator
    fn __rmul__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(n) = other.extract::<f64>() {
            return Ok(Self(RustExpr::number(n) * self.0.clone()));
        }
        if let Ok(n) = other.extract::<i64>() {
            // i64->f64 intentional: Python integers map naturally to floats
            #[allow(
                clippy::cast_precision_loss,
                reason = "i64→f64: Python integers map naturally to floats"
            )]
            // i64→f64: Python integers map naturally to floats
            return Ok(Self(RustExpr::number(n as f64) * self.0.clone()));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be int or float",
        ))
    }

    /// Reverse division operator
    fn __rtruediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(n) = other.extract::<f64>() {
            return Ok(Self(RustExpr::number(n) / self.0.clone()));
        }
        if let Ok(n) = other.extract::<i64>() {
            // i64->f64 intentional: Python integers map naturally to floats
            #[allow(
                clippy::cast_precision_loss,
                reason = "i64→f64: Python integers map naturally to floats"
            )]
            // i64→f64: Python integers map naturally to floats
            return Ok(Self(RustExpr::number(n as f64) / self.0.clone()));
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Operand must be int or float",
        ))
    }

    // Functions
    /// Sine function
    fn sin(&self) -> Self {
        Self(self.0.clone().sin())
    }
    /// Cosine function
    fn cos(&self) -> Self {
        Self(self.0.clone().cos())
    }
    /// Tangent function
    fn tan(&self) -> Self {
        Self(self.0.clone().tan())
    }
    /// Cotangent function
    fn cot(&self) -> Self {
        Self(self.0.clone().cot())
    }
    /// Secant function
    fn sec(&self) -> Self {
        Self(self.0.clone().sec())
    }
    /// Cosecant function
    fn csc(&self) -> Self {
        Self(self.0.clone().csc())
    }

    /// Inverse sine function
    fn asin(&self) -> Self {
        Self(self.0.clone().asin())
    }
    /// Inverse cosine function
    fn acos(&self) -> Self {
        Self(self.0.clone().acos())
    }
    /// Inverse tangent function
    fn atan(&self) -> Self {
        Self(self.0.clone().atan())
    }
    /// Inverse cotangent function
    fn acot(&self) -> Self {
        Self(self.0.clone().acot())
    }
    /// Inverse secant function
    fn asec(&self) -> Self {
        Self(self.0.clone().asec())
    }
    /// Inverse cosecant function
    fn acsc(&self) -> Self {
        Self(self.0.clone().acsc())
    }

    /// Hyperbolic sine function
    fn sinh(&self) -> Self {
        Self(self.0.clone().sinh())
    }
    /// Hyperbolic cosine function
    fn cosh(&self) -> Self {
        Self(self.0.clone().cosh())
    }
    /// Hyperbolic tangent function
    fn tanh(&self) -> Self {
        Self(self.0.clone().tanh())
    }
    /// Hyperbolic cotangent function
    fn coth(&self) -> Self {
        Self(self.0.clone().coth())
    }
    /// Hyperbolic secant function
    fn sech(&self) -> Self {
        Self(self.0.clone().sech())
    }
    /// Hyperbolic cosecant function
    fn csch(&self) -> Self {
        Self(self.0.clone().csch())
    }

    /// Inverse hyperbolic sine function
    fn asinh(&self) -> Self {
        Self(self.0.clone().asinh())
    }
    /// Inverse hyperbolic cosine function
    fn acosh(&self) -> Self {
        Self(self.0.clone().acosh())
    }
    /// Inverse hyperbolic tangent function
    fn atanh(&self) -> Self {
        Self(self.0.clone().atanh())
    }
    /// Inverse hyperbolic cotangent function
    fn acoth(&self) -> Self {
        Self(self.0.clone().acoth())
    }
    /// Inverse hyperbolic secant function
    fn asech(&self) -> Self {
        Self(self.0.clone().asech())
    }
    /// Inverse hyperbolic cosecant function
    fn acsch(&self) -> Self {
        Self(self.0.clone().acsch())
    }

    /// Exponential function
    fn exp(&self) -> Self {
        Self(self.0.clone().exp())
    }
    /// Natural logarithm
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
    /// Base-10 logarithm
    fn log10(&self) -> Self {
        Self(self.0.clone().log10())
    }
    /// Base-2 logarithm
    fn log2(&self) -> Self {
        Self(self.0.clone().log2())
    }

    /// Square root
    fn sqrt(&self) -> Self {
        Self(self.0.clone().sqrt())
    }
    /// Cube root
    fn cbrt(&self) -> Self {
        Self(self.0.clone().cbrt())
    }

    /// Absolute value
    fn abs(&self) -> Self {
        Self(self.0.clone().abs())
    }
    /// Signum function
    fn signum(&self) -> Self {
        Self(self.0.clone().signum())
    }
    /// Sinc function
    fn sinc(&self) -> Self {
        Self(self.0.clone().sinc())
    }
    /// Error function
    fn erf(&self) -> Self {
        Self(self.0.clone().erf())
    }
    /// Complementary error function
    fn erfc(&self) -> Self {
        Self(self.0.clone().erfc())
    }
    /// Gamma function
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
    /// Digamma function
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
    /// Trigamma function
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
    /// Tetragamma function
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
    /// Floor function
    fn floor(&self) -> Self {
        Self(self.0.clone().floor())
    }
    /// Ceiling function
    fn ceil(&self) -> Self {
        Self(self.0.clone().ceil())
    }
    /// Round to nearest integer
    fn round(&self) -> Self {
        Self(self.0.clone().round())
    }
    /// Complete elliptic integral of the first kind
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
    /// Complete elliptic integral of the second kind
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
    /// Exponential in polar form
    fn exp_polar(&self) -> Self {
        Self(self.0.clone().exp_polar())
    }
    /// Polygamma function
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
    /// Beta function
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
    /// Riemann zeta function
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
    /// Lambert W function
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
    /// Bessel function of the first kind
    fn besselj(&self, n: &Bound<'_, PyAny>) -> PyResult<Self> {
        let n_expr = extract_to_expr(n)?;
        Ok(Self(self.0.clone().besselj(n_expr)))
    }
    /// Bessel function of the second kind
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
    /// Modified Bessel function of the first kind
    fn besseli(&self, n: &Bound<'_, PyAny>) -> PyResult<Self> {
        let n_expr = extract_to_expr(n)?;
        Ok(Self(self.0.clone().besseli(n_expr)))
    }
    /// Modified Bessel function of the second kind
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

    /// Power function
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

    /// Spherical harmonic `Y_l^m(theta, phi)` where self is theta
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
                    "spherical_harmonic({l_val}, m, \u{03b8}, \u{03c6}) undefined: l must be non-negative integer"
                )));
            }
            if let Some(m_val) = get_numeric_value(&m_expr)
                && is_legendre_m_domain_error(l_val, m_val)
            {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "spherical_harmonic({l_val}, {m_val}, \u{03b8}, \u{03c6}) undefined: |m| must be <= l"
                )));
            }
        }
        Ok(Self(RustExpr::func_multi(
            "spherical_harmonic",
            vec![l_expr, m_expr, self.0.clone(), phi_expr],
        )))
    }

    /// Alternative spherical harmonic notation `Y_l^m(theta, phi)`
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
                    "ynm({l_val}, m, \u{03b8}, \u{03c6}) undefined: l must be non-negative integer"
                )));
            }
            if let Some(m_val) = get_numeric_value(&m_expr)
                && is_legendre_m_domain_error(l_val, m_val)
            {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "ynm({l_val}, {m_val}, \u{03b8}, \u{03c6}) undefined: |m| must be <= l"
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

    /// Differentiate this expression
    fn diff(&self, var: &str) -> PyResult<Self> {
        let sym = crate::symb(var);
        crate::Diff::new()
            .differentiate(&self.0, &sym)
            .map(PyExpr)
            .map_err(Into::into)
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
    #[allow(
        clippy::needless_pass_by_value,
        reason = "PyO3 requires owned types for Python dict arguments"
    )]
    fn evaluate(&self, vars: std::collections::HashMap<String, f64>) -> Self {
        let var_map: std::collections::HashMap<&str, f64> =
            vars.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        Self(self.0.evaluate(&var_map, &std::collections::HashMap::new()))
    }

    /// Simplify this expression
    fn simplify(&self) -> PyResult<Self> {
        crate::Simplify::new()
            .simplify(&self.0)
            .map(PyExpr)
            .map_err(Into::into)
    }
    /// Get a pattern-matchable view of the expression structure
    ///
    /// Returns an `ExprView` that allows inspection of the expression's structure
    /// without exposing internal implementation details. Internal polynomial
    /// optimizations are transparently presented as Sum nodes.
    ///
    /// Returns:
    ///     `ExprView` object with:
    ///     - kind: Type of node ("Number", "Symbol", "Sum", "Product", "Div", "Pow", "Function", "Derivative")
    ///     - value: Numeric value (for Number nodes)
    ///     - name: Name string (for Symbol/Function nodes)
    ///     - children: List of child expressions
    ///     - `derivative_var`: Variable name (for Derivative nodes)
    ///     - `derivative_order`: Order (for Derivative nodes)
    ///
    /// Example:
    ///     >>> x = symb("x")
    ///     >>> expr = x**2 + 2*x + 1
    ///     >>> view = `expr.view()`
    ///     >>> view.kind
    ///     'Sum'
    ///     >>> len(view.children)
    ///     3
    ///     >>> view.children[0]
    ///     Expr(1)
    fn view(&self) -> super::visitor::PyExprView {
        super::visitor::PyExprView::from_view(self.0.view())
    }
}

// Helper functions: use the library-wide EPSILON tolerance for comparisons.
// This favors consistent, stable floating-point checks while keeping
// behavior performant and predictable for end users.
/// Check if value is a pole of the gamma function
fn is_gamma_pole(n: f64) -> bool {
    n <= 0.0 && (n - n.round()).abs() < EPSILON
}

/// Check if value is a pole of the zeta function
fn is_zeta_pole(s: f64) -> bool {
    (s - 1.0).abs() < EPSILON
}

/// Check if value is outside the domain of Lambert W function
fn is_lambert_w_domain_error(x: f64) -> bool {
    x < -1.0 / std::f64::consts::E - EPSILON
}

/// Check if value is outside the domain of Bessel Y and K functions
fn is_bessel_yk_domain_error(x: f64) -> bool {
    x <= 0.0 + EPSILON
}

/// Check if value is outside the domain of elliptic K function
fn is_elliptic_k_domain_error(k: f64) -> bool {
    k.abs() >= 1.0 - EPSILON
}

/// Check if value is outside the domain of elliptic E function
fn is_elliptic_e_domain_error(k: f64) -> bool {
    k.abs() > 1.0 + EPSILON
}

/// Check if value is invalid for Hermite polynomial order
fn is_hermite_domain_error(n: f64) -> bool {
    n < 0.0 || (n - n.round()).abs() > EPSILON
}

/// Check if value is outside the domain of associated Legendre functions
fn is_assoc_legendre_x_domain_error(x: f64) -> bool {
    x.abs() > 1.0 + EPSILON
}

/// Check if value is invalid for Legendre polynomial degree
fn is_legendre_l_domain_error(l: f64) -> bool {
    l < 0.0 || (l - l.round()).abs() > EPSILON
}

/// Check if values are invalid for Legendre polynomial degree and order
const fn is_legendre_m_domain_error(l: f64, m: f64) -> bool {
    m.abs() > l
}

/// Checks if the polygamma order is invalid (negative or non-integer).
fn is_polygamma_order_domain_error(n: f64) -> bool {
    n < 0.0 || (n - n.round()).abs() > EPSILON
}

/// Checks if the logarithm base is invalid (non-positive or 1).
fn is_log_base_domain_error(base: f64) -> bool {
    base <= 0.0 || (base - 1.0).abs() < EPSILON
}

/// Checks if the logarithm value is invalid (non-positive).
fn is_log_value_domain_error(x: f64) -> bool {
    x <= 0.0 + EPSILON
}

/// Extracts the numeric value from an expression if it is a number.
const fn get_numeric_value(expr: &RustExpr) -> Option<f64> {
    if let crate::ExprKind::Number(n) = &expr.kind {
        Some(*n)
    } else {
        None
    }
}

// Complex if-let chain is clearer than map_or_else for multi-type extraction
#[allow(
    clippy::option_if_let_else,
    reason = "Complex if-let chain is clearer than map_or_else for multi-type extraction"
)]
/// Extracts a Rust expression from a Python object.
pub fn extract_to_expr(value: &Bound<'_, PyAny>) -> PyResult<RustExpr> {
    // Check existing expressions first
    if let Ok(expr) = value.extract::<PyExpr>() {
        return Ok(expr.0);
    }
    if let Ok(sym) = value.extract::<super::symbol::PySymbol>() {
        return Ok(sym.0.to_expr());
    }
    // Numbers
    if let Ok(n) = value.extract::<f64>() {
        return Ok(RustExpr::number(n));
    }
    if let Ok(n) = value.extract::<i64>() {
        // i64->f64 intentional: Python integers map naturally to floats
        #[allow(
            clippy::cast_precision_loss,
            reason = "i64->f64 intentional: Python integers map naturally to floats"
        )]
        return Ok(RustExpr::number(n as f64));
    }
    // Strings are strictly treated as symbols in this constructor path.
    // If the user wants to parse a formula, they should use the global parse() function.
    if let Ok(s) = value.extract::<String>() {
        return Ok(crate::symb(&s).into());
    }
    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
        "Argument must be Expr, Symbol, int, float, or string",
    ))
}
