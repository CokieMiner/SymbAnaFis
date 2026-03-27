//! Function definitions aggregator.
//!
//! This module aggregates all category lists into a single consolidated vector.

use super::FunctionDefinition;
use super::exponential::get_definitions as get_exp_defs;
use super::hyperbolic::get_definitions as get_hyp_defs;
use super::inverse_hyperbolic::get_definitions as inverse_get_hyp_defs;
use super::inverse_trig::get_definitions as get_inv_trig_defs;
use super::special::get_definitions as get_spec_defs;
use super::trigonometric::get_definitions as get_trig_defs;

/// Return all function definitions for populating the registry
pub fn all_definitions() -> Vec<FunctionDefinition> {
    let mut defs = Vec::new();
    defs.extend(get_trig_defs());
    defs.extend(get_inv_trig_defs());
    defs.extend(get_hyp_defs());
    defs.extend(inverse_get_hyp_defs());
    defs.extend(get_exp_defs());
    defs.extend(get_spec_defs());
    defs
}
