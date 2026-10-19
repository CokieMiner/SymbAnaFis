//! Composable simplification passes, presets, and execution outcomes.

use super::{CandidateCost, Limits, PreparedPlan, PreparedStep};

mod domain;
mod sequence;
mod step;
mod target;
mod termination;

pub use domain::DomainSafety;
pub use sequence::{Simplification, SimplificationSequence};
pub use step::SimplificationStep;
pub use target::SimplificationTarget;
pub use termination::SimplificationTermination;
