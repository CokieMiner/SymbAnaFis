//! Direct and reusable numerical execution (VM, native JIT, Rayon batches, multi-output, and WASM export).

use pyo3::{
    Bound, IntoPyObjectExt, Py, PyAny, PyResult, Python,
    exceptions::{PyTypeError, PyValueError},
    pyclass, pymethods,
    types::{PyBytes, PyList, PyListMethods},
};
use symb_anafis::{
    Complex, EvalError, Expr, MultiProgram, NativeScalar, Number, Program, Symbol, WasmScalar,
    Workspace,
};

use super::{EvaluationError, ExprInputs, NativeExpr, NativeNumber};

use buffer::{ColumnBuffer, NativeElement, alloc_output, eval_typed_batch, ptrs_overlap};
use multi::{
    eval_number_batch_multi, eval_number_scalar_multi, eval_typed_batch_multi,
    eval_typed_scalar_multi,
};

mod buffer;
mod multi;

const BACKEND_HELP: &str = "backend must be 'number', 'f64', 'f32', 'complex128', or 'complex64'";

/// Dispatch a read-only expression uniformly across all `Backend` variants.
macro_rules! dispatch_program {
    ($backend:expr, |$program:ident| $body:expr) => {
        match $backend {
            Backend::Number {
                program: $program, ..
            } => $body,
            Backend::F64 {
                program: $program, ..
            } => $body,
            Backend::F32 {
                program: $program, ..
            } => $body,
            Backend::Complex64 {
                program: $program, ..
            } => $body,
            Backend::Complex32 {
                program: $program, ..
            } => $body,
        }
    };
}

/// Dispatch an identical method expression across `Compiled::Single` and `Compiled::Multi`.
macro_rules! on_compiled {
    ($compiled:expr, |$inner:ident| $body:expr) => {
        match $compiled {
            Compiled::Single($inner) => $body,
            Compiled::Multi($inner) => $body,
        }
    };
}

/// Compile a single- or multi-output program and initial workspace into a `Backend` variant.
macro_rules! compile_backend {
    (single, $py:expr, $ty:ty, $variant:ident, $expr:expr, $coords:expr) => {
        $py.detach(|| {
            let program = Program::<$ty>::compile($expr, $coords)?;
            Ok(Backend::$variant {
                program: Compiled::Single(program),
                workspace: Workspace::new(),
            })
        })
    };
    (multi, $py:expr, $ty:ty, $variant:ident, $exprs:expr, $coords:expr) => {
        $py.detach(|| {
            let program = MultiProgram::<$ty>::compile($exprs, $coords)?;
            Ok(Backend::$variant {
                program: Compiled::Multi(program),
                workspace: Workspace::new(),
            })
        })
    };
}

/// Upgrade a native-capable `Compiled<T>` to host machine code into a `Backend` variant.
macro_rules! upgrade_native {
    ($py:expr, $variant:ident, $program:expr) => {
        $py.detach(|| {
            let native = $program.with_native()?;
            Ok(Backend::$variant {
                program: native,
                workspace: Workspace::new(),
            })
        })
    };
}

/// Each evaluator owns a mutable workspace and shares immutable instructions.
#[pyclass(name = "_Evaluator", module = "symb_anafis._native")]
#[derive(Clone, Debug)]
pub struct NativeEvaluator {
    backend: Backend,
}

#[derive(Clone, Debug)]
enum Compiled<Value> {
    Single(Program<Value>),
    Multi(MultiProgram<Value>),
}

impl<Value: NativeScalar> Compiled<Value> {
    fn with_native(&self) -> Result<Self, EvalError> {
        match self {
            Self::Single(program) => program.with_native().map(Self::Single),
            Self::Multi(program) => program.with_native().map(Self::Multi),
        }
    }
}

impl<Value: WasmScalar> Compiled<Value> {
    fn to_wasm_bytes(&self) -> Result<Vec<u8>, EvalError> {
        on_compiled!(self, |program| program.to_wasm_bytes())
    }
}

