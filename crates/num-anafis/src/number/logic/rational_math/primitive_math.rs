#![allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "&T API required for non-Copy backend uniformity"
)]
use crate::number::logic::int_math::IntType;
use core::cmp::Ordering;
use num_rational::Ratio;
extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};

pub(super) type BackingRational = Ratio<IntType>;

#[inline]
pub(super) fn from_integer(value: IntType) -> BackingRational {
    Ratio::from_integer(value)
}

#[inline]
pub(super) fn new(num: IntType, den: IntType) -> BackingRational {
    Ratio::new(num, den)
}

#[inline]
pub(super) const fn numer(value: &BackingRational) -> IntType {
    *value.numer()
}

#[inline]
pub(super) const fn denom(value: &BackingRational) -> IntType {
    *value.denom()
}

#[inline]
pub(super) fn add(lhs: &BackingRational, rhs: &BackingRational) -> BackingRational {
    lhs + rhs
}

#[inline]
pub(super) fn sub(lhs: &BackingRational, rhs: &BackingRational) -> BackingRational {
    lhs - rhs
}

#[inline]
pub(super) fn mul(lhs: &BackingRational, rhs: &BackingRational) -> BackingRational {
    lhs * rhs
}

#[inline]
pub(super) fn div(lhs: &BackingRational, rhs: &BackingRational) -> BackingRational {
    lhs / rhs
}

#[inline]
pub(super) fn neg(value: &BackingRational) -> BackingRational {
    -*value
}

#[inline]
pub(super) fn cmp(lhs: &BackingRational, rhs: &BackingRational) -> Ordering {
    lhs.cmp(rhs)
}

#[inline]
pub(super) fn is_integer(value: &BackingRational) -> bool {
    value.is_integer()
}

#[inline]
pub(super) fn to_integer(value: &BackingRational) -> IntType {
    value.to_integer()
}

#[inline]
pub(super) fn to_string(value: &BackingRational) -> String {
    if value.is_integer() {
        value.numer().to_string()
    } else {
        format!("{}/{}", value.numer(), value.denom())
    }
}
