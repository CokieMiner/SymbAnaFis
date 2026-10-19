//! Real first-order covariance contractions over canonical scalar expressions.

use core::ops::Div;

use alloc::vec::Vec;

use crate::{
    collections::HashMap,
    core::{Expr, Property, Symbol, Truth},
    errors::UncertaintyError,
};

/// Symmetric covariance expressions in an explicit coordinate order.
///
/// Entries represent finite real covariances; diagonal entries represent
/// nonnegative variances. Constructors validate shape and exact symmetry, and
/// reject disproved scalar conditions. Unknown conditions and positive
/// semidefiniteness remain the caller's mathematical responsibility.
/// No approximate tolerance or numeric-to-symbolic promotion is introduced.
#[derive(Clone, Debug)]
pub struct CovarianceMatrix {
    entries: Storage,
    zero: Expr,
}

#[derive(Clone, Debug)]
enum Storage {
    Dense {
        rows: Vec<Vec<Expr>>,
        mirrored: bool,
    },
    Diagonal(Vec<Expr>),
}

#[derive(Clone, Copy)]
struct Sensitivity<'expression> {
    expression: &'expression Expr,
    finite: bool,
    zero: bool,
}

/// A model, its sensitivity coefficients, and its first-order variance.
///
/// For a real differentiable model on real coordinates, propagation constructs
/// \(u_{f}^2=\nabla f^T\Sigma\nabla f\). All coordinates, covariances, and
/// derivatives must be defined at the point of use; the covariance must be
/// positive semidefinite and the linear approximation adequate. Symbolic
/// properties do not establish these local modelling hypotheses.
#[derive(Clone, Debug)]
pub struct Uncertainty {
    expression: Expr,
    sensitivities: Vec<Expr>,
    variance: Expr,
}

impl CovarianceMatrix {
    /// Store a square symmetric covariance matrix. Canonical identity establishes
    /// symbolic symmetry; backend numeric equality also accepts equal literals
    /// with different exact/approximate representations.
    ///
    /// # Errors
    /// Returns [`UncertaintyError`] for shape/symmetry violations, disproved
    /// finite real entries, or disproved nonnegative diagonal variances.
    /// Unknown scalar conditions and positive semidefiniteness are caller-owned.
    ///
    /// # Panics
    /// Propagates numeric backend capacity panics during property analysis.
    pub fn new(entries: Vec<Vec<Expr>>) -> Result<Self, UncertaintyError> {
        let dimension = entries.len();
        let mut mirrored = true;
        for (row_index, row) in entries.iter().enumerate() {
            if row.len() != dimension {
                return Err(UncertaintyError::NonSquare {
                    row: row_index,
                    expected: dimension,
                    actual: row.len(),
                });
            }
            for (column, (value, opposite_row)) in
                row.iter().zip(&entries).enumerate().skip(row_index)
            {
                // This read validates a future row before that row's own boundary
                // check. Missing entries are genuine malformed input, not an
                // infallible kernel failure.
                let opposite = opposite_row
                    .get(row_index)
                    .ok_or(UncertaintyError::NonSquare {
                        row: column,
                        expected: dimension,
                        actual: opposite_row.len(),
                    })?;
                if value != opposite {
                    if !value
                        .as_number()
                        .zip(opposite.as_number())
                        .is_some_and(|(first, second)| first == second)
                    {
                        return Err(UncertaintyError::Asymmetric {
                            row: row_index,
                            column,
                        });
                    }
                    mirrored = false;
                }
                if !Self::valid_entry(value, row_index == column) {
                    return Err(UncertaintyError::InvalidCovariance {
                        row: row_index,
                        column,
                    });
                }
            }
        }
        Ok(Self {
            entries: Storage::Dense {
                rows: entries,
                mirrored,
            },
            zero: Expr::number(0),
        })
    }

    /// Store independent variances in linear space. Inputs are variances,
    /// \(u_{i}^2\), rather than standard deviations \(u_{i}\).
    ///
    /// # Errors
    /// Returns [`UncertaintyError::InvalidCovariance`] for a disproved finite,
    /// real, or nonnegative variance. Unknown hypotheses belong to the caller.
    ///
    /// # Panics
    /// Propagates numeric backend capacity panics during property analysis.
    pub fn diagonal(variances: Vec<Expr>) -> Result<Self, UncertaintyError> {
        for (index, variance) in variances.iter().enumerate() {
            if !Self::valid_entry(variance, true) {
                return Err(UncertaintyError::InvalidCovariance {
                    row: index,
                    column: index,
                });
            }
        }
        Ok(Self {
            entries: Storage::Diagonal(variances),
            zero: Expr::number(0),
        })
    }

