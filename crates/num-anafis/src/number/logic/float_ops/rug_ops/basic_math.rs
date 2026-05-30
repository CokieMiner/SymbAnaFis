use super::{BackingFloat, with_val};
// ============================================================================
// Unary math — result already at correct precision, no wrapper needed
// ============================================================================

#[inline]
pub fn abs(value: &BackingFloat) -> BackingFloat {
    value.clone().abs()
}
#[inline]
pub fn signum(value: &BackingFloat) -> BackingFloat {
    value.clone().signum()
}
#[inline]
pub fn floor(value: &BackingFloat) -> BackingFloat {
    value.clone().floor()
}
#[inline]
pub fn ceil(value: &BackingFloat) -> BackingFloat {
    value.clone().ceil()
}
#[inline]
pub fn round(value: &BackingFloat) -> BackingFloat {
    value.clone().round()
}
#[inline]
pub fn fract(value: &BackingFloat) -> BackingFloat {
    value.clone().fract()
}
#[inline]
pub fn sqrt(value: &BackingFloat) -> BackingFloat {
    value.clone().sqrt()
}
#[inline]
pub fn cbrt(value: &BackingFloat) -> BackingFloat {
    value.clone().cbrt()
}
#[inline]
pub fn sin(value: &BackingFloat) -> BackingFloat {
    value.clone().sin()
}
#[inline]
pub fn cos(value: &BackingFloat) -> BackingFloat {
    value.clone().cos()
}
#[inline]
pub fn tan(value: &BackingFloat) -> BackingFloat {
    value.clone().tan()
}
#[inline]
pub fn asin(value: &BackingFloat) -> BackingFloat {
    value.clone().asin()
}
#[inline]
pub fn acos(value: &BackingFloat) -> BackingFloat {
    value.clone().acos()
}
#[inline]
pub fn atan(value: &BackingFloat) -> BackingFloat {
    value.clone().atan()
}
#[inline]
pub fn sinh(value: &BackingFloat) -> BackingFloat {
    value.clone().sinh()
}
#[inline]
pub fn cosh(value: &BackingFloat) -> BackingFloat {
    value.clone().cosh()
}
#[inline]
pub fn tanh(value: &BackingFloat) -> BackingFloat {
    value.clone().tanh()
}
#[inline]
pub fn asinh(value: &BackingFloat) -> BackingFloat {
    value.clone().asinh()
}
#[inline]
pub fn acosh(value: &BackingFloat) -> BackingFloat {
    value.clone().acosh()
}
#[inline]
pub fn atanh(value: &BackingFloat) -> BackingFloat {
    value.clone().atanh()
}
#[inline]
pub fn exp(value: &BackingFloat) -> BackingFloat {
    value.clone().exp()
}
#[inline]
pub fn expm1(value: &BackingFloat) -> BackingFloat {
    value.clone().exp_m1()
}
#[inline]
pub fn ln(value: &BackingFloat) -> BackingFloat {
    value.clone().ln()
}
#[inline]
pub fn log1p(value: &BackingFloat) -> BackingFloat {
    value.clone().ln_1p()
}
#[inline]
pub fn atan2(y: &BackingFloat, x: &BackingFloat) -> BackingFloat {
    with_val(y.clone().atan2(x))
}
#[inline]
pub fn pow(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    with_val(rug::ops::Pow::pow(lhs.clone(), rhs))
}
