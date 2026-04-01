#![allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "Backend implementation for Copy types, optimization handled by #[inline]"
)]
#![allow(clippy::missing_const_for_fn, reason = "f32 methods are not const")]
#![allow(
    clippy::float_cmp,
    reason = "Exact float comparison is intended for special values (0.0, 1.0, -1.0)"
)]

use crate::number::logic::int_math::{IntRepr, int_to_f64_lossy};
use core::cmp::Ordering;

pub(super) type BackingFloat = f32;

#[allow(
    clippy::cast_possible_truncation,
    reason = "Lossy conversion from int to f32 is intentional"
)]
pub(super) fn from_int(value: &IntRepr) -> BackingFloat {
    int_to_f64_lossy(value) as f32
}

pub(super) fn clone(value: &BackingFloat) -> BackingFloat {
    *value
}

pub(super) fn add(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    *lhs + *rhs
}

pub(super) fn sub(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    *lhs - *rhs
}

pub(super) fn mul(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    *lhs * *rhs
}

pub(super) fn div(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    *lhs / *rhs
}

pub(super) fn neg(value: &BackingFloat) -> BackingFloat {
    -*value
}

pub(super) fn abs(value: &BackingFloat) -> BackingFloat {
    value.abs()
}

pub(super) fn pow(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    lhs.powf(*rhs)
}

pub(super) fn sqrt(value: &BackingFloat) -> BackingFloat {
    value.sqrt()
}

pub(super) fn cbrt(value: &BackingFloat) -> BackingFloat {
    value.cbrt()
}

pub(super) fn fract(value: &BackingFloat) -> BackingFloat {
    value.fract()
}

pub(super) fn round(value: &BackingFloat) -> BackingFloat {
    value.round()
}

pub(super) fn sin(value: &BackingFloat) -> BackingFloat {
    value.sin()
}

pub(super) fn cos(value: &BackingFloat) -> BackingFloat {
    value.cos()
}

pub(super) fn exp(value: &BackingFloat) -> BackingFloat {
    value.exp()
}

pub(super) fn ln(value: &BackingFloat) -> BackingFloat {
    value.ln()
}

pub(super) fn atan(value: &BackingFloat) -> BackingFloat {
    value.atan()
}

pub(super) fn is_zero(value: &BackingFloat) -> bool {
    *value == 0.0
}

pub(super) fn is_one(value: &BackingFloat) -> bool {
    *value == 1.0
}

pub(super) fn is_neg_one(value: &BackingFloat) -> bool {
    *value == -1.0
}

pub(super) fn is_integer(value: &BackingFloat) -> bool {
    value.fract() == 0.0
}

pub(super) fn is_finite(value: &BackingFloat) -> bool {
    value.is_finite()
}

pub(super) fn is_negative(value: &BackingFloat) -> bool {
    *value < 0.0
}

pub(super) fn is_positive(value: &BackingFloat) -> bool {
    *value > 0.0
}

pub(super) fn to_f64(value: &BackingFloat) -> f64 {
    f64::from(*value)
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "f64 to f32 conversion is intentionally lossy for this backend"
)]
pub(super) fn from_f64(value: f64) -> BackingFloat {
    value as f32
}

#[allow(
    clippy::cast_precision_loss,
    reason = "i64 to f32 conversion is intentionally lossy for this backend"
)]
pub(super) fn from_i64(value: i64) -> BackingFloat {
    value as f32
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "Exact integer extraction requires bounded float-to-int casts after explicit range checks"
)]
pub(super) fn to_i64_exact(value: &BackingFloat) -> Option<i64> {
    (value.fract() == 0.0 && *value >= i64::MIN as f32 && *value < -(i64::MIN as f32))
        .then_some(*value as i64)
}

pub(super) fn cmp(lhs: &BackingFloat, rhs: &BackingFloat) -> Option<Ordering> {
    lhs.partial_cmp(rhs)
}
