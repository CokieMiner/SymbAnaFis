//! Python bindings for expression contexts
//!
//! This module provides context management for isolated symbol and function registries.

use crate::Expr as RustExpr;
use crate::core::unified_context::Context as RustContext;
use pyo3::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;

/// Type alias for complex partial derivative function type to improve readability
type PartialDerivativeFn = Arc<dyn Fn(&[Arc<RustExpr>]) -> RustExpr + Send + Sync>;

/// Python wrapper for expression contexts
#[pyclass(unsendable, name = "Context")]
pub struct PyContext {
    pub inner: RustContext,
}

#[pymethods]
impl PyContext {
    /// Create a new empty context
    #[new]
    fn new() -> Self {
        Self {
            inner: RustContext::new(),
        }
    }

    /// Create or get a symbol in this context
    fn symb(&self, name: &str) -> super::symbol::PySymbol {
        super::symbol::PySymbol(self.inner.symb(name))
    }

    /// Create a new symbol (fails if name already exists in this context)
    fn symb_new(&self, name: &str) -> PyResult<super::symbol::PySymbol> {
        if self.inner.contains_symbol(name) {
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Symbol '{name}' already exists in this context"
            )))
        } else {
            Ok(super::symbol::PySymbol(self.inner.symb(name)))
        }
    }

    /// Get an existing symbol by name (returns None if not found)
    fn get_symbol(&self, name: &str) -> Option<super::symbol::PySymbol> {
        self.inner.get_symbol(name).map(super::symbol::PySymbol)
    }

    /// Check if a symbol exists in this context
    fn contains_symbol(&self, name: &str) -> bool {
        self.inner.contains_symbol(name)
    }

    /// Get the number of symbols in this context
    fn symbol_count(&self) -> usize {
        self.inner.symbol_names().len()
    }

    /// Check if context is empty
    fn is_empty(&self) -> bool {
        self.inner.symbol_names().is_empty()
    }

    /// Get all symbol names in this context
    fn symbol_names(&self) -> Vec<String> {
        self.inner.symbol_names()
    }

    /// Get the context's unique ID
    const fn id(&self) -> u64 {
        self.inner.id()
    }

    /// Remove a symbol from the context
    fn remove_symbol(&mut self, name: &str) -> bool {
        self.inner.remove_symbol(name)
    }

    /// Clear all symbols and functions from the context
    fn clear_all(&mut self) {
        self.inner.clear_all();
    }

    /// Register a user function with optional body and partial derivatives.
    // PyO3 requires owned String for name parameter
    #[allow(
        clippy::needless_pass_by_value,
        reason = "PyO3 requires owned String for name parameter"
    )]
    #[pyo3(signature = (name, arity, body_callback=None, partials=None))]
    fn with_function(
        mut self_: PyRefMut<'_, Self>,
        name: String,
        arity: usize,
        body_callback: Option<Py<PyAny>>,
        partials: Option<Vec<Py<PyAny>>>,
    ) -> PyResult<PyRefMut<'_, Self>> {
        use crate::core::unified_context::UserFunction;
        use std::sync::Arc;

        let mut user_fn = UserFunction::new(arity..=arity);

        // Handle optional body function
        if let Some(callback) = body_callback {
            let body_fn: crate::BodyFn = Arc::new(move |args: &[Arc<RustExpr>]| -> RustExpr {
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

        // Handle optional list of partial derivatives
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

        self_.inner = self_.inner.clone().with_function(&name, user_fn);
        Ok(self_)
    }

    fn __repr__(&self) -> String {
        format!(
            "Context(id={}, symbols={})",
            self.inner.id(),
            self.inner.symbol_names().len()
        )
    }
}

/// Custom function context for tracking custom function names.
///
/// This tracks custom function names for parsing. For full custom function
/// support with evaluation and derivatives, use the `Diff.custom_derivative` API.
#[pyclass(unsendable, name = "FunctionContext")]
pub struct PyFunctionContext {
    /// Set of registered custom function names
    functions: HashSet<String>,
}

#[pymethods]
impl PyFunctionContext {
    #[new]
    fn new() -> Self {
        Self {
            functions: HashSet::new(),
        }
    }

    /// Register a custom function name (for parsing support).
    ///
    /// For actual evaluation/differentiation, use `Diff.custom_derivative()`.
    ///
    /// Args:
    ///     name: Function name to register
    fn register(&mut self, name: String) {
        self.functions.insert(name);
    }

    /// Check if a function is registered
    fn contains(&self, name: &str) -> bool {
        self.functions.contains(name)
    }

    /// Get all registered function names
    fn names(&self) -> Vec<String> {
        self.functions.iter().cloned().collect()
    }

    /// Get the number of registered functions
    fn len(&self) -> usize {
        self.functions.len()
    }

    /// Check if context is empty
    fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }

    /// Clear all functions
    fn clear(&mut self) {
        self.functions.clear();
    }
}
