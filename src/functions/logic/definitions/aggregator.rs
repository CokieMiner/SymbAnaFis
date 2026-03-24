//! Function definitions aggregator.
//!
//! This module aggregates all category lists into a single consolidated vector.

use crate::functions::logic::definitions::{
    exponential, hyperbolic, inverse_hyperbolic, inverse_trig, special, trigonometric,
};
use crate::functions::logic::registry::FunctionDefinition;

/// Return all function definitions for populating the registry
pub fn all_definitions() -> Vec<FunctionDefinition> {
    let mut defs = Vec::new();
    defs.extend(trigonometric::get_definitions());
    defs.extend(inverse_trig::get_definitions());
    defs.extend(hyperbolic::get_definitions());
    defs.extend(inverse_hyperbolic::get_definitions());
    defs.extend(exponential::get_definitions());
    defs.extend(special::get_definitions());
    defs
}
