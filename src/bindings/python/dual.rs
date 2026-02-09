//! Python bindings for Dual numbers
//!
//! This module provides the `PyDual` class which wraps Rust's `Dual` type
//! for automatic differentiation.

// Allow: PyO3 from_py_object macro generates FromPyObject with short lifetime names
#![allow(
    clippy::single_char_lifetime_names,
    reason = "PyO3 from_py_object macro generates lifetimes"
)]

use crate::Dual;
use num_traits::Float;
use pyo3::prelude::*;

/// Wrapper for Rust Dual number for automatic differentiation
#[pyclass(name = "Dual", from_py_object)]
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

    /// Get string representation of the dual number
    fn __str__(&self) -> String {
        self.0.to_string()
    }

    /// Get detailed string representation of the dual number
    fn __repr__(&self) -> String {
        format!("Dual({}, {})", self.0.val, self.0.eps)
    }

    /// Add two dual numbers or a dual number and a scalar
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

    /// Subtract two dual numbers or a dual number and a scalar
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

    /// Multiply two dual numbers or a dual number and a scalar
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

    /// Divide two dual numbers or a dual number and a scalar
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

    /// Negate a dual number
    fn __neg__(&self) -> Self {
        Self(-self.0)
    }

    /// Reverse add (for commutative addition)
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

    /// Reverse subtract
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

    /// Reverse multiply (for commutative multiplication)
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

    /// Reverse divide
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

    /// Raise a dual number to a power
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

    /// Reverse power (for commutative exponentiation)
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
    /// Compute the sine of a dual number
    fn sin(&self) -> Self {
        Self(self.0.sin())
    }
    /// Compute the cosine of a dual number
    fn cos(&self) -> Self {
        Self(self.0.cos())
    }
    /// Compute the tangent of a dual number
    fn tan(&self) -> Self {
        Self(self.0.tan())
    }
    /// Compute the inverse sine of a dual number
    fn asin(&self) -> Self {
        Self(self.0.asin())
    }
    /// Compute the inverse cosine of a dual number
    fn acos(&self) -> Self {
        Self(self.0.acos())
    }
    /// Compute the inverse tangent of a dual number
    fn atan(&self) -> Self {
        Self(self.0.atan())
    }
    /// Compute the hyperbolic sine of a dual number
    fn sinh(&self) -> Self {
        Self(self.0.sinh())
    }
    /// Compute the hyperbolic cosine of a dual number
    fn cosh(&self) -> Self {
        Self(self.0.cosh())
    }
    /// Compute the hyperbolic tangent of a dual number
    fn tanh(&self) -> Self {
        Self(self.0.tanh())
    }
    /// Compute the inverse hyperbolic sine of a dual number
    fn asinh(&self) -> Self {
        Self(self.0.asinh())
    }
    /// Compute the inverse hyperbolic cosine of a dual number
    fn acosh(&self) -> Self {
        Self(self.0.acosh())
    }
    /// Compute the inverse hyperbolic tangent of a dual number
    fn atanh(&self) -> Self {
        Self(self.0.atanh())
    }
    /// Exponential function
    fn exp(&self) -> Self {
        Self(self.0.exp())
    }
    /// Base-2 exponential function
    fn exp2(&self) -> Self {
        Self(self.0.exp2())
    }
    /// Natural logarithm
    fn ln(&self) -> Self {
        Self(self.0.ln())
    }
    /// Base-2 logarithm
    fn log2(&self) -> Self {
        Self(self.0.log2())
    }
    /// Base-10 logarithm
    fn log10(&self) -> Self {
        Self(self.0.log10())
    }
    /// Square root
    fn sqrt(&self) -> Self {
        Self(self.0.sqrt())
    }
    /// Cube root
    fn cbrt(&self) -> Self {
        Self(self.0.cbrt())
    }
    /// Absolute value
    fn abs(&self) -> Self {
        Self(self.0.abs())
    }
    /// Signum function
    fn signum(&self) -> Self {
        Self(self.0.signum())
    }
    /// Floor function
    fn floor(&self) -> Self {
        Self(self.0.floor())
    }
    /// Ceiling function
    fn ceil(&self) -> Self {
        Self(self.0.ceil())
    }
    /// Round to nearest integer
    fn round(&self) -> Self {
        Self(self.0.round())
    }
    /// Truncate to integer
    fn trunc(&self) -> Self {
        Self(self.0.trunc())
    }
    /// Fractional part
    fn fract(&self) -> Self {
        Self(self.0.fract())
    }
    /// Reciprocal
    fn recip(&self) -> Self {
        Self(self.0.recip())
    }
    /// Exponential minus one
    fn exp_m1(&self) -> Self {
        Self(self.0.exp_m1())
    }
    /// Natural logarithm of one plus self
    fn ln_1p(&self) -> Self {
        Self(self.0.ln_1p())
    }

    /// Hypotenuse function
    fn hypot(&self, other: &Self) -> Self {
        Self(self.0.hypot(other.0))
    }
    /// Four-quadrant arctangent
    fn atan2(&self, other: &Self) -> Self {
        Self(self.0.atan2(other.0))
    }
    /// Logarithm with specified base
    fn log(&self, base: &Self) -> Self {
        Self(self.0.log(base.0))
    }
    /// Power function with float exponent
    fn powf(&self, n: f64) -> Self {
        Self(self.0.powf(Dual::constant(n)))
    }
    /// Power function with integer exponent
    fn powi(&self, n: i32) -> Self {
        Self(self.0.powi(n))
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
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Gamma function undefined")
        })
    }
    /// Digamma function
    fn digamma(&self) -> PyResult<Self> {
        self.0.digamma().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Digamma function undefined")
        })
    }
    /// Trigamma function
    fn trigamma(&self) -> PyResult<Self> {
        self.0.trigamma().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Trigamma function undefined")
        })
    }
    /// Polygamma function
    fn polygamma(&self, n: i32) -> PyResult<Self> {
        self.0.polygamma(n).map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Polygamma function undefined")
        })
    }
    /// Riemann zeta function
    fn zeta(&self) -> PyResult<Self> {
        self.0.zeta().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Zeta function undefined")
        })
    }
    /// Lambert W function
    fn lambert_w(&self) -> PyResult<Self> {
        self.0.lambert_w().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Lambert W function undefined")
        })
    }
    /// Bessel function of the first kind
    fn bessel_j(&self, n: i32) -> PyResult<Self> {
        self.0.bessel_j(n).map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Bessel J function undefined")
        })
    }
    /// Sinc function
    fn sinc(&self) -> Self {
        Self(self.0.sinc())
    }
    /// Complete elliptic integral of the first kind
    fn elliptic_k(&self) -> PyResult<Self> {
        self.0.elliptic_k().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Elliptic K function undefined")
        })
    }
    /// Complete elliptic integral of the second kind
    fn elliptic_e(&self) -> PyResult<Self> {
        self.0.elliptic_e().map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Elliptic E function undefined")
        })
    }
    /// Beta function
    fn beta(&self, b: &Self) -> PyResult<Self> {
        self.0.beta(b.0).map(PyDual).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Beta function undefined")
        })
    }
}
