use crate::number::logic::int_math::{IntRepr, int_to_string};
use core::cmp::Ordering;

use rug::{
    Float,
    float::{prec_max, prec_min},
    ops::Pow,
};

pub(super) type BackingFloat = Float;

const DEFAULT_PRECISION_BITS: usize = 256;

#[allow(
    clippy::needless_pass_by_value,
    reason = "Maintains consistent signature with other backends that require references"
)]
pub(super) fn from_int(value: IntRepr) -> BackingFloat {
    let s = int_to_string(&value);
    Float::parse(&s).ok().map_or_else(
        || BackingFloat::with_val(precision_u32(), f64::NAN),
        |parsed| BackingFloat::with_val(precision_u32(), parsed),
    )
}

pub(super) fn clone(value: &BackingFloat) -> BackingFloat {
    value.clone()
}

const fn min_precision_bits() -> usize {
    prec_min() as usize
}

const fn max_precision_bits() -> usize {
    prec_max() as usize
}

fn clamp_precision(bits: usize) -> usize {
    bits.clamp(min_precision_bits(), max_precision_bits())
}

fn precision_u32() -> u32 {
    u32::try_from(clamp_precision(DEFAULT_PRECISION_BITS)).unwrap_or_else(|_| prec_max())
}

fn from_i32(value: i32) -> BackingFloat {
    BackingFloat::with_val(precision_u32(), value)
}

pub(super) fn add(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    BackingFloat::with_val(precision_u32(), lhs + rhs)
}

pub(super) fn sub(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    BackingFloat::with_val(precision_u32(), lhs - rhs)
}

pub(super) fn mul(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    BackingFloat::with_val(precision_u32(), lhs * rhs)
}

pub(super) fn div(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    BackingFloat::with_val(precision_u32(), lhs / rhs)
}

pub(super) fn neg(value: &BackingFloat) -> BackingFloat {
    BackingFloat::with_val(precision_u32(), -value)
}

pub(super) fn abs(value: &BackingFloat) -> BackingFloat {
    BackingFloat::with_val(precision_u32(), value.clone().abs())
}

pub(super) fn pow(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    BackingFloat::with_val(precision_u32(), lhs.pow(rhs))
}

pub(super) fn sqrt(value: &BackingFloat) -> BackingFloat {
    BackingFloat::with_val(precision_u32(), value.clone().sqrt())
}

pub(super) fn cbrt(value: &BackingFloat) -> BackingFloat {
    BackingFloat::with_val(precision_u32(), value.cbrt_ref())
}

pub(super) fn fract(value: &BackingFloat) -> BackingFloat {
    BackingFloat::with_val(precision_u32(), value.clone().fract())
}

pub(super) fn round(value: &BackingFloat) -> BackingFloat {
    BackingFloat::with_val(precision_u32(), value.round_ref())
}

pub(super) fn sin(value: &BackingFloat) -> BackingFloat {
    BackingFloat::with_val(precision_u32(), value.sin_ref())
}

pub(super) fn cos(value: &BackingFloat) -> BackingFloat {
    BackingFloat::with_val(precision_u32(), value.cos_ref())
}

pub(super) fn exp(value: &BackingFloat) -> BackingFloat {
    BackingFloat::with_val(precision_u32(), value.exp_ref())
}

pub(super) fn ln(value: &BackingFloat) -> BackingFloat {
    BackingFloat::with_val(precision_u32(), value.ln_ref())
}

pub(super) fn atan(value: &BackingFloat) -> BackingFloat {
    BackingFloat::with_val(precision_u32(), value.atan_ref())
}

pub(super) const fn is_zero(value: &BackingFloat) -> bool {
    value.is_zero()
}

pub(super) fn is_one(value: &BackingFloat) -> bool {
    value == &from_i32(1)
}

pub(super) fn is_neg_one(value: &BackingFloat) -> bool {
    value == &from_i32(-1)
}

pub(super) fn is_integer(value: &BackingFloat) -> bool {
    value.is_integer()
}

pub(super) const fn is_finite(value: &BackingFloat) -> bool {
    value.is_finite()
}

pub(super) const fn is_negative(value: &BackingFloat) -> bool {
    value.is_sign_negative() && !value.is_zero()
}

pub(super) const fn is_positive(value: &BackingFloat) -> bool {
    value.is_sign_positive() && !value.is_zero()
}

pub(super) fn to_f64(value: &BackingFloat) -> f64 {
    value.to_f64()
}

pub(super) fn from_f64(value: f64) -> BackingFloat {
    BackingFloat::with_val(precision_u32(), value)
}

pub(super) fn from_i64(value: i64) -> BackingFloat {
    BackingFloat::with_val(precision_u32(), value)
}

pub(super) fn to_i64_exact(value: &BackingFloat) -> Option<i64> {
    if !value.is_finite() || !value.is_integer() {
        return None;
    }

    let integer = value.to_integer()?;
    integer.to_string().parse::<i64>().ok()
}

pub(super) fn cmp(lhs: &BackingFloat, rhs: &BackingFloat) -> Option<Ordering> {
    lhs.partial_cmp(rhs)
}
