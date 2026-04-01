#![allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "Maintain abstraction for non-Copy high-precision types"
)]
#![allow(
    clippy::missing_const_for_fn,
    reason = "Maintain consistency with backends where methods are not const"
)]

use core::cmp::Ordering;

// Priority order when multiple features are enabled:
// 1. backend_big_rug / backend_big_astro (arbitrary precision integer, BigInt)
// 2. backend32 (i32, memory-optimized)
// 3. backend64 (i64)
//
// Note: When --all-features is used, priority order is applied automatically.
//       For production use, enable only ONE int backend feature.

#[cfg(any(feature = "backend_big_astro", feature = "backend_big_rug"))]
use super::bigint_math as backend;

#[cfg(all(
    feature = "backend32",
    not(any(feature = "backend_big_astro", feature = "backend_big_rug"))
))]
use super::i32_math as backend;

#[cfg(all(
    not(feature = "backend32"),
    not(feature = "backend_big_astro"),
    not(feature = "backend_big_rug")
))]
use super::i64_math as backend;

pub type IntRepr = backend::BackingInt;

pub(in crate::number::logic) fn int_zero() -> IntRepr {
    backend::zero()
}

pub(in crate::number::logic) fn int_clone(value: &IntRepr) -> IntRepr {
    backend::clone(value)
}

pub(in crate::number::logic) fn int_from_i64_exact(value: i64) -> Option<IntRepr> {
    backend::from_i64_exact(value)
}

pub(in crate::number::logic) fn int_from_u64_exact(value: u64) -> Option<IntRepr> {
    backend::from_u64_exact(value)
}

pub(in crate::number::logic) fn int_from_i128_exact(value: i128) -> Option<IntRepr> {
    backend::from_i128_exact(value)
}

pub(in crate::number::logic) fn int_from_u128_exact(value: u128) -> Option<IntRepr> {
    backend::from_u128_exact(value)
}

pub(in crate::number::logic) fn int_to_i64_exact(value: &IntRepr) -> Option<i64> {
    backend::to_i64_exact(value)
}

pub(in crate::number::logic) fn int_to_f64_lossy(value: &IntRepr) -> f64 {
    backend::to_f64_lossy(value)
}

pub(in crate::number::logic) fn int_checked_abs(value: &IntRepr) -> Option<IntRepr> {
    backend::checked_abs(value)
}

pub(in crate::number::logic) fn int_checked_neg(value: &IntRepr) -> Option<IntRepr> {
    backend::checked_neg(value)
}

pub(in crate::number::logic) fn int_checked_add(lhs: &IntRepr, rhs: &IntRepr) -> Option<IntRepr> {
    backend::checked_add(lhs, rhs)
}

pub(in crate::number::logic) fn int_checked_mul(lhs: &IntRepr, rhs: &IntRepr) -> Option<IntRepr> {
    backend::checked_mul(lhs, rhs)
}

pub(in crate::number::logic) fn int_cmp(lhs: &IntRepr, rhs: &IntRepr) -> Ordering {
    backend::cmp(lhs, rhs)
}

pub(in crate::number::logic) fn int_is_zero(value: &IntRepr) -> bool {
    backend::is_zero(value)
}

pub(in crate::number::logic) fn int_is_one(value: &IntRepr) -> bool {
    backend::is_one(value)
}

pub(in crate::number::logic) fn int_is_neg_one(value: &IntRepr) -> bool {
    backend::is_neg_one(value)
}

pub(in crate::number::logic) fn int_is_negative(value: &IntRepr) -> bool {
    backend::is_negative(value)
}

pub(in crate::number::logic) fn int_is_positive(value: &IntRepr) -> bool {
    backend::is_positive(value)
}

pub(in crate::number::logic) fn int_is_even(value: &IntRepr) -> bool {
    backend::is_even(value)
}

pub(in crate::number::logic) fn int_mod(lhs: &IntRepr, rhs: &IntRepr) -> IntRepr {
    backend::modulo(lhs, rhs)
}

pub(in crate::number::logic) fn int_div_exact(lhs: &IntRepr, rhs: &IntRepr) -> IntRepr {
    backend::div_exact(lhs, rhs)
}

pub(in crate::number::logic) fn int_gcd(lhs: &IntRepr, rhs: &IntRepr) -> IntRepr {
    backend::gcd(lhs, rhs)
}

pub(in crate::number::logic) fn int_perfect_square(value: &IntRepr) -> Option<IntRepr> {
    backend::perfect_square(value)
}

pub(in crate::number::logic) fn int_perfect_cube(value: &IntRepr) -> Option<IntRepr> {
    backend::perfect_cube(value)
}

#[cfg_attr(
    not(any(feature = "backend_big_rug", feature = "backend_big_astro")),
    allow(
        dead_code,
        reason = "String conversion helper is only needed by high-precision float backends"
    )
)]
pub(in crate::number::logic) fn int_to_string(value: &IntRepr) -> String {
    backend::to_string(value)
}
