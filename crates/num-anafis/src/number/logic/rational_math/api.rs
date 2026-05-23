#![allow(
    clippy::missing_const_for_fn,
    clippy::trivially_copy_pass_by_ref,
    reason = "Delegation wrappers can't be const for all backends; &T API for non-Copy uniformity"
)]

use crate::number::logic::int_math::IntRepr;
use alloc::string::String;
use core::cmp::Ordering;

// Backend selection priority:
// 1. Ratio<i64> (default)
// 2. backend_rug   (rug::Rational — GMP-based arbitrary precision)
// 3. backend32     (Ratio<i32> — memory-optimized)

#[cfg(not(feature = "backendrug"))]
use super::primitive_math as backend;

#[cfg(feature = "backendrug")]
use super::rug_ops as backend;

/// Internal representation of rational numbers.
pub type RationalRepr = backend::BackingRational;

macro_rules! delegate_rational_ops {
    (
        $(
            fn $name:ident( $($arg:ident : $ty:ty),* ) -> $ret:ty;
        )*
    ) => {
        $(
            #[inline]
            #[allow(clippy::missing_const_for_fn, reason = "Backend functions cannot be const due to heap allocation")]
            pub(in crate::number) fn $name( $($arg : $ty),* ) -> $ret {
                backend::$name( $($arg),* )
            }
        )*
    };
}

delegate_rational_ops! {
    // --- Construction & Extraction ---
    fn from_integer(value: IntRepr) -> RationalRepr;
    fn new(num: IntRepr, den: IntRepr) -> RationalRepr;

    // We clone the components out of the rational to maintain abstraction
    fn numer(value: &RationalRepr) -> IntRepr;
    fn denom(value: &RationalRepr) -> IntRepr;

    // --- Arithmetic ---
    fn add(lhs: &RationalRepr, rhs: &RationalRepr) -> RationalRepr;
    fn sub(lhs: &RationalRepr, rhs: &RationalRepr) -> RationalRepr;
    fn mul(lhs: &RationalRepr, rhs: &RationalRepr) -> RationalRepr;
    fn div(lhs: &RationalRepr, rhs: &RationalRepr) -> RationalRepr;
    fn neg(value: &RationalRepr) -> RationalRepr;

    // --- Comparison ---
    fn cmp(lhs: &RationalRepr, rhs: &RationalRepr) -> Ordering;

    // --- Properties ---
    fn is_integer(value: &RationalRepr) -> bool;
    fn to_integer(value: &RationalRepr) -> IntRepr;

    fn to_string(value: &RationalRepr) -> String;
}
