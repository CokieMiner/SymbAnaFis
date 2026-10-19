//! Pin Python expression owners while borrowing validated coordinate identities.

use pyo3::{Bound, PyAny, PyRef, PyResult, types::PyAnyMethods};
use symb_anafis::{Expr, Symbol};

use super::NativeExpr;

/// An extracted input sequence retains all owners until the native call ends.
#[derive(Debug)]
pub struct ExprInputs<'python> {
    values: Vec<PyRef<'python, NativeExpr>>,
}

impl<'python> ExprInputs<'python> {
    pub fn new(values: &Bound<'python, PyAny>) -> PyResult<Self> {
        values.extract().map(|values| Self { values })
    }

    pub fn coordinates(&self) -> PyResult<Vec<Symbol<'_>>> {
        self.values.iter().map(|value| value.symbol()).collect()
    }

    pub fn into_expressions(self) -> Vec<Expr> {
        self.values
            .into_iter()
            .map(|value| value.value().clone())
            .collect()
    }
}
