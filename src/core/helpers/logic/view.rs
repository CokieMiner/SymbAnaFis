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

use crate::Expr;
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
