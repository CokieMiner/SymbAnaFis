//! Direct and reusable numerical execution without expression interning.

use pyo3::{
    Bound, IntoPyObjectExt, Py, PyAny, PyResult, Python,
    exceptions::{PyMemoryError, PyValueError},
    pyclass, pymethods,
    types::{PyAnyMethods, PyList, PyListMethods},
};
use symb_anafis::{Evaluator, Number};

use super::{EvaluationError, ExprInputs, NativeExpr, NativeNumber};

/// Each evaluator owns a mutable workspace and shares immutable instructions.
#[pyclass(name = "_Evaluator", module = "symb_anafis._native")]
#[derive(Clone, Debug)]
pub struct NativeEvaluator {
    program: Program,
}

#[derive(Clone, Debug)]
enum Program {
    Number(Evaluator<Number>),
    Float(Evaluator<f64>),
}

impl NativeEvaluator {
    fn numbers(values: &Bound<'_, PyList>) -> PyResult<Vec<Number>> {
        // The Python facade materializes each input column once. Reserve its
        // complete length instead of growing a generic iterator's output.
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
        py.detach(|| {
            if floating {
                Evaluator::<f64>::compile(expression.value(), &coordinates).map(Program::Float)
            } else {
                Evaluator::<Number>::compile(expression.value(), &coordinates).map(Program::Number)
            }
        })
        .map(|program| Self { program })
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

    fn evaluate(&mut self, values: &Bound<'_, PyList>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match &mut self.program {
            Program::Number(program) => {
                let numbers = Self::numbers(values)?;
                let result = py
                    .detach(|| program.evaluate(&numbers))
                    .map_err(|error| EvaluationError::new_err(error.to_string()))?;
                NativeNumber::from(result).into_py_any(py)
            }
            Program::Float(program) => {
                let numbers = values.extract::<Vec<f64>>()?;
                py.detach(|| program.evaluate(&numbers))
                    .map_err(|error| EvaluationError::new_err(error.to_string()))?
                    .into_py_any(py)
            }
        }
    }

    fn evaluate_batch(
        &mut self,
        columns: &Bound<'_, PyList>,
        row_count: usize,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        match &mut self.program {
            Program::Number(program) => {
                let values = columns
                    .iter()
                    .map(|column| Self::numbers(column.cast::<PyList>()?))
                    .collect::<PyResult<Vec<_>>>()?;
                let slices: Vec<_> = values.iter().map(Vec::as_slice).collect();
                let mut output = Self::output(Number::zero(), row_count)?;
                py.detach(|| program.evaluate_batch(&slices, &mut output))
                    .map_err(|error| EvaluationError::new_err(error.to_string()))?;
                output
                    .into_iter()
                    .map(NativeNumber::from)
                    .collect::<Vec<_>>()
                    .into_py_any(py)
            }
            Program::Float(program) => {
                let values = columns.extract::<Vec<Vec<f64>>>()?;
                let slices: Vec<_> = values.iter().map(Vec::as_slice).collect();
                let mut output = Self::output(0.0, row_count)?;
                py.detach(|| program.evaluate_batch(&slices, &mut output))
                    .map_err(|error| EvaluationError::new_err(error.to_string()))?;
                output.into_py_any(py)
            }
        }
    }

    fn copy(&self) -> Self {
        self.clone()
    }

    fn parameters(&self) -> Vec<NativeExpr> {
        match &self.program {
            Program::Number(program) => program
                .parameters()
                .map(|symbol| symbol.to_expr().into())
                .collect(),
            Program::Float(program) => program
                .parameters()
                .map(|symbol| symbol.to_expr().into())
                .collect(),
        }
    }

    fn instruction_count(&self) -> usize {
        match &self.program {
            Program::Number(program) => program.instruction_count(),
            Program::Float(program) => program.instruction_count(),
        }
    }

    const fn workspace_size(&self) -> usize {
        match &self.program {
            Program::Number(program) => program.workspace_size(),
            Program::Float(program) => program.workspace_size(),
        }
    }

    fn disassemble(&self) -> String {
        match &self.program {
            Program::Number(program) => program.disassemble().to_string(),
            Program::Float(program) => program.disassemble().to_string(),
        }
    }
}
