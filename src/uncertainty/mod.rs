//! First-order symbolic uncertainty propagation with explicit covariances.

mod covariance;
mod propagation;

pub use covariance::{CovarianceMatrix, check_unique_variables};
pub use propagation::Uncertainty;

#[cfg(test)]
mod tests;
