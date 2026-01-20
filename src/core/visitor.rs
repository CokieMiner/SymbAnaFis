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
/// N-ary operations (Sum, Product) have dedicated `visit_sum` and `visit_product` methods.
/// Binary operations (Div, Pow) use `visit_binary`.
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

    /// Visit an N-ary sum, returns true to visit all terms.
    /// Default: returns true (continue traversal). Override to inspect the sum.
    fn visit_sum(&mut self, _terms: &[Arc<Expr>]) -> bool {
        true
    }

    /// Visit an N-ary product, returns true to visit all factors.
    /// Default: returns true (continue traversal). Override to inspect the product.
    fn visit_product(&mut self, _factors: &[Arc<Expr>]) -> bool {
        true
    }

    /// Visit a derivative expression, returns true to visit inner expression
    fn visit_derivative(&mut self, inner: &Expr, var: &str, order: u32) -> bool;
}

/// Walk an expression tree with a visitor
///
/// Visits nodes in pre-order (parent before children).
/// The visitor methods return true to continue walking children, false to skip.
///
/// # Safety
/// This function uses recursion and may cause stack overflow on very deeply
/// nested expressions. For safety, expressions are limited to reasonable depth
/// in normal usage, but extremely deep expressions should be handled carefully.
pub fn walk_expr<V: ExprVisitor>(expr: &Expr, visitor: &mut V) {
    walk_expr_with_depth(expr, visitor, 0);
}

/// Internal recursive walker with depth tracking
fn walk_expr_with_depth<V: ExprVisitor>(expr: &Expr, visitor: &mut V, depth: usize) {
    // Prevent stack overflow on extremely deep expressions
    const MAX_DEPTH: usize = 1000;
    if depth > MAX_DEPTH {
        // In debug builds, panic to catch issues early
        debug_assert!(
            false,
            "Expression tree too deep (>{MAX_DEPTH} levels). \
             This may indicate a malformed expression or infinite recursion."
        );
        // In release builds, log warning and skip further traversal to prevent stack overflow
        #[cfg(not(debug_assertions))]
        eprintln!(
            "Warning: Expression tree exceeds maximum depth ({MAX_DEPTH}). \
             Traversal truncated to prevent stack overflow."
        );
        return;
    }

    // All ExprKind variants are handled below. If new variants are added to ExprKind,
    // this match statement must be updated accordingly.
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
                    walk_expr_with_depth(arg, visitor, depth + 1);
                }
            }
        }
        // N-ary Sum - use dedicated visit_sum method
        ExprKind::Sum(terms) => {
            if visitor.visit_sum(terms) {
                for term in terms {
                    walk_expr_with_depth(term, visitor, depth + 1);
                }
            }
        }
        // N-ary Product - use dedicated visit_product method
        ExprKind::Product(factors) => {
            if visitor.visit_product(factors) {
                for factor in factors {
                    walk_expr_with_depth(factor, visitor, depth + 1);
                }
            }
        }
        ExprKind::Div(l, r) => {
            if visitor.visit_binary("/", l, r) {
                walk_expr_with_depth(l, visitor, depth + 1);
                walk_expr_with_depth(r, visitor, depth + 1);
            }
        }
        ExprKind::Pow(l, r) => {
            if visitor.visit_binary("^", l, r) {
                walk_expr_with_depth(l, visitor, depth + 1);
                walk_expr_with_depth(r, visitor, depth + 1);
            }
        }
        ExprKind::Derivative { inner, var, order } => {
            if visitor.visit_derivative(inner, var.as_str(), *order) {
                walk_expr_with_depth(inner, visitor, depth + 1);
            }
        }
        // Poly: walk the base expression directly (avoid to_expr_terms() allocation)
        ExprKind::Poly(poly) => {
            // Walk the base expression which contains all variables
            walk_expr_with_depth(poly.base(), visitor, depth + 1);
        }
    }
}

/// A simple visitor that counts nodes in an expression.
#[derive(Default)]
pub struct NodeCounter {
    /// The number of nodes visited so far.
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

    fn visit_sum(&mut self, _terms: &[Arc<Expr>]) -> bool {
        self.count += 1;
        true
    }

    fn visit_product(&mut self, _factors: &[Arc<Expr>]) -> bool {
        self.count += 1;
        true
    }

    fn visit_derivative(&mut self, _inner: &Expr, _var: &str, _order: u32) -> bool {
        self.count += 1;
        true
    }
}

/// A visitor that collects all unique variable names in an expression.
#[derive(Default)]
pub struct VariableCollector {
    /// Set of all variable names found in the expression.
    pub variables: std::collections::HashSet<String>,
}

impl ExprVisitor for VariableCollector {
    fn visit_number(&mut self, _n: f64) -> bool {
        true
    }

    fn visit_symbol(&mut self, name: &str) -> bool {
        self.variables.insert(name.to_owned());
        true
    }

    fn visit_function(&mut self, _name: &str, _args: &[Arc<Expr>]) -> bool {
        true
    }

    fn visit_binary(&mut self, _op: &str, _left: &Expr, _right: &Expr) -> bool {
        true
    }

    fn visit_derivative(&mut self, _inner: &Expr, var: &str, _order: u32) -> bool {
        self.variables.insert(var.to_owned());
        true
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::let_underscore_must_use,
    clippy::no_effect_underscore_binding
)]
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

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "Expression tree too deep")]
    fn test_depth_limit() {
        // Create a deeply nested expression that exceeds MAX_DEPTH
        let x = symb("x");
        let mut expr = x.sin(); // Start with Expr
        for _ in 0..1200 {
            // Exceed the 1000 limit
            expr = expr.sin();
        }

        let mut counter = NodeCounter::default();
        // This should panic in debug builds to prevent stack overflow
        walk_expr(&expr, &mut counter);
    }
}
