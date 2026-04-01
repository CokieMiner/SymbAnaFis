#![allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "Backend implementation for Copy types, optimization handled by #[inline]"
)]
#![allow(clippy::missing_const_for_fn, reason = "f64 methods are not const")]
#![allow(
    clippy::float_cmp,
    reason = "Exact float comparison is intended in this context"
)]

use crate::number::logic::int_math::{IntRepr, int_to_f64_lossy};
use core::cmp::Ordering;

pub(super) type BackingFloat = f64;

#[inline]
pub(super) fn from_int(value: &IntRepr) -> BackingFloat {
    int_to_f64_lossy(value)
}

#[inline]
pub(super) const fn clone(value: &BackingFloat) -> BackingFloat {
    *value
}

#[inline]
pub(super) fn add(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    *lhs + *rhs
}

#[inline]
pub(super) fn sub(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    *lhs - *rhs
}

#[inline]
pub(super) fn mul(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    *lhs * *rhs
}

#[inline]
pub(super) fn div(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    *lhs / *rhs
}

#[inline]
pub(super) fn neg(value: &BackingFloat) -> BackingFloat {
    -*value
}

#[inline]
pub(super) fn abs(value: &BackingFloat) -> BackingFloat {
    value.abs()
}

#[inline]
pub(super) fn pow(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    lhs.powf(*rhs)
}

#[inline]
pub(super) fn sqrt(value: &BackingFloat) -> BackingFloat {
    value.sqrt()
}

#[inline]
pub(super) fn cbrt(value: &BackingFloat) -> BackingFloat {
    value.cbrt()
}

#[inline]
pub(super) fn fract(value: &BackingFloat) -> BackingFloat {
    value.fract()
}

#[inline]
pub(super) fn round(value: &BackingFloat) -> BackingFloat {
    value.round()
}

#[inline]
pub(super) fn sin(value: &BackingFloat) -> BackingFloat {
    value.sin()
}

#[inline]
pub(super) fn cos(value: &BackingFloat) -> BackingFloat {
    value.cos()
}

#[inline]
pub(super) fn exp(value: &BackingFloat) -> BackingFloat {
    value.exp()
}

#[inline]
pub(super) fn ln(value: &BackingFloat) -> BackingFloat {
    value.ln()
}

#[inline]
pub(super) fn atan(value: &BackingFloat) -> BackingFloat {
    value.atan()
}

#[inline]
pub(super) fn is_zero(value: &BackingFloat) -> bool {
    *value == 0.0
}

#[inline]
#[allow(clippy::float_cmp, reason = "Exact comparison for canonical values")]
pub(super) fn is_one(value: &BackingFloat) -> bool {
    *value == 1.0
}

#[inline]
#[allow(clippy::float_cmp, reason = "Exact comparison for canonical values")]
pub(super) fn is_neg_one(value: &BackingFloat) -> bool {
    *value == -1.0
}

#[inline]
pub(super) fn is_integer(value: &BackingFloat) -> bool {
    value.fract() == 0.0
}

#[inline]
pub(super) const fn is_finite(value: &BackingFloat) -> bool {
    value.is_finite()
}

#[inline]
pub(super) fn is_negative(value: &BackingFloat) -> bool {
    *value < 0.0
}

#[inline]
pub(super) fn is_positive(value: &BackingFloat) -> bool {
    *value > 0.0
}

#[inline]
pub(super) const fn to_f64(value: &BackingFloat) -> f64 {
    *value
}

#[inline]
pub(super) const fn from_f64(value: f64) -> BackingFloat {
    value
}

#[inline]
#[allow(clippy::cast_precision_loss, reason = "Intended lossy conversion")]
pub(super) const fn from_i64(value: i64) -> BackingFloat {
    value as f64
}

#[inline]
#[allow(
    clippy::if_then_some_else_none,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "Precision loss and truncation are intentional for float-to-i64 conversions"
)]
pub(super) fn to_i64_exact(value: &BackingFloat) -> Option<i64> {
    (value.fract() == 0.0 && *value >= i64::MIN as f64 && *value < -(i64::MIN as f64))
        .then_some(*value as i64)
}

#[inline]
pub(super) fn cmp(lhs: &BackingFloat, rhs: &BackingFloat) -> Option<Ordering> {
    lhs.partial_cmp(rhs)
}
