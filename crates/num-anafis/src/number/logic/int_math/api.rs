#![allow(
    clippy::missing_const_for_fn,
    clippy::trivially_copy_pass_by_ref,
    reason = "Delegation wrappers can't be const for all backends; &T API for non-Copy uniformity"
)]

use alloc::string::String;
use core::cmp::Ordering;

// Backend selection priority:
// 1. i64        (default)
// 2. backendrug   (rug::Integer — GMP-based arbitrary precision)
// 3. backend32     (i32 — memory-optimized)

#[cfg(all(not(feature = "backend32"), not(feature = "backendrug")))]
use super::i64_math as backend;

#[cfg(feature = "backendrug")]
use super::rug_int as backend;

#[cfg(all(feature = "backend32", not(feature = "backendrug")))]
use super::i32_math as backend;

/// The integer representation type selected by the active backend feature.
pub type IntRepr = backend::BackingInt;

/// Macro that generates delegation functions for the integer backend.
/// Every function listed here MUST be implemented by every backend module,
/// or compilation will fail with a clear missing-function error.
macro_rules! delegate_int_ops {
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

delegate_int_ops! {
    fn zero() -> IntRepr;
    fn clone(value: &IntRepr) -> IntRepr;
    fn from_i64(value: i64) -> Option<IntRepr>;
    fn abs(value: &IntRepr) -> Option<IntRepr>;
    fn neg(value: &IntRepr) -> Option<IntRepr>;
    fn add(lhs: &IntRepr, rhs: &IntRepr) -> Option<IntRepr>;
    fn sub(lhs: &IntRepr, rhs: &IntRepr) -> Option<IntRepr>;
    fn mul(lhs: &IntRepr, rhs: &IntRepr) -> Option<IntRepr>;
    fn cmp(lhs: &IntRepr, rhs: &IntRepr) -> Ordering;
    fn is_zero(value: &IntRepr) -> bool;
    fn is_one(value: &IntRepr) -> bool;
    fn is_neg_one(value: &IntRepr) -> bool;
    fn is_negative(value: &IntRepr) -> bool;
    fn is_positive(value: &IntRepr) -> bool;
    fn is_even(value: &IntRepr) -> bool;
    fn modulo(lhs: &IntRepr, rhs: &IntRepr) -> IntRepr;
    fn div_exact(lhs: &IntRepr, rhs: &IntRepr) -> IntRepr;
    fn gcd(lhs: &IntRepr, rhs: &IntRepr) -> IntRepr;
    fn perfect_square(value: &IntRepr) -> Option<IntRepr>;
    fn perfect_cube(value: &IntRepr) -> Option<IntRepr>;
    fn to_string(value: &IntRepr) -> String;
}

#[cfg(feature = "serde")]
#[inline]
pub(in crate::number) fn from_str(value: &str) -> Option<IntRepr> {
    backend::from_str(value)
}
