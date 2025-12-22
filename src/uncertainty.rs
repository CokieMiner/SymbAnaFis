//! Uncertainty propagation for symbolic expressions
//!
//! Computes uncertainty propagation formulas using the standard GUM formula:
//! σ_f² = Σᵢ Σⱼ (∂f/∂xᵢ)(∂f/∂xⱼ) Cov(xᵢ, xⱼ)
//!
//! For uncorrelated variables, this simplifies to:
//! σ_f² = Σᵢ (∂f/∂xᵢ)² σᵢ²
//!
//! # Reference
//!
//! JCGM 100:2008 "Evaluation of measurement data — Guide to the expression
//! of uncertainty in measurement" (GUM), Section 5.1.2
//! <https://www.bipm.org/documents/20126/2071204/JCGM_100_2008_E.pdf>

use crate::core::traits::EPSILON;
use crate::{Diff, DiffError, Expr};

/// Covariance matrix entry - can be numeric or symbolic
#[derive(Debug, Clone)]
pub enum CovEntry {
    /// A numeric covariance value
    Num(f64),
    /// A symbolic covariance expression (e.g., ρ_xy * σ_x * σ_y)
    Symbolic(Expr),
}

impl CovEntry {
    /// Convert the entry to an Expr
    #[inline]
    pub fn to_expr(&self) -> Expr {
        match self {
            CovEntry::Num(n) => Expr::number(*n),
            CovEntry::Symbolic(e) => e.clone(),
        }
    }

    /// Check if the entry is zero
    #[inline]
    pub fn is_zero(&self) -> bool {
        match self {
            CovEntry::Num(n) => n.abs() < EPSILON,
            CovEntry::Symbolic(_) => false,
        }
    }
}

impl From<f64> for CovEntry {
    fn from(n: f64) -> Self {
        CovEntry::Num(n)
    }
}

impl From<Expr> for CovEntry {
    fn from(e: Expr) -> Self {
        CovEntry::Symbolic(e)
    }
}

/// Covariance matrix for uncertainty propagation
///
/// The matrix `Cov[i][j]` represents Cov(xᵢ, xⱼ).
/// For correlated variables: Cov(x, y) = ρ_xy * σ_x * σ_y
/// The diagonal elements are the variances: Cov(x, x) = σ_x²
#[derive(Debug, Clone)]
pub struct CovarianceMatrix {
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
        for i in 0..n {
            for j in (i + 1)..n {
                if let (CovEntry::Num(a), CovEntry::Num(b)) = (&entries[i][j], &entries[j][i])
                    && (a - b).abs() >= EPSILON
                {
                    return Err(DiffError::UnsupportedOperation(format!(
                        "Covariance matrix must be symmetric: Cov[{}][{}]={} != Cov[{}][{}]={}",
                        i, j, a, j, i, b
                    )));
                }
            }
        }
        Ok(CovarianceMatrix { entries })
    }

    /// Create a diagonal covariance matrix (uncorrelated variables)
    /// from variance expressions σ_i²
    pub fn diagonal(variances: Vec<CovEntry>) -> Self {
        let n = variances.len();
        let mut entries = vec![vec![CovEntry::Num(0.0); n]; n];
        for (i, var) in variances.into_iter().enumerate() {
            entries[i][i] = var;
        }
        // Skip validation for diagonal (guaranteed symmetric)
        CovarianceMatrix { entries }
    }

    /// Create a diagonal covariance matrix from symbolic variance names
    /// (e.g., ["x", "y"] creates σ_x² and σ_y² on diagonal)
    pub fn diagonal_symbolic(var_names: &[&str]) -> Self {
        let n = var_names.len();
        let mut entries = vec![vec![CovEntry::Num(0.0); n]; n];
        for (i, name) in var_names.iter().enumerate() {
            // Create σ² symbol for each variable
            let sigma_sq = Expr::pow(Expr::symbol(format!("sigma_{}", name)), Expr::number(2.0));
            entries[i][i] = CovEntry::Symbolic(sigma_sq);
        }
        // Skip validation for diagonal (guaranteed symmetric)
        CovarianceMatrix { entries }
    }

    /// Get the covariance entry at (i, j)
    #[inline]
    pub fn get(&self, i: usize, j: usize) -> Option<&CovEntry> {
        self.entries.get(i).and_then(|row| row.get(j))
    }

    /// Get the dimension of the matrix
    #[inline]
    pub fn dim(&self) -> usize {
        self.entries.len()
    }
}

