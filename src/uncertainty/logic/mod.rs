pub(super) mod propagate;

pub(super) use super::CovarianceMatrix;
pub(super) use propagate::compute_uncertainty_terms;

#[cfg(test)]
mod tests;
