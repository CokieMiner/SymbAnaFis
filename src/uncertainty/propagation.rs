//! First-order propagation of a model's sensitivities through a covariance.

use core::ops::Div;

use alloc::vec::Vec;

use crate::{
    collections::FixedSet,
    core::{Expr, Property, Symbol, Truth},
    errors::UncertaintyError,
};

use super::{CovarianceMatrix, check_unique_variables};

/// A model, its sensitivity coefficients, and its first-order variance.
///
/// For a real differentiable model on real coordinates, propagation constructs
/// $u_{f}^2=\nabla f^T\Sigma\nabla f$. All coordinates, covariances, and
/// derivatives must be defined at the point of use; the covariance must be
/// positive semidefinite and the linear approximation adequate. Symbolic
/// properties do not establish these local modelling hypotheses.
///
/// The returned variance is an exact symbolic expression, so it carries no
/// rounding error of its own. Evaluating it in floating point does: the
/// expanded form cancels for correlated inputs, and the library cannot control
/// how the caller evaluates the result. A caller who needs a numerically stable
/// value should build the covariance with [`CovarianceMatrix::from_factor`],
/// which contracts a sum of squares instead, and evaluate that.
#[derive(Clone, Debug)]
pub struct Uncertainty {
    expression: Expr,
    sensitivities: Vec<Expr>,
    variance: Expr,
}

impl Uncertainty {
    /// Propagate $u_{f}^2=\sum_{i} g_{i}^2\Sigma_{ii}
    /// +2\sum_{i<j}g_{i} g_{j}\Sigma_{ij}$, where $g_{i}=\partial_{x_{i}}f$.
    ///
    /// Coordinates must be distinct identities in covariance order. A
    /// coordinate the model does not mention contributes exactly zero, which is
    /// correct for a parameter the model does not use; a coordinate that shares
    /// a name with a model symbol but is a different identity is rejected,
    /// because that is what a coordinate taken from another [`crate::Context`]
    /// looks like and it would otherwise drop its variance silently. Other
    /// symbols are held fixed. The caller supplies real-coordinate, regularity,
    /// covariance validity, and first-order adequacy hypotheses. Neither
    /// differentiation nor propagation invokes a separate simplifier.
    ///
    /// # Errors
    /// Returns [`UncertaintyError`] for a dimension mismatch, repeated
    /// coordinate, coordinate that shares a model symbol's name under another
    /// identity, or disproved finite/real model, sensitivity, or nonnegative
    /// variance.
    ///
    /// # Panics
    /// Propagates panics of custom partial-derivative callbacks.
    ///
    /// ```
    /// use symb_anafis::{Context, CovarianceMatrix, Expr, Uncertainty};
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let y = context.symb("y")?;
    /// let covariance = CovarianceMatrix::diagonal(vec![Expr::number(4), Expr::number(9)])?;
    /// let result = Uncertainty::propagate(&(x + 2 * y), &[x, y], &covariance)?;
    /// assert_eq!(result.variance(), &Expr::number(40));
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn propagate(
        expression: &Expr,
        variables: &[Symbol<'_>],
        covariance: &CovarianceMatrix,
    ) -> Result<Self, UncertaintyError> {
        if variables.len() != covariance.dimension() {
            return Err(UncertaintyError::DimensionMismatch {
                variables: variables.len(),
                covariance: covariance.dimension(),
            });
        }
        if !variables.is_empty() {
            check_unique_variables(variables)?;
            // A coordinate the model never mentions has a zero sensitivity, so it
            // contributes nothing. That is right for a parameter the model does not
            // use, but it is also what a coordinate from another context looks
            // like: the same spelling, a different identity. The two are told apart
            // by name, which is the only thing they share. One traversal collects
            // the model's identities and names; a per-coordinate search would be a
            // traversal each.
            let symbols = expression.symbols();
            let mut named: Option<FixedSet<&str>> = None;
            for (index, variable) in variables.iter().enumerate() {
                if symbols.contains(variable) {
                    continue;
                }
                let Some(name) = variable.as_str() else {
                    continue;
                };
                let known = named.get_or_insert_with(|| {
                    let mut set = FixedSet::default();
                    for symbol in &symbols {
                        if let Some(symbol_name) = symbol.as_str() {
                            set.insert(symbol_name);
                        }
                    }
                    set
                });
                if known.contains(name) {
                    return Err(UncertaintyError::ForeignCoordinate { index });
                }
            }
        }
        let properties = expression.properties();
        if properties.get(Property::Finite) == Truth::False
            || properties.get(Property::Real) == Truth::False
        {
            return Err(UncertaintyError::InvalidModel);
        }
        let sensitivities = expression.gradient(variables);
        let variance = covariance.contract(&sensitivities)?;
        Ok(Self {
            expression: expression.clone(),
            sensitivities,
            variance,
        })
    }

    /// Contract the cached sensitivities against another covariance of the
    /// same dimension, without differentiating the model again.
    ///
    /// The gradient does not depend on the covariance, so a study over $k$
    /// covariances pays one differentiation and $k$ contractions instead of
    /// $k$ differentiations. The contraction policy, including zero
    /// annihilation and numeric-tier retention, is the one
    /// [`CovarianceMatrix::contract`] applies.
    ///
    /// # Errors
    /// Returns [`UncertaintyError::DimensionMismatch`] when the covariance
    /// dimension differs from the coordinate count, or the errors of
    /// [`CovarianceMatrix::contract`].
    pub fn recontract(&self, covariance: &CovarianceMatrix) -> Result<Expr, UncertaintyError> {
        covariance.contract(&self.sensitivities)
    }

    /// Borrow the model whose uncertainty was propagated.
    #[must_use]
    pub const fn expression(&self) -> &Expr {
        &self.expression
    }

    /// Borrow $(\partial_{x_{i}}f)_{i}$ in covariance coordinate order.
    #[must_use]
    pub fn sensitivities(&self) -> &[Expr] {
        &self.sensitivities
    }

    /// Borrow the first-order combined variance $u_{f}^2$.
    #[must_use]
    pub const fn variance(&self) -> &Expr {
        &self.variance
    }

    /// Construct the combined standard uncertainty $u_{f}=\sqrt{u_{f}^2}$.
    #[must_use]
    pub fn standard_deviation(&self) -> Expr {
        self.variance.sqrt()
    }

    /// Construct relative standard uncertainty $u_{f}/\lvert f\rvert$.
    /// The expression retains the exclusion $f=0$, including zero uncertainty.
    #[must_use]
    pub fn relative(&self) -> Expr {
        Div::div(self.variance.sqrt(), self.expression.abs())
    }
}
