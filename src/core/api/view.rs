//! Borrowed expression matching without exposing construction internals.

use num_anafis::Number;

use super::{Expr, ExprChildren, Function, Symbol};

/// A borrowed view of an expression's mathematical structure.
///
/// Compact additive storage appears as a sum. Its terms are reconstructed only
/// when iterated. Creating a view does not allocate or consult a registry.
/// Symbol identities borrow their expression's metadata and cannot outlive it.
///
/// ```compile_fail
/// use symb_anafis::{Context, ExprView};
/// let symbol = {
///     let context = Context::new();
///     let expression = context.symb("x").expect("symbol allocation").to_expr();
///     match expression.view() {
///         ExprView::Symbol(symbol) => symbol,
///         _ => unreachable!(),
///     }
/// };
/// let expression = symbol + 1;
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub enum ExprView<'expr> {
    /// A literal value.
    Number(&'expr Number),
    /// The exact scalar imaginary unit, independent of variable names.
    ImaginaryUnit,
    /// The exact mathematical constant pi.
    Pi,
    /// Euler's exact number e.
    Euler,
    /// A validated identity borrowed from this expression's metadata.
    Symbol(Symbol<'expr>),
    /// A function with ordered arguments.
    Function {
        /// Function name.
        name: &'expr Function,
        /// Shared argument handles.
        args: &'expr [Expr],
    },
    /// Logical additive terms in deterministic order.
    Sum(ExprChildren<'expr>),
    /// Canonically ordered multiplicative factors.
    Product(&'expr [Expr]),
    /// A numerator and denominator.
    Div(&'expr Expr, &'expr Expr),
    /// A base and exponent.
    Pow(&'expr Expr, &'expr Expr),
    /// An unevaluated derivative.
    Derivative {
        /// The inner expression.
        inner: &'expr Expr,
        /// Differentiation variable's identity.
        var: Symbol<'expr>,
        /// Positive differentiation order.
        order: u32,
    },
}
