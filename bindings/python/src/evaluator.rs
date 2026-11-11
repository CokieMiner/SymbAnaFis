//! Direct and reusable numerical execution without expression interning.

#![expect(
    unsafe_code,
    reason = "zero-copy borrowing of contiguous f64 slices from Python buffer protocol exports"
)]

use core::slice;

use pyo3::{
    Bound, IntoPyObjectExt, Py, PyAny, PyResult, Python,
    buffer::PyBuffer,
    exceptions::{PyMemoryError, PyValueError},
    pyclass, pymethods,
    types::{PyAnyMethods, PyList, PyListMethods},
};
use symb_anafis::{EvalError, Number, Program, Workspace};

use super::{EvaluationError, ExprInputs, NativeExpr, NativeNumber};

/// Borrowed or materialized column buffer for batch evaluation.
#[derive(Debug)]
enum ColumnBuffer {
    Buffer(PyBuffer<f64>),
    Owned(Vec<f64>),
}

impl ColumnBuffer {
    fn extract(column: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(buf) = PyBuffer::<f64>::get(column)
            && buf.as_slice_ptr().is_some()
        {
            return Ok(Self::Buffer(buf));
        }
        column.extract::<Vec<f64>>().map(Self::Owned)
    }

    fn as_slice(&self) -> &[f64] {
        match self {
            Self::Buffer(buf) => {
                if let Some(ptr) = buf.as_slice_ptr() {
                    // SAFETY: `buf` holds an active Python buffer export for the lifetime of `self`,
                    // ensuring memory remains allocated and valid, and `as_slice_ptr` verified matching
                    // f64 alignment, item size, and C-contiguity.
                    unsafe { slice::from_raw_parts(ptr.as_ptr().cast::<f64>(), buf.item_count()) }
                } else {
                    &[]
                }
            }
            Self::Owned(vec) => vec.as_slice(),
        }
    }
}

/// Each evaluator owns a mutable workspace and shares immutable instructions.
#[pyclass(name = "_Evaluator", module = "symb_anafis._native")]
#[derive(Clone, Debug)]
pub struct NativeEvaluator {
    backend: Backend,
}

#[derive(Clone, Debug)]
enum Backend {
    Number {
        program: Program<Number>,
        workspace: Workspace<Number>,
    },
    Float {
        program: Program<f64>,
        workspace: Workspace<f64>,
    },
}

impl NativeEvaluator {
    fn numbers(values: &Bound<'_, PyList>) -> PyResult<Vec<Number>> {
        let mut numbers = Vec::with_capacity(values.len());
        for value in values.iter() {
            numbers.push(NativeNumber::extract(&value)?);
        }
        Ok(numbers)
    }

    fn output<Value: Clone>(zero: Value, row_count: usize) -> PyResult<Vec<Value>> {
        let mut output = Vec::new();
        output
            .try_reserve_exact(row_count)
            .map_err(|error| PyMemoryError::new_err(error.to_string()))?;
        output.resize(row_count, zero);
        Ok(output)
    }
}

#[pymethods]
impl NativeEvaluator {
    #[new]
    fn new(
        expression: &NativeExpr,
        parameters: &Bound<'_, PyAny>,
        backend: &str,
        py: Python<'_>,
    ) -> PyResult<Self> {
        let owners = ExprInputs::new(parameters)?;
        let coordinates = owners.coordinates()?;
        let floating = match backend {
            "number" => false,
            "f64" => true,
            _ => return Err(PyValueError::new_err("backend must be 'number' or 'f64'")),
        };
        py.detach(|| -> Result<Backend, EvalError> {
            if floating {
                let program = Program::<f64>::compile(expression.value(), &coordinates)?;
                let workspace = program.workspace();
                Ok(Backend::Float { program, workspace })
            } else {
                let program = Program::<Number>::compile(expression.value(), &coordinates)?;
                let workspace = program.workspace();
                Ok(Backend::Number { program, workspace })
            }
        })
        .map(|backend| Self { backend })
        .map_err(|error| EvaluationError::new_err(error.to_string()))
    }

    #[staticmethod]
    fn direct(
        expression: &NativeExpr,
        parameters: &Bound<'_, PyAny>,
        values: &Bound<'_, PyList>,
        backend: &str,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        let owners = ExprInputs::new(parameters)?;
        let coordinates = owners.coordinates()?;
        match backend {
            "number" => {
                let numbers = Self::numbers(values)?;
                if numbers.len() != coordinates.len() {
                    return Err(EvaluationError::new_err(
                        "binding values and coordinate counts differ",
                    ));
                }
                let result = py
                    .detach(|| {
                        expression
                            .value()
                            .evaluate(coordinates.into_iter().zip(numbers))
                    })
                    .map_err(|error| EvaluationError::new_err(error.to_string()))?;
                NativeNumber::from(result).into_py_any(py)
            }
            "f64" => {
                let numbers = values.extract::<Vec<f64>>()?;
                if numbers.len() != coordinates.len() {
                    return Err(EvaluationError::new_err(
                        "binding values and coordinate counts differ",
                    ));
                }
                py.detach(|| {
                    expression
                        .value()
                        .evaluate(coordinates.into_iter().zip(numbers))
                })
                .map_err(|error| EvaluationError::new_err(error.to_string()))?
                .into_py_any(py)
            }
            _ => Err(PyValueError::new_err("backend must be 'number' or 'f64'")),
        }
    }

