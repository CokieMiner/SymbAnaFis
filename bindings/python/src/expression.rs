//! Owning expression handles, scoped symbol borrows, and scalar construction.

use core::{
    hash::{Hash, Hasher},
    ops::{Add, Div, Mul, Neg, Sub},
};

use std::collections::hash_map::DefaultHasher;

use pyo3::{
    Bound, Py, PyAny, PyRef, PyResult, Python,
    exceptions::{PyTypeError, PyValueError},
    pyclass, pymethods,
    types::PyAnyMethods,
};
use symb_anafis::{Expr, ExprView, Property, Symbol, Truth};

use super::{ExprInputs, NativeFunction, NativeNumber};

/// A Python handle owns its expression; symbols borrow only within a Rust call.
#[pyclass(name = "_Expr", frozen, module = "symb_anafis._native")]
#[derive(Clone, Debug)]
pub struct NativeExpr {
    expression: Expr,
}

impl From<Expr> for NativeExpr {
    fn from(expression: Expr) -> Self {
        Self { expression }
    }
}

impl NativeExpr {
    pub const fn value(&self) -> &Expr {
        &self.expression
    }

    pub fn symbol(&self) -> PyResult<Symbol<'_>> {
        self.expression
            .as_symbol()
            .ok_or_else(|| PyTypeError::new_err("a coordinate must be a Symbol expression"))
    }

    pub fn property(name: &str) -> PyResult<Property> {
        match name {
            "finite" => Ok(Property::Finite),
            "real" => Ok(Property::Real),
            "integer" => Ok(Property::Integer),
            "rational" => Ok(Property::Rational),
            "zero" => Ok(Property::Zero),
            "nonzero" => Ok(Property::Nonzero),
            "positive" => Ok(Property::Positive),
            "negative" => Ok(Property::Negative),
            "nonnegative" => Ok(Property::Nonnegative),
            "nonpositive" => Ok(Property::Nonpositive),
            _ => Err(PyValueError::new_err("unknown scalar property")),
        }
    }
}

#[pymethods]
impl NativeExpr {
    #[new]
    fn new(value: &NativeNumber) -> Self {
        Expr::number(value.value().clone()).into()
    }

    #[staticmethod]
    fn constant(name: &str) -> PyResult<Self> {
        match name {
            "pi" => Ok(Expr::pi().into()),
            "e" => Ok(Expr::e().into()),
            "i" => Ok(Expr::i().into()),
            _ => Err(PyValueError::new_err("unknown mathematical constant")),
        }
    }

