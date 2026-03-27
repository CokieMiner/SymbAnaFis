/// Angle-related trigonometric rules
pub mod angles;
/// Trigonometric simplification rules
pub mod basic;
mod helpers;
/// Trigonometric identities
pub mod identities;
/// Inverse trigonometric function rules
pub mod inverse;
/// Trigonometric transformations
pub mod transformations;
/// Triple angle formulas
pub mod triple_angle;

/// Rule list collector
pub mod rules;
pub use rules::get_trigonometric_rules;

pub(super) use super::{Rule, RuleCategory, RuleContext, RuleExprKind, extract_coeff_arc};
pub(super) use crate::simplification::logic::helpers::{
    approx_eq, get_numeric_value, is_multiple_of_two_pi, is_pi, is_three_pi_over_two,
};
