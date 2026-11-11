//! Invocation-local Python exceptions for infallible symbolic callback interfaces.

use std::sync::{Mutex, PoisonError};

use pyo3::{
    Py, PyAny, PyErr, PyRef, PyResult, Python,
    types::{PyAnyMethods, PyDict, PyDictMethods},
};
use symb_anafis::{Expr, HashMap};

use super::NativeExpr;

/// One invocation retains its first exception and never publishes provisional output.
/// Locks protect only error bookkeeping and are released before entering Python.
#[derive(Debug, Default)]
pub struct Callbacks {
    failure: Mutex<Option<PyErr>>,
}

impl Callbacks {
    pub fn finish(&self) -> PyResult<()> {
        self.failure
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
            .map_or(Ok(()), Err)
    }

    pub fn replacement(&self, callback: &Py<PyAny>, captures: &HashMap<&str, Expr>) -> Expr {
        self.invoke(|| {
            Python::attach(|py| {
                let bindings = PyDict::new(py);
                for (name, value) in captures {
                    bindings.set_item(name, NativeExpr::from(value.clone()))?;
                }
                let result = callback.bind(py).call1((bindings,))?;
                let expression = result.extract::<PyRef<'_, NativeExpr>>()?;
                Ok(expression.value().clone())
            })
        })
        .unwrap_or_else(|| Expr::number(0))
    }

    pub fn mapping(&self, callback: &Py<PyAny>, expression: &Expr) -> Option<Expr> {
        self.invoke(|| {
            Python::attach(|py| {
                let result = callback
                    .bind(py)
                    .call1((NativeExpr::from(expression.clone()),))?;
                if result.is_none() {
                    Ok(None)
                } else {
                    let replacement = result.extract::<PyRef<'_, NativeExpr>>()?;
                    Ok(Some(replacement.value().clone()))
                }
            })
        })
        // The runner will discard this invocation. Collapse the remaining map
        // to a constant so later passes cannot expand the failed input further.
        .unwrap_or_else(|| Some(Expr::number(0)))
    }

    pub fn score(&self, callback: &Py<PyAny>, expression: &Expr) -> usize {
        self.invoke(|| {
            Python::attach(|py| {
                callback
                    .bind(py)
                    .call1((NativeExpr::from(expression.clone()),))?
                    .extract::<usize>()
            })
        })
        .unwrap_or(usize::MAX)
    }

    pub fn guard(&self, callback: &Py<PyAny>, captures: &HashMap<&str, Expr>) -> bool {
        self.invoke(|| {
            Python::attach(|py| {
                let bindings = PyDict::new(py);
                for (name, value) in captures {
                    bindings.set_item(name, NativeExpr::from(value.clone()))?;
                }
                callback.bind(py).call1((bindings,))?.is_truthy()
            })
        })
        // Accept a provisional assignment after failure to stop backtracking;
        // finish() returns the recorded exception before any output is published.
        .unwrap_or(true)
    }

    fn invoke<Value>(&self, callback: impl FnOnce() -> PyResult<Value>) -> Option<Value> {
        if self
            .failure
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_some()
        {
            return None;
        }
        match callback() {
            Ok(value) => Some(value),
            Err(error) => {
                let mut failure = self.failure.lock().unwrap_or_else(PoisonError::into_inner);
                if failure.is_none() {
                    *failure = Some(error);
                }
                None
            }
        }
    }
}
