//! Python bindings for Dual numbers
//!
//! This module provides the `PyDual` class which wraps Rust's `Dual` type
//! for automatic differentiation.

use crate::Dual;
use num_traits::Float;
use pyo3::prelude::*;

/// Wrapper for Rust Dual number for automatic differentiation
#[pyclass(name = "Dual")]
#[derive(Clone, Copy)]
pub struct PyDual(pub Dual<f64>);

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
    pub const fn new(val: f64, eps: f64) -> Self {
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
    pub fn constant(val: f64) -> Self {
        Self(Dual::constant(val))
    }

    /// Get the real value component
    #[getter]
    pub const fn val(&self) -> f64 {
        self.0.val
    }

    /// Get the infinitesimal derivative component
    #[getter]
    pub const fn eps(&self) -> f64 {
        self.0.eps
    }

    fn __str__(&self) -> String {
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
    fn asinh(&self) -> Self {
        Self(self.0.asinh())
    }
    fn acosh(&self) -> Self {
        Self(self.0.acosh())
    }
    fn atanh(&self) -> Self {
        Self(self.0.atanh())
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
    fn powf(&self, n: f64) -> Self {
        Self(self.0.powf(Dual::constant(n)))
    }
    fn powi(&self, n: i32) -> Self {
        Self(self.0.powi(n))
    }

    // Special functions
    fn erf(&self) -> Self {
        Self(self.0.erf())
    }
    fn erfc(&self) -> Self {
        Self(self.0.erfc())
    }
    fn gamma(&self) -> PyResult<Self> {
        self.0.gamma().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Gamma function undefined")
        })
    }
    fn digamma(&self) -> PyResult<Self> {
        self.0.digamma().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Digamma function undefined")
        })
    }
    fn trigamma(&self) -> PyResult<Self> {
        self.0.trigamma().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Trigamma function undefined")
        })
    }
    fn polygamma(&self, n: i32) -> PyResult<Self> {
        self.0.polygamma(n).map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Polygamma function undefined")
        })
    }
    fn zeta(&self) -> PyResult<Self> {
        self.0.zeta().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Zeta function undefined")
        })
    }
    fn lambert_w(&self) -> PyResult<Self> {
        self.0.lambert_w().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Lambert W function undefined")
        })
    }
    fn bessel_j(&self, n: i32) -> PyResult<Self> {
        self.0.bessel_j(n).map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Bessel J function undefined")
        })
    }
    fn sinc(&self) -> Self {
        Self(self.0.sinc())
    }
    fn elliptic_k(&self) -> PyResult<Self> {
        self.0.elliptic_k().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Elliptic K function undefined")
        })
    }
    fn elliptic_e(&self) -> PyResult<Self> {
        self.0.elliptic_e().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Elliptic E function undefined")
        })
    }
    fn beta(&self, b: &Self) -> PyResult<Self> {
        self.0.beta(b.0).map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Beta function undefined")
        })
    }
}
