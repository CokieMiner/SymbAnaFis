use super::logic::compute_uncertainty_terms;
use crate::core::known_symbols as ks;
use crate::{Diff, DiffError, Expr};

#[cfg(feature = "parallel")]
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

/// Standard deviation calculation builder.
///
/// Follows the GUM formula for uncertainty propagation.
#[derive(Debug, Clone, Default)]
pub struct Uncertainty<'ctx> {
    context: Option<&'ctx crate::core::context::Context>,
    covariance: Option<&'ctx CovarianceMatrix>,
}

impl<'ctx> Uncertainty<'ctx> {
    /// Create a new uncertainty propagation builder
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the context for custom functions and symbol resolution
    #[inline]
    #[must_use]
    pub const fn context(mut self, context: &'ctx crate::core::context::Context) -> Self {
        self.context = Some(context);
        self
    }

    /// Set the covariance matrix
    #[inline]
    #[must_use]
    pub const fn covariance(mut self, covariance: &'ctx CovarianceMatrix) -> Self {
        self.covariance = Some(covariance);
        self
    }

    /// Propagate uncertainties through the expression
    ///
    /// # Errors
    /// Returns `DiffError` if differentiation fails or matrix dimensions mismatch.
    pub fn propagate(&self, expr: &Expr, variables: &[&str]) -> Result<Expr, DiffError> {
        let n = variables.len();
        if n == 0 {
            return Ok(Expr::number(0.0));
        }

        // Compute all partial derivatives using the provided context
        let mut diff = Diff::new();
        if let Some(ctx) = self.context {
            diff = diff.context(ctx);
        }

        #[cfg(not(feature = "parallel"))]
        let partials: Result<Vec<Expr>, DiffError> = variables
            .iter()
            .map(|&var| {
                let partial = diff.differentiate_by_name(expr, var)?;
                partial.simplified()
            })
            .collect();

        #[cfg(feature = "parallel")]
        let partials: Result<Vec<Expr>, DiffError> = variables
            .par_iter()
            .map(|&var| {
                let partial = diff.differentiate_by_name(expr, var)?;
                partial.simplified()
            })
            .collect();

        let partials = partials?;

        // Get or create covariance matrix
        let default_cov;
        let cov = if let Some(c) = self.covariance {
            if c.dim() != n {
                return Err(DiffError::UnsupportedOperation(format!(
                    "Covariance matrix dimension ({}) doesn't match number of variables ({})",
                    c.dim(),
                    n
                )));
            }
            c
        } else {
            default_cov = CovarianceMatrix::diagonal_symbolic(variables);
            &default_cov
        };

        let terms = compute_uncertainty_terms(&partials, cov, n)?;

        let variance = Expr::sum(terms);
        let simplified_variance = variance.simplified()?;
        let std_dev = Expr::func_symbol(ks::get_symbol(ks::KS.sqrt), simplified_variance);

        std_dev.simplified()
    }
}

/// Compute the uncertainty propagation expression
///
/// Returns `σ_f` = sqrt(Σᵢ Σⱼ (∂f/∂xᵢ)(∂f/∂xⱼ) Cov(xᵢ, xⱼ))
///
/// # Arguments
/// * `expr` - The expression f(x₁, x₂, ..., xₙ)
/// * `variables` - The variables [x₁, x₂, ..., xₙ] to propagate uncertainty for
/// * `covariance` - Optional covariance matrix. If None, creates symbolic diagonal matrix.
///
/// # Returns
/// The symbolic expression for `σ_f` (standard deviation of f)
///
/// # Errors
/// Returns `DiffError` if differentiation fails or matrix dimensions mismatch.
pub fn uncertainty_propagation(
    expr: &Expr,
    variables: &[&str],
    covariance: Option<&CovarianceMatrix>,
) -> Result<Expr, DiffError> {
    let mut builder = Uncertainty::new();
    if let Some(cov) = covariance {
        builder = builder.covariance(cov);
    }
    builder.propagate(expr, variables)
}

/// Compute relative uncertainty expression: `σ_f` / |f|
///
/// Returns the symbolic expression for the relative uncertainty.
///
/// # Errors
/// Returns `DiffError` if uncertainty propagation fails.
pub fn relative_uncertainty(
    expr: &Expr,
    variables: &[&str],
    covariance: Option<&CovarianceMatrix>,
) -> Result<Expr, DiffError> {
    let std_dev = uncertainty_propagation(expr, variables, covariance)?;
    let abs_f = Expr::func_symbol(ks::get_symbol(ks::KS.abs), expr.clone());
    Ok(Expr::div_expr(std_dev, abs_f))
}

