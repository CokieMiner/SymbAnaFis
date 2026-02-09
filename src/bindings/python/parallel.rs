//! Python bindings for parallel evaluation
//!
//! This module provides parallel evaluation capabilities when the "parallel" feature is enabled.

#[cfg(feature = "parallel")]
use crate::bindings::python::expr::PyExpr;
#[cfg(feature = "parallel")]
use crate::parallel::{self, ExprInput, Value, VarInput};
#[cfg(feature = "parallel")]
use numpy::PyReadonlyArray1;
#[cfg(feature = "parallel")]
use pyo3::prelude::*;

/// Parallel evaluation functionality (only available with "parallel" feature)
#[cfg(feature = "parallel")]
pub mod parallel_impl {
    use super::{
        Bound, ExprInput, IntoPyObject, Py, PyAny, PyAnyMethods, PyErr, PyExpr, PyReadonlyArray1,
        PyResult, Python, Value, VarInput, parallel, pyfunction,
    };

    /// Parallel evaluation of multiple expressions at multiple points.
    #[pyfunction]
    #[pyo3(name = "evaluate_parallel")]
    #[allow(
        clippy::too_many_lines,
        reason = "Complex dispatch logic, length is justified"
    )]
    pub fn evaluate_parallel(
        py: Python<'_>,
        expressions: Vec<Bound<'_, PyAny>>,
        variables: Vec<Vec<String>>,
        values: Vec<Vec<Bound<'_, PyAny>>>,
    ) -> PyResult<Vec<Vec<Py<PyAny>>>> {
        // Track which expressions were Expr vs String for output type
        let mut was_expr: Vec<bool> = Vec::with_capacity(expressions.len());

        // Convert expressions to ExprInput, tracking input type
        let exprs: Vec<ExprInput> = expressions
            .into_iter()
            .map(|e| {
                if let Ok(py_expr) = e.extract::<PyExpr>() {
                    was_expr.push(true);
                    Ok(ExprInput::Parsed(py_expr.0))
                } else if let Ok(s) = e.extract::<String>() {
                    was_expr.push(false);
                    Ok(ExprInput::String(s))
                } else {
                    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                        "expressions must be str or Expr",
                    ))
                }
            })
            .collect::<PyResult<Vec<_>>>()?;

        let vars: Vec<Vec<VarInput>> = variables
            .into_iter()
            .map(|vs| vs.into_iter().map(VarInput::from).collect())
            .collect();

        // Track if all values for each expression are numeric (no None/Skip values)
        let mut is_fully_numeric: Vec<bool> = Vec::with_capacity(values.len());

        // Convert values - supports both lists and NumPy arrays
        let converted_values: Vec<Vec<Vec<Value>>> = values
            .into_iter()
            .map(|expr_vals| {
                let mut expr_is_numeric = true;
                let converted: Vec<Vec<Value>> = expr_vals
                    .into_iter()
                    .map(|var_vals| {
                        // Try NumPy array first (zero-copy path)
                        if let Ok(arr) = var_vals.extract::<PyReadonlyArray1<f64>>()
                            && let Ok(slice) = arr.as_slice()
                        {
                            return slice.iter().map(|&n| Value::Num(n)).collect();
                        }
                        // Fallback to Python list with Option<f64>
                        if let Ok(list) = var_vals.extract::<Vec<Option<f64>>>() {
                            // Side-effect in else branch makes map_or_else unsuitable
                            #[allow(
                                clippy::option_if_let_else,
                                reason = "Side-effect in else branch makes map_or_else unsuitable"
                            )]
                            return list
                                .into_iter()
                                .map(|v| {
                                    if let Some(n) = v {
                                        Value::Num(n)
                                    } else {
                                        expr_is_numeric = false;
                                        Value::Skip
                                    }
                                })
                                .collect();
                        }
                        // Try pure f64 list (no None values)
                        if let Ok(list) = var_vals.extract::<Vec<f64>>() {
                            return list.into_iter().map(Value::Num).collect();
                        }
                        // Empty fallback
                        expr_is_numeric = false;
                        vec![]
                    })
                    .collect();
                is_fully_numeric.push(expr_is_numeric);
                converted
            })
            .collect();

        // Use the hint-based version to skip double-scan
        let results: Vec<Vec<parallel::EvalResult>> = parallel::evaluate_parallel_with_hint(
            exprs,
            vars,
            converted_values,
            Some(is_fully_numeric),
        )?;

        // Convert Rust results to Python objects
        results
            .into_iter()
            .zip(was_expr.iter())
            .map(|(expr_results, &input_was_expr)| {
                expr_results
                    .into_iter()
                    .map(|r| -> PyResult<Py<PyAny>> {
                        Ok(match r {
                            parallel::EvalResult::String(s) => {
                                if let Ok(n) = s.parse::<f64>() {
                                    // Numeric result → float
                                    n.into_pyobject(py)?.into_any().unbind()
                                } else {
                                    // Symbolic result → str
                                    s.into_pyobject(py)?.into_any().unbind()
                                }
                            }
                            parallel::EvalResult::Expr(e) => {
                                if let crate::ExprKind::Number(n) = &e.kind {
                                    // Numeric result → float
                                    n.into_pyobject(py)?.into_any().unbind()
                                } else if input_was_expr {
                                    // Symbolic result, input was Expr → Expr
                                    PyExpr(e).into_pyobject(py)?.into_any().unbind()
                                } else {
                                    // Symbolic result, input was str → str
                                    e.to_string().into_pyobject(py)?.into_any().unbind()
                                }
                            }
                        })
                    })
                    .collect::<PyResult<Vec<_>>>()
            })
            .collect::<PyResult<Vec<_>>>()
    }

    /// High-performance parallel batch evaluation for multiple expressions.
    ///
    /// This uses SIMD and parallel processing for maximum performance.
    /// Best for pure numeric workloads with many data points.
    ///
    /// Args:
    ///     expressions: List of Expr objects
    ///     `var_names`: List of variable name lists, one per expression
    ///     data: 3D list of values: data[expr_idx][var_idx] = [values...]
    ///
    /// Returns:
    ///     2D `NumPy` array or list of results: result[expr_idx][point_idx]
    #[pyfunction]
    #[pyo3(name = "eval_f64")]
    #[allow(
        clippy::needless_pass_by_value,
        reason = "PyO3 requires owned Vec for Python list arguments"
    )]
    pub fn eval_f64<'py>(
        py: Python<'py>,
        expressions: Vec<PyExpr>,
        var_names: Vec<Vec<String>>,
        data: Vec<Vec<Bound<'py, PyAny>>>,
    ) -> PyResult<Py<PyAny>> {
        use crate::Expr as RustExpr;
        use crate::bindings::eval_f64::eval_f64 as rust_eval_f64;
        use crate::bindings::python::evaluator::{DataInput, extract_data_input};
        use numpy::{PyArray1, PyArray2, PyArrayMethods};

        let expr_refs: Vec<&RustExpr> = expressions.iter().map(|e| &e.0).collect();

        // Convert var_names to the required format
        let var_refs: Vec<Vec<&str>> = var_names
            .iter()
            .map(|vs| vs.iter().map(std::string::String::as_str).collect())
            .collect();
        let var_slice_refs: Vec<&[&str]> = var_refs.iter().map(std::vec::Vec::as_slice).collect();

        // Convert data to the required format (Zero-Copy or List)
        let data_inputs: Vec<Vec<DataInput>> = data
            .iter()
            .map(|expr_data| {
                expr_data
                    .iter()
                    .map(extract_data_input)
                    .collect::<PyResult<Vec<_>>>()
            })
            .collect::<PyResult<Vec<_>>>()?;

        // Check if any input is a NumPy array to decide output format
        let use_numpy = data_inputs
            .iter()
            .flat_map(|row: &Vec<DataInput>| row.iter())
            .any(|d| matches!(d, DataInput::Array(_)));

        let data_refs: Vec<Vec<&[f64]>> = data_inputs
            .iter()
            .map(|expr_data| {
                expr_data
                    .iter()
                    .map(|col| col.as_slice())
                    .collect::<PyResult<Vec<_>>>()
            })
            .collect::<PyResult<Vec<_>>>()?;
        let data_slice_refs: Vec<&[&[f64]]> =
            data_refs.iter().map(std::vec::Vec::as_slice).collect();

        let results =
            rust_eval_f64(&expr_refs, &var_slice_refs, &data_slice_refs).map_err(PyErr::from)?;

        if use_numpy {
            // Convert Vec<Vec<f64>> to 2D NumPy array
            if results.is_empty() {
                return Ok(PyArray2::<f64>::zeros(py, [0, 0], false)
                    .into_any()
                    .unbind());
            }

            let n_exprs = results.len();
            let n_points = results[0].len();

            // Flatten the results into a single contiguous vector
            let flat_results: Vec<f64> = results.into_iter().flatten().collect();

            // Create the 2D array from the valid vector
            let arr = PyArray1::from_vec(py, flat_results)
                .reshape([n_exprs, n_points])
                .map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Failed to reshape output: {e:?}"
                    ))
                })?;

            Ok(arr.into_any().unbind())
        } else {
            // Return standard Python list of lists
            Ok(results.into_pyobject(py)?.into_any().unbind())
        }
    }
}

#[cfg(feature = "parallel")]
pub use parallel_impl::*;