    /// Read the number of coordinates, including zero-dimensional matrices.
    #[must_use]
    pub const fn dimension(&self) -> usize {
        match &self.entries {
            Storage::Dense { rows, .. } => rows.len(),
            Storage::Diagonal(variances) => variances.len(),
        }
    }

    /// Borrow an entry; diagonal storage supplies its owned exact off-diagonal zero.
    /// Out-of-range indices return `None`.
    #[must_use]
    pub fn get(&self, row: usize, column: usize) -> Option<&Expr> {
        match &self.entries {
            Storage::Dense { rows, .. } => rows.get(row)?.get(column),
            Storage::Diagonal(variances) => {
                if row >= variances.len() || column >= variances.len() {
                    None
                } else if row == column {
                    variances.get(row)
                } else {
                    Some(&self.zero)
                }
            }
        }
    }
}

impl Uncertainty {
    /// Propagate \(u_{f}^2=\sum_{i} g_{i}^2\Sigma_{ii}
    /// +2\sum_{i<j}g_{i} g_{j}\Sigma_{ij}\), where \(g_{i}=\partial_{x_{i}}f\).
    ///
    /// Coordinates must be distinct identities in covariance order. Other
    /// symbols are held fixed. The caller supplies real-coordinate, regularity,
    /// covariance validity, and first-order adequacy hypotheses. Neither
    /// differentiation nor propagation invokes a separate simplifier.
    ///
    /// # Errors
    /// Returns [`UncertaintyError`] for a dimension mismatch, repeated coordinate,
    /// or disproved finite/real model, sensitivity, or nonnegative variance.
    ///
    /// # Panics
    /// Propagates partial callback and numeric backend capacity panics.
    ///
    /// ```
    /// use symb_anafis::{Context, CovarianceMatrix, Expr, Uncertainty};
    /// let context = Context::new();
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
        let mut identities = HashMap::with_capacity(variables.len());
        for (index, variable) in variables.iter().enumerate() {
            if let Some(first) = identities.insert(*variable, index) {
                return Err(UncertaintyError::DuplicateVariable {
                    first,
                    duplicate: index,
                });
            }
        }
        if !Self::finite_real(expression) {
            return Err(UncertaintyError::InvalidModel);
        }
        let sensitivities = expression.gradient(variables);
        let mut checked = Vec::with_capacity(sensitivities.len());
        for (index, partial) in sensitivities.iter().enumerate() {
            let properties = partial.properties();
            let finite = properties.get(Property::Finite);
            if finite == Truth::False || properties.get(Property::Real) == Truth::False {
                return Err(UncertaintyError::InvalidSensitivity { index });
            }
            checked.push(Sensitivity {
                expression: partial,
                finite: finite == Truth::True,
                zero: partial.is_zero_num(),
            });
        }
        let variance = covariance.contract(&checked);
        if !CovarianceMatrix::valid_entry(&variance, true) {
            return Err(UncertaintyError::InvalidVariance);
        }
        Ok(Self {
            expression: expression.clone(),
            sensitivities,
            variance,
        })
    }

    /// Borrow the model whose uncertainty was propagated.
    #[must_use]
    pub const fn expression(&self) -> &Expr {
        &self.expression
    }

    /// Borrow \((\partial_{x_{i}}f)_{i}\) in covariance coordinate order.
    #[must_use]
    pub fn sensitivities(&self) -> &[Expr] {
        &self.sensitivities
    }

    /// Borrow the first-order combined variance \(u_{f}^2\).
    #[must_use]
    pub const fn variance(&self) -> &Expr {
        &self.variance
    }

    /// Construct the combined standard uncertainty \(u_{f}=\sqrt{u_{f}^2}\).
    ///
    /// # Panics
    /// Propagates numeric backend capacity panics.
    #[must_use]
    pub fn standard_deviation(&self) -> Expr {
        self.variance.sqrt()
    }

    /// Construct relative standard uncertainty \(u_{f}/\lvert f\rvert\).
    /// The expression retains the exclusion \(f=0\), including zero uncertainty.
    ///
    /// # Panics
    /// Propagates numeric backend capacity panics.
    #[must_use]
    pub fn relative(&self) -> Expr {
        Div::div(self.variance.sqrt(), self.expression.abs())
    }

    fn finite_real(expression: &Expr) -> bool {
        let properties = expression.properties();
        properties.get(Property::Finite) != Truth::False
            && properties.get(Property::Real) != Truth::False
    }
}

