//! Helper math functions for evaluator-only special cases.

use crate::core::traits::EPSILON;

/// Compute sinc function with removable singularity handling.
///
/// `sinc(x) = sin(x)/x` with `sinc(0) = 1`
#[inline]
pub(super) fn eval_sinc(x: f64) -> f64 {
    if x.abs() < EPSILON { 1.0 } else { x.sin() / x }
}

/// Evaluate `acot(x)` with range (0, π), matching the interpreter convention.
///
/// - `acot(0)  = π/2`
/// - `acot(x)  = atan(1/x)` for x > 0
/// - `acot(x)  = atan(1/x) + π` for x < 0
#[inline]
pub(super) fn eval_acot(x: f64) -> f64 {
    if x.abs() < EPSILON {
        std::f64::consts::FRAC_PI_2
    } else if let Some(ordering) = x.partial_cmp(&0.0) {
        let inv = 1.0 / x;
        if ordering == std::cmp::Ordering::Greater {
            inv.atan()
        } else {
            inv.atan() + std::f64::consts::PI
        }
    } else {
        f64::NAN
    }
}

/// Evaluate `acsch(x)`
#[inline]
pub(super) fn eval_acsch(x: f64) -> f64 {
    (1.0 / x + (1.0 / (x * x) + 1.0).sqrt()).ln()
}

/// Evaluate `asech(x)`
#[inline]
pub(super) fn eval_asech(x: f64) -> f64 {
    (1.0 / x + (1.0 / (x * x) - 1.0).sqrt()).ln()
}

/// Evaluate `acoth(x)`
#[inline]
pub(super) fn eval_acoth(x: f64) -> f64 {
    // acoth(x) = 0.5 * ln((x + 1) / (x - 1))
    0.5 * ((x + 1.0) / (x - 1.0)).ln()
}
