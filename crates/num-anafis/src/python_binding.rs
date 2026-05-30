use crate::number::{self, Number, Scalar};
use pyo3::prelude::*;

#[pyclass(name = "Scalar", frozen, skip_from_py_object)]
#[derive(Clone)]
struct PyScalar(Scalar);

#[pymethods]
impl PyScalar {
    #[new]
    fn new(x: f64) -> Self {
        Self(number::s(x))
    }

    #[staticmethod]
    fn from_ints(num: i64, den: i64) -> Self {
        Self(number::r(num, den))
    }

    #[staticmethod]
    fn set_precision(bits: u32) -> bool {
        Scalar::set_precision(bits)
    }

    fn __repr__(&self) -> String {
        format!("{}", self.0)
    }
    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
    fn __add__(&self, other: &Self) -> Self {
        Self(&self.0 + &other.0)
    }
    fn __sub__(&self, other: &Self) -> Self {
        Self(&self.0 - &other.0)
    }
    fn __mul__(&self, other: &Self) -> Self {
        Self(&self.0 * &other.0)
    }
    fn __truediv__(&self, other: &Self) -> Self {
        Self(&self.0 / &other.0)
    }
    fn __neg__(&self) -> Self {
        Self(-&self.0)
    }
    fn __abs__(&self) -> Self {
        Self(self.0.abs())
    }
    fn __pow__(&self, exp: &Self, _mod: Option<&Self>) -> Self {
        Self(self.0.pow(&exp.0))
    }

    // --- Properties ---
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
    fn is_one(&self) -> bool {
        self.0.is_one()
    }
    fn is_neg_one(&self) -> bool {
        self.0.is_neg_one()
    }
    fn is_integer(&self) -> bool {
        self.0.is_integer()
    }
    fn is_finite(&self) -> bool {
        self.0.is_finite()
    }
    fn is_negative(&self) -> bool {
        self.0.is_negative()
    }
    fn is_positive(&self) -> bool {
        self.0.is_positive()
    }

    fn to_float(&self) -> Self {
        Self(self.0.to_float())
    }
    fn approx_eq(&self, other: &Self, tol: &Self) -> bool {
        self.0.approx_eq_number(&other.0, &tol.0)
    }
    fn total_cmp(&self, other: &Self) -> i8 {
        use core::cmp::Ordering;
        match self.0.total_cmp(&other.0) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }
    fn num_max(&self, other: &Self) -> Self {
        Self(self.0.num_max(&other.0))
    }
    fn num_min(&self, other: &Self) -> Self {
        Self(self.0.num_min(&other.0))
    }

    // --- Basic math ---
    fn floor(&self) -> Self {
        Self(self.0.floor())
    }
    fn ceil(&self) -> Self {
        Self(self.0.ceil())
    }
    fn round(&self) -> Self {
        Self(self.0.round())
    }
    fn fract(&self) -> Self {
        Self(self.0.fract())
    }
    fn signum(&self) -> Self {
        Self(self.0.signum())
    }
    fn negate(&self) -> Self {
        Self(self.0.negate())
    }
    fn sqrt(&self) -> Self {
        Self(self.0.sqrt())
    }
    fn cbrt(&self) -> Self {
        Self(self.0.cbrt())
    }

