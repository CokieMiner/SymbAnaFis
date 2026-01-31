//! Python bindings for differentiation and simplification builders
//!
//! This module provides the `PyDiff` and `PySimplify` classes which wrap
//! Rust's builder types for fine-grained control over operations.

use crate::Expr as RustExpr;
use crate::api::builder;
use pyo3::prelude::*;
use std::sync::Arc;

/// Type alias for complex partial derivative function type to improve readability
type PartialDerivativeFn = Arc<dyn Fn(&[Arc<RustExpr>]) -> RustExpr + Send + Sync>;

/// Builder for differentiation operations
#[pyclass(name = "Diff")]
pub struct PyDiff {
    pub inner: builder::Diff,
}

#[pymethods]
impl PyDiff {
    #[new]
    fn new() -> Self {
        Self {
            inner: builder::Diff::new(),
        }
    }

    fn domain_safe(mut self_: PyRefMut<'_, Self>, safe: bool) -> PyRefMut<'_, Self> {
        self_.inner = self_.inner.clone().domain_safe(safe);
        self_
    }

    fn fixed_var<'py>(
        mut self_: PyRefMut<'py, Self>,
        var: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        if let Ok(s) = var.extract::<String>() {
            self_.inner = self_.inner.clone().fixed_var(&s);
        } else if let Ok(sym) = var.extract::<super::symbol::PySymbol>() {
            self_.inner = self_.inner.clone().fixed_var(&sym.0);
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "fixed_var requires a string or Symbol object.",
            ));
        }
        Ok(self_)
    }

    fn fixed_vars<'py>(
        mut self_: PyRefMut<'py, Self>,
        vars: Vec<Bound<'_, PyAny>>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        for var in vars {
            if let Ok(s) = var.extract::<String>() {
                self_.inner = self_.inner.clone().fixed_var(&s);
            } else if let Ok(sym) = var.extract::<super::symbol::PySymbol>() {
                self_.inner = self_.inner.clone().fixed_var(&sym.0);
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "fixed_vars requires a list of strings or Symbol objects.",
                ));
            }
        }
        Ok(self_)
    }

    fn max_depth(mut self_: PyRefMut<'_, Self>, depth: usize) -> PyRefMut<'_, Self> {
        self_.inner = self_.inner.clone().max_depth(depth);
        self_
    }

    fn max_nodes(mut self_: PyRefMut<'_, Self>, nodes: usize) -> PyRefMut<'_, Self> {
        self_.inner = self_.inner.clone().max_nodes(nodes);
        self_
    }

    fn with_context<'py>(
        mut self_: PyRefMut<'py, Self>,
        context: &'py super::context::PyContext,
    ) -> PyRefMut<'py, Self> {
        self_.inner = self_.inner.clone().with_context(&context.inner);
        self_
    }

    /// Register a user-defined function with optional body and partial derivatives.
    #[pyo3(signature = (name, arity, body_callback=None, partials=None))]
    fn user_fn(
        mut self_: PyRefMut<'_, Self>,
        name: String,
        arity: usize,
        body_callback: Option<Py<PyAny>>,
        partials: Option<Vec<Py<PyAny>>>,
    ) -> PyResult<PyRefMut<'_, Self>> {
        use crate::core::unified_context::BodyFn;
        use crate::core::unified_context::UserFunction;

        let mut user_fn = UserFunction::new(arity..=arity);

        if let Some(callback) = body_callback {
            let body_fn: BodyFn = Arc::new(move |args: &[Arc<RustExpr>]| -> RustExpr {
                Python::attach(|py| {
                    let py_args: Vec<super::expr::PyExpr> = args
                        .iter()
                        .map(|a| super::expr::PyExpr((**a).clone()))
                        .collect();
                    let Ok(py_list) = py_args.into_pyobject(py) else {
                        return RustExpr::number(0.0);
                    };

                    let result = callback.call1(py, (py_list,));

                    result.map_or_else(
                        |_| RustExpr::number(0.0),
                        |res| {
                            res.extract::<super::expr::PyExpr>(py)
                                .map_or_else(|_| RustExpr::number(0.0), |py_expr| py_expr.0)
                        },
                    )
                })
            });
            user_fn = user_fn.body_arc(body_fn);
        }

        if let Some(callbacks) = partials {
            for (i, callback) in callbacks.into_iter().enumerate() {
                let partial_fn: PartialDerivativeFn =
                    Arc::new(move |args: &[Arc<RustExpr>]| -> RustExpr {
                        Python::attach(|py| {
                            let py_args: Vec<super::expr::PyExpr> = args
                                .iter()
                                .map(|a| super::expr::PyExpr((**a).clone()))
                                .collect();
                            let Ok(py_list) = py_args.into_pyobject(py) else {
                                return RustExpr::number(0.0);
                            };

                            let result = callback.call1(py, (py_list,));

                            result.map_or_else(
                                |_| RustExpr::number(0.0),
                                |res| {
                                    res.extract::<super::expr::PyExpr>(py)
                                        .map_or_else(|_| RustExpr::number(0.0), |py_expr| py_expr.0)
                                },
                            )
                        })
                    });

                user_fn = user_fn.partial_arc(i, partial_fn).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e:?}"))
                })?;
            }
        }

        self_.inner = self_.inner.clone().user_fn(name, user_fn);
        Ok(self_)
    }

    fn diff_str(&self, formula: &str, var: &str) -> PyResult<String> {
        self.inner.diff_str(formula, var, &[]).map_err(Into::into)
    }

    fn differentiate(
        &self,
        expr: &Bound<'_, PyAny>,
        var: &Bound<'_, PyAny>,
    ) -> PyResult<super::expr::PyExpr> {
        let rust_expr = super::expr::extract_to_expr(expr)?;
        let sym = if let Ok(var_str) = var.extract::<String>() {
            crate::symb(&var_str)
        } else if let Ok(var_sym) = var.extract::<super::symbol::PySymbol>() {
            var_sym.0
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "Variable must be a string or Symbol",
            ));
        };

        self.inner
            .differentiate(&rust_expr, &sym)
            .map(super::expr::PyExpr)
            .map_err(Into::into)
    }
}