#[derive(Clone, Debug)]
enum Backend {
    Number {
        program: Compiled<Number>,
        workspace: Workspace<Number>,
    },
    F64 {
        program: Compiled<f64>,
        workspace: Workspace<f64>,
    },
    F32 {
        program: Compiled<f32>,
        workspace: Workspace<f32>,
    },
    Complex64 {
        program: Compiled<Complex<f64>>,
        workspace: Workspace<Complex<f64>>,
    },
    Complex32 {
        program: Compiled<Complex<f32>>,
        workspace: Workspace<Complex<f32>>,
    },
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
        let expr = expression.value();
        let compiled: Result<Backend, EvalError> = match backend {
            "number" => compile_backend!(single, py, Number, Number, expr, &coordinates),
            "f64" => compile_backend!(single, py, f64, F64, expr, &coordinates),
            "f32" => compile_backend!(single, py, f32, F32, expr, &coordinates),
            "complex128" => {
                compile_backend!(single, py, Complex<f64>, Complex64, expr, &coordinates)
            }
            "complex64" => {
                compile_backend!(single, py, Complex<f32>, Complex32, expr, &coordinates)
            }
            _ => return Err(PyValueError::new_err(BACKEND_HELP)),
        };
        compiled
            .map(|backend| Self { backend })
            .map_err(|error| EvaluationError::new_err(error.to_string()))
    }

    #[staticmethod]
    fn multi(
        expressions: &Bound<'_, PyAny>,
        parameters: &Bound<'_, PyAny>,
        backend: &str,
        py: Python<'_>,
    ) -> PyResult<Self> {
        let exprs = ExprInputs::new(expressions)?.into_expressions();
        let owners = ExprInputs::new(parameters)?;
        let coordinates = owners.coordinates()?;
        let compiled: Result<Backend, EvalError> = match backend {
            "number" => compile_backend!(multi, py, Number, Number, &exprs, &coordinates),
            "f64" => compile_backend!(multi, py, f64, F64, &exprs, &coordinates),
            "f32" => compile_backend!(multi, py, f32, F32, &exprs, &coordinates),
            "complex128" => {
                compile_backend!(multi, py, Complex<f64>, Complex64, &exprs, &coordinates)
            }
            "complex64" => {
                compile_backend!(multi, py, Complex<f32>, Complex32, &exprs, &coordinates)
            }
            _ => return Err(PyValueError::new_err(BACKEND_HELP)),
        };
        compiled
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
        if values.len() != coordinates.len() {
            return Err(EvaluationError::new_err(
                "binding values and coordinate counts differ",
            ));
        }
        let expr = expression.value();
        match backend {
            "number" => {
                let numbers = extract_numbers(values)?;
                let result = py
                    .detach(|| expr.evaluate(coordinates.into_iter().zip(numbers)))
                    .map_err(|error| EvaluationError::new_err(error.to_string()))?;
                NativeNumber::from(result).into_py_any(py)
            }
            "f64" => eval_typed_direct::<f64>(expr, coordinates, values, py),
            "f32" => eval_typed_direct::<f32>(expr, coordinates, values, py),
            "complex128" => eval_typed_direct::<Complex<f64>>(expr, coordinates, values, py),
            "complex64" => eval_typed_direct::<Complex<f32>>(expr, coordinates, values, py),
            _ => Err(PyValueError::new_err(BACKEND_HELP)),
        }
    }

    fn evaluate(&mut self, values: &Bound<'_, PyAny>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match &mut self.backend {
            Backend::Number { program, workspace } => {
                let numbers = extract_numbers(values.cast::<PyList>()?)?;
                match program {
                    Compiled::Single(prog) => {
                        let result = py
                            .detach(|| prog.eval_with(workspace, &numbers))
                            .map_err(|error| EvaluationError::new_err(error.to_string()))?;
                        NativeNumber::from(result).into_py_any(py)
                    }
                    Compiled::Multi(prog) => {
                        eval_number_scalar_multi(prog, workspace, &numbers, py)
                    }
                }
            }
            Backend::F64 { program, workspace } => {
                eval_compiled_scalar(program, workspace, values, py)
            }
            Backend::F32 { program, workspace } => {
                eval_compiled_scalar(program, workspace, values, py)
            }
            Backend::Complex64 { program, workspace } => {
                eval_compiled_scalar(program, workspace, values, py)
            }
            Backend::Complex32 { program, workspace } => {
                eval_compiled_scalar(program, workspace, values, py)
            }
        }
    }

    #[pyo3(signature = (columns, row_count, out = None, parallel = false))]
    fn evaluate_batch(
        &mut self,
        columns: &Bound<'_, PyAny>,
        row_count: usize,
        out: Option<&Bound<'_, PyAny>>,
        parallel: bool,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        match &mut self.backend {
            Backend::Number { program, workspace } => {
                if out.is_some() {
                    return Err(PyTypeError::new_err(
                        "out buffer is only supported for floating-point and complex backends",
                    ));
                }
                let values = columns
                    .cast::<PyList>()?
                    .iter()
                    .map(|column| extract_numbers(column.cast::<PyList>()?))
                    .collect::<PyResult<Vec<_>>>()?;
                let slices: Vec<_> = values.iter().map(Vec::as_slice).collect();
                match program {
                    Compiled::Single(prog) => {
                        let mut output = alloc_output(Number::zero(), row_count)?;
                        py.detach(|| {
                            if parallel {
                                prog.eval_batch_par(&slices, &mut output)
                            } else {
                                prog.eval_batch_with(workspace, &slices, &mut output)
                            }
                        })
                        .map_err(|error| EvaluationError::new_err(error.to_string()))?;
                        output
                            .into_iter()
                            .map(NativeNumber::from)
                            .collect::<Vec<_>>()
                            .into_py_any(py)
                    }
                    Compiled::Multi(prog) => {
                        eval_number_batch_multi(prog, workspace, &slices, row_count, parallel, py)
                    }
                }
            }
            Backend::F64 { program, workspace } => {
                eval_compiled_batch(program, workspace, columns, row_count, out, parallel, py)
            }
            Backend::F32 { program, workspace } => {
                eval_compiled_batch(program, workspace, columns, row_count, out, parallel, py)
            }
            Backend::Complex64 { program, workspace } => {
                eval_compiled_batch(program, workspace, columns, row_count, out, parallel, py)
            }
            Backend::Complex32 { program, workspace } => {
                eval_compiled_batch(program, workspace, columns, row_count, out, parallel, py)
            }
        }
    }

    fn with_native(&self, py: Python<'_>) -> PyResult<Self> {
        let upgraded: Result<Backend, EvalError> = match &self.backend {
            Backend::Number { .. } => {
                return Err(EvaluationError::new_err(
                    "native JIT compilation is only supported for f64, f32, complex128, and complex64 backends",
                ));
            }
            Backend::F64 { program, .. } => upgrade_native!(py, F64, program),
            Backend::F32 { program, .. } => upgrade_native!(py, F32, program),
            Backend::Complex64 { program, .. } => upgrade_native!(py, Complex64, program),
            Backend::Complex32 { program, .. } => upgrade_native!(py, Complex32, program),
        };
        upgraded
            .map(|backend| Self { backend })
            .map_err(|error| EvaluationError::new_err(error.to_string()))
    }

    fn native_bytes<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        let bytes = dispatch_program!(&self.backend, |program| {
            on_compiled!(program, |inner| inner.native_bytes())
        });
        bytes.map(|slice| PyBytes::new(py, slice))
    }

    fn to_wasm_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = match &self.backend {
            Backend::F64 { program, .. } => py.detach(|| program.to_wasm_bytes()),
            Backend::F32 { program, .. } => py.detach(|| program.to_wasm_bytes()),
            Backend::Number { .. } | Backend::Complex64 { .. } | Backend::Complex32 { .. } => {
                return Err(EvaluationError::new_err(
                    "WebAssembly export is only supported for f64 and f32 backends",
                ));
            }
        }
        .map_err(|error| EvaluationError::new_err(error.to_string()))?;
        Ok(PyBytes::new(py, &bytes))
    }

    fn copy(&self) -> Self {
        self.clone()
    }

    fn parameters(&self) -> Vec<NativeExpr> {
        dispatch_program!(&self.backend, |program| {
            on_compiled!(program, |inner| {
                inner.parameters().map(|sym| sym.to_expr().into()).collect()
            })
        })
    }

    fn instruction_count(&self) -> usize {
        dispatch_program!(&self.backend, |program| {
            on_compiled!(program, |inner| inner.instruction_count())
        })
    }

    fn workspace_size(&self) -> usize {
        dispatch_program!(&self.backend, |program| {
            on_compiled!(program, |inner| inner.workspace_size())
        })
    }

    fn disassemble(&self) -> String {
        dispatch_program!(&self.backend, |program| {
            on_compiled!(program, |inner| inner.disassemble().to_string())
        })
    }
}

