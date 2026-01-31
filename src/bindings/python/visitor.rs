//! Python bindings for AST visitor utilities
//!
//! This module provides functions for traversing and analyzing expression trees.

use super::expr::PyExpr;
use crate::visitor::{NodeCounter, VariableCollector, walk_expr};
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
