#![allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "Backend implementation for Copy types, optimization handled by #[inline]"
)]

use core::cmp::Ordering;

pub(super) type BackingInt = i64;

#[inline]
pub(super) const fn zero() -> BackingInt {
    0
}

#[inline]
pub(super) const fn clone(value: &BackingInt) -> BackingInt {
    *value
}

#[inline]
#[allow(
    clippy::unnecessary_wraps,
    reason = "Maintain consistent API across backends"
)]
pub(super) const fn from_i64_exact(value: i64) -> Option<BackingInt> {
    Some(value)
}

#[inline]
pub(super) fn from_u64_exact(value: u64) -> Option<BackingInt> {
    i64::try_from(value).ok()
}

#[inline]
pub(super) fn from_i128_exact(value: i128) -> Option<BackingInt> {
    i64::try_from(value).ok()
}

#[inline]
pub(super) fn from_u128_exact(value: u128) -> Option<BackingInt> {
    i64::try_from(value).ok()
}

#[inline]
#[allow(
    clippy::unnecessary_wraps,
    reason = "Maintain consistent API across backends"
)]
pub(super) const fn to_i64_exact(value: &BackingInt) -> Option<i64> {
    Some(*value)
}

#[inline]
#[allow(clippy::cast_precision_loss, reason = "Lossy conversion is intended")]
pub(super) const fn to_f64_lossy(value: &BackingInt) -> f64 {
    *value as f64
}

#[inline]
pub(super) const fn checked_abs(value: &BackingInt) -> Option<BackingInt> {
    value.checked_abs()
}

#[inline]
pub(super) const fn checked_neg(value: &BackingInt) -> Option<BackingInt> {
    value.checked_neg()
}

#[inline]
pub(super) const fn checked_add(lhs: &BackingInt, rhs: &BackingInt) -> Option<BackingInt> {
    lhs.checked_add(*rhs)
}

#[inline]
pub(super) const fn checked_mul(lhs: &BackingInt, rhs: &BackingInt) -> Option<BackingInt> {
    lhs.checked_mul(*rhs)
}

#[inline]
pub(super) fn cmp(lhs: &BackingInt, rhs: &BackingInt) -> Ordering {
    lhs.cmp(rhs)
}

#[inline]
pub(super) const fn is_zero(value: &BackingInt) -> bool {
    *value == 0
}

#[inline]
pub(super) const fn is_one(value: &BackingInt) -> bool {
    *value == 1
}

#[inline]
pub(super) const fn is_neg_one(value: &BackingInt) -> bool {
    *value == -1
}

#[inline]
pub(super) const fn is_negative(value: &BackingInt) -> bool {
    *value < 0
}

#[inline]
pub(super) const fn is_positive(value: &BackingInt) -> bool {
    *value > 0
}

#[inline]
pub(super) const fn is_even(value: &BackingInt) -> bool {
    *value % 2 == 0
}

#[inline]
pub(super) const fn modulo(lhs: &BackingInt, rhs: &BackingInt) -> BackingInt {
    *lhs % *rhs
}

#[inline]
#[allow(clippy::integer_division, reason = "Exact division is intended")]
pub(super) const fn div_exact(lhs: &BackingInt, rhs: &BackingInt) -> BackingInt {
    *lhs / *rhs
}

#[inline]
pub(super) fn gcd(lhs: &BackingInt, rhs: &BackingInt) -> BackingInt {
    let mut a = lhs.unsigned_abs();
    let mut b = rhs.unsigned_abs();

    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }

    if a == 0 {
        1
    } else {
        i64::try_from(a).unwrap_or(1)
    }
}

#[inline]
// The `as f64` cast is safe because the root fits entirely within the f64's 53-bit mantissa.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "Precision loss and truncation are intentional for float-based root finding"
)]
pub(super) fn perfect_square(value: &BackingInt) -> Option<BackingInt> {
    if *value < 0 {
        return None;
    }

    let root = (*value as f64).sqrt() as i64;
    for candidate in [root.saturating_sub(1), root, root.saturating_add(1)] {
        if let Some(square) = candidate.checked_mul(candidate)
            && square == *value
        {
            return Some(candidate);
        }
    }

    None
}

// The `as f64` cast is safe because the root fits entirely within the f64's 53-bit mantissa.
#[inline]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "Precision loss and truncation are intentional for float-based root finding"
)]
pub(super) fn perfect_cube(value: &BackingInt) -> Option<BackingInt> {
    let root = (*value as f64).cbrt().round() as i64;
    for candidate in [root.saturating_sub(1), root, root.saturating_add(1)] {
        if let Some(square) = candidate.checked_mul(candidate)
            && let Some(cube) = square.checked_mul(candidate)
            && cube == *value
        {
            return Some(candidate);
        }
    }

    None
}

#[cfg_attr(
    not(any(feature = "backend_big_rug", feature = "backend_big_astro")),
    allow(
        dead_code,
        reason = "Function is shared across feature combinations where it may be unused"
    )
)]
pub(super) fn to_string(value: &BackingInt) -> String {
    value.to_string()
}
