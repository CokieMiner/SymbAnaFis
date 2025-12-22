//! Expression visitor pattern for AST traversal
//!
//! Provides a clean interface for walking the expression tree without
//! manually handling the recursive structure.

use crate::{Expr, ExprKind};
use std::sync::Arc;

/// Trait for visiting expression nodes in the AST
///
/// Implement this trait to define custom behavior when traversing expressions.
/// Each method returns a boolean indicating whether to continue visiting children.
///
/// # Example
/// ```
/// use symb_anafis::symb;
/// use symb_anafis::visitor::{ExprVisitor, walk_expr, NodeCounter};
///
/// let x = symb("visitor_example_x");
/// let expr = x + x.pow(2.0);
/// let mut counter = NodeCounter::default();
/// walk_expr(&expr, &mut counter);
/// assert!(counter.count > 0);
/// ```
pub trait ExprVisitor {
    /// Visit a number literal, returns true to continue visiting
    fn visit_number(&mut self, n: f64) -> bool;

    /// Visit a symbol/variable, returns true to continue visiting
    fn visit_symbol(&mut self, name: &str) -> bool;

    /// Visit a function call, returns true to visit arguments
    fn visit_function(&mut self, name: &str, args: &[Arc<Expr>]) -> bool;

    /// Visit a binary operation (+, -, *, /, ^), returns true to visit operands
    fn visit_binary(&mut self, op: &str, left: &Expr, right: &Expr) -> bool;

    /// Visit a derivative expression, returns true to visit inner expression
    fn visit_derivative(&mut self, inner: &Expr, var: &str, order: u32) -> bool;
}

/// Walk an expression tree with a visitor
///
/// Visits nodes in pre-order (parent before children).
/// The visitor methods return true to continue walking children, false to skip.
pub fn walk_expr<V: ExprVisitor>(expr: &Expr, visitor: &mut V) {
    match &expr.kind {
        ExprKind::Number(n) => {
            visitor.visit_number(*n);
        }
        ExprKind::Symbol(s) => {
            visitor.visit_symbol(s.as_ref());
        }
        ExprKind::FunctionCall { name, args } => {
            if visitor.visit_function(name.as_str(), args) {
                for arg in args {
                    walk_expr(arg, visitor);
                }
            }
        }
        // N-ary Sum - visit as "+" with first two terms, then recurse through all
        ExprKind::Sum(terms) => {
            if terms.len() >= 2 {
                // Visit binary for compatibility, then walk all children
                if visitor.visit_binary("+", &terms[0], &terms[1]) {
                    for term in terms {
                        walk_expr(term, visitor);
                    }
                }
            } else if terms.len() == 1 {
                walk_expr(&terms[0], visitor);
            }
        }
        // N-ary Product - visit as "*" with first two factors, then recurse through all
        ExprKind::Product(factors) => {
            if factors.len() >= 2 {
                if visitor.visit_binary("*", &factors[0], &factors[1]) {
                    for factor in factors {
                        walk_expr(factor, visitor);
                    }
                }
            } else if factors.len() == 1 {
                walk_expr(&factors[0], visitor);
            }
        }
        ExprKind::Div(l, r) => {
            if visitor.visit_binary("/", l, r) {
                walk_expr(l, visitor);
                walk_expr(r, visitor);
            }
        }
        ExprKind::Pow(l, r) => {
            if visitor.visit_binary("^", l, r) {
                walk_expr(l, visitor);
                walk_expr(r, visitor);
            }
        }
        ExprKind::Derivative { inner, var, order } => {
            if visitor.visit_derivative(inner, var, *order) {
                walk_expr(inner, visitor);
            }
        }
        // Poly: walk the base expression directly (avoid to_expr_terms() allocation)
        ExprKind::Poly(poly) => {
            // Walk the base expression which contains all variables
            walk_expr(poly.base(), visitor);
        }
    }
}

/// A simple visitor that counts nodes
#[derive(Default)]
pub struct NodeCounter {
    pub count: usize,
}

impl ExprVisitor for NodeCounter {
    fn visit_number(&mut self, _n: f64) -> bool {
        self.count += 1;
        true
    }

    fn visit_symbol(&mut self, _name: &str) -> bool {
        self.count += 1;
        true
    }

    fn visit_function(&mut self, _name: &str, _args: &[Arc<Expr>]) -> bool {
        self.count += 1;
        true
    }

    fn visit_binary(&mut self, _op: &str, _left: &Expr, _right: &Expr) -> bool {
        self.count += 1;
        true
    }

    fn visit_derivative(&mut self, _inner: &Expr, _var: &str, _order: u32) -> bool {
        self.count += 1;
        true
    }
}

/// A visitor that collects all unique variable names
#[derive(Default)]
pub struct VariableCollector {
    pub variables: std::collections::HashSet<String>,
}

impl ExprVisitor for VariableCollector {
    fn visit_number(&mut self, _n: f64) -> bool {
        true
    }

    fn visit_symbol(&mut self, name: &str) -> bool {
        self.variables.insert(name.to_string());
        true
    }

    fn visit_function(&mut self, _name: &str, _args: &[Arc<Expr>]) -> bool {
        true
    }

    fn visit_binary(&mut self, _op: &str, _left: &Expr, _right: &Expr) -> bool {
        true
    }

    fn visit_derivative(&mut self, _inner: &Expr, var: &str, _order: u32) -> bool {
        self.variables.insert(var.to_string());
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symb;

    #[test]
    fn test_node_counter() {
        let x = symb("x");
        let expr = x + x.pow(2.0); // x + x^2 = Poly with base x
        let mut counter = NodeCounter::default();
        walk_expr(&expr, &mut counter);
        // With optimized Poly: only walks the base expression (x = 1 symbol node)
        assert_eq!(counter.count, 1);
    }

    #[test]
    fn test_variable_collector() {
        let x = symb("x");
        let y = symb("y");
        let expr = x + y;
        let mut collector = VariableCollector::default();
        walk_expr(&expr, &mut collector);
        assert!(collector.variables.contains("x"));
        assert!(collector.variables.contains("y"));
        assert_eq!(collector.variables.len(), 2);
    }
}
