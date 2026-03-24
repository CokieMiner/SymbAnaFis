//! Helper math functions for evaluator-only special cases.

use crate::core::traits::EPSILON;

/// Compute sinc function with removable singularity handling.
///
/// `sinc(x) = sin(x)/x` with `sinc(0) = 1`
#[inline]
pub(super) fn eval_sinc(x: f64) -> f64 {
    if x.abs() < EPSILON { 1.0 } else { x.sin() / x }
}

/// Round a float to i32, returning None if out of range or NaN.
#[inline]
pub(super) fn round_to_i32(x: f64) -> Option<i32> {
    let rounded = x.round();
    if !(rounded >= f64::from(i32::MIN) && rounded <= f64::from(i32::MAX)) {
        return None;
    }
    #[allow(
        clippy::cast_possible_truncation,
        reason = "Range checked above before casting"
    )]
    Some(rounded as i32)
}
