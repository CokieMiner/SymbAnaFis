//! Compiled replacement templates and invocation-local owning Python callbacks.

use std::sync::Arc;

use pyo3::{
    Py, PyAny, PyRef, PyResult, PyTraverseError, PyVisit, Python,
    exceptions::{PyRuntimeError, PyTypeError},
    pyclass, pymethods,
    types::PyAnyMethods,
};
use symb_anafis::{Expr, HashMap, Pattern, Replacement, RewriteRule};

use super::{Callbacks, NativeExpr, NativePattern, RewriteError};

/// Immutable rule definitions never share an exception channel between invocations.
#[pyclass(name = "_Rule", module = "symb_anafis._native")]
#[derive(Debug)]
pub struct NativeRule {
    replacement: Rule,
}

#[derive(Debug)]
enum Rule {
    Compiled(RewriteRule),
    Callback {
        pattern: Pattern,
        callback: Py<PyAny>,
    },
    Released,
}

impl NativeRule {
    pub fn prepare(&self, state: &Arc<Callbacks>, py: Python<'_>) -> PyResult<RewriteRule> {
        match &self.replacement {
            Rule::Compiled(rule) => Ok(rule.clone()),
            Rule::Callback { pattern, callback } => {
                let callable = callback.clone_ref(py);
                let errors = Arc::clone(state);
                RewriteRule::new(
                    pattern.clone(),
                    Replacement::Callback(Arc::new(move |captures| {
                        errors.replacement(&callable, captures)
                    })),
                )
                .map_err(|error| RewriteError::new_err(error.to_string()))
            }
            Rule::Released => Err(PyRuntimeError::new_err(
                "rewrite rule was released by cyclic garbage collection",
            )),
        }
    }

    pub fn compiled(&self) -> Option<RewriteRule> {
        match &self.replacement {
            Rule::Compiled(rule) => Some(rule.clone()),
            Rule::Callback { .. } | Rule::Released => None,
        }
    }

    pub fn clone_ref(&self, py: Python<'_>) -> Self {
        let replacement = match &self.replacement {
            Rule::Compiled(rule) => Rule::Compiled(rule.clone()),
            Rule::Callback { pattern, callback } => Rule::Callback {
                pattern: pattern.clone(),
                callback: callback.clone_ref(py),
            },
            Rule::Released => Rule::Released,
        };
        Self { replacement }
    }

    pub fn visit(&self, visitor: &PyVisit<'_>) -> Result<(), PyTraverseError> {
        if let Rule::Callback { callback, .. } = &self.replacement {
            visitor.call(callback)?;
        }
        Ok(())
    }
}

#[pymethods]
impl NativeRule {
    #[new]
    fn new(pattern: &NativePattern, replacement: &pyo3::Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(template) = replacement.extract::<PyRef<'_, NativePattern>>() {
            return RewriteRule::new(pattern.value().clone(), template.value().clone())
                .map(|rule| Self {
                    replacement: Rule::Compiled(rule),
                })
                .map_err(|error| RewriteError::new_err(error.to_string()));
        }
        if !replacement.is_callable() {
            return Err(PyTypeError::new_err(
                "replacement must be a Pattern or callable",
            ));
        }
        Ok(Self {
            replacement: Rule::Callback {
                pattern: pattern.value().clone(),
                callback: replacement.clone().unbind(),
            },
        })
    }

    #[pyo3(signature = (expression, match_limit, guard=None))]
    fn apply(
        &self,
        expression: &NativeExpr,
        match_limit: Option<usize>,
        guard: Option<&pyo3::Bound<'_, PyAny>>,
        py: Python<'_>,
    ) -> PyResult<NativeExpr> {
        if let Some(callback) = guard
            && !callback.is_callable()
        {
            return Err(PyTypeError::new_err("guard must be callable"));
        }
        let callable = guard.map(|callback| callback.clone().unbind());
        let state = Arc::new(Callbacks::default());
        let rule = self.prepare(&state, py)?;
        let result = py.detach(|| {
            callable.as_ref().map_or_else(
                || rule.apply_all(expression.value(), match_limit, None),
                |callback| {
                    let mut accepts =
                        |captures: &HashMap<&str, Expr>| state.guard(callback, captures);
                    rule.apply_all(expression.value(), match_limit, Some(&mut accepts))
                },
            )
        });
        state.finish()?;
        result
            .map(NativeExpr::from)
            .map_err(|error| RewriteError::new_err(error.to_string()))
    }

    fn __traverse__(&self, visitor: PyVisit<'_>) -> Result<(), PyTraverseError> {
        self.visit(&visitor)
    }

    fn __clear__(&mut self) {
        self.replacement = Rule::Released;
    }
}
