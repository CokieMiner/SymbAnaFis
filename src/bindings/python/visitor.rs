//! Python bindings for AST visitor utilities
//!
//! This module provides functions for traversing and analyzing expression trees.

// Allow: PyO3 from_py_object macro generates FromPyObject with short lifetime names
#![allow(
    clippy::single_char_lifetime_names,
    reason = "PyO3 from_py_object macro generates lifetimes"
)]

use super::expr::PyExpr;
use crate::visitor::{ExprView, NodeCounter, VariableCollector, walk_expr};
use pyo3::prelude::*;
use std::collections::HashSet;

/// Count the number of nodes in an expression tree.
///
/// Args:
///     expr: Expr object to count nodes in
///
/// Returns:
///     Number of nodes (symbols, numbers, operators, functions)
#[pyfunction]
pub fn count_nodes(expr: &PyExpr) -> usize {
    let mut counter = NodeCounter::default();
    walk_expr(&expr.0, &mut counter);
    counter.count
}

/// Collect all unique variable names in an expression.
///
/// Args:
///     expr: Expr object to collect variables from
///
/// Returns:
///     Set of variable name strings
#[pyfunction]
pub fn collect_variables(expr: &PyExpr) -> HashSet<String> {
    let mut collector = VariableCollector::default();
    walk_expr(&expr.0, &mut collector);
    collector.variable_names()
}

// =============================================================================
// ExprView Python Bindings
// =============================================================================

/// Python wrapper for expression structure view
///
/// Provides pattern-matchable access to expression structure without exposing
/// internal implementation details. Use `expr.view()` to get an `ExprView`.
///
/// Example:
///     >>> x = symb("x")
///     >>> expr = x**2 + x
///     >>> view = `expr.view()`
///     >>> view.kind
///     'Sum'
///     >>> len(view.children)
///     2
#[pyclass(name = "ExprView", from_py_object)]
#[derive(Clone)]
pub struct PyExprView {
    /// Expression kind: "Number", "Symbol", "Sum", "Product", "Div", "Pow", "Function", or "Derivative"
    kind: String,
    /// Numeric value (only set for Number nodes)
    value: Option<f64>,
    /// Symbol or function name (only set for Symbol/Function nodes)
    name: Option<String>,
    /// Child expression nodes
    children: Vec<PyExpr>,
    /// Variable name for differentiation (only set for Derivative nodes)
    derivative_var: Option<String>,
    /// Order of differentiation (only set for Derivative nodes)
    derivative_order: Option<u32>,
}

#[pymethods]
impl PyExprView {
    /// Get the type of expression node
    ///
    /// Returns:
    ///     One of: "Number", "Symbol", "Sum", "Product", "Div", "Pow", "Function", "Derivative"
    #[getter]
    fn kind(&self) -> String {
        self.kind.clone()
    }

    /// Get numeric value (only for Number nodes)
    #[getter]
    const fn value(&self) -> Option<f64> {
        self.value
    }

    /// Get name (for Symbol or Function nodes)
    ///
    /// For anonymous symbols, returns their "$ID" representation.
    #[getter]
    fn name(&self) -> Option<String> {
        self.name.clone()
    }

    /// Get child expressions
    ///
    /// Returns:
    ///     List of child Expr objects:
    ///     - Sum/Product: List of terms/factors
    ///     - Div: [numerator, denominator]
    ///     - Pow: [base, exponent]
    ///     - Function: List of arguments
    ///     - Derivative: [`inner_expr`]
    ///     - Number/Symbol: []
    #[getter]
    fn children(&self) -> Vec<PyExpr> {
        self.children.clone()
    }

    /// Get derivative variable (only for Derivative nodes)
    #[getter]
    fn derivative_var(&self) -> Option<String> {
        self.derivative_var.clone()
    }

    /// Get derivative order (only for Derivative nodes)
    #[getter]
    const fn derivative_order(&self) -> Option<u32> {
        self.derivative_order
    }

    /// Check if this is a Number node
    #[getter]
    fn is_number(&self) -> bool {
        self.kind == "Number"
    }

    /// Check if this is a Symbol node
    #[getter]
    fn is_symbol(&self) -> bool {
        self.kind == "Symbol"
    }

    /// Check if this is a Sum node
    #[getter]
    fn is_sum(&self) -> bool {
        self.kind == "Sum"
    }

    /// Check if this is a Product node
    #[getter]
    fn is_product(&self) -> bool {
        self.kind == "Product"
    }

    /// Check if this is a Div node
    #[getter]
    fn is_div(&self) -> bool {
        self.kind == "Div"
    }

    /// Check if this is a Pow node
    #[getter]
    fn is_pow(&self) -> bool {
        self.kind == "Pow"
    }

    /// Check if this is a Function node
    #[getter]
    fn is_function(&self) -> bool {
        self.kind == "Function"
    }

    /// Check if this is a Derivative node
    #[getter]
    fn is_derivative(&self) -> bool {
        self.kind == "Derivative"
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!("ExprView(kind='{}')", self.kind)
    }

    /// Get the number of children
    const fn __len__(&self) -> usize {
        self.children.len()
    }

    /// Get child by index
    fn __getitem__(&self, idx: isize) -> PyResult<PyExpr> {
        let len = self.children.len().cast_signed();
        let index = if idx < 0 { len + idx } else { idx };

        if index < 0 || index >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                "Index {idx} out of range for {len} children"
            )));
        }

        Ok(self.children[index.cast_unsigned()].clone())
    }
}

impl PyExprView {
    /// Create a `PyExprView` from a Rust `ExprView`
    pub fn from_view(view: ExprView<'_>) -> Self {
        match view {
            ExprView::Number(n) => Self {
                kind: "Number".to_owned(),
                value: Some(n),
                name: None,
                children: vec![],
                derivative_var: None,
                derivative_order: None,
            },
            ExprView::Symbol(name) => Self {
                kind: "Symbol".to_owned(),
                value: None,
                name: Some(name.to_string()),
                children: vec![],
                derivative_var: None,
                derivative_order: None,
            },
            ExprView::Sum(terms) => Self {
                kind: "Sum".to_owned(),
                value: None,
                name: None,
                children: terms.iter().map(|t| PyExpr((**t).clone())).collect(),
                derivative_var: None,
                derivative_order: None,
            },
            ExprView::Product(factors) => Self {
                kind: "Product".to_owned(),
                value: None,
                name: None,
                children: factors.iter().map(|f| PyExpr((**f).clone())).collect(),
                derivative_var: None,
                derivative_order: None,
            },
            ExprView::Div(num, den) => Self {
                kind: "Div".to_owned(),
                value: None,
                name: None,
                children: vec![PyExpr(num.clone()), PyExpr(den.clone())],
                derivative_var: None,
                derivative_order: None,
            },
            ExprView::Pow(base, exp) => Self {
                kind: "Pow".to_owned(),
                value: None,
                name: None,
                children: vec![PyExpr(base.clone()), PyExpr(exp.clone())],
                derivative_var: None,
                derivative_order: None,
            },
            ExprView::Function { name, args } => Self {
                kind: "Function".to_owned(),
                value: None,
                name: Some(name.to_owned()),
                children: args.iter().map(|a| PyExpr((**a).clone())).collect(),
                derivative_var: None,
                derivative_order: None,
            },
            ExprView::Derivative { inner, var, order } => Self {
                kind: "Derivative".to_owned(),
                value: None,
                name: None,
                children: vec![PyExpr(inner.clone())],
                derivative_var: Some(var.to_owned()),
                derivative_order: Some(order),
            },
        }
    }
}
