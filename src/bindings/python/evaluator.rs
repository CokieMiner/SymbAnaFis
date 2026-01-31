//! Python bindings for compiled evaluators
//!
//! This module provides the `PyCompiledEvaluator` class for fast numerical
//! evaluation of symbolic expressions.

use crate::evaluator::CompiledEvaluator as RustCompiledEvaluator;
use numpy::{PyArray1, PyReadonlyArray1};
use pyo3::prelude::*;

/// Python wrapper for compiled evaluators
#[pyclass(unsendable, name = "CompiledEvaluator")]
pub struct PyCompiledEvaluator {
    evaluator: RustCompiledEvaluator,
}

#[pymethods]
impl PyCompiledEvaluator {
    /// Compile an expression with specified parameter order and optional context.
    // PyO3 requires owned types; if-let pattern clearer than map_or_else here
    #[allow(
        clippy::needless_pass_by_value,
        clippy::option_if_let_else,
        reason = "PyO3 requires owned types; if-let pattern clearer than map_or_else here"
    )]
    #[new]
    #[pyo3(signature = (expr, params=None, context=None))]
    fn new(
        expr: &super::expr::PyExpr,
        params: Option<Vec<String>>,
        context: Option<&super::context::PyContext>,
    ) -> PyResult<Self> {
        let param_refs: Vec<&str>;
        let rust_context = context.map(|c| &c.inner);

        let evaluator = if let Some(p) = &params {
            param_refs = p.iter().map(std::string::String::as_str).collect();
            RustCompiledEvaluator::compile(&expr.0, &param_refs, rust_context)
        } else {
            RustCompiledEvaluator::compile_auto(&expr.0, rust_context)
        };

        evaluator.map(|e| Self { evaluator: e }).map_err(Into::into)
    }

    /// Evaluate at a single point
    /// Evaluate at a single point (Zero-Copy `NumPy` or List)
    // PyO3 requires Bound<'_, PyAny> by value for flexible input types
    #[allow(
        clippy::needless_pass_by_value,
        reason = "PyO3 requires Bound<'_, PyAny> by value for flexible input types"
    )]
    fn evaluate(&self, input: Bound<'_, PyAny>) -> PyResult<f64> {
        let data = extract_data_input(&input)?;
        let slice = data.as_slice()?;
        Ok(self.evaluator.evaluate(slice))
    }

    /// Batch evaluate at multiple points (columnar data)
    /// columns[`var_idx`][point_idx] -> f64
    ///
    /// Accepts `NumPy` arrays (Zero-Copy) or Python Lists (fallback).
    // PyO3 requires owned Vec for Python list arguments
    #[allow(
        clippy::needless_pass_by_value,
        reason = "PyO3 requires owned Vec for Python list arguments"
    )]
    fn eval_batch<'py>(
        &self,
        py: Python<'py>,
        columns: Vec<Bound<'py, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let inputs: Vec<DataInput> = columns
            .iter()
            .map(|c| extract_data_input(c))
            .collect::<PyResult<Vec<_>>>()?;

        let n_points = if inputs.is_empty() {
            1
        } else {
            inputs[0].len()?
        };

        // Check if any input is a NumPy array to decide output format
        let use_numpy = inputs.iter().any(|d| matches!(d, DataInput::Array(_)));

        // Zero-copy access (or slice from vec)
        let col_refs: Vec<&[f64]> = inputs
            .iter()
            .map(|c| c.as_slice())
            .collect::<PyResult<Vec<_>>>()?;

        let mut output = vec![0.0; n_points];
        self.evaluator
            .eval_batch(&col_refs, &mut output, None)
            .map_err(PyErr::from)?;

        if use_numpy {
            // Return NumPy array (this copies the result, but input was zero-copy)
            Ok(PyArray1::from_vec(py, output).into_any().unbind())
        } else {
            // Return standard Python list
            Ok(output
                .into_pyobject(py)
                .expect("PyO3 object conversion failed")
                .into_any()
                .unbind())
        }
    }

    /// Get parameter names in order
    fn param_names(&self) -> Vec<String> {
        self.evaluator.param_names().to_vec()
    }

    /// Get number of parameters
    fn param_count(&self) -> usize {
        self.evaluator.param_count()
    }

    /// Get number of bytecode instructions
    fn instruction_count(&self) -> usize {
        self.evaluator.instruction_count()
    }

    /// Get required stack size
    const fn stack_size(&self) -> usize {
        self.evaluator.stack_size()
    }
}

// ============================================================================
// Data input utilities (extracted from legacy)
// ============================================================================

/// Enum for flexible data input (`NumPy` arrays or Python lists)
pub enum DataInput<'py> {
    /// `NumPy` array (zero-copy)
    Array(PyReadonlyArray1<'py, f64>),
    /// Python list (fallback)
    List(Vec<f64>),
}

impl DataInput<'_> {
    /// Get the length of the data
    pub fn len(&self) -> PyResult<usize> {
        match self {
            Self::Array(arr) => Ok(arr.len()?),
            Self::List(vec) => Ok(vec.len()),
        }
    }

    /// Get a slice view of the data
    pub fn as_slice(&self) -> PyResult<&[f64]> {
        match self {
            Self::Array(arr) => arr.as_slice().map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "All input arrays must be C-contiguous and f64: {e}"
                ))
            }),
            Self::List(vec) => Ok(vec),
        }
    }
}

/// Extract data from `NumPy` array or Python list
pub fn extract_data_input<'py>(input: &Bound<'py, PyAny>) -> PyResult<DataInput<'py>> {
    // Try NumPy array first (zero-copy)
    if let Ok(array) = input.extract::<PyReadonlyArray1<f64>>() {
        return Ok(DataInput::Array(array));
    }

    // Fallback to Python list
    if let Ok(list) = input.extract::<Vec<f64>>() {
        return Ok(DataInput::List(list));
    }

    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
        "Input must be a NumPy array or list of floats",
    ))
}
