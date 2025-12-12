//! Expression visitor pattern for AST traversal
//!
//! Provides a clean interface for walking the expression tree without
//! manually handling the recursive structure.

use crate::{Expr, ExprKind};

/// Trait for visiting expression nodes in the AST
///
/// Implement this trait to define custom behavior when traversing expressions.
/// Each method returns a boolean indicating whether to continue visiting children.
///
/// # Example
/// ```ignore
/// use symb_anafis::{Expr, ExprVisitor};
///
/// struct NodeCounter { count: usize }
///
/// impl ExprVisitor for NodeCounter {
///     fn visit_number(&mut self, _n: f64) -> bool { self.count += 1; true }
///     fn visit_symbol(&mut self, _s: &str) -> bool { self.count += 1; true }
///     fn visit_function(&mut self, _name: &str, _args: &[Expr]) -> bool { self.count += 1; true }
///     fn visit_binary(&mut self, _op: &str, _left: &Expr, _right: &Expr) -> bool { self.count += 1; true }
///     fn visit_derivative(&mut self, _inner: &Expr, _var: &str, _order: u32) -> bool { self.count += 1; true }
/// }
/// ```
pub trait ExprVisitor {
    /// Visit a number literal, returns true to continue visiting
    fn visit_number(&mut self, n: f64) -> bool;

    /// Visit a symbol/variable, returns true to continue visiting
    fn visit_symbol(&mut self, name: &str) -> bool;

    /// Visit a function call, returns true to visit arguments
    fn visit_function(&mut self, name: &str, args: &[Expr]) -> bool;

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
            if visitor.visit_function(name, args) {
                for arg in args {
                    walk_expr(arg, visitor);
                }
            }
        }
        ExprKind::Add(l, r) => {
            if visitor.visit_binary("+", l, r) {
                walk_expr(l, visitor);
                walk_expr(r, visitor);
            }
        }
        ExprKind::Sub(l, r) => {
            if visitor.visit_binary("-", l, r) {
                walk_expr(l, visitor);
                walk_expr(r, visitor);
            }
        }
        ExprKind::Mul(l, r) => {
            if visitor.visit_binary("*", l, r) {
                walk_expr(l, visitor);
                walk_expr(r, visitor);
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

    fn visit_function(&mut self, _name: &str, _args: &[Expr]) -> bool {
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

    fn visit_function(&mut self, _name: &str, _args: &[Expr]) -> bool {
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
        let expr = x + x.pow(2.0); // x + x^2 = 5 nodes
        let mut counter = NodeCounter::default();
        walk_expr(&expr, &mut counter);
        assert_eq!(counter.count, 5); // +, x, ^, x, 2
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
