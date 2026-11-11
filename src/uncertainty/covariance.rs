//! Covariance storage, validation, and the first-order contraction.

use alloc::{format, vec, vec::Vec};

use crate::{
    collections::FixedMap,
    core::{Context, Expr, Property, Symbol, Truth},
    errors::UncertaintyError,
};

/// Symmetric covariance expressions in an explicit coordinate order.
///
/// Entries represent finite real covariances; diagonal entries represent
/// nonnegative variances. Constructors validate shape and exact symmetry, and
/// reject disproved scalar conditions. Unknown conditions and positive
/// semidefiniteness remain the caller's mathematical responsibility.
/// No approximate tolerance or numeric-to-symbolic promotion is introduced.
///
/// [`CovarianceMatrix::new`] and [`CovarianceMatrix::diagonal`] contract the
/// expanded quadratic form $\sum_{ij} g_{i} g_{j} \Sigma_{ij}$, which is the
/// arrangement that cancels most for correlated inputs: for $f = x - y$ and
/// $\Sigma = [[1, 1-\varepsilon], [1-\varepsilon, 1]]$ the variance is
/// $1 - 2(1-\varepsilon) + 1$, so at $\varepsilon = 10^{-9}$ in `f64` the
/// evaluation loses about five digits. [`CovarianceMatrix::from_factor`] takes
/// a factor $L$ with $\Sigma = L L^{T}$ and contracts
/// $\sum_{j} (\sum_{i} L_{ij} g_{i})^{2}$ instead, which is a sum of squares
/// and cannot cancel.
#[derive(Clone, Debug)]
pub struct CovarianceMatrix {
    entries: Storage,
    zero: Expr,
}

#[derive(Clone, Debug)]
enum Storage {
    /// Row-major entries of a `dimension`-square matrix in one allocation.
    Dense {
        entries: Vec<Expr>,
        dimension: usize,
    },
    Diagonal(Vec<Expr>),
    /// Row-major $\Sigma = L L^{T}$ for [`CovarianceMatrix::get`] and
    /// column-major $L$ for sum-of-squares contraction.
    Factored {
        entries: Vec<Expr>,
        columns: Vec<Expr>,
        dimension: usize,
    },
}

