//! Ordered multivariate derivatives over symbol identities and shared roots.

use core::{cell::OnceCell, num::NonZeroU32};

use alloc::{sync::Arc, vec::Vec};

use crate::diff::Differentiation;

use super::{Expr, Node, Symbol, SymbolData};

impl Expr {
    /// Compute \(\nabla f=(\partial_{x_{i}}f)_{i}\) in the supplied variable order.
    ///
    /// Symbols are compared by identity, including across contexts. Repeated
    /// variables repeat components; an empty list returns an empty vector.
    /// Formulas use the same regular principal-branch domains as [`Self::derivative`].
    /// Unsupported partials remain symbolic. No simplification sequence runs.
    ///
    /// # Panics
    /// Propagates user partial callbacks and numerical backend capacity panics.
    ///
    /// ```
    /// use symb_anafis::{Context, Expr};
    /// let context = Context::new();
    /// let x = context.symb("x")?;
    /// let y = context.symb("y")?;
    /// assert_eq!((x.pow(2) + x * y).gradient(&[x, y]), vec![2 * x + y, x.to_expr()]);
    /// # Ok::<(), symb_anafis::SymbolError>(())
    /// ```
    #[must_use]
    pub fn gradient(&self, variables: &[Symbol<'_>]) -> Vec<Self> {
        variables
            .iter()
            .map(|variable| Self::derivative(self.clone(), *variable, 1))
            .collect()
    }

    /// Compute \(J_{ij}=\partial_{x_{j}}f_{i}\) with outputs as rows and variables
    /// as columns. A column shares one differentiation memo across all outputs.
    ///
    /// Input order and repeated identities are preserved. Zero outputs return
    /// no rows; zero variables return one empty row per output. Derivative domains
    /// and unsupported partials follow [`Self::derivative`].
    ///
    /// # Panics
    /// Propagates callback/backend panics and allocation capacity exhaustion.
    #[must_use]
    pub fn jacobian(expressions: &[Self], variables: &[Symbol<'_>]) -> Vec<Vec<Self>> {
        if expressions.is_empty() {
            return Vec::new();
        }
        let mut rows: Vec<Vec<Self>> = expressions
            .iter()
            .map(|_| Vec::with_capacity(variables.len()))
            .collect();
        for variable in variables {
            let mut differentiation = Differentiation::new(*variable);
            let metadata = OnceCell::new();
            for (expression, row) in expressions.iter().zip(&mut rows) {
                let unresolved = |node: &Self| {
                    Node::normalize_derivative(
                        node.clone(),
                        Arc::clone(metadata.get_or_init(|| SymbolData::for_symbol(*variable))),
                        NonZeroU32::MIN,
                    )
                };
                let partial = differentiation
                    .evaluate(expression, unresolved)
                    .unwrap_or_else(|| unresolved(expression));
                row.push(partial);
            }
        }
        rows
    }

    /// Compute \(H_{ij}=\partial_{x_{j}}(\partial_{x_{i}}f)\).
    ///
    /// Rows follow the first derivative variable, columns the second. Both
    /// triangles are differentiated in their requested order: no symmetry or
    /// interchange of unsupported mixed partials is assumed. Smooth scalar
    /// functions have the usual symmetric Hessian on their regular domains.
    /// No additional simplifier runs.
    ///
    /// # Panics
    /// Propagates callback/backend panics and allocation capacity exhaustion.
    ///
    /// ```
    /// use symb_anafis::{Context, Expr};
    /// let context = Context::new();
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
