#![allow(
    clippy::missing_const_for_fn,
    clippy::trivially_copy_pass_by_ref,
    reason = "Delegation wrappers can't be const for all backends; &T API for non-Copy uniformity"
)]

use crate::number::logic::int_math::IntType;
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
pub type RationalType = backend::BackingRational;

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
    fn from_integer(value: IntType) -> RationalType;
    fn new(num: IntType, den: IntType) -> RationalType;

    // We clone the components out of the rational to maintain abstraction
    fn numer(value: &RationalType) -> IntType;
    fn denom(value: &RationalType) -> IntType;

    // --- Arithmetic ---
    fn add(lhs: &RationalType, rhs: &RationalType) -> RationalType;
    fn sub(lhs: &RationalType, rhs: &RationalType) -> RationalType;
    fn mul(lhs: &RationalType, rhs: &RationalType) -> RationalType;
    fn div(lhs: &RationalType, rhs: &RationalType) -> RationalType;
    fn neg(value: &RationalType) -> RationalType;

    // --- Comparison ---
    fn cmp(lhs: &RationalType, rhs: &RationalType) -> Ordering;

    // --- Properties ---
    fn is_integer(value: &RationalType) -> bool;
    fn to_integer(value: &RationalType) -> IntType;

    fn to_string(value: &RationalType) -> String;
}
