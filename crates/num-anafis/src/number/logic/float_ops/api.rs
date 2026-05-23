#![allow(
    clippy::missing_const_for_fn,
    clippy::trivially_copy_pass_by_ref,
    reason = "Delegation wrappers can't be const for all backends; &T API for non-Copy uniformity"
)]

use crate::number::logic::int_math::IntRepr;
use alloc::string::String;
use core::cmp::Ordering;

// Backend selection priority:
// 1. f64        (default)
// 2. backendrug   (rug::Float — GMP/MPFR-based arbitrary precision)
// 3. backend32     (f32 — memory-optimized)

#[cfg(all(not(feature = "backend32"), not(feature = "backendrug")))]
use super::f64_ops as backend;

#[cfg(feature = "backendrug")]
use super::rug_ops as backend;

#[cfg(all(feature = "backend32", not(feature = "backendrug")))]
use super::f32_ops as backend;

/// The float representation type selected by the active backend feature.
pub type FloatRepr = backend::BackingFloat;

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
    fn nan() -> FloatRepr;
    fn from_f32(value: f32) -> FloatRepr;
    fn from_f64(value: f64) -> FloatRepr;
    fn from_i64(value: i64) -> FloatRepr;
    fn from_int(value: &IntRepr) -> FloatRepr;
    fn clone(value: &FloatRepr) -> FloatRepr;
    fn to_int(value: &FloatRepr) -> Option<IntRepr>;
    fn to_rational(value: &FloatRepr) -> Option<crate::number::logic::rational_math::RationalRepr>;
    fn to_string(value: &FloatRepr) -> String;

    // --- Arithmetic ---
    fn add(lhs: &FloatRepr, rhs: &FloatRepr) -> FloatRepr;
    fn sub(lhs: &FloatRepr, rhs: &FloatRepr) -> FloatRepr;
    fn mul(lhs: &FloatRepr, rhs: &FloatRepr) -> FloatRepr;
    fn div(lhs: &FloatRepr, rhs: &FloatRepr) -> FloatRepr;
    fn neg(value: &FloatRepr) -> FloatRepr;

    // --- Comparison ---
    fn cmp(lhs: &FloatRepr, rhs: &FloatRepr) -> Option<Ordering>;

    // --- Properties ---
    fn is_zero(value: &FloatRepr) -> bool;
    fn is_one(value: &FloatRepr) -> bool;
    fn is_neg_one(value: &FloatRepr) -> bool;
    fn is_integer(value: &FloatRepr) -> bool;
    fn is_finite(value: &FloatRepr) -> bool;
    fn is_negative(value: &FloatRepr) -> bool;
    fn is_positive(value: &FloatRepr) -> bool;
    fn is_nan(value: &FloatRepr) -> bool;

    // --- Basic math ---
    fn abs(value: &FloatRepr) -> FloatRepr;
    fn signum(value: &FloatRepr) -> FloatRepr;
    fn floor(value: &FloatRepr) -> FloatRepr;
    fn ceil(value: &FloatRepr) -> FloatRepr;
    fn round(value: &FloatRepr) -> FloatRepr;
    fn fract(value: &FloatRepr) -> FloatRepr;
    fn pow(lhs: &FloatRepr, rhs: &FloatRepr) -> FloatRepr;
    fn sqrt(value: &FloatRepr) -> FloatRepr;
    fn cbrt(value: &FloatRepr) -> FloatRepr;

    // --- Trigonometric ---
    fn sin(value: &FloatRepr) -> FloatRepr;
    fn cos(value: &FloatRepr) -> FloatRepr;
    fn tan(value: &FloatRepr) -> FloatRepr;
    fn asin(value: &FloatRepr) -> FloatRepr;
    fn acos(value: &FloatRepr) -> FloatRepr;
    fn atan(value: &FloatRepr) -> FloatRepr;
    fn atan2(y: &FloatRepr, x: &FloatRepr) -> FloatRepr;

    // --- Hyperbolic ---
    fn sinh(value: &FloatRepr) -> FloatRepr;
    fn cosh(value: &FloatRepr) -> FloatRepr;
    fn tanh(value: &FloatRepr) -> FloatRepr;
    fn asinh(value: &FloatRepr) -> FloatRepr;
    fn acosh(value: &FloatRepr) -> FloatRepr;
    fn atanh(value: &FloatRepr) -> FloatRepr;

    // --- Exponential & logarithmic ---
    fn exp(value: &FloatRepr) -> FloatRepr;
    fn expm1(value: &FloatRepr) -> FloatRepr;
    fn ln(value: &FloatRepr) -> FloatRepr;
    fn log1p(value: &FloatRepr) -> FloatRepr;

    // --- Special functions ---
    fn erf(value: &FloatRepr) -> FloatRepr;
    fn erfc(value: &FloatRepr) -> FloatRepr;
    fn gamma(value: &FloatRepr) -> FloatRepr;
    fn lgamma(value: &FloatRepr) -> FloatRepr;
    fn digamma(value: &FloatRepr) -> FloatRepr;
    fn trigamma(value: &FloatRepr) -> FloatRepr;
    fn tetragamma(value: &FloatRepr) -> FloatRepr;
    fn lambert_w(value: &FloatRepr) -> FloatRepr;
    fn lambert_wm1(value: &FloatRepr) -> FloatRepr;
    fn elliptic_k(value: &FloatRepr) -> FloatRepr;
    fn elliptic_e(value: &FloatRepr) -> FloatRepr;
    fn zeta(value: &FloatRepr) -> FloatRepr;
    fn bessel_j(n: &IntRepr, value: &FloatRepr) -> FloatRepr;
    fn bessel_y(n: &IntRepr, value: &FloatRepr) -> FloatRepr;
    fn bessel_i(n: &IntRepr, value: &FloatRepr) -> FloatRepr;
    fn bessel_k(n: &IntRepr, value: &FloatRepr) -> FloatRepr;
    fn polygamma(n: &IntRepr, value: &FloatRepr) -> FloatRepr;
    fn beta(a: &FloatRepr, b: &FloatRepr) -> FloatRepr;
    fn zeta_deriv(n: &IntRepr, value: &FloatRepr) -> FloatRepr;
    fn hermite(n: &IntRepr, value: &FloatRepr) -> FloatRepr;
    fn assoc_legendre(l: &IntRepr, m: &IntRepr, value: &FloatRepr) -> FloatRepr;
    fn spherical_harmonic(l: &IntRepr, m: &IntRepr, theta: &FloatRepr, phi: &FloatRepr) -> FloatRepr;
}

#[cfg(feature = "serde")]
#[inline]
pub(in crate::number) fn from_str(value: &str) -> Option<FloatRepr> {
    backend::from_str(value)
}