    fn evaluate(&mut self, values: &Bound<'_, PyAny>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match &mut self.backend {
            Backend::Number { program, workspace } => {
                let list = values.cast::<PyList>()?;
                let numbers = Self::numbers(list)?;
                let result = py
                    .detach(|| program.evaluate(workspace, &numbers))
                    .map_err(|error| EvaluationError::new_err(error.to_string()))?;
                NativeNumber::from(result).into_py_any(py)
            }
            Backend::Float { program, workspace } => {
                let buffer = ColumnBuffer::extract(values)?;
                let slice = buffer.as_slice();
                py.detach(|| program.evaluate(workspace, slice))
                    .map_err(|error| EvaluationError::new_err(error.to_string()))?
                    .into_py_any(py)
            }
        }
    }

    #[pyo3(signature = (columns, row_count, out = None))]
    fn evaluate_batch(
        &mut self,
        columns: &Bound<'_, PyAny>,
        row_count: usize,
        out: Option<&Bound<'_, PyAny>>,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        match &mut self.backend {
            Backend::Number { program, workspace } => {
                let columns_list = columns.cast::<PyList>()?;
                let values = columns_list
                    .iter()
                    .map(|column| Self::numbers(column.cast::<PyList>()?))
                    .collect::<PyResult<Vec<_>>>()?;
                let slices: Vec<_> = values.iter().map(Vec::as_slice).collect();
                let mut output = Self::output(Number::zero(), row_count)?;
                py.detach(|| program.evaluate_batch(workspace, &slices, &mut output))
                    .map_err(|error| EvaluationError::new_err(error.to_string()))?;
                output
                    .into_iter()
                    .map(NativeNumber::from)
                    .collect::<Vec<_>>()
                    .into_py_any(py)
            }
            Backend::Float { program, workspace } => {
                let col_buffers = columns
                    .try_iter()?
                    .map(|col| ColumnBuffer::extract(&col?))
                    .collect::<PyResult<Vec<_>>>()?;
                let slices: Vec<&[f64]> = col_buffers.iter().map(ColumnBuffer::as_slice).collect();

                if let Some(out_obj) = out {
                    let out_buf = PyBuffer::<f64>::get(out_obj)?;
                    if let Some(ptr) = out_buf.as_slice_ptr() {
                        let out_count = out_buf.item_count();
                        if out_count < row_count {
                            return Err(PyValueError::new_err(format!(
                                "output buffer size ({out_count}) is smaller than row_count ({row_count})"
                            )));
                        }
                        // SAFETY: `out_buf` holds an active, non-readonly Python buffer export for the duration of this call.
                        // `as_slice_ptr` verified matching f64 alignment, item size, and C-contiguity.
                        let full_slice: &mut [f64] = unsafe {
                            slice::from_raw_parts_mut(ptr.as_ptr().cast::<f64>(), out_count)
                        };
                        let target_slice = &mut full_slice[..row_count];
                        py.detach(|| program.evaluate_batch(workspace, &slices, target_slice))
                            .map_err(|error| EvaluationError::new_err(error.to_string()))?;
                        return Ok(out_obj.clone().unbind());
                    }
                    return Err(PyValueError::new_err(
                        "output buffer must be C-contiguous and writable",
                    ));
                }

                let mut output = Self::output(0.0, row_count)?;
                py.detach(|| program.evaluate_batch(workspace, &slices, &mut output))
                    .map_err(|error| EvaluationError::new_err(error.to_string()))?;
                output.into_py_any(py)
            }
        }
    }

    fn copy(&self) -> Self {
        self.clone()
    }

    fn parameters(&self) -> Vec<NativeExpr> {
        match &self.backend {
            Backend::Number { program, .. } => program
                .parameters()
                .map(|symbol| symbol.to_expr().into())
                .collect(),
            Backend::Float { program, .. } => program
                .parameters()
                .map(|symbol| symbol.to_expr().into())
                .collect(),
        }
    }

    fn instruction_count(&self) -> usize {
        match &self.backend {
            Backend::Number { program, .. } => program.instruction_count(),
            Backend::Float { program, .. } => program.instruction_count(),
        }
    }

    fn workspace_size(&self) -> usize {
        match &self.backend {
            Backend::Number { program, .. } => program.workspace_size(),
            Backend::Float { program, .. } => program.workspace_size(),
        }
    }

    fn disassemble(&self) -> String {
        match &self.backend {
            Backend::Number { program, .. } => program.disassemble().to_string(),
            Backend::Float { program, .. } => program.disassemble().to_string(),
        }
    }
}
