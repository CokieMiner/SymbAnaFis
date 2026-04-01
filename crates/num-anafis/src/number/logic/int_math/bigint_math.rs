use core::cmp::Ordering;

use num_bigint::{BigInt, Sign};
use num_traits::{One, Signed, ToPrimitive, Zero};

pub(super) type BackingInt = BigInt;

pub(super) fn zero() -> BackingInt {
    BigInt::zero()
}

pub(super) fn clone(value: &BackingInt) -> BackingInt {
    value.clone()
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "Maintain consistent API with overflow-prone backends (i64, i32)"
)]
pub(super) fn from_i64_exact(value: i64) -> Option<BackingInt> {
    Some(BigInt::from(value))
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "Maintain consistent API with overflow-prone backends (i64, i32)"
)]
pub(super) fn from_u64_exact(value: u64) -> Option<BackingInt> {
    Some(BigInt::from(value))
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "Maintain consistent API with overflow-prone backends (i64, i32)"
)]
pub(super) fn from_i128_exact(value: i128) -> Option<BackingInt> {
    Some(BigInt::from(value))
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "Maintain consistent API with overflow-prone backends (i64, i32)"
)]
pub(super) fn from_u128_exact(value: u128) -> Option<BackingInt> {
    Some(BigInt::from(value))
}

pub(super) fn to_i64_exact(value: &BackingInt) -> Option<i64> {
    value.to_i64()
}

pub(super) fn to_f64_lossy(value: &BackingInt) -> f64 {
    value.to_f64().unwrap_or_else(|| {
        if value.sign() == Sign::Minus {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        }
    })
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "Maintain consistent API with overflow-prone backends (i64, i32)"
)]
pub(super) fn checked_abs(value: &BackingInt) -> Option<BackingInt> {
    Some(value.abs())
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "Maintain consistent API with overflow-prone backends (i64, i32)"
)]
pub(super) fn checked_neg(value: &BackingInt) -> Option<BackingInt> {
    Some(-value)
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "Maintain consistent API with overflow-prone backends (i64, i32)"
)]
pub(super) fn checked_add(lhs: &BackingInt, rhs: &BackingInt) -> Option<BackingInt> {
    Some(lhs + rhs)
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "Maintain consistent API with overflow-prone backends (i64, i32)"
)]
pub(super) fn checked_mul(lhs: &BackingInt, rhs: &BackingInt) -> Option<BackingInt> {
    Some(lhs * rhs)
}

pub(super) fn cmp(lhs: &BackingInt, rhs: &BackingInt) -> Ordering {
    lhs.cmp(rhs)
}

pub(super) fn is_zero(value: &BackingInt) -> bool {
    value.is_zero()
}

pub(super) fn is_one(value: &BackingInt) -> bool {
    value.is_one()
}

pub(super) fn is_neg_one(value: &BackingInt) -> bool {
    value == &-BigInt::one()
}

pub(super) fn is_negative(value: &BackingInt) -> bool {
    value.is_negative()
}

pub(super) fn is_positive(value: &BackingInt) -> bool {
    value.is_positive()
}

pub(super) fn is_even(value: &BackingInt) -> bool {
    (value % BigInt::from(2_u8)).is_zero()
}

pub(super) fn modulo(lhs: &BackingInt, rhs: &BackingInt) -> BackingInt {
    lhs % rhs
}

pub(super) fn div_exact(lhs: &BackingInt, rhs: &BackingInt) -> BackingInt {
    lhs / rhs
}

pub(super) fn gcd(lhs: &BackingInt, rhs: &BackingInt) -> BackingInt {
    let mut a = lhs.abs();
    let mut b = rhs.abs();

    while !b.is_zero() {
        let r = &a % &b;
        a = b;
        b = r;
    }

    if a.is_zero() { BigInt::one() } else { a }
}

fn perfect_root_non_negative(value: &BackingInt, degree: u32) -> Option<BackingInt> {
    debug_assert!(!value.is_negative());

    if value.is_zero() {
        return Some(BigInt::zero());
    }

    let one = BigInt::one();
    let bits = value.bits();
    let degree_u64 = u64::from(degree);
    #[allow(
        clippy::integer_division,
        reason = "Integer division for bit estimation is intentional"
    )]
    let approx_bits = ((bits.saturating_sub(1)) / degree_u64).saturating_add(1);
    let mut low = BigInt::zero();
    let mut high = &one << usize::try_from(approx_bits).ok()?;

    while low <= high {
        let mid = (&low + &high) >> 1_u32;
        let mid_pow = mid.pow(degree);

        match mid_pow.cmp(value) {
            Ordering::Equal => return Some(mid),
            Ordering::Less => low = mid + &one,
            Ordering::Greater => {
                if mid.is_zero() {
                    break;
                }
                high = mid - &one;
            }
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

pub(super) fn perfect_square(value: &BackingInt) -> Option<BackingInt> {
    if value.is_negative() {
        return None;
    }

    perfect_root_non_negative(value, 2)
}

pub(super) fn perfect_cube(value: &BackingInt) -> Option<BackingInt> {
    if value.is_negative() {
        let root = perfect_root_non_negative(&value.abs(), 3)?;
        return Some(-root);
    }

    perfect_root_non_negative(value, 3)
}
