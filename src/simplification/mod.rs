//! Target selection and reusable sequences of domain-preserving passes.

mod api;
mod logic;

pub use api::{
    DomainSafety, Simplification, SimplificationSequence, SimplificationStep, SimplificationTarget,
    SimplificationTermination,
};
pub use logic::{CandidateCost, Limits, PreparedPlan, PreparedStep};

#[cfg(test)]
mod tests;
