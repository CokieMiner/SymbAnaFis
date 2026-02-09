//! Expression visitor pattern for AST traversal
//!
//! Provides a clean interface for walking the expression tree without
//! manually handling the recursive structure.
//!
//! ## View API
//!
//! The `ExprView` enum provides a public, pattern-matchable view of expressions
//! without exposing internal implementation details like `Poly`. Use `expr.view()`
//! to get a stable API that works across internal representation changes.
//!
//! ```rust
//! use symb_anafis::{symb, visitor::ExprView};
//!
//! let x = symb("x");
//! let expr = x.pow(2.0) + x + 1.0;  // May be stored as Poly internally
//!
//! match expr.view() {
//!     ExprView::Sum(terms) => println!("Sum with {} terms", terms.len()),
//!     _ => println!("Not a sum"),
//! }
//! ```

use crate::{Expr, ExprKind};
use std::borrow::Cow;
use std::sync::Arc;

// =============================================================================
// EXPR VIEW - Public pattern-matchable API
// =============================================================================

/// A pattern-matchable view of an expression's structure.
///
/// This enum provides a stable public API for matching on expression types
/// without exposing internal implementation details. Unlike `ExprKind`, this
/// view abstracts over internal optimizations:
///
/// - `Poly` (internal optimization) is always presented as `Sum`
/// - Future changes to internal representation won't break user code
///
/// # Example
///
/// ```rust
/// use symb_anafis::{symb, visitor::ExprView};
///
/// let x = symb("view_x");
/// let expr = x.pow(2.0) + 2.0*x + 1.0;
///
/// match expr.view() {
///     ExprView::Sum(terms) => {
///         println!("Sum with {} terms", terms.len());
///         for term in terms.iter() {
///             println!("  term: {}", term);
///         }
///     }
///     ExprView::Number(n) => println!("Number: {}", n),
///     _ => println!("Other expression type"),
/// }
/// ```
#[derive(Debug)]
pub enum ExprView<'expr> {
    /// Number literal
    Number(f64),

    /// Variable or constant symbol
    ///
    /// The string is borrowed if it's a named symbol, or owned if it's an anonymous symbol
    /// (represented as "$id").
    Symbol(Cow<'expr, str>),

    /// Function call with name and arguments
    Function {
        /// Function name
        name: &'expr str,
        /// Function arguments (may be borrowed or owned depending on internal representation)
        args: &'expr [Arc<Expr>],
    },

    /// N-ary sum (a + b + c + ...)
    ///
    /// Note: May contain owned data if the expression was internally stored as a polynomial
    /// and needed to be expanded. Use `Cow::as_ref()` or iterate directly.
    Sum(Cow<'expr, [Arc<Expr>]>),

    /// N-ary product (a * b * c * ...)
    Product(Cow<'expr, [Arc<Expr>]>),

    /// Division (a / b)
    Div(&'expr Expr, &'expr Expr),

    /// Exponentiation (a ^ b)
    Pow(&'expr Expr, &'expr Expr),

    /// Derivative ∂^order/∂var^order of inner
    Derivative {
        /// Expression being differentiated
        inner: &'expr Expr,
        /// Variable name
        var: &'expr str,
        /// Derivative order (1 = first derivative, 2 = second, etc.)
        order: u32,
    },
}

impl ExprView<'_> {
    /// Check if this view represents a number
    #[must_use]
    pub const fn is_number(&self) -> bool {
        matches!(self, Self::Number(_))
    }

    /// Check if this view represents a symbol
    #[must_use]
    pub const fn is_symbol(&self) -> bool {
        matches!(self, Self::Symbol(_))
    }

    /// Check if this view represents a sum
    #[must_use]
    pub const fn is_sum(&self) -> bool {
        matches!(self, Self::Sum(_))
    }

    /// Check if this view represents a product
    #[must_use]
    pub const fn is_product(&self) -> bool {
        matches!(self, Self::Product(_))
    }

    /// Get the number value if this is a number
    #[must_use]
    pub const fn as_number(&self) -> Option<f64> {
        if let Self::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    /// Get the symbol name if this is a symbol
    #[must_use]
    pub fn as_symbol(&self) -> Option<&str> {
        if let Self::Symbol(s) = self {
            Some(s.as_ref())
        } else {
            None
        }
    }
}

// =============================================================================
// VISITOR TRAIT
// =============================================================================

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

/// A visitor that collects all unique variable symbols in an expression.
///
/// Uses `InternedSymbol` to avoid string allocations during traversal.
/// To get variable names as strings, use `variable_names()` method.
#[derive(Default)]
pub struct VariableCollector {
    /// Set of all variable symbols found in the expression (allocation-free collection).
    pub variables: rustc_hash::FxHashSet<crate::core::symbol::InternedSymbol>,
}

impl VariableCollector {
    /// Get variable names as strings (for compatibility).
    /// This allocates strings only when names are actually needed.
    #[must_use]
    pub fn variable_names(&self) -> std::collections::HashSet<String> {
        self.variables
            .iter()
            .filter_map(|s| s.name().map(str::to_owned))
            .collect()
    }
}

impl ExprVisitor for VariableCollector {
    fn visit_number(&mut self, _n: f64) -> bool {
        true
    }

    fn visit_symbol(&mut self, name: &str) -> bool {
        // Intern the symbol (O(1) if already exists) to store without allocation
        self.variables
            .insert(crate::core::symbol::symb_interned(name));
        true
    }

    fn visit_function(&mut self, _name: &str, _args: &[Arc<Expr>]) -> bool {
        true
    }

    fn visit_binary(&mut self, _op: &str, _left: &Expr, _right: &Expr) -> bool {
        true
    }

    fn visit_derivative(&mut self, _inner: &Expr, var: &str, _order: u32) -> bool {
        self.variables
            .insert(crate::core::symbol::symb_interned(var));
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
    clippy::no_effect_underscore_binding,
    reason = "Standard test relaxations"
)]
mod tests {
    use super::*;
    use crate::Expr;
    use crate::symb;

    #[test]
    fn test_node_counter() {
        let x = symb("x");
        let expr = x + x.pow(2.0); // x + x^2 = Poly with base x
        let mut counter = NodeCounter::default();
        walk_expr(&expr, &mut counter);
        // With optimized Poly: only walks the base expression (x = 1 symbol node)
        // Note: This test is dependent on Poly optimization. If Poly is disabled, this count will change.
        assert_eq!(counter.count, 1);
    }

    #[test]
    fn test_variable_collector() {
        let x = symb("x");
        let y = symb("y");
        let expr = x + y;
        let mut collector = VariableCollector::default();
        walk_expr(&expr, &mut collector);
        // Use variable_names() for string-based checking
        let names = collector.variable_names();
        assert!(names.contains("x"));
        assert!(names.contains("y"));
        assert_eq!(collector.variables.len(), 2);
    }

    #[test]
    fn test_expr_view_number() {
        let expr = Expr::number(42.0);
        let view = expr.view();
        assert!(view.is_number());
        assert_eq!(view.as_number(), Some(42.0));
    }

    #[test]
    fn test_expr_view_symbol() {
        let x: Expr = symb("view_test_x").into();
        let view = x.view();
        assert!(view.is_symbol());
        assert_eq!(view.as_symbol(), Some("view_test_x"));
    }

    #[test]
    fn test_expr_view_sum() {
        let x: Expr = symb("view_sum_x").into();
        let expr = x + 1.0;
        let view = expr.view();
        assert!(view.is_sum());
        if let ExprView::Sum(terms) = view {
            assert_eq!(terms.len(), 2);
        } else {
            panic!("Expected Sum view");
        }
    }

    #[test]
    fn test_expr_view_poly_as_sum() {
        // Create a polynomial expression (x^2 + 2x + 1)
        let x: Expr = symb("poly_view_x").into();
        let expr = x.clone().pow(2.0) + 2.0 * x + 1.0;

        // View should present it as a Sum, even if internally stored as Poly
        let view = expr.view();
        assert!(view.is_sum(), "Poly should be viewed as Sum");

        if let ExprView::Sum(terms) = view {
            assert!(!terms.is_empty(), "Should have at least one term");
        } else {
            panic!("Expected Sum view for polynomial");
        }
    }

    #[test]
    fn test_expr_view_div() {
        let x: Expr = symb("div_view_x").into();
        let expr = x / 2.0;
        if let ExprView::Div(num, den) = expr.view() {
            assert_eq!(num.to_string(), "div_view_x");
            assert_eq!(den.to_string(), "2");
        } else {
            panic!("Expected Div view");
        }
    }

    #[test]
    fn test_expr_view_function() {
        let x: Expr = symb("func_view_x").into();
        let expr = x.sin();
        if let ExprView::Function { name, args } = expr.view() {
            assert_eq!(name, "sin");
            assert_eq!(args.len(), 1);
        } else {
            panic!("Expected Function view");
        }
    }

    #[test]
    fn test_expr_view_pattern_matching() {
        let x: Expr = symb("pattern_x").into();
        let y: Expr = symb("pattern_y").into();

        let exprs = vec![
            (Expr::number(42.0), "number"),
            (x.clone(), "symbol"),
            (x.clone() + y.clone(), "sum"),
            (x.clone() * y.clone(), "product"),
            (x.clone() / y, "div"),
            (x.clone().pow(2.0), "pow"),
            (x.sin(), "function"),
        ];

        for (expr, expected) in exprs {
            let kind = match expr.view() {
                ExprView::Number(_) => "number",
                ExprView::Symbol(_) => "symbol",
                ExprView::Sum(_) => "sum",
                ExprView::Product(_) => "product",
                ExprView::Div(_, _) => "div",
                ExprView::Pow(_, _) => "pow",
                ExprView::Function { .. } => "function",
                ExprView::Derivative { .. } => "derivative",
            };
            assert_eq!(kind, expected, "Failed for expression: {expr}");
        }
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