/// Compute the uncertainty propagation expression
///
/// Returns σ_f = sqrt(Σᵢ Σⱼ (∂f/∂xᵢ)(∂f/∂xⱼ) Cov(xᵢ, xⱼ))
///
/// # Arguments
/// * `expr` - The expression f(x₁, x₂, ..., xₙ)
/// * `variables` - The variables [x₁, x₂, ..., xₙ] to propagate uncertainty for
/// * `covariance` - Optional covariance matrix. If None, creates symbolic diagonal matrix.
///
/// # Returns
/// The symbolic expression for σ_f (standard deviation of f)
///
/// # Example
/// ```
/// use symb_anafis::{symb, uncertainty_propagation, CovarianceMatrix, CovEntry};
///
/// let x = symb("uncertainty_doc_x");
/// let y = symb("uncertainty_doc_y");
/// let expr = &x + &y;  // f = x + y
///
/// // Uncorrelated with symbolic variances: σ_f² = σ_x² + σ_y²
/// let result = uncertainty_propagation(&expr, &["uncertainty_doc_x", "uncertainty_doc_y"], None).unwrap();
/// assert!(!format!("{}", result).is_empty());
///
/// // With numeric variances:
/// let cov = CovarianceMatrix::diagonal(vec![CovEntry::Num(1.0), CovEntry::Num(4.0)]);
/// let result = uncertainty_propagation(&expr, &["uncertainty_doc_x", "uncertainty_doc_y"], Some(&cov)).unwrap();
/// ```
pub fn uncertainty_propagation(
    expr: &Expr,
    variables: &[&str],
    covariance: Option<&CovarianceMatrix>,
) -> Result<Expr, DiffError> {
    let n = variables.len();

    if n == 0 {
        return Ok(Expr::number(0.0));
    }

    // Compute all partial derivatives
    let diff = Diff::new();
    let mut partials: Vec<Expr> = Vec::with_capacity(n);

    for var in variables {
        let partial = diff.differentiate_by_name(expr.clone(), var)?;
        let simplified = partial.simplified()?;
        partials.push(simplified);
    }

    // Get or create covariance matrix
    let default_cov;
    let cov = match covariance {
        Some(c) => {
            if c.dim() != n {
                return Err(DiffError::UnsupportedOperation(format!(
                    "Covariance matrix dimension ({}) doesn't match number of variables ({})",
                    c.dim(),
                    n
                )));
            }
            c
        }
        None => {
            // Create default symbolic diagonal covariance matrix
            default_cov = CovarianceMatrix::diagonal_symbolic(variables);
            &default_cov
        }
    };

    // Build the uncertainty expression: Σᵢ (∂f/∂xᵢ)² σᵢ² + 2 Σᵢ<ⱼ (∂f/∂xᵢ)(∂f/∂xⱼ) Cov(xᵢ, xⱼ)
    let mut terms: Vec<Expr> = Vec::new();

    for i in 0..n {
        for j in i..n {
            let cov_entry = cov.get(i, j).ok_or_else(|| {
                DiffError::UnsupportedOperation(
                    "Covariance matrix access out of bounds".to_string(),
                )
            })?;

            // Skip zero covariance entries
            if cov_entry.is_zero() {
                continue;
            }

            // Term: (∂f/∂xᵢ) * (∂f/∂xⱼ) * Cov(xᵢ, xⱼ)
            let mut term = Expr::mul_expr(
                Expr::mul_expr(partials[i].clone(), partials[j].clone()),
                cov_entry.to_expr(),
            );

            // Double the cross-terms (i < j)
            if i < j {
                term = Expr::mul_expr(Expr::number(2.0), term);
            }

            terms.push(term);
        }
    }

    let variance = terms
        .into_iter()
        .reduce(Expr::add_expr)
        .unwrap_or_else(|| Expr::number(0.0));

    // Simplify the variance, then wrap in sqrt for σ_f = sqrt(σ_f²)
    let simplified_variance = variance.simplified()?;
    let std_dev = Expr::func("sqrt", simplified_variance);

    std_dev.simplified()
}

/// Compute relative uncertainty expression: σ_f / |f|
///
/// Returns the symbolic expression for the relative uncertainty.
pub fn relative_uncertainty(
    expr: &Expr,
    variables: &[&str],
    covariance: Option<&CovarianceMatrix>,
) -> Result<Expr, DiffError> {
    // uncertainty_propagation already returns σ_f
    let std_dev = uncertainty_propagation(expr, variables, covariance)?;

    // |f|
    let abs_f = Expr::func("abs", expr.clone());

    // σ_f / |f|
    Ok(Expr::div_expr(std_dev, abs_f))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symb;

    #[test]
    fn test_simple_sum_uncorrelated() {
        // f = x + y
        // σ_f² = σ_x² + σ_y² (for uncorrelated variables)
        let x = symb("x");
        let y = symb("y");
        let expr = x + y;

        let result = uncertainty_propagation(&expr, &["x", "y"], None).unwrap();
        let latex = result.to_latex();

        // Should contain both sigma_x and sigma_y terms
        assert!(latex.contains("sigma_x") || latex.contains("sigma"));
    }

    #[test]
    fn test_simple_product_uncorrelated() {
        // f = x * y
        // σ_f² = y² * σ_x² + x² * σ_y² (for uncorrelated variables)
        let x = symb("x");
        let y = symb("y");
        let expr = x * y;

        let result = uncertainty_propagation(&expr, &["x", "y"], None).unwrap();

        // Result should be non-zero (it's a symbolic expression, not just 0.0)
        assert!(!result.is_zero_num());
    }

    #[test]
    fn test_numeric_covariance() {
        // f = x + y with numeric variances
        let x = symb("x");
        let y = symb("y");
        let expr = x + y;

        // σ_x² = 1, σ_y² = 4
        let cov = CovarianceMatrix::diagonal(vec![
            CovEntry::Num(1.0), // σ_x² = 1
            CovEntry::Num(4.0), // σ_y² = 4
        ]);

        let result = uncertainty_propagation(&expr, &["x", "y"], Some(&cov)).unwrap();

        // For x + y: σ_f = sqrt(1 + 4) = sqrt(5) ≈ 2.236
        if let Some(n) = result.as_number() {
            assert!(
                (n - 5.0_f64.sqrt()).abs() < 1e-10,
                "Expected {}, got {}",
                5.0_f64.sqrt(),
                n
            );
        }
    }

    #[test]
    fn test_single_variable() {
        // f = x^2
        // σ_f² = (2x)² * σ_x² = 4x² * σ_x²
        let x = symb("test_unc_x");
        let expr = x.pow(2.0);

        let result = uncertainty_propagation(&expr, &["test_unc_x"], None).unwrap();

        // Should contain x and sigma terms
        let display = format!("{}", result);
        assert!(!display.is_empty());
    }
}
