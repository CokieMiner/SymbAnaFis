//! Local symbol namespaces and immutable symbolic custom definitions.

use std::sync::Arc;

use pyo3::{Bound, PyAny, PyRef, PyResult, Python, exceptions::PyValueError, pyclass, pymethods};
use symb_anafis::{Context, HashSet, UserFunction};

use super::{Diagnostics, ExprInputs, FunctionTemplate, NativeExpr, NativeFunction};

/// A local namespace never exports a borrowed Rust symbol to Python.
#[pyclass(name = "_Context", module = "symb_anafis._native")]
#[derive(Clone, Debug)]
pub struct NativeContext {
    context: Context,
}

#[pymethods]
impl NativeContext {
    #[new]
    fn new() -> Self {
        Self {
            context: Context::new(),
        }
    }

    fn symb(&self, name: &str) -> PyResult<NativeExpr> {
        self.context
            .symb(name)
            .map(|symbol| symbol.to_expr().into())
            .map_err(Diagnostics::value)
    }

    fn get_symbol(&self, name: &str) -> Option<NativeExpr> {
        self.context
            .get_symbol(name)
            .map(|symbol| symbol.to_expr().into())
    }

    fn symbol_names(&self) -> Vec<String> {
        self.context.symbol_names()
    }

    fn remove_symbol(&self, name: &str) -> bool {
        self.context.remove_symbol(name)
    }

    fn clear_symbols(&self) {
        self.context.clear_symbols();
    }

    fn remove_function(&self, name: &str) -> bool {
        self.context.remove_function(name)
    }

    fn get_function(&self, name: &str) -> Option<NativeFunction> {
        self.context.get_function(name).map(NativeFunction::from)
    }

    fn functions(&self) -> Vec<NativeFunction> {
        self.context
            .functions()
            .into_iter()
            .map(NativeFunction::from)
            .collect()
    }

    fn reset(&mut self) {
        self.context.reset();
    }

    const fn id(&self) -> usize {
        self.context.id()
    }

    fn parse(&self, source: &str, py: Python<'_>) -> PyResult<NativeExpr> {
        match py.detach(|| self.context.parse(source)) {
            Ok(expression) => Ok(expression.into()),
            Err(error) => Err(Diagnostics::parse(&error, source, py)?),
        }
    }

    fn register_function(
        &self,
        name: &str,
        arity: usize,
        parameters: &Bound<'_, PyAny>,
        body: Option<PyRef<'_, NativeExpr>>,
        partials: Vec<(usize, PyRef<'_, NativeExpr>)>,
    ) -> PyResult<NativeFunction> {
        let owners = ExprInputs::new(parameters)?;
        let coordinates = owners.coordinates()?;
        if coordinates.len() != arity {
            return Err(PyValueError::new_err(
                "template parameter count must equal the function arity",
            ));
        }
        let unique: HashSet<_> = coordinates.into_iter().collect();
        if unique.len() != arity {
            return Err(PyValueError::new_err(
                "function template parameters must be distinct identities",
            ));
        }
        drop(unique);
        let sources = owners.into_expressions();
        let mut definition = UserFunction::new(arity..=arity);
        if let Some(expression) = body {
            let template = FunctionTemplate::new(sources.clone(), expression.value().clone())?;
            definition =
                definition.body(Arc::new(move |arguments| template.instantiate(arguments)));
        }
        for (index, expression) in partials {
            let template = FunctionTemplate::new(sources.clone(), expression.value().clone())?;
            definition = definition
                .partial(
                    index,
                    Arc::new(move |arguments| template.instantiate(arguments)),
                )
                .map_err(Diagnostics::value)?;
        }
        self.context
            .register_function(name, definition)
            .map(NativeFunction::from)
            .map_err(Diagnostics::value)
    }
}
