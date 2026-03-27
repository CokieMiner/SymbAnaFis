//! Python bindings for symbols
//!
//! This module provides the `PySymbol` class which wraps Rust's `Symbol` type.

// Allow: PyO3 from_py_object macro generates FromPyObject with short lifetime names
#![allow(
    clippy::single_char_lifetime_names,
    reason = "PyO3 from_py_object macro generates lifetimes"
)]

use super::expr::{PyExpr, extract_to_expr};
use crate::core::Symbol as RustSymbol;
use crate::core::{Expr as RustExpr, remove_symbol, symb, symb_get, symb_new};
use pyo3::prelude::*;

/// Python wrapper for symbols
#[pyclass(unsendable, name = "Symbol", from_py_object)]
#[derive(Clone)]
pub struct PySymbol(pub RustSymbol);

#[pymethods]
impl PySymbol {
    /// Creates a new symbol with the given name.
    #[new]
    fn new(name: &str) -> Self {
        Self(symb(name))
    }

    /// Creates a new anonymous symbol.
    ///
    /// Anonymous symbols have a unique ID but no string name.
    /// They cannot be retrieved by name and are useful for intermediate computations.
    ///
    /// Returns:
    ///     A new anonymous symbol.
    ///
    /// Example:
    ///     >>> anon = `Symbol.anon()`
    ///     >>> expr = anon + 1
    ///     >>> print(expr)  # Shows as "$123" where 123 is the unique ID
    #[staticmethod]
    fn anon() -> Self {
        Self(RustSymbol::anon())
    }

    /// Returns the string representation of the symbol.
    fn __str__(&self) -> String {
        self.0.name().unwrap_or_default()
    }

    /// Returns the repr representation of the symbol.
    fn __repr__(&self) -> String {
        format!("Symbol(\"{}\")", self.0.name().unwrap_or_default())
    }

