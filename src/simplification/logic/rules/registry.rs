//! Compile one selected rule family before sequence execution.

use alloc::vec::Vec;

use super::{DomainSafety, PreparedRule, PreparedStep, SimplificationStep};

impl PreparedStep<'_> {
    pub fn rules(step: &SimplificationStep, safety: DomainSafety) -> Vec<PreparedRule> {
        let templates = if safety == DomainSafety::AssumptionDependent {
            Self::assumed_rules(step)
        } else {
            match step {
                SimplificationStep::Algebraic => Self::algebraic_rules(),
                SimplificationStep::Exponential => Self::exponential_rules(),
                SimplificationStep::Trigonometric => Self::trigonometric_rules(),
                SimplificationStep::Hyperbolic => Self::hyperbolic_rules(),
                SimplificationStep::Roots => Self::root_rules(),
                SimplificationStep::Factor => Self::factoring_rules(),
                SimplificationStep::Compilation => Self::compilation_rules(),
                SimplificationStep::Expand
                | SimplificationStep::Rewrite(_)
                | SimplificationStep::Map(_) => Vec::new(),
            }
        };
        templates
            .into_iter()
            .map(|(source, destination)| PreparedRule::new(source, destination, safety))
            .collect()
    }
}
