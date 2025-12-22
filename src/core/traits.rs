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
pub(crate) const EPSILON: f64 = 1e-14;

/// A trait comprising all operations required for mathematical scalars
/// in the SymbAnaFis library.
///
/// This aggregates `num_traits::Float` (providing sin, cos, exp, etc.),
/// `FloatConst` (PI, E), and standard arithmetic/debug traits.
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
pub(crate) fn is_zero(n: f64) -> bool {
    n.abs() < EPSILON
}

/// Check if a float is approximately one (within tolerance)
#[inline]
pub(crate) fn is_one(n: f64) -> bool {
    (n - 1.0).abs() < EPSILON
}

/// Check if a float is approximately negative one (within tolerance)
#[inline]
pub(crate) fn is_neg_one(n: f64) -> bool {
    (n + 1.0).abs() < EPSILON
}

#[cfg(test)]
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
