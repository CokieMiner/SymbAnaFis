//! Python bindings for symbols
//!
//! This module provides the `PySymbol` class which wraps Rust's `Symbol` type.

use crate::core::symbol::Symbol as RustSymbol;
use crate::{Expr as RustExpr, remove_symbol, symb, symb_get, symb_new};
use pyo3::prelude::*;

/// Python wrapper for symbols
#[pyclass(unsendable, name = "Symbol")]
#[derive(Clone)]
pub struct PySymbol(pub RustSymbol);

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
    #[allow(
        clippy::cast_possible_wrap,
        clippy::cast_possible_truncation,
        reason = "Python __hash__ requires isize"
    )]
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
    fn to_expr(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.to_expr())
    }

    // Arithmetic operators - accept Expr, Symbol, int, or float
    fn __add__(&self, other: &Bound<'_, PyAny>) -> PyResult<super::expr::PyExpr> {
        let other_expr = super::expr::extract_to_expr(other)?;
        Ok(super::expr::PyExpr(self.0.to_expr() + other_expr))
    }

    fn __sub__(&self, other: &Bound<'_, PyAny>) -> PyResult<super::expr::PyExpr> {
        let other_expr = super::expr::extract_to_expr(other)?;
        Ok(super::expr::PyExpr(self.0.to_expr() - other_expr))
    }

    fn __mul__(&self, other: &Bound<'_, PyAny>) -> PyResult<super::expr::PyExpr> {
        let other_expr = super::expr::extract_to_expr(other)?;
        Ok(super::expr::PyExpr(self.0.to_expr() * other_expr))
    }

    fn __truediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<super::expr::PyExpr> {
        let other_expr = super::expr::extract_to_expr(other)?;
        Ok(super::expr::PyExpr(self.0.to_expr() / other_expr))
    }

    fn __pow__(
        &self,
        other: &Bound<'_, PyAny>,
        _modulo: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<super::expr::PyExpr> {
        let other_expr = super::expr::extract_to_expr(other)?;
        Ok(super::expr::PyExpr(RustExpr::pow_static(
            self.0.to_expr(),
            other_expr,
        )))
    }

    fn __radd__(&self, other: &Bound<'_, PyAny>) -> PyResult<super::expr::PyExpr> {
        let other_expr = super::expr::extract_to_expr(other)?;
        Ok(super::expr::PyExpr(other_expr + self.0.to_expr()))
    }

    fn __rsub__(&self, other: &Bound<'_, PyAny>) -> PyResult<super::expr::PyExpr> {
        let other_expr = super::expr::extract_to_expr(other)?;
        Ok(super::expr::PyExpr(other_expr - self.0.to_expr()))
    }

    fn __rmul__(&self, other: &Bound<'_, PyAny>) -> PyResult<super::expr::PyExpr> {
        let other_expr = super::expr::extract_to_expr(other)?;
        Ok(super::expr::PyExpr(other_expr * self.0.to_expr()))
    }

    fn __rtruediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<super::expr::PyExpr> {
        let other_expr = super::expr::extract_to_expr(other)?;
        Ok(super::expr::PyExpr(other_expr / self.0.to_expr()))
    }

    fn __rpow__(
        &self,
        other: &Bound<'_, PyAny>,
        _modulo: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<super::expr::PyExpr> {
        let other_expr = super::expr::extract_to_expr(other)?;
        Ok(super::expr::PyExpr(RustExpr::pow_static(
            other_expr,
            self.0.to_expr(),
        )))
    }

    fn __neg__(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(RustExpr::number(0.0) - self.0.to_expr())
    }

    // Math function methods - convert Symbol to Expr and apply function
    fn sin(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.sin())
    }
    fn cos(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.cos())
    }
    fn tan(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.tan())
    }
    fn cot(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.cot())
    }
    fn sec(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.sec())
    }
    fn csc(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.csc())
    }

    fn asin(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.asin())
    }
    fn acos(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.acos())
    }
    fn atan(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.atan())
    }

    fn sinh(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.sinh())
    }
    fn cosh(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.cosh())
    }
    fn tanh(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.tanh())
    }

    fn asinh(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.asinh())
    }
    fn acosh(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.acosh())
    }
    fn atanh(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.atanh())
    }

    fn exp(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.exp())
    }
    fn ln(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.ln())
    }
    fn log10(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.log10())
    }
    fn log2(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.log2())
    }

    fn sqrt(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.sqrt())
    }
    fn cbrt(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.cbrt())
    }
    fn abs(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.abs())
    }

    fn floor(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.floor())
    }
    fn ceil(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.ceil())
    }
    fn round(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.round())
    }

    fn erf(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.erf())
    }
    fn erfc(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.erfc())
    }
    fn gamma(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.gamma())
    }

    // Additional inverse trig
    fn acot(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.acot())
    }
    fn asec(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.asec())
    }
    fn acsc(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.acsc())
    }

    // Additional hyperbolic
    fn coth(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.coth())
    }
    fn sech(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.sech())
    }
    fn csch(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.csch())
    }

    // Additional inverse hyperbolic
    fn acoth(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.acoth())
    }
    fn asech(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.asech())
    }
    fn acsch(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.acsch())
    }

    // Additional special functions
    fn signum(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.signum())
    }
    fn sinc(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.sinc())
    }
    fn lambertw(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.lambertw())
    }
    fn zeta(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.zeta())
    }
    fn elliptic_k(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.elliptic_k())
    }
    fn elliptic_e(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.elliptic_e())
    }
    fn exp_polar(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.exp_polar())
    }

    // Additional gamma family
    fn digamma(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.digamma())
    }
    fn trigamma(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.trigamma())
    }
    fn tetragamma(&self) -> super::expr::PyExpr {
        super::expr::PyExpr(self.0.tetragamma())
    }

    // Multi-argument functions
    /// Logarithm with arbitrary base: log(base, x)
    fn log(&self, base: &Bound<'_, PyAny>) -> PyResult<super::expr::PyExpr> {
        let base_expr = super::expr::extract_to_expr(base)?;
        Ok(super::expr::PyExpr(self.0.log(base_expr)))
    }

    /// Polygamma function: ψ^(n)(x)
    fn polygamma(&self, n: &Bound<'_, PyAny>) -> PyResult<super::expr::PyExpr> {
        let n_expr = super::expr::extract_to_expr(n)?;
        Ok(super::expr::PyExpr(self.0.polygamma(n_expr)))
    }

    /// Beta function: B(self, other)
    fn beta(&self, other: &Bound<'_, PyAny>) -> PyResult<super::expr::PyExpr> {
        let other_expr = super::expr::extract_to_expr(other)?;
        Ok(super::expr::PyExpr(self.0.beta(other_expr)))
    }

    /// Bessel function of the first kind: `J_n(x)`
    fn besselj(&self, n: &Bound<'_, PyAny>) -> PyResult<super::expr::PyExpr> {
        let n_expr = super::expr::extract_to_expr(n)?;
        Ok(super::expr::PyExpr(self.0.besselj(n_expr)))
    }

    /// Bessel function of the second kind: `Y_n(x)`
    fn bessely(&self, n: &Bound<'_, PyAny>) -> PyResult<super::expr::PyExpr> {
        let n_expr = super::expr::extract_to_expr(n)?;
        Ok(super::expr::PyExpr(self.0.bessely(n_expr)))
    }

    /// Modified Bessel function of the first kind: `I_n(x)`
    fn besseli(&self, n: &Bound<'_, PyAny>) -> PyResult<super::expr::PyExpr> {
        let n_expr = super::expr::extract_to_expr(n)?;
        Ok(super::expr::PyExpr(self.0.besseli(n_expr)))
    }

    /// Modified Bessel function of the second kind: `K_n(x)`
    fn besselk(&self, n: &Bound<'_, PyAny>) -> PyResult<super::expr::PyExpr> {
        let n_expr = super::expr::extract_to_expr(n)?;
        Ok(super::expr::PyExpr(self.0.besselk(n_expr)))
    }

    /// Two-argument arctangent: atan2(self, x) = angle to point (x, self)
    fn atan2(&self, x: &Bound<'_, PyAny>) -> PyResult<super::expr::PyExpr> {
        let x_expr = super::expr::extract_to_expr(x)?;
        Ok(super::expr::PyExpr(RustExpr::func_multi(
            "atan2",
            vec![self.0.to_expr(), x_expr],
        )))
    }

    /// Hermite polynomial `H_n(self)`
    fn hermite(&self, n: &Bound<'_, PyAny>) -> PyResult<super::expr::PyExpr> {
        let n_expr = super::expr::extract_to_expr(n)?;
        Ok(super::expr::PyExpr(RustExpr::func_multi(
            "hermite",
            vec![n_expr, self.0.to_expr()],
        )))
    }

    /// Associated Legendre polynomial `P_l^m(self)`
    fn assoc_legendre(
        &self,
        l: &Bound<'_, PyAny>,
        m: &Bound<'_, PyAny>,
    ) -> PyResult<super::expr::PyExpr> {
        let l_expr = super::expr::extract_to_expr(l)?;
        let m_expr = super::expr::extract_to_expr(m)?;
        Ok(super::expr::PyExpr(RustExpr::func_multi(
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
    ) -> PyResult<super::expr::PyExpr> {
        let l_expr = super::expr::extract_to_expr(l)?;
        let m_expr = super::expr::extract_to_expr(m)?;
        let phi_expr = super::expr::extract_to_expr(phi)?;
        Ok(super::expr::PyExpr(RustExpr::func_multi(
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
    ) -> PyResult<super::expr::PyExpr> {
        let l_expr = super::expr::extract_to_expr(l)?;
        let m_expr = super::expr::extract_to_expr(m)?;
        let phi_expr = super::expr::extract_to_expr(phi)?;
        Ok(super::expr::PyExpr(RustExpr::func_multi(
            "ynm",
            vec![l_expr, m_expr, self.0.to_expr(), phi_expr],
        )))
    }

    /// Derivative of Riemann zeta function: zeta^(n)(self)
    fn zeta_deriv(&self, n: &Bound<'_, PyAny>) -> PyResult<super::expr::PyExpr> {
        let n_expr = super::expr::extract_to_expr(n)?;
        Ok(super::expr::PyExpr(RustExpr::func_multi(
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
