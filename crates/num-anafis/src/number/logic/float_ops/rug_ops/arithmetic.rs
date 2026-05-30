use super::{BackingFloat, with_val};
use core::cmp::Ordering;
// ============================================================================
// Arithmetic — binary ops need with_val for precision control
// ============================================================================

#[inline]
pub fn add(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    with_val(lhs + rhs)
}
#[inline]
pub fn sub(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    with_val(lhs - rhs)
}
#[inline]
pub fn mul(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    with_val(lhs * rhs)
}
#[inline]
pub fn div(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    with_val(lhs / rhs)
}
#[inline]
pub fn neg(value: &BackingFloat) -> BackingFloat {
    with_val(-value)
}

// ============================================================================
// Comparison & properties
// ============================================================================

#[inline]
pub fn cmp(lhs: &BackingFloat, rhs: &BackingFloat) -> Option<Ordering> {
    lhs.partial_cmp(rhs)
}
#[inline]
pub const fn is_zero(value: &BackingFloat) -> bool {
    value.is_zero()
}
#[inline]
pub fn is_one(value: &BackingFloat) -> bool {
    *value == 1.0
}
#[inline]
pub fn is_neg_one(value: &BackingFloat) -> bool {
    *value == -1.0
}
#[inline]
pub fn is_integer(value: &BackingFloat) -> bool {
    value.is_integer()
}
#[inline]
pub const fn is_finite(value: &BackingFloat) -> bool {
    value.is_finite()
}
#[inline]
pub const fn is_negative(value: &BackingFloat) -> bool {
    value.is_sign_negative() && !value.is_zero()
}
#[inline]
pub const fn is_positive(value: &BackingFloat) -> bool {
    value.is_sign_positive() && !value.is_zero()
}
#[inline]
pub const fn is_nan(value: &BackingFloat) -> bool {
    value.is_nan()
}