/// One coordinate's partial derivative with the proofs the contraction reuses.
#[derive(Clone, Copy)]
struct Sensitivity<'expression> {
    expression: &'expression Expr,
    finite: bool,
    zero: bool,
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
    pub fn new(entries: Vec<Vec<Expr>>) -> Result<Self, UncertaintyError> {
        let dimension = entries.len();
        if let Some((row, actual)) = entries
            .iter()
            .map(Vec::len)
            .enumerate()
            .find(|(_, length)| *length != dimension)
        {
            return Err(UncertaintyError::NonSquare {
                row,
                expected: dimension,
                actual,
            });
        }
        for (row_index, row) in entries.iter().enumerate() {
            for (column, value) in row.iter().enumerate().skip(row_index) {
                let Some(opposite) = entries.get(column).and_then(|line| line.get(row_index))
                else {
                    continue;
                };
                if value != opposite
                    && !value
                        .as_number()
                        .zip(opposite.as_number())
                        .is_some_and(|(first, second)| first == second)
                {
                    return Err(UncertaintyError::Asymmetric {
                        row: row_index,
                        column,
                    });
                }
                if !Self::valid_entry(value, row_index == column) {
                    return Err(UncertaintyError::InvalidCovariance {
                        row: row_index,
                        column,
                    });
                }
            }
        }
        // Every row has `dimension` entries, so the flat length is their
        // product; an overflowing product leaves the vector to grow.
        let mut flat = Vec::with_capacity(dimension.checked_mul(dimension).unwrap_or(0));
        for row in entries {
            flat.extend(row);
        }
        Ok(Self {
            entries: Storage::Dense {
                entries: flat,
                dimension,
            },
            zero: Expr::number(0),
        })
    }

    /// Store independent variances in linear space. Inputs are variances,
    /// $u_{i}^2$, rather than standard deviations $u_{i}$.
    ///
    /// # Errors
    /// Returns [`UncertaintyError::InvalidCovariance`] for a disproved finite,
    /// real, or nonnegative variance. Unknown hypotheses belong to the caller.
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

    /// Construct an independent symbolic covariance matrix in `context`, with
    /// diagonal variances $\sigma_{x_{i}}^{2}$ for each coordinate $x_{i}$.
    ///
    /// Each standard-deviation symbol is looked up or created in `context`
    /// under `"sigma_{var}"` (preserving any existing symbol binding or
    /// properties in `context`).
    ///
    /// # Errors
    /// Returns [`UncertaintyError::DuplicateVariable`] when a coordinate
    /// identity appears more than once, [`UncertaintyError::InvalidCovariance`]
    /// when an existing symbol in `context` violates the finite real
    /// nonnegative variance contract, or
    /// [`UncertaintyError::SymbolIdExhausted`] when `context` cannot allocate
    /// a new symbol identity.
    ///
    /// ```
    /// use symb_anafis::{Context, CovarianceMatrix, Uncertainty};
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let y = context.symb("y")?;
    /// let covariance = CovarianceMatrix::symbolic_diagonal(&context, &[x, y])?;
    /// let result = Uncertainty::propagate(&(x + 2 * y), &[x, y], &covariance)?;
    /// let sigma_x = context.symb("sigma_x")?;
    /// let sigma_y = context.symb("sigma_y")?;
    /// assert_eq!(result.variance(), &(sigma_x.pow(2) + 4 * sigma_y.pow(2)));
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn symbolic_diagonal(
        context: &Context,
        variables: &[Symbol<'_>],
    ) -> Result<Self, UncertaintyError> {
        check_unique_variables(variables)?;
        let mut variances = Vec::with_capacity(variables.len());
        for variable in variables {
            let sigma = context
                .symb(&format!("sigma_{variable}"))
                .map_err(|_error| UncertaintyError::SymbolIdExhausted)?;
            variances.push(sigma.pow(2));
        }
        Self::diagonal(variances)
    }

    /// Construct a fully correlated symbolic covariance matrix in `context`,
    /// with diagonal variances $\sigma_{x_{i}}^{2}$ and off-diagonal
    /// covariances $\rho_{x_{i}, x_{j}} \sigma_{x_{i}} \sigma_{x_{j}}$ for
    /// $i < j$.
    ///
    /// Standard-deviation symbols are looked up or created in `context` under
    /// `"sigma_{var}"`, and correlation-coefficient symbols under
    /// `"rho_{first}_{second}"` in coordinate order (preserving any existing
    /// symbol bindings or properties in `context`).
    ///
    /// # Errors
    /// Returns [`UncertaintyError::DuplicateVariable`] when a coordinate
    /// identity appears more than once, [`UncertaintyError::InvalidCovariance`]
    /// when an existing symbol in `context` violates the finite real
    /// covariance contract, or [`UncertaintyError::SymbolIdExhausted`] when
    /// `context` cannot allocate a new symbol identity.
    ///
    /// ```
    /// use symb_anafis::{Context, CovarianceMatrix, Uncertainty};
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let y = context.symb("y")?;
    /// let covariance = CovarianceMatrix::symbolic_correlated(&context, &[x, y])?;
    /// let result = Uncertainty::propagate(&(x + y), &[x, y], &covariance)?;
    /// let sigma_x = context.symb("sigma_x")?;
    /// let sigma_y = context.symb("sigma_y")?;
    /// let rho_x_y = context.symb("rho_x_y")?;
    /// assert_eq!(
    ///     result.variance(),
    ///     &(sigma_x.pow(2) + 2 * rho_x_y * sigma_x * sigma_y + sigma_y.pow(2))
    /// );
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn symbolic_correlated(
        context: &Context,
        variables: &[Symbol<'_>],
    ) -> Result<Self, UncertaintyError> {
        check_unique_variables(variables)?;
        let dimension = variables.len();
        let mut sigmas = Vec::with_capacity(dimension);
        for variable in variables {
            let sigma = context
                .symb(&format!("sigma_{variable}"))
                .map_err(|_error| UncertaintyError::SymbolIdExhausted)?;
            sigmas.push(sigma.to_expr());
        }
        let mut entries: Vec<Expr> =
            Vec::with_capacity(dimension.checked_mul(dimension).unwrap_or(0));
        for (row, (first_var, first_sigma)) in variables.iter().zip(&sigmas).enumerate() {
            for (column, (second_var, second_sigma)) in variables.iter().zip(&sigmas).enumerate() {
                if column < row
                    && let Some(existing) = column
                        .checked_mul(dimension)
                        .and_then(|base| base.checked_add(row))
                        .and_then(|position| entries.get(position))
                {
                    entries.push(existing.clone());
                    continue;
                }
                let entry = if row == column {
                    first_sigma.pow(2)
                } else {
                    let rho = context
                        .symb(&format!("rho_{first_var}_{second_var}"))
                        .map_err(|_error| UncertaintyError::SymbolIdExhausted)?;
                    Expr::product(vec![
                        rho.to_expr(),
                        first_sigma.clone(),
                        second_sigma.clone(),
                    ])
                };
                if !Self::valid_entry(&entry, row == column) {
                    return Err(UncertaintyError::InvalidCovariance { row, column });
                }
                entries.push(entry);
            }
        }
        Ok(Self {
            entries: Storage::Dense { entries, dimension },
            zero: Expr::number(0),
        })
    }

    /// Store a factor $L$ with $\Sigma = L L^{T}$, so the contraction is the
    /// sum of squares $\sum_{j} (\sum_{i} L_{ij} g_{i})^{2}$.
    ///
    /// A Cholesky factor of a covariance matrix is the usual source. Any square
    /// $L$ is accepted: $\Sigma = L L^{T}$ is symmetric and positive
    /// semidefinite by construction, so no symmetry or definiteness check is
    /// needed, and the entries of $L$ need not be triangular. The expanded
    /// $\Sigma$ is materialized for [`CovarianceMatrix::get`]; the contraction
    /// uses $L$ and never forms it.
    ///
    /// The stability is the factor's, not the contraction's: a factor computed
    /// by subtracting nearly equal numbers reintroduces the cancellation the
    /// sum of squares avoids. For $\Sigma = [[1, 1-\varepsilon], [1-\varepsilon,
    /// 1]]$ at $\varepsilon = 10^{-9}$, forming $L_{22}$ as
    /// $\sqrt{1 - (1-\varepsilon)^{2}}$ in `f64` keeps a relative error of
    /// $2.8 \times 10^{-8}$, while forming it as $\sqrt{2\varepsilon -
    /// \varepsilon^{2}}$ is exact.
    ///
    /// # Errors
    /// Returns [`UncertaintyError::NonSquare`] when a row length differs from
    /// the dimension, or [`UncertaintyError::InvalidCovariance`] for a disproved
    /// finite real factor entry.
    pub fn from_factor(factor: Vec<Vec<Expr>>) -> Result<Self, UncertaintyError> {
        let dimension = factor.len();
        if let Some((row, actual)) = factor
            .iter()
            .map(Vec::len)
            .enumerate()
            .find(|(_, length)| *length != dimension)
        {
            return Err(UncertaintyError::NonSquare {
                row,
                expected: dimension,
                actual,
            });
        }
        for (row, entries) in factor.iter().enumerate() {
            for (column, value) in entries.iter().enumerate() {
                if !Self::valid_entry(value, false) {
                    return Err(UncertaintyError::InvalidCovariance { row, column });
                }
            }
        }
        let zero = Expr::number(0);
        let flat_len = dimension.checked_mul(dimension).unwrap_or(0);
        // $\Sigma_{ij} = \sum_{k} L_{ik} L_{jk}$; the full matrix is stored so
        // `get` needs no arithmetic. Symmetry lets lower-triangle entries
        // reuse the normalized upper-triangle product.
        let mut entries: Vec<Expr> = Vec::with_capacity(flat_len);
        for (row_index, left) in factor.iter().enumerate() {
            for (column_index, right) in factor.iter().enumerate() {
                if column_index < row_index
                    && let Some(existing) = column_index
                        .checked_mul(dimension)
                        .and_then(|base| base.checked_add(row_index))
                        .and_then(|position| entries.get(position))
                {
                    entries.push(existing.clone());
                    continue;
                }
                let mut products = Vec::with_capacity(dimension);
                for (first, second) in left.iter().zip(right) {
                    if first == &zero && second == &zero {
                        continue;
                    }
                    let second_finite = second.as_number().is_some()
                        || second.properties().get(Property::Finite) == Truth::True;
                    products.push(Self::linear_term(
                        first,
                        Sensitivity {
                            expression: second,
                            finite: second_finite,
                            zero: second.is_zero_num(),
                        },
                    ));
                }
                entries.push(Expr::sum(products));
            }
        }
        // Store $L$ flat in column-major order, moving entries out of `factor`
        // so contraction scans each column contiguously without cloning.
        let mut columns = Vec::with_capacity(flat_len);
        let mut rows: Vec<_> = factor.into_iter().map(Vec::into_iter).collect();
        for _ in 0..dimension {
            for row in &mut rows {
                if let Some(entry) = row.next() {
                    columns.push(entry);
                }
            }
        }
        Ok(Self {
            entries: Storage::Factored {
                entries,
                columns,
                dimension,
            },
            zero,
        })
    }

    /// Read the number of coordinates, including zero-dimensional matrices.
    #[must_use]
    pub const fn dimension(&self) -> usize {
        match &self.entries {
            Storage::Dense { dimension, .. } | Storage::Factored { dimension, .. } => *dimension,
            Storage::Diagonal(variances) => variances.len(),
        }
    }

    /// Borrow an entry; diagonal storage supplies its owned exact off-diagonal zero.
    /// Out-of-range indices return `None`.
    #[must_use]
    pub fn get(&self, row: usize, column: usize) -> Option<&Expr> {
        match &self.entries {
            Storage::Dense { entries, dimension }
            | Storage::Factored {
                entries, dimension, ..
            } => {
                if row >= *dimension || column >= *dimension {
                    return None;
                }
                row.checked_mul(*dimension)
                    .and_then(|base| base.checked_add(column))
                    .and_then(|position| entries.get(position))
            }
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

    /// Contract the sensitivities into $u_{f}^2$, validating dimension and
    /// domain conditions first.
    ///
    /// # Errors
    /// Returns [`UncertaintyError::DimensionMismatch`] when `sensitivities.len()`
    /// differs from [`CovarianceMatrix::dimension`],
    /// [`UncertaintyError::InvalidSensitivity`] for a disproved finite real
    /// partial derivative, or [`UncertaintyError::InvalidVariance`] when the
    /// result is disproved finite, real, or nonnegative.
    pub fn contract(&self, sensitivities: &[Expr]) -> Result<Expr, UncertaintyError> {
        let dimension = self.dimension();
        if sensitivities.len() != dimension {
            return Err(UncertaintyError::DimensionMismatch {
                variables: sensitivities.len(),
                covariance: dimension,
            });
        }
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
        let variance = self.contract_checked(&checked);
        if !Self::valid_entry(&variance, true) {
            return Err(UncertaintyError::InvalidVariance);
        }
        Ok(variance)
    }

    /// Shape, identity order, and equal sensitivity count have been validated.
    /// Each diagonal is retained, so a zero covariance cannot hide a sensitivity
    /// pole. Diagonal storage needs only O(n) terms; dense storage visits the
    /// upper triangle and doubles a pair only when both of its entries are the
    /// same canonical node. A pair that is merely numerically equal contributes
    /// both entries, so neither representation's tier is dropped. Cached
    /// sensitivity proofs avoid repeated DAG analyses.
    fn contract_checked(&self, sensitivities: &[Sensitivity<'_>]) -> Expr {
        if sensitivities.is_empty() {
            return self.zero.clone();
        }
        match &self.entries {
            Storage::Factored {
                columns, dimension, ..
            } => Self::contract_factored(columns, *dimension, sensitivities),
            Storage::Diagonal(variances) => {
                let mut terms = Vec::with_capacity(sensitivities.len());
                for (partial, variance) in sensitivities.iter().zip(variances) {
                    terms.push(Self::term(*partial, *partial, variance, None));
                }
                Expr::sum(terms)
            }
            Storage::Dense { entries, dimension } => {
                let count = sensitivities.len();
                // A dense matrix contributes at most one term per upper-triangle
                // pair; a mixed-representation pair contributes two, and the
                // vector grows for those.
                let mut terms = Vec::with_capacity(
                    count
                        .checked_mul(count.saturating_add(1))
                        .map_or(count, |pairs| pairs >> 1),
                );
                let two = Expr::number(2);
                for (index, (first, row)) in sensitivities
                    .iter()
                    .zip(entries.chunks_exact(*dimension))
                    .enumerate()
                {
                    // The upper triangle of this row, the sensitivities it
                    // multiplies, and the transposed entries of the rows below,
                    // all split at the diagonal in one step.
                    let (Some(rest), Some(tail), Some(column)) = (
                        sensitivities.get(index..),
                        row.get(index..),
                        entries.get(index..),
                    ) else {
                        continue;
                    };
                    for (offset, ((second, covariance), mirror)) in rest
                        .iter()
                        .zip(tail)
                        .zip(column.iter().step_by(*dimension).skip(index))
                        .enumerate()
                    {
                        // Every diagonal retains its sensitivity's exclusions.
                        if offset == 0 {
                            terms.push(Self::term(*first, *second, covariance, None));
                            continue;
                        }
                        // A pair whose entries are the same canonical node is
                        // one term carrying the symmetric factor of two. A pair
                        // that is only numerically equal keeps both entries, so
                        // neither representation's tier is dropped. An exact
                        // off-diagonal zero adds no further domain; an
                        // approximate zero remains to retain its numeric tier.
                        let doubled = mirror == covariance;
                        if covariance != &self.zero {
                            terms.push(Self::term(
                                *first,
                                *second,
                                covariance,
                                doubled.then_some(&two),
                            ));
                        }
                        if !doubled && mirror != &self.zero {
                            terms.push(Self::term(*second, *first, mirror, None));
                        }
                    }
                }
                Expr::sum(terms)
            }
        }
    }

    /// $\sum_{j} (\sum_{i} L_{ij} g_{i})^{2}$: a sum of squares, so no term can
    /// cancel another and the result is nonnegative by construction. Column $j$
    /// of $L$ dotted with $g$ is component $j$ of $L^{T} g$, and
    /// $|L^{T} g|^{2} = g^{T} L L^{T} g$.
    fn contract_factored(
        columns: &[Expr],
        dimension: usize,
        sensitivities: &[Sensitivity<'_>],
    ) -> Expr {
        let mut squares = Vec::with_capacity(dimension);
        for column in columns.chunks_exact(dimension) {
            let products = column
                .iter()
                .zip(sensitivities)
                .map(|(entry, partial)| Self::linear_term(entry, *partial))
                .collect();
            squares.push(Expr::sum(products).pow(2));
        }
        Expr::sum(squares)
    }

    fn linear_term(entry: &Expr, partial: Sensitivity<'_>) -> Expr {
        let entry_zero = partial.finite && entry.is_zero_num();
        let annihilates = entry_zero
            || (partial.zero
                && (entry.as_number().is_some()
                    || entry.properties().get(Property::Finite) == Truth::True));
        if annihilates {
            match (
                entry.as_number().is_some(),
                partial.expression.as_number().is_some(),
            ) {
                (true, false) => return entry.clone(),
                (false, true) => return partial.expression.clone(),
                _ => {}
            }
        }
        if annihilates && partial.zero {
            Expr::product(vec![partial.expression.clone(), entry.clone()])
        } else {
            Expr::product(vec![entry.clone(), partial.expression.clone()])
        }
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
        let [lead, other, third] = if covariance_zero {
            [covariance, first.expression, second.expression]
        } else if annihilates && second.zero {
            [second.expression, first.expression, covariance]
        } else {
            [first.expression, second.expression, covariance]
        };
        // Multiplying a single surviving zero (`0`, `0.0`, or `-0.0`) by an
        // exact positive multiplier leaves it unchanged without allocating.
        if annihilates && other.as_number().is_none() && third.as_number().is_none() {
            return lead.clone();
        }
        // At most three original factors and one cross-term multiplier: no
        // reallocation is needed when adding the symmetric factor of two.
        let mut factors = Vec::with_capacity(4);
        for expression in [lead, other, third] {
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

/// Reject repeated coordinate identities in linear or hashed time.
pub fn check_unique_variables(variables: &[Symbol<'_>]) -> Result<(), UncertaintyError> {
    if variables.len() <= 8 {
        for (index, variable) in variables.iter().enumerate() {
            if let Some(first) = variables
                .get(..index)
                .and_then(|prefix| prefix.iter().position(|seen| seen == variable))
            {
                return Err(UncertaintyError::DuplicateVariable {
                    first,
                    duplicate: index,
                });
            }
        }
    } else {
        let mut identities = FixedMap::default();
        identities.reserve(variables.len());
        for (index, variable) in variables.iter().enumerate() {
            if let Some(first) = identities.insert(*variable, index) {
                return Err(UncertaintyError::DuplicateVariable {
                    first,
                    duplicate: index,
                });
            }
        }
    }
    Ok(())
}