/// Builder for simplification operations
#[pyclass(name = "Simplify")]
pub struct PySimplify {
    pub inner: builder::Simplify,
}

#[pymethods]
impl PySimplify {
    #[new]
    fn new() -> Self {
        Self {
            inner: builder::Simplify::new(),
        }
    }

    fn domain_safe(mut self_: PyRefMut<'_, Self>, safe: bool) -> PyRefMut<'_, Self> {
        self_.inner = self_.inner.clone().domain_safe(safe);
        self_
    }

    fn fixed_var<'py>(
        mut self_: PyRefMut<'py, Self>,
        var: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        if let Ok(s) = var.extract::<String>() {
            self_.inner = self_.inner.clone().fixed_var(&s);
        } else if let Ok(sym) = var.extract::<super::symbol::PySymbol>() {
            self_.inner = self_.inner.clone().fixed_var(&sym.0);
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "fixed_var requires a string or Symbol object.",
            ));
        }
        Ok(self_)
    }

    fn fixed_vars<'py>(
        mut self_: PyRefMut<'py, Self>,
        vars: Vec<Bound<'_, PyAny>>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        for var in vars {
            if let Ok(s) = var.extract::<String>() {
                self_.inner = self_.inner.clone().fixed_var(&s);
            } else if let Ok(sym) = var.extract::<super::symbol::PySymbol>() {
                self_.inner = self_.inner.clone().fixed_var(&sym.0);
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "fixed_vars requires a list of strings or Symbol objects.",
                ));
            }
        }
        Ok(self_)
    }

    fn max_depth(mut self_: PyRefMut<'_, Self>, depth: usize) -> PyRefMut<'_, Self> {
        self_.inner = self_.inner.clone().max_depth(depth);
        self_
    }

    fn max_nodes(mut self_: PyRefMut<'_, Self>, nodes: usize) -> PyRefMut<'_, Self> {
        self_.inner = self_.inner.clone().max_nodes(nodes);
        self_
    }

    fn with_context<'py>(
        mut self_: PyRefMut<'py, Self>,
        context: &'py super::context::PyContext,
    ) -> PyRefMut<'py, Self> {
        self_.inner = self_.inner.clone().with_context(&context.inner);
        self_
    }

    fn simplify(&self, expr: &Bound<'_, PyAny>) -> PyResult<super::expr::PyExpr> {
        let rust_expr = super::expr::extract_to_expr(expr)?;
        self.inner
            .simplify(&rust_expr)
            .map(super::expr::PyExpr)
            .map_err(Into::into)
    }

    fn simplify_str(&self, formula: &str) -> PyResult<String> {
        self.inner.simplify_str(formula, &[]).map_err(Into::into)
    }
}
