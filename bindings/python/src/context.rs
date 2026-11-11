//! Local symbol namespaces and immutable symbolic custom definitions.

use std::sync::Arc;

use pyo3::{Bound, PyAny, PyRef, PyResult, Python, exceptions::PyValueError, pyclass, pymethods};
use symb_anafis::{Context, HashSet, UserFunction};

use super::{Diagnostics, ExprInputs, FunctionTemplate, NativeExpr, NativeFunction, properties};

/// A local namespace never exports a borrowed Rust symbol to Python.
#[pyclass(name = "_Context", module = "symb_anafis._native")]
#[derive(Clone, Debug)]
pub struct NativeContext {
    context: Context,
}

impl NativeContext {
    pub const fn value(&self) -> &Context {
        &self.context
    }
}

#[pymethods]
impl NativeContext {
    #[new]
    fn new() -> PyResult<Self> {
        Ok(Self {
            context: Context::new().map_err(Diagnostics::value)?,
        })
    }

    fn symb(&self, name: &str) -> PyResult<NativeExpr> {
        self.context
            .symb(name)
            .map(|symbol| symbol.to_expr().into())
            .map_err(Diagnostics::value)
    }

    fn anon(&self) -> PyResult<NativeExpr> {
        self.context
            .anon()
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

    fn symbol_count(&self) -> usize {
        self.context.symbol_count()
    }

    fn remove_symbol(&self, name: &str) -> bool {
        self.context.remove_symbol(name)
    }

    fn clear_symbols(&self) {
        self.context.clear_symbols();
    }

    fn clear_functions(&self) {
        self.context.clear_functions();
    }

    fn clear_all(&self) {
        self.context.clear_all();
    }

    fn is_empty(&self) -> bool {
        self.context.is_empty()
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

    fn reset(&mut self) -> PyResult<()> {
        self.context.reset().map_err(Diagnostics::value)
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

    #[expect(
        clippy::too_many_arguments,
        reason = "PyO3 internal bridge forwards all UserFunction builder options in one call"
    )]
    fn register_function(
        &self,
        name: &str,
        min_arity: usize,
        max_arity: usize,
        parameters: &Bound<'_, PyAny>,
        body: Option<PyRef<'_, NativeExpr>>,
        partials: Vec<(usize, PyRef<'_, NativeExpr>)>,
        returns: Vec<String>,
    ) -> PyResult<NativeFunction> {
        if min_arity > max_arity {
            return Err(PyValueError::new_err(
                "minimum arity cannot exceed maximum arity",
            ));
        }
        let has_templates = body.is_some() || !partials.is_empty();
        let owners = ExprInputs::new(parameters)?;
        let coordinates = owners.coordinates()?;
        if has_templates {
            if min_arity != max_arity {
                return Err(PyValueError::new_err(
                    "body and partial templates require a fixed arity",
                ));
            }
            if coordinates.len() != min_arity {
                return Err(PyValueError::new_err(
                    "template parameter count must equal the function arity",
                ));
            }
            let unique: HashSet<_> = coordinates.into_iter().collect();
            if unique.len() != min_arity {
                return Err(PyValueError::new_err(
                    "function template parameters must be distinct identities",
                ));
            }
            drop(unique);
        } else if !coordinates.is_empty() {
            return Err(PyValueError::new_err(
                "parameters are only used with body or partial templates",
            ));
        }
        let sources = owners.into_expressions();
        let mut definition = UserFunction::new(min_arity..=max_arity);
        if !returns.is_empty() {
            let props = properties::parse(returns)?;
            definition = definition.returns(&props).map_err(Diagnostics::value)?;
        }
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
