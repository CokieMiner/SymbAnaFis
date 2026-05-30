#![allow(
    clippy::missing_const_for_fn,
    clippy::trivially_copy_pass_by_ref,
    reason = "Delegation wrappers can't be const for all backends; &T API for non-Copy uniformity"
)]

use crate::number::logic::int_math::IntType;
use alloc::string::String;
use core::cmp::Ordering;

// Backend selection priority:
// 1. f64        (default)
// 2. backendrug   (rug::Float — GMP/MPFR-based arbitrary precision)
// 3. backend32     (f32 — memory-optimized)

#[cfg(all(
    any(not(feature = "backend32"), feature = "backend64"),
    not(feature = "backendrug")
))]
use super::f64_ops as backend;

#[cfg(feature = "backendrug")]
use super::rug_ops as backend;

#[cfg(all(
    feature = "backend32",
    not(feature = "backend64"),
    not(feature = "backendrug")
))]
use super::f32_ops as backend;

/// The float representation type selected by the active backend feature.
pub type FloatType = backend::BackingFloat;

/// Macro that generates delegation functions for the float backend.
/// Every function listed here MUST be implemented by every backend module,
/// or compilation will fail with a clear missing-function error.
macro_rules! delegate_float_ops {
    (
        $(
            fn $name:ident( $($arg:ident : $ty:ty),* ) -> $ret:ty;
        )*
    ) => {
        $(
            #[inline]
            pub(in crate::number) fn $name( $($arg : $ty),* ) -> $ret {
                backend::$name( $($arg),* )
            }
        )*
    };
}

delegate_float_ops! {
    // --- Precision control (arbitrary-precision backends → Some; fixed → None) ---
    fn set_precision(bits: u32) -> bool;
    fn get_precision() -> u32;

    // --- Construction & conversion ---
    fn nan() -> FloatType;
    fn from_f32(value: f32) -> FloatType;
    fn from_f64(value: f64) -> FloatType;
    fn from_i64(value: i64) -> FloatType;
    fn from_int(value: &IntType) -> FloatType;
    fn clone(value: &FloatType) -> FloatType;
    fn to_int(value: &FloatType) -> Option<IntType>;
    fn to_rational(value: &FloatType) -> Option<crate::number::logic::rational_math::RationalType>;
    fn to_string(value: &FloatType) -> String;

    // --- Arithmetic ---
    fn add(lhs: &FloatType, rhs: &FloatType) -> FloatType;
    fn sub(lhs: &FloatType, rhs: &FloatType) -> FloatType;
    fn mul(lhs: &FloatType, rhs: &FloatType) -> FloatType;
    fn div(lhs: &FloatType, rhs: &FloatType) -> FloatType;
    fn neg(value: &FloatType) -> FloatType;

    // --- Comparison ---
    fn cmp(lhs: &FloatType, rhs: &FloatType) -> Option<Ordering>;

    // --- Properties ---
    fn is_zero(value: &FloatType) -> bool;
    fn is_one(value: &FloatType) -> bool;
    fn is_neg_one(value: &FloatType) -> bool;
    fn is_integer(value: &FloatType) -> bool;
    fn is_finite(value: &FloatType) -> bool;
    fn is_negative(value: &FloatType) -> bool;
    fn is_positive(value: &FloatType) -> bool;
    fn is_nan(value: &FloatType) -> bool;

    // --- Basic math ---
    fn abs(value: &FloatType) -> FloatType;
    fn signum(value: &FloatType) -> FloatType;
    fn floor(value: &FloatType) -> FloatType;
    fn ceil(value: &FloatType) -> FloatType;
    fn round(value: &FloatType) -> FloatType;
    fn fract(value: &FloatType) -> FloatType;
    fn pow(lhs: &FloatType, rhs: &FloatType) -> FloatType;
    fn sqrt(value: &FloatType) -> FloatType;
    fn cbrt(value: &FloatType) -> FloatType;

    // --- Trigonometric ---
    fn sin(value: &FloatType) -> FloatType;
    fn cos(value: &FloatType) -> FloatType;
    fn tan(value: &FloatType) -> FloatType;
    fn asin(value: &FloatType) -> FloatType;
    fn acos(value: &FloatType) -> FloatType;
    fn atan(value: &FloatType) -> FloatType;
    fn atan2(y: &FloatType, x: &FloatType) -> FloatType;

    // --- Hyperbolic ---
    fn sinh(value: &FloatType) -> FloatType;
    fn cosh(value: &FloatType) -> FloatType;
    fn tanh(value: &FloatType) -> FloatType;
    fn asinh(value: &FloatType) -> FloatType;
    fn acosh(value: &FloatType) -> FloatType;
    fn atanh(value: &FloatType) -> FloatType;

    // --- Exponential & logarithmic ---
    fn exp(value: &FloatType) -> FloatType;
    fn expm1(value: &FloatType) -> FloatType;
    fn ln(value: &FloatType) -> FloatType;
    fn log1p(value: &FloatType) -> FloatType;

    // --- Special functions ---
    fn erf(value: &FloatType) -> FloatType;
    fn erfc(value: &FloatType) -> FloatType;
    fn gamma(value: &FloatType) -> FloatType;
    fn lgamma(value: &FloatType) -> FloatType;
    fn digamma(value: &FloatType) -> FloatType;
    fn trigamma(value: &FloatType) -> FloatType;
    fn tetragamma(value: &FloatType) -> FloatType;
    fn lambertw(order: &IntType, value: &FloatType) -> FloatType;
    fn elliptic_k(value: &FloatType) -> FloatType;
    fn elliptic_e(value: &FloatType) -> FloatType;
    fn zeta(value: &FloatType) -> FloatType;
    fn besselj(n: &IntType, value: &FloatType) -> FloatType;
    fn bessely(n: &IntType, value: &FloatType) -> FloatType;
    fn besseli(n: &IntType, value: &FloatType) -> FloatType;
    fn besselk(n: &IntType, value: &FloatType) -> FloatType;
    fn polygamma(n: &IntType, value: &FloatType) -> FloatType;
    fn beta(a: &FloatType, b: &FloatType) -> FloatType;
    fn zeta_deriv(n: &IntType, value: &FloatType) -> FloatType;
    fn hermite(n: &IntType, value: &FloatType) -> FloatType;
    fn assoc_legendre(l: &IntType, m: &IntType, value: &FloatType) -> FloatType;
    fn spherical_harmonic(l: &IntType, m: &IntType, theta: &FloatType, phi: &FloatType) -> FloatType;
}

#[cfg(feature = "serde")]
#[inline]
pub(in crate::number) fn from_str(value: &str) -> Option<FloatType> {
    backend::from_str(value)
}
