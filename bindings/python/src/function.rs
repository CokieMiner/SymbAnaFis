//! Owning function identities and simultaneous symbolic function templates.

use core::{
    convert::Infallible,
    hash::{Hash, Hasher},
};

use std::collections::hash_map::DefaultHasher;

use pyo3::{Bound, PyAny, PyResult, Python, exceptions::PyValueError, pyclass, pymethods};
use symb_anafis::{Expr, ExprView, Function, HashMap};

use super::{Diagnostics, ExprInputs, NativeExpr, properties};

/// A function retains its definition independently of its context binding.
#[pyclass(name = "_Function", frozen, module = "symb_anafis._native")]
#[derive(Clone, Debug)]
pub struct NativeFunction {
    function: Function,
}

#[derive(Clone, Debug)]
pub struct FunctionTemplate {
    parameters: HashMap<Expr, usize>,
    expression: Expr,
}

impl From<Function> for NativeFunction {
    fn from(function: Function) -> Self {
        Self { function }
    }
}

impl NativeFunction {
    pub const fn value(&self) -> &Function {
        &self.function
    }
}

#[pymethods]
impl NativeFunction {
    #[staticmethod]
    fn builtin(name: &str) -> Option<Self> {
        Function::builtin(name).map(Self::from)
    }

    fn call(&self, arguments: &Bound<'_, PyAny>, py: Python<'_>) -> PyResult<NativeExpr> {
        let expressions = ExprInputs::new(arguments)?.into_expressions();
        py.detach(|| self.function.call(expressions))
            .map(NativeExpr::from)
            .map_err(Diagnostics::value)
    }

    fn name(&self) -> &str {
        self.function.as_str()
    }

    fn arity(&self) -> (usize, usize) {
        let arity = self.function.arity();
        (*arity.start(), *arity.end())
    }

    fn equal(&self, other: &Self) -> bool {
        self.function == other.function
    }

    fn hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.function.hash(&mut hasher);
        hasher.finish()
    }

    fn facts(&self) -> Option<Vec<(&'static str, Option<bool>)>> {
        self.function
            .definition()
            .map(|definition| properties::facts(definition.properties()))
    }
}

impl FunctionTemplate {
    /// Scalar substitution cannot transform an unresolved differential operator.
    /// Reject such templates once, before an infallible body can be registered.
    pub fn new(parameters: Vec<Expr>, expression: Expr) -> PyResult<Self> {
        if expression.fold(false, |found, node| {
            found || matches!(node.view(), ExprView::Derivative { .. })
        }) {
            return Err(PyValueError::new_err(
                "function templates cannot contain unresolved differential operators",
            ));
        }
        Ok(Self {
            parameters: parameters
                .into_iter()
                .enumerate()
                .map(|(index, parameter)| (parameter, index))
                .collect(),
            expression,
        })
    }

    /// Function arity and distinct source identities are validated at registration.
    /// Mapping visits original nodes once; inserted arguments are never revisited.
    pub fn instantiate(&self, arguments: &[Expr]) -> Expr {
        if self.parameters.is_empty() {
            return self.expression.clone();
        }
        self.expression
            .map(|node| {
                Ok::<_, Infallible>(self.parameters.get(node).map(|&index| {
                    // Every index is below the immutable registered arity, and
                    // Function::call validates that arity before storing a call.
                    arguments[index].clone()
                }))
            })
            .unwrap_or_else(|never| match never {})
    }
}