// ============================================================================
// Covariance Matrix Data Structures
// ============================================================================

use crate::core::traits::EPSILON;

/// Covariance matrix entry - can be numeric or symbolic
#[derive(Debug, Clone)]
pub enum CovEntry {
    /// A numeric covariance value
    Num(f64),
    /// A symbolic covariance expression (e.g., `ρ_xy` * `σ_x` * `σ_y`)
    Symbolic(Expr),
}

impl CovEntry {
    /// Convert the entry to an Expr
    #[inline]
    #[must_use]
    pub fn to_expr(&self) -> Expr {
        match self {
            Self::Num(n) => Expr::number(*n),
            Self::Symbolic(e) => e.clone(),
        }
    }

    /// Check if the entry is zero
    #[inline]
    #[must_use]
    pub fn is_zero(&self) -> bool {
        match self {
            Self::Num(n) => n.abs() < EPSILON,
            Self::Symbolic(e) => e.is_zero_num(),
        }
    }
}

impl From<f64> for CovEntry {
    fn from(n: f64) -> Self {
        Self::Num(n)
    }
}

impl From<Expr> for CovEntry {
    fn from(e: Expr) -> Self {
        Self::Symbolic(e)
    }
}

/// Covariance matrix for uncertainty propagation
///
/// The matrix `Cov[i][j]` represents Cov(xᵢ, xⱼ).
/// For correlated variables: Cov(x, y) = `ρ_xy` * `σ_x` * `σ_y`
/// The diagonal elements are the variances: Cov(x, x) = `σ_x²`
#[derive(Debug, Clone, Default)]
pub struct CovarianceMatrix {
    /// The covariance matrix entries
    entries: Vec<Vec<CovEntry>>,
}

impl CovarianceMatrix {
    /// Create a new covariance matrix from a 2D vector of entries
    ///
    /// # Errors
    /// Returns an error if the matrix is not square or if it's not symmetric for numeric entries.
    pub fn new(entries: Vec<Vec<CovEntry>>) -> Result<Self, DiffError> {
        let n = entries.len();
        // Validate square matrix
        for (i, row) in entries.iter().enumerate() {
            if row.len() != n {
                return Err(DiffError::UnsupportedOperation(format!(
                    "Covariance matrix must be square: row {} has {} entries, expected {}",
                    i,
                    row.len(),
                    n
                )));
            }
        }
        // Validate symmetry for numeric entries
        for (i, row_i) in entries.iter().enumerate() {
            for (j, entry_ij) in row_i.iter().enumerate().skip(i.saturating_add(1)) {
                if let (CovEntry::Num(a), CovEntry::Num(b)) = (entry_ij, &entries[j][i])
                    && (a - b).abs() >= EPSILON
                {
                    return Err(DiffError::UnsupportedOperation(format!(
                        "Covariance matrix must be symmetric: Cov[{i}][{j}]={a} != Cov[{j}][{i}]={b}"
                    )));
                }
            }
        }
        Ok(Self { entries })
    }

    /// Create a diagonal covariance matrix (uncorrelated variables)
    /// from variance expressions `σ_i²`
    #[must_use]
    pub fn diagonal(variances: Vec<CovEntry>) -> Self {
        let n = variances.len();
        let mut entries = vec![vec![CovEntry::Num(0.0); n]; n];
        for (i, var) in variances.into_iter().enumerate() {
            entries[i][i] = var;
        }
        Self { entries }
    }

    /// Create a diagonal covariance matrix from symbolic variance names
    /// (e.g., `["x", "y"]` creates `σ_x²` and `σ_y²` on diagonal)
    #[must_use]
    pub fn diagonal_symbolic(var_names: &[&str]) -> Self {
        let n = var_names.len();
        let mut entries = vec![vec![CovEntry::Num(0.0); n]; n];
        for (i, name) in var_names.iter().enumerate() {
            let sigma_sq =
                Expr::pow_static(Expr::symbol(format!("sigma_{name}")), Expr::number(2.0));
            entries[i][i] = CovEntry::Symbolic(sigma_sq);
        }
        Self { entries }
    }

    /// Get the covariance entry at (i, j)
    #[inline]
    #[must_use]
    pub fn get(&self, i: usize, j: usize) -> Option<&CovEntry> {
        self.entries.get(i).and_then(|row| row.get(j))
    }

    /// Get the dimension of the matrix
    #[inline]
    #[must_use]
    pub const fn dim(&self) -> usize {
        self.entries.len()
    }
}
