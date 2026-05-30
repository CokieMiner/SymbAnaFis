use crate::number::logic::int_math::IntType;
use core::cmp::Ordering;
use rug::Rational;
extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};

pub(super) type BackingRational = Rational;

pub(super) fn from_integer(value: IntType) -> BackingRational {
    Rational::from(value)
}

pub(super) fn new(num: IntType, den: IntType) -> BackingRational {
    Rational::from((num, den))
}

pub(super) fn numer(value: &BackingRational) -> IntType {
    value.numer().clone()
}

pub(super) fn denom(value: &BackingRational) -> IntType {
    value.denom().clone()
}

pub(super) fn add(lhs: &BackingRational, rhs: &BackingRational) -> BackingRational {
    Rational::from(lhs + rhs)
}

pub(super) fn sub(lhs: &BackingRational, rhs: &BackingRational) -> BackingRational {
    Rational::from(lhs - rhs)
}

pub(super) fn mul(lhs: &BackingRational, rhs: &BackingRational) -> BackingRational {
    Rational::from(lhs * rhs)
}

pub(super) fn div(lhs: &BackingRational, rhs: &BackingRational) -> BackingRational {
    Rational::from(lhs / rhs)
}

pub(super) fn neg(value: &BackingRational) -> BackingRational {
    Rational::from(-value)
}

pub(super) fn cmp(lhs: &BackingRational, rhs: &BackingRational) -> Ordering {
    lhs.cmp(rhs)
}

pub(super) const fn is_integer(value: &BackingRational) -> bool {
    value.is_integer()
}

pub(super) fn to_integer(value: &BackingRational) -> IntType {
    IntType::from(value.numer() / value.denom())
}

pub(super) fn to_string(value: &BackingRational) -> String {
    if is_integer(value) {
        value.numer().to_string()
    } else {
        format!("{}/{}", value.numer(), value.denom())
    }
}