    /// Checks equality with another object.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        if let Ok(other_sym) = other.extract::<Self>() {
            return self.0 == other_sym.0;
        }
        false
    }

    // u64->isize: Python __hash__ requires isize, wrap is expected
    #[allow(
        clippy::cast_possible_wrap,
        clippy::cast_possible_truncation,
        reason = "Python __hash__ requires isize"
    )]
    /// Returns the hash value of the symbol.
    fn __hash__(&self) -> isize {
        self.0.id() as isize
    }

    /// Get the symbol name
    fn name(&self) -> Option<String> {
        self.0.name()
    }

    /// Unique identifier for this symbol (pointer address based)
    #[getter]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "Python ID requires isize"
    )]
    fn id(&self) -> isize {
        self.0.id() as isize
    }

    /// Convert to an expression
    fn to_expr(&self) -> PyExpr {
        PyExpr(self.0.to_expr())
    }

    // Arithmetic operators - accept Expr, Symbol, int, or float
    /// Addition operator.
    fn __add__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(self.0.to_expr() + other_expr))
    }

    /// Subtraction operator.
    fn __sub__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(self.0.to_expr() - other_expr))
    }

    /// Multiplication operator.
    fn __mul__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(self.0.to_expr() * other_expr))
    }

    /// Division operator.
    fn __truediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(self.0.to_expr() / other_expr))
    }

    /// Power operator.
    fn __pow__(
        &self,
        other: &Bound<'_, PyAny>,
        _modulo: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(RustExpr::pow_static(self.0.to_expr(), other_expr)))
    }

    /// Reverse addition operator.
    fn __radd__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(other_expr + self.0.to_expr()))
    }

    /// Reverse subtraction operator.
    fn __rsub__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(other_expr - self.0.to_expr()))
    }

    /// Reverse multiplication operator.
    fn __rmul__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(other_expr * self.0.to_expr()))
    }

    /// Reverse division operator.
    fn __rtruediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(other_expr / self.0.to_expr()))
    }

    /// Reverse power operator.
    fn __rpow__(
        &self,
        other: &Bound<'_, PyAny>,
        _modulo: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyExpr> {
        let other_expr = extract_to_expr(other)?;
        Ok(PyExpr(RustExpr::pow_static(other_expr, self.0.to_expr())))
    }

    /// Negation operator.
    fn __neg__(&self) -> PyExpr {
        PyExpr(RustExpr::number(0.0) - self.0.to_expr())
    }

    // Math function methods - convert Symbol to Expr and apply function
    /// Sine function.
    fn sin(&self) -> PyExpr {
        PyExpr(self.0.sin())
    }
    /// Cosine function.
    fn cos(&self) -> PyExpr {
        PyExpr(self.0.cos())
    }
    /// Tangent function.
    fn tan(&self) -> PyExpr {
        PyExpr(self.0.tan())
    }
    /// Cotangent function.
    fn cot(&self) -> PyExpr {
        PyExpr(self.0.cot())
    }
    /// Secant function.
    fn sec(&self) -> PyExpr {
        PyExpr(self.0.sec())
    }
    /// Cosecant function.
    fn csc(&self) -> PyExpr {
        PyExpr(self.0.csc())
    }

    /// Inverse sine.
    fn asin(&self) -> PyExpr {
        PyExpr(self.0.asin())
    }
    /// Inverse cosine.
    fn acos(&self) -> PyExpr {
        PyExpr(self.0.acos())
    }
    /// Inverse tangent.
    fn atan(&self) -> PyExpr {
        PyExpr(self.0.atan())
    }

    /// Hyperbolic sine.
    fn sinh(&self) -> PyExpr {
        PyExpr(self.0.sinh())
    }
    /// Hyperbolic cosine.
    fn cosh(&self) -> PyExpr {
        PyExpr(self.0.cosh())
    }
    /// Hyperbolic tangent.
    fn tanh(&self) -> PyExpr {
        PyExpr(self.0.tanh())
    }

    /// Inverse hyperbolic sine.
    fn asinh(&self) -> PyExpr {
        PyExpr(self.0.asinh())
    }
    /// Inverse hyperbolic cosine.
    fn acosh(&self) -> PyExpr {
        PyExpr(self.0.acosh())
    }
    /// Inverse hyperbolic tangent.
    fn atanh(&self) -> PyExpr {
        PyExpr(self.0.atanh())
    }

    /// Exponential function.
    fn exp(&self) -> PyExpr {
        PyExpr(self.0.exp())
    }
    /// Natural logarithm.
    fn ln(&self) -> PyExpr {
        PyExpr(self.0.ln())
    }
    /// Base-10 logarithm.
    fn log10(&self) -> PyExpr {
        PyExpr(self.0.log10())
    }
    /// Base-2 logarithm.
    fn log2(&self) -> PyExpr {
        PyExpr(self.0.log2())
    }

    /// Square root.
    fn sqrt(&self) -> PyExpr {
        PyExpr(self.0.sqrt())
    }
    /// Cube root.
    fn cbrt(&self) -> PyExpr {
        PyExpr(self.0.cbrt())
    }
    /// Absolute value.
    fn abs(&self) -> PyExpr {
        PyExpr(self.0.abs())
    }

    /// Floor function.
    fn floor(&self) -> PyExpr {
        PyExpr(self.0.floor())
    }
    /// Ceiling function.
    fn ceil(&self) -> PyExpr {
        PyExpr(self.0.ceil())
    }
    /// Round function.
    fn round(&self) -> PyExpr {
        PyExpr(self.0.round())
    }

    /// Error function.
    fn erf(&self) -> PyExpr {
        PyExpr(self.0.erf())
    }
    /// Complementary error function.
    fn erfc(&self) -> PyExpr {
        PyExpr(self.0.erfc())
    }
    /// Gamma function.
    fn gamma(&self) -> PyExpr {
        PyExpr(self.0.gamma())
    }
    /// Log-Gamma function.
    fn lgamma(&self) -> PyExpr {
        PyExpr(self.0.lgamma())
    }

    // Additional inverse trig
    /// Inverse cotangent.
    fn acot(&self) -> PyExpr {
        PyExpr(self.0.acot())
    }
    /// Inverse secant.
    fn asec(&self) -> PyExpr {
        PyExpr(self.0.asec())
    }
    /// Inverse cosecant.
    fn acsc(&self) -> PyExpr {
        PyExpr(self.0.acsc())
    }

    // Additional hyperbolic
    /// Hyperbolic cotangent.
    fn coth(&self) -> PyExpr {
        PyExpr(self.0.coth())
    }
    /// Hyperbolic secant.
    fn sech(&self) -> PyExpr {
        PyExpr(self.0.sech())
    }
    /// Hyperbolic cosecant.
    fn csch(&self) -> PyExpr {
        PyExpr(self.0.csch())
    }

    // Additional inverse hyperbolic
    /// Inverse hyperbolic cotangent.
    fn acoth(&self) -> PyExpr {
        PyExpr(self.0.acoth())
    }
    /// Inverse hyperbolic secant.
    fn asech(&self) -> PyExpr {
        PyExpr(self.0.asech())
    }
    /// Inverse hyperbolic cosecant.
    fn acsch(&self) -> PyExpr {
        PyExpr(self.0.acsch())
    }

    // Additional special functions
    /// Sign function.
    fn signum(&self) -> PyExpr {
        PyExpr(self.0.signum())
    }
    /// Sinc function.
    fn sinc(&self) -> PyExpr {
        PyExpr(self.0.sinc())
    }
    /// Lambert W function.
    fn lambertw(&self) -> PyExpr {
        PyExpr(self.0.lambertw())
    }
    /// Riemann zeta function.
    fn zeta(&self) -> PyExpr {
        PyExpr(self.0.zeta())
    }
    /// Complete elliptic integral of the first kind.
    fn elliptic_k(&self) -> PyExpr {
        PyExpr(self.0.elliptic_k())
    }
    /// Complete elliptic integral of the second kind.
    fn elliptic_e(&self) -> PyExpr {
        PyExpr(self.0.elliptic_e())
    }
    /// Exponential in polar form.
    fn exp_polar(&self) -> PyExpr {
        PyExpr(self.0.exp_polar())
    }

    // Additional gamma family
    /// Digamma function.
    fn digamma(&self) -> PyExpr {
        PyExpr(self.0.digamma())
    }
    /// Trigamma function.
    fn trigamma(&self) -> PyExpr {
        PyExpr(self.0.trigamma())
    }
    /// Tetragamma function.
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
pub fn py_symb(name: &str) -> PySymbol {
    PySymbol(symb(name))
}

/// Create a new symbol (fails if already exists)
#[pyfunction]
#[pyo3(name = "symb_new")]
pub fn py_symb_new(name: &str) -> PyResult<PySymbol> {
    symb_new(name).map(PySymbol).map_err(Into::into)
}

/// Get an existing symbol (fails if not found)
#[pyfunction]
#[pyo3(name = "symb_get")]
pub fn py_symb_get(name: &str) -> PyResult<PySymbol> {
    symb_get(name).map(PySymbol).map_err(Into::into)
}

/// Remove a symbol from global context
#[pyfunction]
#[pyo3(name = "remove_symbol")]
pub fn py_remove_symbol(name: &str) -> bool {
    remove_symbol(name)
}
