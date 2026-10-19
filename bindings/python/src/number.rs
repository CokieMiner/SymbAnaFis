//! Exact Python integers and fractions, approximate floats, and scalar complex components.

use core::{
    num::NonZeroU64,
    ops::{Add, Div, Mul, Neg, Sub},
};

use num_anafis::RationalType;
use pyo3::{
    Bound, IntoPyObjectExt, Py, PyAny, PyRef, PyResult, Python,
    exceptions::{PyTypeError, PyValueError, PyZeroDivisionError},
    pyclass, pymethods,
    types::{PyAnyMethods, PyBool, PyComplex, PyComplexMethods, PyFloat, PyInt, PyModule},
};
use symb_anafis::{Number, Real};

use super::Diagnostics;

/// A scalar numeric payload independent of expression interning.
#[pyclass(name = "_Number", frozen, module = "symb_anafis._native")]
#[derive(Clone, Debug)]
pub struct NativeNumber {
    value: Number,
}

impl NativeNumber {
    pub const fn value(&self) -> &Number {
        &self.value
    }

    pub fn extract(value: &Bound<'_, PyAny>) -> PyResult<Number> {
        if let Ok(number) = value.extract::<PyRef<'_, Self>>() {
            return Ok(number.value.clone());
        }
        if value.is_instance_of::<PyBool>() {
            return Err(PyTypeError::new_err(
                "booleans are not scalar numeric inputs",
            ));
        }
        if value.is_instance_of::<PyInt>() {
            return value.extract::<i64>().map(Number::from_int);
        }
        if value.is_instance_of::<PyFloat>() {
            return value.extract::<f64>().map(Number::from_float);
        }
        if let Ok(number) = value.cast::<PyComplex>() {
            return Ok(Number::from_complex(
                Number::from_float(number.real()),
                Number::from_float(number.imag()),
            ));
        }
        let fraction = PyModule::import(value.py(), "fractions")?.getattr("Fraction")?;
        if value.is_instance(&fraction)? {
            let numerator = value.getattr("numerator")?.extract::<i64>()?;
            let denominator = value.getattr("denominator")?.extract::<u64>()?;
            let positive = NonZeroU64::new(denominator).ok_or_else(|| {
                PyZeroDivisionError::new_err("a rational denominator must be positive")
            })?;
            return Ok(Number::from_rational(RationalType::from_parts(
                numerator,
                positive.get(),
            )));
        }
        Err(PyTypeError::new_err(
            "expected Number, int, float, complex, or fractions.Fraction",
        ))
    }

    fn component(component: Real, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match component {
            Real::Int(value) => value.into_py_any(py),
            Real::Float(value) => value.into_py_any(py),
            Real::Rational(value) => Ok(PyModule::import(py, "fractions")?
                .getattr("Fraction")?
                .call1((*value.numer(), *value.denom()))?
                .unbind()),
            _ => Err(PyValueError::new_err(
                "unsupported numeric component representation",
            )),
        }
    }
}

impl From<Number> for NativeNumber {
    fn from(value: Number) -> Self {
        Self { value }
    }
}

#[pymethods]
impl NativeNumber {
    #[new]
    fn new(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        Self::extract(value).map(Self::from)
    }

    #[staticmethod]
    fn complex(real: &Self, imaginary: &Self) -> PyResult<Self> {
        if !real.value.is_real() || !imaginary.value.is_real() {
            return Err(PyValueError::new_err(
                "complex components must be real scalars",
            ));
        }
        Ok(Number::from_complex(real.value.clone(), imaginary.value.clone()).into())
    }

    fn components(&self, py: Python<'_>) -> PyResult<(Py<PyAny>, Py<PyAny>)> {
        let parts = self.value.extract_complex().map_err(Diagnostics::value)?;
        Ok((
            Self::component(parts.re, py)?,
            Self::component(parts.im, py)?,
        ))
    }

    fn binary(&self, operation: &str, other: &Self) -> PyResult<Self> {
        let value = match operation {
            "add" => Add::add(&self.value, &other.value),
            "sub" => Sub::sub(&self.value, &other.value),
            "mul" => Mul::mul(&self.value, &other.value),
            "div" => Div::div(&self.value, &other.value),
            "pow" => self.value.pow(&other.value),
            _ => return Err(PyValueError::new_err("unknown numeric operation")),
        };
        Ok(value.into())
    }

    fn negative(&self) -> Self {
        Neg::neg(&self.value).into()
    }

    fn equal(&self, other: &Self) -> bool {
        self.value == other.value
    }

    const fn is_finite(&self) -> bool {
        self.value.is_finite()
    }

    fn is_real(&self) -> bool {
        self.value.is_real()
    }

    fn is_zero(&self) -> bool {
        self.value.is_zero()
    }

    fn __str__(&self) -> String {
        self.value.to_string()
    }
}