impl CovarianceMatrix {
    /// Shape, identity order, and equal sensitivity count have been validated.
    /// Each diagonal is retained, so a zero covariance cannot hide a sensitivity
    /// pole. Diagonal storage needs only O(n) terms; dense storage visits the
    /// upper triangle when opposite entries share their canonical representation.
    /// Mixed numeric representations use both entries to retain approximation.
    /// Cached sensitivity proofs avoid repeated DAG analyses.
    fn contract(&self, sensitivities: &[Sensitivity<'_>]) -> Expr {
        if sensitivities.is_empty() {
            return self.zero.clone();
        }
        let mut terms = Vec::with_capacity(sensitivities.len());
        match &self.entries {
            Storage::Diagonal(variances) => {
                for (partial, variance) in sensitivities.iter().zip(variances) {
                    terms.push(Self::term(*partial, *partial, variance, None));
                }
            }
            Storage::Dense {
                rows,
                mirrored: true,
            } => {
                let two = Expr::number(2);
                for (index, (first, row)) in sensitivities.iter().zip(rows).enumerate() {
                    for (offset, (second, covariance)) in
                        sensitivities.iter().zip(row).skip(index).enumerate()
                    {
                        // Every diagonal retains its sensitivity's exclusions.
                        // An exact off-diagonal zero adds no further domain;
                        // approximate zeros remain to retain their numeric tier.
                        if offset == 0 || covariance != &self.zero {
                            terms.push(Self::term(
                                *first,
                                *second,
                                covariance,
                                (offset != 0).then_some(&two),
                            ));
                        }
                    }
                }
            }
            Storage::Dense {
                rows,
                mirrored: false,
            } => {
                for (index, (first, row)) in sensitivities.iter().zip(rows).enumerate() {
                    for (column, (second, covariance)) in sensitivities.iter().zip(row).enumerate()
                    {
                        if column == index || covariance != &self.zero {
                            terms.push(Self::term(*first, *second, covariance, None));
                        }
                    }
                }
            }
        }
        Expr::sum(terms)
    }

    fn term(
        first: Sensitivity<'_>,
        second: Sensitivity<'_>,
        covariance: &Expr,
        multiplier: Option<&Expr>,
    ) -> Expr {
        // A zero can annihilate only finite other factors. Unknown poles remain
        // explicit instead of being removed by a zero variance or covariance.
        let covariance_zero = first.finite && second.finite && covariance.is_zero_num();
        // Constructors already established finite numeric entries. Only
        // nonliteral covariances can need a further finite-value proof here.
        let annihilates = covariance_zero
            || ((first.zero && second.finite) || (second.zero && first.finite))
                && (covariance.as_number().is_some()
                    || covariance.properties().get(Property::Finite) == Truth::True);
        // A zero leads numeric folding so finite coefficients cannot overflow
        // before multiplication by zero. Numeric factors remain: 0 * 2.0 must
        // stay approximate even when proved finite symbolic factors disappear.
        let expressions = if covariance_zero {
            [covariance, first.expression, second.expression]
        } else if annihilates && second.zero {
            [second.expression, first.expression, covariance]
        } else {
            [first.expression, second.expression, covariance]
        };
        // At most three original factors and one cross-term multiplier: no
        // reallocation is needed when adding the symmetric factor of two.
        let mut factors = Vec::with_capacity(4);
        for expression in expressions {
            if !annihilates || expression.as_number().is_some() {
                factors.push(expression.clone());
            }
        }
        if let Some(factor) = multiplier {
            factors.push(factor.clone());
        }
        Expr::product(factors)
    }

    fn valid_entry(expression: &Expr, diagonal: bool) -> bool {
        let properties = expression.properties();
        properties.get(Property::Finite) != Truth::False
            && properties.get(Property::Real) != Truth::False
            && (!diagonal || properties.get(Property::Nonnegative) != Truth::False)
    }
}
