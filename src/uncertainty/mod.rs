//! First-order symbolic uncertainty propagation with explicit covariances.

mod propagation;

pub use propagation::{CovarianceMatrix, Uncertainty};

#[cfg(test)]
mod tests;
