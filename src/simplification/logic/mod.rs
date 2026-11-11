//! Bounded DAG transformations and guarded mathematical identities.

use super::{DomainSafety, SimplificationStep, SimplificationTarget};

mod controls;
mod polynomials;
mod scalar;
mod scoring;
mod step;

pub use controls::{Domain, Limits, PassFeatures, PreparedPlan};
pub use polynomials::Polynomials;
pub use scalar::{Algebraic, Branches, Identities, Powers};
pub use scoring::CandidateCost;
pub use step::PreparedStep;
