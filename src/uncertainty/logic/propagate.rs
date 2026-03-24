use super::super::api::CovarianceMatrix;
use crate::{DiffError, Expr};

/// Compute the sum of uncertainty terms using the GUM formula
/// `σ_f²` = Σᵢ Σⱼ (∂f/∂xᵢ)(∂f/∂xⱼ) Cov(xᵢ, xⱼ)
///
/// # Arguments
/// * `partials` - The precomputed, simplified partial derivatives [∂f/∂x₁, ∂f/∂x₂, ...]
/// * `cov` - The covariance matrix Cov(xᵢ, xⱼ)
/// * `n` - Number of variables
///
/// # Returns
/// A list of terms to be summed up for variance
pub fn compute_uncertainty_terms(
    partials: &[Expr],
    cov: &CovarianceMatrix,
    n: usize,
) -> Result<Vec<Expr>, DiffError> {
    let mut terms: Vec<Expr> = Vec::new();
    for i in 0..n {
        for j in i..n {
            if partials[i].is_zero_num() || partials[j].is_zero_num() {
                continue;
            }

            let cov_entry = cov.get(i, j).ok_or_else(|| {
                DiffError::UnsupportedOperation("Covariance matrix access out of bounds".to_owned())
            })?;

            if cov_entry.is_zero() {
                continue;
            }

            let mut term = Expr::mul_expr(
                Expr::mul_expr(partials[i].clone(), partials[j].clone()),
                cov_entry.to_expr(),
            );

            if i < j {
                term = Expr::mul_expr(Expr::number(2.0), term);
            }
            terms.push(term);
        }
    }
    Ok(terms)
}
