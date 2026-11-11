//! Real first-order covariance propagation through owning Python handles.

use pyo3::{Bound, PyAny, PyRef, PyResult, Python, pyclass, pymethods};
use symb_anafis::{CovarianceMatrix, Uncertainty};

use super::{ExprInputs, NativeExpr, UncertaintyError};

/// Validated shape and symmetry retain caller-owned covariance hypotheses.
#[pyclass(name = "_CovarianceMatrix", frozen, module = "symb_anafis._native")]
#[derive(Clone, Debug)]
pub struct NativeCovariance {
    covariance: CovarianceMatrix,
}

/// A model, ordered sensitivities, and its combined variance.
#[pyclass(name = "_Uncertainty", frozen, module = "symb_anafis._native")]
#[derive(Clone, Debug)]
pub struct NativeUncertainty {
    result: Uncertainty,
}

#[pymethods]
impl NativeCovariance {
    #[new]
    fn new(entries: Vec<Vec<PyRef<'_, NativeExpr>>>) -> PyResult<Self> {
        CovarianceMatrix::new(
            entries
                .into_iter()
                .map(|row| row.into_iter().map(|value| value.value().clone()).collect())
                .collect(),
        )
        .map(|covariance| Self { covariance })
        .map_err(|error| UncertaintyError::new_err(error.to_string()))
    }

    #[staticmethod]
    fn diagonal(variances: Vec<PyRef<'_, NativeExpr>>) -> PyResult<Self> {
        CovarianceMatrix::diagonal(
            variances
                .into_iter()
                .map(|value| value.value().clone())
                .collect(),
        )
        .map(|covariance| Self { covariance })
        .map_err(|error| UncertaintyError::new_err(error.to_string()))
    }

    const fn dimension(&self) -> usize {
        self.covariance.dimension()
    }

    fn get(&self, row: usize, column: usize) -> Option<NativeExpr> {
        self.covariance
            .get(row, column)
            .cloned()
            .map(NativeExpr::from)
    }
}

#[pymethods]
impl NativeUncertainty {
    #[staticmethod]
    fn propagate(
        expression: &NativeExpr,
        variables: &Bound<'_, PyAny>,
        covariance: &NativeCovariance,
        py: Python<'_>,
    ) -> PyResult<Self> {
        let owners = ExprInputs::new(variables)?;
        let coordinates = owners.coordinates()?;
        py.detach(|| {
            Uncertainty::propagate(expression.value(), &coordinates, &covariance.covariance)
        })
        .map(|result| Self { result })
        .map_err(|error| UncertaintyError::new_err(error.to_string()))
    }

    fn expression(&self) -> NativeExpr {
        self.result.expression().clone().into()
    }

    fn sensitivities(&self) -> Vec<NativeExpr> {
        self.result
            .sensitivities()
            .iter()
            .cloned()
            .map(NativeExpr::from)
            .collect()
    }

    fn variance(&self) -> NativeExpr {
        self.result.variance().clone().into()
    }

    fn standard_deviation(&self) -> NativeExpr {
        self.result.standard_deviation().into()
    }

    fn relative(&self) -> NativeExpr {
        self.result.relative().into()
    }
}
