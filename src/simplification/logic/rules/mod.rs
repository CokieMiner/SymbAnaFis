//! Rule infrastructure for the simplification engine

#[macro_use]
mod core;
mod registry;

// Re-exports
pub(super) use super::helpers::{
    compare_expr, compare_mul_factors, exprs_equivalent, extract_coeff, extract_coeff_arc, gcd,
    is_fractional_root_exponent, is_known_non_negative,
};
pub(super) use core::*;
pub(super) use registry::*;

/// Numeric simplification rules
pub mod numeric;

/// Algebraic simplification rules
pub mod algebraic;

/// Trigonometric simplification rules
pub mod trigonometric;

/// Exponential and logarithmic simplification rules
pub mod exponential;

/// Root simplification rules
pub mod root;

/// Hyperbolic function simplification rules
pub mod hyperbolic;
