/// Absolute value and sign function rules
pub mod abs_sign;
/// Expression canonicalization rules
pub mod canonicalization;
/// Term combination and consolidation rules
pub mod combination;
/// Expression expansion rules
pub mod expansion;
/// Factoring and decomposition rules
pub mod factoring;
/// Fraction simplification rules
pub mod fractions;
/// Algebraic simplification rules
pub mod identities;
/// Power and exponentiation rules
pub mod powers;

/// Rule list collector
pub mod rules;
pub use rules::get_algebraic_rules;

pub(super) use super::{
    ExprKind, Rule, RuleCategory, RuleContext, compare_expr, compare_mul_factors, exprs_equivalent,
    extract_coeff, extract_coeff_arc, gcd, is_fractional_root_exponent, is_known_non_negative,
};