fn eval_compiled_scalar<T: NativeElement>(
    program: &Compiled<T>,
    workspace: &mut Workspace<T>,
    values: &Bound<'_, PyAny>,
    py: Python<'_>,
) -> PyResult<Py<PyAny>> {
    match program {
        Compiled::Single(prog) => {
            let buffer = ColumnBuffer::<T>::extract(values)?;
            let slice = buffer.as_slice();
            let result = py
                .detach(|| prog.eval_with(workspace, slice))
                .map_err(|error| EvaluationError::new_err(error.to_string()))?;
            result.into_py_scalar(py)
        }
        Compiled::Multi(prog) => eval_typed_scalar_multi(prog, workspace, values, py),
    }
}

fn eval_compiled_batch<T: NativeElement>(
    program: &Compiled<T>,
    workspace: &mut Workspace<T>,
    columns: &Bound<'_, PyAny>,
    row_count: usize,
    out: Option<&Bound<'_, PyAny>>,
    parallel: bool,
    py: Python<'_>,
) -> PyResult<Py<PyAny>> {
    match program {
        Compiled::Single(prog) => {
            eval_typed_batch(prog, workspace, columns, row_count, out, parallel, py)
        }
        Compiled::Multi(prog) => {
            eval_typed_batch_multi(prog, workspace, columns, row_count, out, parallel, py)
        }
    }
}

fn eval_typed_direct<T: NativeElement>(
    expr: &Expr,
    coordinates: Vec<Symbol<'_>>,
    values: &Bound<'_, PyList>,
    py: Python<'_>,
) -> PyResult<Py<PyAny>> {
    let scalars = values
        .iter()
        .map(|item| T::extract_scalar(&item))
        .collect::<PyResult<Vec<_>>>()?;
    let result = py
        .detach(|| expr.evaluate(coordinates.into_iter().zip(scalars)))
        .map_err(|error| EvaluationError::new_err(error.to_string()))?;
    result.into_py_scalar(py)
}

fn extract_numbers(values: &Bound<'_, PyList>) -> PyResult<Vec<Number>> {
    let mut numbers = Vec::with_capacity(values.len());
    for value in values.iter() {
        numbers.push(NativeNumber::extract(&value)?);
    }
    Ok(numbers)
}
