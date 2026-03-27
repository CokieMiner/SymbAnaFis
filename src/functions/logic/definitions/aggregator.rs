//! Function definitions aggregator.
//!
//! This module aggregates all category lists into a single consolidated vector.

use super::FunctionDefinition;
use super::exponential::get_exponential_definitions;
use super::hyperbolic::get_hyperbolic_definitions;
use super::inverse_hyperbolic::get_inverse_hyperbolic_definitions;
use super::inverse_trig::get_inverse_trig_definitions;
use super::special::get_special_definitions;
use super::trigonometric::get_trigonometric_definitions;

/// Return all function definitions for populating the registry
pub fn all_definitions() -> Vec<FunctionDefinition> {
    let mut defs = Vec::new();
    defs.extend(get_trigonometric_definitions());
    defs.extend(get_inverse_trig_definitions());
    defs.extend(get_hyperbolic_definitions());
    defs.extend(get_inverse_hyperbolic_definitions());
    defs.extend(get_exponential_definitions());
    defs.extend(get_special_definitions());
    defs
}
