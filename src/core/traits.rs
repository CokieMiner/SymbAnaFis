//! Math scalar trait and floating-point tolerance helpers
//!
//! Provides `MathScalar` trait aggregating all numeric operations, and
//! tolerance-based comparison functions for floating-point values.

use num_traits::{Float, FloatConst, FromPrimitive, Signed, ToPrimitive};
use std::fmt::{Debug, Display};
use std::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};

/// Tolerance for floating-point comparisons (used throughout expression operations)
pub const EPSILON: f64 = 1e-14;

/// A trait comprising all operations required for mathematical scalars
/// in the `SymbAnaFis` library.
///
/// This trait aggregates essential numeric operations needed for symbolic computation:
/// - `num_traits::Float`: Trigonometric, exponential, and logarithmic functions
/// - `FloatConst`: Mathematical constants (π, e)
/// - Standard arithmetic operations and comparisons
/// - Display and debugging traits
///
/// # Implementations
/// Automatically implemented for any type satisfying all the bounds.
/// The primary implementation is for `f64`.
///
/// # Usage
/// This trait enables generic programming over numeric types, particularly
/// useful for dual number arithmetic and other mathematical abstractions.
///
/// ```
/// use symb_anafis::MathScalar;
///
/// fn compute<T: MathScalar>(x: T) -> T {
///     x.sin() * x.exp()  // Works for f64, Dual<f64>, etc.
/// }
/// ```
pub trait MathScalar:
    Float
    + FloatConst
    + FromPrimitive
    + ToPrimitive
    + Signed
    + Debug
    + Display
    + Copy
    + Clone
    + PartialEq
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Rem<Output = Self>
    + Neg<Output = Self>
    + AddAssign
    + SubAssign
    + MulAssign
    + DivAssign
    + RemAssign
    + 'static
{
    // Helpful extra methods if needed not covered by Float
}

// Blanket implementation for any type that satisfies the bounds
impl<T> MathScalar for T where
    T: Float
        + FloatConst
        + FromPrimitive
        + ToPrimitive
        + Signed
        + Debug
        + Display
        + Copy
        + Clone
        + PartialEq
        + PartialOrd
        + Add<Output = T>
        + Sub<Output = T>
        + Mul<Output = T>
        + Div<Output = T>
        + Rem<Output = T>
        + Neg<Output = T>
        + AddAssign
        + SubAssign
        + MulAssign
        + DivAssign
        + RemAssign
        + 'static
{
}

// ===== Float tolerance helpers =====
// These functions provide safe floating-point comparisons to avoid
// precision issues like `1.0/3.0 * 3.0 != 1.0`.

/// Check if a float is approximately zero (within tolerance)
#[inline]
pub fn is_zero(n: f64) -> bool {
    n.abs() < EPSILON
}

/// Check if a float is approximately one (within tolerance)
#[inline]
pub fn is_one(n: f64) -> bool {
    (n - 1.0).abs() < EPSILON
}

/// Check if a float is approximately negative one (within tolerance)
#[inline]
pub fn is_neg_one(n: f64) -> bool {
    (n + 1.0).abs() < EPSILON
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::let_underscore_must_use,
    clippy::no_effect_underscore_binding,
    reason = "Standard test relaxations"
)]
mod tests {
    use super::*;

    #[test]
    fn test_is_zero() {
        assert!(is_zero(0.0));
        assert!(is_zero(1e-15));
        assert!(is_zero(-1e-15));
        assert!(!is_zero(0.1));
        assert!(!is_zero(-0.1));
    }

    #[test]
    fn test_is_one() {
        assert!(is_one(1.0));
        assert!(is_one(1.0 + 1e-15));
        assert!(is_one(1.0 - 1e-15));
        assert!(!is_one(1.1));
        assert!(!is_one(0.9));
    }

    #[test]
    fn test_is_neg_one() {
        assert!(is_neg_one(-1.0));
        assert!(is_neg_one(-1.0 + 1e-15));
        assert!(!is_neg_one(1.0));
    }
}
