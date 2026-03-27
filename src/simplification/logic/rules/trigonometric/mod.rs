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

pub(super) use super::helpers as rule_helpers;
pub(super) use super::{ExprKind, Rule, RuleCategory, RuleContext, extract_coeff_arc};