    // --- Trigonometric ---
    fn sin(&self) -> Self {
        Self(self.0.sin())
    }
    fn cos(&self) -> Self {
        Self(self.0.cos())
    }
    fn tan(&self) -> Self {
        Self(self.0.tan())
    }
    fn cot(&self) -> Self {
        Self(self.0.cot())
    }
    fn sec(&self) -> Self {
        Self(self.0.sec())
    }
    fn csc(&self) -> Self {
        Self(self.0.csc())
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
    fn acot(&self) -> Self {
        Self(self.0.acot())
    }
    fn asec(&self) -> Self {
        Self(self.0.asec())
    }
    fn acsc(&self) -> Self {
        Self(self.0.acsc())
    }
    fn atan2(&self, x: &Self) -> Self {
        Self(self.0.atan2(&x.0))
    }

    // --- Hyperbolic ---
    fn sinh(&self) -> Self {
        Self(self.0.sinh())
    }
    fn cosh(&self) -> Self {
        Self(self.0.cosh())
    }
    fn tanh(&self) -> Self {
        Self(self.0.tanh())
    }
    fn coth(&self) -> Self {
        Self(self.0.coth())
    }
    fn sech(&self) -> Self {
        Self(self.0.sech())
    }
    fn csch(&self) -> Self {
        Self(self.0.csch())
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
    fn acoth(&self) -> Self {
        Self(self.0.acoth())
    }
    fn acsch(&self) -> Self {
        Self(self.0.acsch())
    }
    fn asech(&self) -> Self {
        Self(self.0.asech())
    }

    // --- Exponential & logarithmic ---
    fn exp(&self) -> Self {
        Self(self.0.exp())
    }
    fn expm1(&self) -> Self {
        Self(self.0.expm1())
    }
    fn exp_neg(&self) -> Self {
        Self(self.0.exp_neg())
    }
    fn ln(&self) -> Self {
        Self(self.0.ln())
    }
    fn log1p(&self) -> Self {
        Self(self.0.log1p())
    }
    fn log_base(&self, base: &Self) -> Self {
        Self(self.0.log_base(&base.0))
    }

    // --- Special functions ---
    fn erf(&self) -> Self {
        Self(self.0.erf())
    }
    fn erfc(&self) -> Self {
        Self(self.0.erfc())
    }
    fn gamma(&self) -> Self {
        Self(self.0.gamma())
    }
    fn lgamma(&self) -> Self {
        Self(self.0.lgamma())
    }
    fn digamma(&self) -> Self {
        Self(self.0.digamma())
    }
    fn trigamma(&self) -> Self {
        Self(self.0.trigamma())
    }
    fn tetragamma(&self) -> Self {
        Self(self.0.tetragamma())
    }
    fn sinc(&self) -> Self {
        Self(self.0.sinc())
    }
    fn lambertw(&self, n: &Self) -> Self {
        Self(self.0.lambertw(&n.0))
    }
    fn zeta(&self) -> Self {
        Self(self.0.zeta())
    }
    fn elliptic_k(&self) -> Self {
        Self(self.0.elliptic_k())
    }
    fn elliptic_e(&self) -> Self {
        Self(self.0.elliptic_e())
    }
    fn exp_polar(&self) -> Self {
        Self(self.0.exp_polar())
    }

    fn besselj(&self, n: &Self) -> Self {
        Self(self.0.besselj(&n.0))
    }
    fn bessely(&self, n: &Self) -> Self {
        Self(self.0.bessely(&n.0))
    }
    fn besseli(&self, n: &Self) -> Self {
        Self(self.0.besseli(&n.0))
    }
    fn besselk(&self, n: &Self) -> Self {
        Self(self.0.besselk(&n.0))
    }
    fn polygamma(&self, n: &Self) -> Self {
        Self(self.0.polygamma(&n.0))
    }
    fn beta(&self, other: &Self) -> Self {
        Self(self.0.beta(&other.0))
    }
    fn zeta_deriv(&self, n: &Self) -> Self {
        Self(self.0.zeta_deriv(&n.0))
    }
    fn hermite(&self, n: &Self) -> Self {
        Self(self.0.hermite(&n.0))
    }

    fn assoc_legendre(&self, l: &Self, m: &Self) -> Self {
        Self(self.0.assoc_legendre(&l.0, &m.0))
    }
    fn spherical_harmonic(&self, l: &Self, m: &Self, phi: &Self) -> Self {
        Self(self.0.spherical_harmonic(&l.0, &m.0, &phi.0))
    }
}

#[pyfunction]
fn s(x: f64) -> PyScalar {
    PyScalar::new(x)
}

#[pyfunction]
fn r(num: i64, den: i64) -> PyScalar {
    PyScalar::from_ints(num, den)
}

#[pyfunction]
fn get_precision() -> u32 {
    Scalar::get_precision()
}

#[pymodule]
fn num_anafis_py(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyScalar>()?;
    m.add_function(wrap_pyfunction!(s, m)?)?;
    m.add_function(wrap_pyfunction!(r, m)?)?;
    m.add_function(wrap_pyfunction!(get_precision, m)?)?;
    Ok(())
}
