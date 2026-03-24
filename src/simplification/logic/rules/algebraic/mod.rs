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