    #[staticmethod]
    fn sum(values: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Expr::sum(ExprInputs::new(values)?.into_expressions()).into())
    }

    #[staticmethod]
    fn product(values: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Expr::product(ExprInputs::new(values)?.into_expressions()).into())
    }

    fn binary(&self, operation: &str, other: &Self) -> PyResult<Self> {
        let value = match operation {
            "add" => Add::add(&self.expression, &other.expression),
            "sub" => Sub::sub(&self.expression, &other.expression),
            "mul" => Mul::mul(&self.expression, &other.expression),
            "div" => Div::div(&self.expression, &other.expression),
            "pow" => self.expression.pow(&other.expression),
            _ => return Err(PyValueError::new_err("unknown expression operation")),
        };
        Ok(value.into())
    }

    fn negative(&self) -> Self {
        Neg::neg(&self.expression).into()
    }

    fn derivative(&self, variable: &Self, order: u32, py: Python<'_>) -> PyResult<Self> {
        let symbol = variable.symbol()?;
        Ok(py
            .detach(|| Expr::derivative(self.expression.clone(), symbol, order))
            .into())
    }

    fn gradient(&self, variables: &Bound<'_, PyAny>, py: Python<'_>) -> PyResult<Vec<Self>> {
        let owners = ExprInputs::new(variables)?;
        let coordinates = owners.coordinates()?;
        Ok(py
            .detach(|| self.expression.gradient(&coordinates))
            .into_iter()
            .map(Self::from)
            .collect())
    }

    #[staticmethod]
    fn jacobian(
        outputs: &Bound<'_, PyAny>,
        variables: &Bound<'_, PyAny>,
        py: Python<'_>,
    ) -> PyResult<Vec<Vec<Self>>> {
        let owners = ExprInputs::new(variables)?;
        let coordinates = owners.coordinates()?;
        let expressions = ExprInputs::new(outputs)?.into_expressions();
        Ok(py
            .detach(|| Expr::jacobian(&expressions, &coordinates))
            .into_iter()
            .map(|row| row.into_iter().map(Self::from).collect())
            .collect())
    }

    fn hessian(&self, variables: &Bound<'_, PyAny>, py: Python<'_>) -> PyResult<Vec<Vec<Self>>> {
        let owners = ExprInputs::new(variables)?;
        let coordinates = owners.coordinates()?;
        Ok(py
            .detach(|| self.expression.hessian(&coordinates))
            .into_iter()
            .map(|row| row.into_iter().map(Self::from).collect())
            .collect())
    }

    fn children(&self) -> Vec<Self> {
        self.expression.children().map(Self::from).collect()
    }

    fn symbols(&self) -> Vec<Self> {
        let mut symbols: Vec<_> = self.expression.symbols().into_iter().collect();
        symbols.sort_unstable_by_key(Symbol::id);
        symbols
            .into_iter()
            .map(|symbol| symbol.to_expr().into())
            .collect()
    }

    fn symbol_id(&self) -> PyResult<usize> {
        self.symbol().map(|symbol| symbol.id())
    }

    fn symbol_name(&self) -> Option<String> {
        self.expression.symbol_name().map(str::to_owned)
    }

    fn number(&self) -> Option<NativeNumber> {
        self.expression.as_number().cloned().map(NativeNumber::from)
    }

    fn function(&self) -> Option<NativeFunction> {
        match self.expression.view() {
            ExprView::Function { name, .. } => Some(name.clone().into()),
            _ => None,
        }
    }

    fn kind(&self) -> &'static str {
        match self.expression.view() {
            ExprView::Number(_) => "number",
            ExprView::ImaginaryUnit => "i",
            ExprView::Pi => "pi",
            ExprView::Euler => "e",
            ExprView::Symbol(_) => "symbol",
            ExprView::Sum(_) => "sum",
            ExprView::Product(_) => "product",
            ExprView::Div(..) => "division",
            ExprView::Pow(..) => "power",
            ExprView::Function { .. } => "function",
            ExprView::Derivative { .. } => "derivative",
            _ => "unknown",
        }
    }

    fn derivative_metadata(&self) -> Option<(Self, u32)> {
        match self.expression.view() {
            ExprView::Derivative { var, order, .. } => Some((var.to_expr().into(), order)),
            _ => None,
        }
    }

    fn facts(&self) -> Vec<(&'static str, Option<bool>)> {
        let properties = self.expression.properties();
        [
            ("finite", Property::Finite),
            ("real", Property::Real),
            ("integer", Property::Integer),
            ("rational", Property::Rational),
            ("zero", Property::Zero),
            ("nonzero", Property::Nonzero),
            ("positive", Property::Positive),
            ("negative", Property::Negative),
            ("nonnegative", Property::Nonnegative),
            ("nonpositive", Property::Nonpositive),
        ]
        .into_iter()
        .map(|(name, property)| {
            (
                name,
                match properties.get(property) {
                    Truth::True => Some(true),
                    Truth::False => Some(false),
                    Truth::Unknown | _ => None,
                },
            )
        })
        .collect()
    }

    fn node_count(&self) -> usize {
        self.expression.node_count()
    }

    fn max_depth(&self) -> usize {
        self.expression.max_depth()
    }

    fn has_derivatives(&self) -> bool {
        self.expression.fold(false, |found, node| {
            found || matches!(node.view(), ExprView::Derivative { .. })
        })
    }

    fn equal(&self, other: &Self) -> bool {
        self.expression == other.expression
    }

    fn hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.expression.hash(&mut hasher);
        hasher.finish()
    }

    fn format(&self, notation: &str) -> PyResult<String> {
        match notation {
            "text" => Ok(self.expression.to_string()),
            "latex" => Ok(self.expression.to_latex()),
            "unicode" => Ok(self.expression.to_unicode()),
            _ => Err(PyValueError::new_err("unknown expression notation")),
        }
    }

    fn map(&self, callback: &Bound<'_, PyAny>) -> PyResult<Self> {
        self.expression
            .map(|node| -> PyResult<Option<Expr>> {
                let result = callback.call1((Self::from(node.clone()),))?;
                if result.is_none() {
                    Ok(None)
                } else {
                    let value = result.extract::<PyRef<'_, Self>>()?;
                    Ok(Some(value.expression.clone()))
                }
            })
            .map(Self::from)
    }

    fn fold(&self, initial: Py<PyAny>, callback: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        self.expression.fold(Ok(initial), |state, node| {
            callback
                .call1((state?, Self::from(node.clone())))
                .map(Bound::unbind)
        })
    }
}
