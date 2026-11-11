//! Simultaneous exact replacements over owning expression identities.

use pyo3::{PyRef, PyResult, Python, pyclass, pymethods};
use symb_anafis::Substitutions;

use super::{Diagnostics, NativeExpr};

/// Validated replacement entries retain their source and destination DAGs.
#[pyclass(name = "_Substitutions", frozen, module = "symb_anafis._native")]
#[derive(Clone, Debug)]
pub struct NativeSubstitutions {
    substitutions: Substitutions,
}

#[pymethods]
impl NativeSubstitutions {
    #[new]
    fn new(entries: Vec<(PyRef<'_, NativeExpr>, PyRef<'_, NativeExpr>)>) -> PyResult<Self> {
        Substitutions::new(
            entries
                .into_iter()
                .map(|(source, destination)| (source.value().clone(), destination.value().clone())),
        )
        .map(|substitutions| Self { substitutions })
        .map_err(Diagnostics::value)
    }

    fn apply(&self, expression: &NativeExpr, py: Python<'_>) -> NativeExpr {
        py.detach(|| self.substitutions.apply_all(expression.value()))
            .into()
    }

    fn __len__(&self) -> usize {
        self.substitutions.len()
    }
}
