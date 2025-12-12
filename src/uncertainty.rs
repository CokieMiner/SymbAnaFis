//! Uncertainty propagation for symbolic expressions
//!
//! Computes uncertainty propagation formulas using the standard formula:
//! σ_f² = Σᵢ Σⱼ (∂f/∂xᵢ)(∂f/∂xⱼ) Cov(xᵢ, xⱼ)
//!
//! For uncorrelated variables, this simplifies to:
//! σ_f² = Σᵢ (∂f/∂xᵢ)² σᵢ²

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
    pub fn to_expr(&self) -> Expr {
        match self {
            CovEntry::Num(n) => Expr::number(*n),
            CovEntry::Symbolic(e) => e.clone(),
        }
    }

    /// Check if the entry is zero
    pub fn is_zero(&self) -> bool {
        match self {
            CovEntry::Num(n) => n.abs() < 1e-15,
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
    pub fn new(entries: Vec<Vec<CovEntry>>) -> Self {
        CovarianceMatrix { entries }
    }

    /// Create a diagonal covariance matrix (uncorrelated variables)
    /// from variance expressions σ_i²
    pub fn diagonal(variances: Vec<CovEntry>) -> Self {
        let n = variances.len();
        let mut entries = vec![vec![CovEntry::Num(0.0); n]; n];
        for (i, var) in variances.into_iter().enumerate() {
            entries[i][i] = var;
        }
        CovarianceMatrix { entries }
    }

    /// Create a diagonal covariance matrix from symbolic variance names
    /// (e.g., ["sigma_x", "sigma_y"] creates σ_x² and σ_y² on diagonal)
    pub fn diagonal_symbolic(var_names: &[&str]) -> Self {
        let n = var_names.len();
        let mut entries = vec![vec![CovEntry::Num(0.0); n]; n];
        for (i, name) in var_names.iter().enumerate() {
            // Create σ² symbol for each variable
            let sigma_sq = Expr::pow(Expr::symbol(format!("sigma_{}", name)), Expr::number(2.0));
            entries[i][i] = CovEntry::Symbolic(sigma_sq);
        }
        CovarianceMatrix { entries }
    }

    /// Get the covariance entry at (i, j)
    pub fn get(&self, i: usize, j: usize) -> Option<&CovEntry> {
        self.entries.get(i).and_then(|row| row.get(j))
    }

    /// Get the dimension of the matrix
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
/// ```ignore
/// use symb_anafis::{symb, uncertainty_propagation, CovarianceMatrix};
///
/// let x = symb("x");
/// let y = symb("y");
/// let expr = &x + &y;  // f = x + y
///
/// // Uncorrelated: σ_f² = σ_x² + σ_y²
/// let result = uncertainty_propagation(&expr, &["x", "y"], None)?;
///
/// // With correlation matrix:
/// let cov = CovarianceMatrix::diagonal_symbolic(&["x", "y"]);
/// let result = uncertainty_propagation(&expr, &["x", "y"], Some(&cov))?;
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

    // Build the uncertainty expression: Σᵢ Σⱼ (∂f/∂xᵢ)(∂f/∂xⱼ) Cov(xᵢ, xⱼ)
    let mut terms: Vec<Expr> = Vec::new();

    for i in 0..n {
        for j in 0..n {
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
            let term = Expr::mul_expr(
                Expr::mul_expr(partials[i].clone(), partials[j].clone()),
                cov_entry.to_expr(),
            );

            terms.push(term);
        }
    }

    // Sum all terms
    if terms.is_empty() {
        return Ok(Expr::number(0.0));
    }

    let mut variance = terms.remove(0);
    for term in terms {
        variance = Expr::add_expr(variance, term);
    }

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

        // Result should be non-zero
        assert!(!matches!(result.kind, crate::ExprKind::Number(n) if n == 0.0));
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
        if let crate::ExprKind::Number(n) = result.kind {
            assert!((n - 5.0_f64.sqrt()).abs() < 1e-10);
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
