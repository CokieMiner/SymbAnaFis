//! Derivatives of any order, mixed partials, directional derivatives,
//! gradients, Jacobians, Hessians, and Laplacians.

use core::num::NonZeroU32;

use alloc::{vec, vec::Vec};

use crate::core::{Expr, Postorder, Symbol, SymbolData};

use super::Differentiation;

impl Expr {
    /// $\partial^n f / \partial v^n$ for $f =$ `self`, $v =$ `var`, and
    /// $n =$ `order`.
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
            let (next, resolved) = engine.evaluate(&Postorder::compact(&current));
            if !resolved {
                return if remaining == NonZeroU32::MIN {
                    next
                } else {
                    SymbolData::of(var).derivative_of(current, remaining)
                };
            }
            let Some(rest) = NonZeroU32::new(remaining.get().saturating_sub(1)) else {
                return next;
            };
            if next.is_exact_zero() || next == current {
                return next;
            }
            current = next;
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

    /// Mixed partial derivative applying each `(variable, order)` in sequence,
    /// stopping early if exact zero is reached.
    ///
    /// An empty slice returns `self`. Operator order is preserved, so
    /// unresolved mixed partials keep the exact sequence in which they were
    /// requested.
    ///
    /// # Panics
    /// Propagates panics of custom partial-derivative callbacks.
    ///
    /// ```
    /// use symb_anafis::{Context, Expr};
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let y = context.symb("y")?;
    /// assert_eq!((x.pow(3) * y.pow(2)).partial_derivatives(&[(x, 2), (y, 1)]), 12 * x * y);
    /// # Ok::<(), symb_anafis::SymbolError>(())
    /// ```
    #[must_use]
    pub fn partial_derivatives(mut self, orders: &[(Symbol<'_>, u32)]) -> Self {
        for &(variable, order) in orders {
            if self.is_exact_zero() {
                return self;
            }
            self = self.derivative(variable, order);
        }
        self
    }

    /// The directional derivative (forward-mode Jacobian-vector product)
    /// $D_{v} f = \sum_{i} (\partial f / \partial x_{i}) v_{i}$ along the
    /// `(variable, component)` pairs in `direction`, evaluated in a single pass
    /// over the DAG.
    ///
    /// Duplicate variables sum their direction components, and an empty
    /// direction yields exact zero. Domains and unsupported partials follow
    /// [`Self::derivative`]; no simplification runs.
    ///
    /// # Panics
    /// Propagates panics of custom partial-derivative callbacks.
    ///
    /// ```
    /// use symb_anafis::{Context, Expr};
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let y = context.symb("y")?;
    /// let f = x.pow(2) + y.pow(3);
    /// assert_eq!(
    ///     f.directional_derivative(&[(x, Expr::number(2)), (y, Expr::number(3))]),
    ///     4 * x + 9 * y.pow(2)
    /// );
    /// # Ok::<(), symb_anafis::SymbolError>(())
    /// ```
    #[must_use]
    pub fn directional_derivative(&self, direction: &[(Symbol<'_>, Self)]) -> Self {
        let Some(mut engine) = Differentiation::directional(direction) else {
            return Self::number(0);
        };
        engine.evaluate(&Postorder::compact(self)).0
    }

    /// The gradient $\nabla f = (\partial f / \partial x_{i})_{i}$ in the given
    /// variable order.
    ///
    /// Symbols compare by identity, also across contexts. A repeated variable
    /// reuses its first computed component, and no variables give an empty
    /// vector. Domains and unsupported partials follow [`Self::derivative`];
    /// no simplification runs.
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
        let mut gradient = Vec::with_capacity(variables.len());
        let Some((first, _)) = variables.split_first() else {
            return gradient;
        };
        // One numbering serves every variable, and the row is built directly
        // instead of through `jacobian`.
        let order = Postorder::compact(self);
        let mut engine = Differentiation::new(*first);
        for (index, variable) in variables.iter().enumerate() {
            if let Some(previous) = variables
                .get(..index)
                .and_then(|prior| prior.iter().position(|seen| seen == variable))
                .and_then(|slot| gradient.get(slot))
            {
                gradient.push(previous.clone());
                continue;
            }
            engine.rebind(*variable);
            gradient.push(engine.evaluate(&order).0);
        }
        gradient
    }

    /// The Jacobian $J_{ij} = \partial f_{i} / \partial x_{j}$, with one row per
    /// expression and one column per variable.
    ///
    /// All rows are numbered once, so each column differentiates a node shared
    /// between rows once, and repeated variables reuse their first column. No
    /// expressions give no rows; no variables give one empty row per expression.
    /// Domains and unsupported partials follow [`Self::derivative`].
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
        let Some((first, _)) = variables.split_first() else {
            return rows;
        };
        let forest = Postorder::forest(expressions);
        // One engine serves every column: the constant handles and the argument
        // scratch outlive the variable they were built for.
        let mut engine = Differentiation::new(*first);
        for (index, variable) in variables.iter().enumerate() {
            if let Some(previous) = variables
                .get(..index)
                .and_then(|prior| prior.iter().position(|seen| seen == variable))
            {
                for row in &mut rows {
                    if let Some(cached) = row.get(previous).cloned() {
                        row.push(cached);
                    }
                }
                continue;
            }
            engine.rebind(*variable);
            let column = engine.evaluate_forest(&forest);
            for (row, partial) in rows.iter_mut().zip(column) {
                row.push(partial);
            }
        }
        rows
    }

    /// The Hessian $H_{ij} = \partial (\partial f / \partial x_{i}) / \partial x_{j}$.
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

    /// The Laplacian $\Delta f = \sum_{i} \partial^{2} f / \partial x_{i}^{2}$,
    /// computing only the diagonal second derivatives without forming the full
    /// Hessian.
    ///
    /// An empty variable list returns exact zero. Repeated variables contribute
    /// their diagonal entry once per occurrence, matching the trace of
    /// [`Self::hessian`].
    ///
    /// # Panics
    /// Propagates panics of custom partial-derivative callbacks.
    ///
    /// ```
    /// use symb_anafis::{Context, Expr};
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let y = context.symb("y")?;
    /// assert_eq!((x.pow(3) + y.pow(2)).laplacian(&[x, y]), 6 * x + 2);
    /// # Ok::<(), symb_anafis::SymbolError>(())
    /// ```
    #[must_use]
    pub fn laplacian(&self, variables: &[Symbol<'_>]) -> Self {
        let Some((first, _)) = variables.split_first() else {
            return Self::number(0);
        };
        let order = Postorder::compact(self);
        let mut engine = Differentiation::new(*first);
        let mut computed: Vec<(Symbol<'_>, Self)> = Vec::with_capacity(variables.len());
        let mut terms = Vec::with_capacity(variables.len());
        for &variable in variables {
            let second =
                if let Some((_, cached)) = computed.iter().find(|(seen, _)| *seen == variable) {
                    cached.clone()
                } else {
                    engine.rebind(variable);
                    let (first_partial, resolved) = engine.evaluate(&order);
                    let second = if !resolved {
                        SymbolData::of(variable)
                            .derivative_of(self.clone(), NonZeroU32::MIN.saturating_add(1))
                    } else if first_partial.is_exact_zero() {
                        first_partial
                    } else {
                        engine.evaluate(&Postorder::compact(&first_partial)).0
                    };
                    computed.push((variable, second.clone()));
                    second
                };
            if !second.is_exact_zero() {
                terms.push(second);
            }
        }
        Self::sum(terms)
    }
}

impl Symbol<'_> {
    /// The derivative of this symbol; see [`Expr::derivative`].
    #[must_use]
    pub fn derivative(&self, var: Self, order: u32) -> Expr {
        match order {
            0 => self.to_expr(),
            1 if *self == var => Expr::number(1),
            _ => Expr::number(0),
        }
    }

    /// The gradient of this symbol; see [`Expr::gradient`].
    #[must_use]
    pub fn gradient(&self, variables: &[Self]) -> Vec<Expr> {
        variables
            .iter()
            .map(|&variable| Expr::number(i64::from(variable == *self)))
            .collect()
    }

    /// The Hessian of this symbol; see [`Expr::hessian`].
    #[must_use]
    pub fn hessian(&self, variables: &[Self]) -> Vec<Vec<Expr>> {
        vec![vec![Expr::number(0); variables.len()]; variables.len()]
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
