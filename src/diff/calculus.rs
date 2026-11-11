//! Derivatives of any order, gradients, Jacobians, and Hessians.

use core::{num::NonZeroU32, slice::from_ref};

use alloc::vec::Vec;

use crate::core::{ExactIdentity, Expr, Postorder, Symbol};

use super::Differentiation;

impl Expr {
    /// \(\partial^n f / \partial v^n\) for \(f =\) `self`, \(v =\) `var`, and
    /// \(n =\) `order`.
    ///
    /// Supported derivatives evaluate immediately with principal-branch formulas,
    /// valid where their factors are regular; domain restrictions are not stored.
    /// Unsupported cases keep a derivative node, and no simplification pass runs.
    /// Order zero returns `self`. Nested derivatives in the same variable combine
    /// while the total order fits `u32`.
    ///
    /// When repeated differentiation returns to an earlier expression, the
    /// remaining order is reduced modulo the cycle length, found with Brent's
    /// cycle detection (R. P. Brent, BIT 20 (1980) 176-184).
    ///
    /// # Panics
    /// Propagates panics of custom partial-derivative callbacks.
    #[must_use]
    pub fn derivative(self, var: Symbol<'_>, order: u32) -> Self {
        let Some(mut remaining) = NonZeroU32::new(order) else {
            return self;
        };
        let mut current = self;
        let mut checkpoint = (order > 1).then(|| current.clone());
        let mut span = 0_u32;
        let mut window = 1_u32;
        let mut engine = Differentiation::new(var);
        loop {
            let Some(next) = engine.evaluate(&Postorder::compact(&current)) else {
                return var.metadata().derivative_of(current, remaining);
            };
            if next.is_exact_zero() || next == current {
                return next;
            }
            current = next;
            let Some(rest) = NonZeroU32::new(remaining.get().saturating_sub(1)) else {
                return current;
            };
            remaining = rest;
            // Differentiation is deterministic, so a repeated state repeats with
            // a fixed period. Brent's method keeps one checkpoint and doubles its
            // window; `span` stays below the requested `u32` order.
            span = span.saturating_add(1);
            if checkpoint.as_ref() == Some(&current) {
                let Some(reduced) = remaining.get().checked_rem(span).and_then(NonZeroU32::new)
                else {
                    return current;
                };
                remaining = reduced;
                span = 0;
                checkpoint = Some(current.clone());
            } else if span == window {
                checkpoint = Some(current.clone());
                span = 0;
                window = window.saturating_mul(2);
            }
        }
    }

    /// The gradient \(\nabla f = (\partial f / \partial x_{i})_{i}\) in the given
    /// variable order.
    ///
    /// Symbols compare by identity, also across contexts. A repeated variable
    /// repeats its component, and no variables give an empty vector. Domains and
    /// unsupported partials follow [`Self::derivative`]; no simplification runs.
    ///
    /// # Panics
    /// Propagates panics of custom partial-derivative callbacks.
    ///
    /// ```
    /// use symb_anafis::{Context, Expr};
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let y = context.symb("y")?;
    /// assert_eq!((x.pow(2) + x * y).gradient(&[x, y]), vec![2 * x + y, x.to_expr()]);
    /// # Ok::<(), symb_anafis::SymbolError>(())
    /// ```
    #[must_use]
    pub fn gradient(&self, variables: &[Symbol<'_>]) -> Vec<Self> {
        // The single row of the Jacobian; one numbering serves every variable.
        Self::jacobian(from_ref(self), variables)
            .pop()
            .unwrap_or_default()
    }

    /// The Jacobian \(J_{ij} = \partial f_{i} / \partial x_{j}\), with one row per
    /// expression and one column per variable.
    ///
    /// All rows are numbered once, so each column differentiates a node shared
    /// between rows once. Order and repeated variables are kept. No expressions
    /// give no rows; no variables give one empty row per expression. Domains
    /// and unsupported partials follow [`Self::derivative`].
    ///
    /// # Panics
    /// Propagates panics of custom partial-derivative callbacks.
    #[must_use]
    pub fn jacobian(expressions: &[Self], variables: &[Symbol<'_>]) -> Vec<Vec<Self>> {
        if expressions.is_empty() {
            return Vec::new();
        }
        let mut rows: Vec<Vec<Self>> = expressions
            .iter()
            .map(|_| Vec::with_capacity(variables.len()))
            .collect();
        if variables.is_empty() {
            return rows;
        }
        let forest = Postorder::forest(expressions);
        for variable in variables {
            let column = Differentiation::new(*variable).evaluate_forest(&forest);
            for (row, partial) in rows.iter_mut().zip(column) {
                row.push(partial);
            }
        }
        rows
    }

    /// The Hessian \(H_{ij} = \partial (\partial f / \partial x_{i}) / \partial x_{j}\).
    ///
    /// Rows follow the first differentiation variable and columns the second.
    /// Each entry is differentiated in its own order, so the symmetry of mixed
    /// partials is not assumed; it holds for smooth functions on their regular
    /// domains. No simplification runs.
    ///
    /// # Panics
    /// Propagates panics of custom partial-derivative callbacks.
    ///
    /// ```
    /// use symb_anafis::{Context, Expr};
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let y = context.symb("y")?;
    /// assert_eq!((x.pow(2) + x * y).hessian(&[x, y]), vec![
    ///     vec![Expr::number(2), Expr::number(1)],
    ///     vec![Expr::number(1), Expr::number(0)],
    /// ]);
    /// # Ok::<(), symb_anafis::SymbolError>(())
    /// ```
    #[must_use]
    pub fn hessian(&self, variables: &[Symbol<'_>]) -> Vec<Vec<Self>> {
        Self::jacobian(&self.gradient(variables), variables)
    }
}

/// Unified Jacobian method on collections of expressions.
///
/// Implemented for slices `[Expr]`, arrays `[Expr; N]`, and [`Vec<Expr>`].
///
/// # Examples
///
/// ```
/// use symb_anafis::{Context, Expr, Jacobian};
///
/// let ctx = Context::new()?;
/// let x = ctx.symb("x")?;
/// let y = ctx.symb("y")?;
///
/// // Directly on a fixed array of expressions:
/// let j = [x.pow(2) + x * y, x * y].jacobian(&[x, y]);
/// assert_eq!(j, vec![
///     vec![2 * x + y, x.to_expr()],
///     vec![y.to_expr(), x.to_expr()],
/// ]);
///
/// // Or on a Vec / slice:
/// let outputs = vec![x.pow(2) + x * y, x * y];
/// assert_eq!(outputs.jacobian(&[x, y]), j);
/// # Ok::<(), symb_anafis::SymbolError>(())
/// ```
pub trait Jacobian {
    /// The Jacobian matrix with one row per expression and one column per variable.
    fn jacobian(&self, variables: &[Symbol<'_>]) -> Vec<Vec<Expr>>;
}

impl Jacobian for [Expr] {
    fn jacobian(&self, variables: &[Symbol<'_>]) -> Vec<Vec<Expr>> {
        Expr::jacobian(self, variables)
    }
}

impl Jacobian for Vec<Expr> {
    fn jacobian(&self, variables: &[Symbol<'_>]) -> Vec<Vec<Expr>> {
        Expr::jacobian(self, variables)
    }
}

impl<const N: usize> Jacobian for [Expr; N] {
    fn jacobian(&self, variables: &[Symbol<'_>]) -> Vec<Vec<Expr>> {
        Expr::jacobian(self, variables)
    }
}
