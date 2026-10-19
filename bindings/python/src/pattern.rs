//! Commutative templates with conservative scalar capture conditions.

use core::ops::{Add, Div, Mul, Neg, Sub};

use std::sync::Arc;

use pyo3::{PyRef, PyResult, exceptions::PyValueError, pyclass, pymethods};
use symb_anafis::{Pattern, Truth};

use super::{Diagnostics, NativeExpr, NativeFunction};

/// A compiled pattern owns exact expressions and capture predicates.
#[pyclass(name = "_Pattern", frozen, module = "symb_anafis._native")]
#[derive(Clone, Debug)]
pub struct NativePattern {
    pattern: Pattern,
}

impl From<Pattern> for NativePattern {
    fn from(pattern: Pattern) -> Self {
        Self { pattern }
    }
}

impl NativePattern {
    pub const fn value(&self) -> &Pattern {
        &self.pattern
    }
}

#[pymethods]
impl NativePattern {
    #[staticmethod]
    fn capture(name: &str, properties: Vec<String>) -> PyResult<Self> {
        let conditions = properties
            .into_iter()
            .map(|name| NativeExpr::property(&name))
            .collect::<PyResult<Vec<_>>>()?;
        let predicate = (!conditions.is_empty()).then(|| {
            Arc::new(move |node: &symb_anafis::Expr| {
                let facts = node.properties();
                conditions
                    .iter()
                    .all(|property| facts.get(*property) == Truth::True)
            }) as Arc<dyn Fn(&symb_anafis::Expr) -> bool + Send + Sync>
        });
        Ok(Pattern::capture(name, predicate).into())
    }

    #[staticmethod]
    fn exact(expression: &NativeExpr) -> Self {
        Pattern::exact(expression.value().clone()).into()
    }

    #[staticmethod]
    fn call(function: &NativeFunction, arguments: Vec<PyRef<'_, Self>>) -> PyResult<Self> {
        Pattern::apply(
            function.value().clone(),
            arguments
                .into_iter()
                .map(|value| value.pattern.clone())
                .collect(),
        )
        .map(Self::from)
        .map_err(Diagnostics::value)
    }

    fn binary(&self, operation: &str, other: &Self) -> PyResult<Self> {
        let value = match operation {
            "add" => Add::add(&self.pattern, &other.pattern),
            "sub" => Sub::sub(&self.pattern, &other.pattern),
            "mul" => Mul::mul(&self.pattern, &other.pattern),
            "div" => Div::div(&self.pattern, &other.pattern),
            "pow" => self.pattern.pow(&other.pattern),
            _ => return Err(PyValueError::new_err("unknown pattern operation")),
        };
        Ok(value.into())
    }

    fn negative(&self) -> Self {
        Neg::neg(&self.pattern).into()
    }
}
