use crate::number::logic::int_math::{IntRepr, int_to_string};
use core::cmp::Ordering;

use astro_float::{BigFloat, Consts, Radix, RoundingMode, Sign};

pub(super) type BackingFloat = BigFloat;

const DEFAULT_PRECISION_BITS: usize = 256;
const MIN_PRECISION_BITS: usize = 2;

#[allow(
    clippy::needless_pass_by_value,
    reason = "Maintains consistent signature with other backends that require references"
)]
pub(super) fn from_int(value: IntRepr) -> BackingFloat {
    let s = int_to_string(&value);
    let Ok(mut consts) = Consts::new() else {
        return BigFloat::nan(None);
    };
    BigFloat::parse(&s, Radix::Dec, precision(), rounding_mode(), &mut consts)
}

pub(super) fn clone(value: &BackingFloat) -> BackingFloat {
    value.clone()
}

const fn rounding_mode() -> RoundingMode {
    RoundingMode::ToEven
}

fn precision() -> usize {
    DEFAULT_PRECISION_BITS.max(MIN_PRECISION_BITS)
}

fn finite_to_f64(value: &BackingFloat) -> f64 {
    value.to_string().parse::<f64>().unwrap_or_else(|_| {
        if value.is_zero() {
            0.0
        } else if value.is_negative() {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        }
    })
}

pub(super) fn add(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    lhs.add(rhs, precision(), rounding_mode())
}

pub(super) fn sub(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    lhs.sub(rhs, precision(), rounding_mode())
}

pub(super) fn mul(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    lhs.mul(rhs, precision(), rounding_mode())
}

pub(super) fn div(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    lhs.div(rhs, precision(), rounding_mode())
}

pub(super) fn neg(value: &BackingFloat) -> BackingFloat {
    value.neg()
}

pub(super) fn abs(value: &BackingFloat) -> BackingFloat {
    value.abs()
}

pub(super) fn pow(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    Consts::new().ok().map_or_else(
        || from_f64(to_f64(lhs).powf(to_f64(rhs))),
        |mut constants| lhs.pow(rhs, precision(), rounding_mode(), &mut constants),
    )
}

pub(super) fn sqrt(value: &BackingFloat) -> BackingFloat {
    value.sqrt(precision(), rounding_mode())
}

pub(super) fn cbrt(value: &BackingFloat) -> BackingFloat {
    value.cbrt(precision(), rounding_mode())
}

pub(super) fn fract(value: &BackingFloat) -> BackingFloat {
    value.fract()
}

pub(super) fn round(value: &BackingFloat) -> BackingFloat {
    value.round(precision(), rounding_mode())
}

pub(super) fn sin(value: &BackingFloat) -> BackingFloat {
    Consts::new().ok().map_or_else(
        || from_f64(to_f64(value).sin()),
        |mut consts| value.sin(precision(), rounding_mode(), &mut consts),
    )
}

pub(super) fn cos(value: &BackingFloat) -> BackingFloat {
    Consts::new().ok().map_or_else(
        || from_f64(to_f64(value).cos()),
        |mut consts| value.cos(precision(), rounding_mode(), &mut consts),
    )
}

pub(super) fn exp(value: &BackingFloat) -> BackingFloat {
    Consts::new().ok().map_or_else(
        || from_f64(to_f64(value).exp()),
        |mut consts| value.exp(precision(), rounding_mode(), &mut consts),
    )
}

pub(super) fn ln(value: &BackingFloat) -> BackingFloat {
    Consts::new().ok().map_or_else(
        || from_f64(to_f64(value).ln()),
        |mut consts| value.ln(precision(), rounding_mode(), &mut consts),
    )
}

pub(super) fn atan(value: &BackingFloat) -> BackingFloat {
    Consts::new().ok().map_or_else(
        || from_f64(to_f64(value).atan()),
        |mut consts| value.atan(precision(), rounding_mode(), &mut consts),
    )
}

pub(super) fn is_zero(value: &BackingFloat) -> bool {
    value.is_zero()
}

pub(super) fn is_one(value: &BackingFloat) -> bool {
    cmp(value, &from_f64(1.0)).is_some_and(|ordering| ordering == Ordering::Equal)
}

pub(super) fn is_neg_one(value: &BackingFloat) -> bool {
    cmp(value, &from_f64(-1.0)).is_some_and(|ordering| ordering == Ordering::Equal)
}

pub(super) fn is_integer(value: &BackingFloat) -> bool {
    value.is_int()
}

pub(super) fn is_finite(value: &BackingFloat) -> bool {
    !value.is_nan() && !value.is_inf()
}

pub(super) fn is_negative(value: &BackingFloat) -> bool {
    value.is_negative()
}

pub(super) fn is_positive(value: &BackingFloat) -> bool {
    value.is_positive()
}

pub(super) fn to_f64(value: &BackingFloat) -> f64 {
    if value.is_nan() {
        f64::NAN
    } else if value.is_inf_pos() {
        f64::INFINITY
    } else if value.is_inf_neg() {
        f64::NEG_INFINITY
    } else {
        finite_to_f64(value)
    }
}

pub(super) fn from_f64(value: f64) -> BackingFloat {
    BackingFloat::from_f64(value, precision())
}

pub(super) fn from_i64(value: i64) -> BackingFloat {
    let Ok(mut consts) = Consts::new() else {
        return value
            .to_string()
            .parse::<f64>()
            .map_or_else(|_| BigFloat::nan(None), from_f64);
    };
    BigFloat::parse(
        &value.to_string(),
        Radix::Dec,
        precision(),
        rounding_mode(),
        &mut consts,
    )
}

pub(super) fn to_i64_exact(value: &BackingFloat) -> Option<i64> {
    if !is_finite(value) || !value.is_int() {
        return None;
    }

    let mut constants = Consts::new().ok()?;
    let (sign, digits, exponent) = value
        .convert_to_radix(Radix::Dec, RoundingMode::None, &mut constants)
        .ok()?;

    if digits.is_empty() {
        return Some(0);
    }

    let digits_len = i64::try_from(digits.len()).ok()?;
    let shift = i64::from(exponent) - digits_len;

    let mut acc = 0_i64;
    let significant_end = if shift < 0 {
        let trailing_digits = usize::try_from(-shift).ok()?;
        if trailing_digits > digits.len() {
            return None;
        }
        let end = digits.len() - trailing_digits;
        if digits[end..].iter().any(|digit| *digit != 0) {
            return None;
        }
        end
    } else {
        digits.len()
    };

    for digit in &digits[..significant_end] {
        acc = acc.checked_mul(10)?;
        acc = acc.checked_add(i64::from(*digit))?;
    }

    if shift > 0 {
        let zeros = usize::try_from(shift).ok()?;
        for _ in 0..zeros {
            acc = acc.checked_mul(10)?;
        }
    }

    match sign {
        Sign::Pos => Some(acc),
        Sign::Neg => acc.checked_neg(),
    }
}

pub(super) fn cmp(lhs: &BackingFloat, rhs: &BackingFloat) -> Option<Ordering> {
    lhs.cmp(rhs).map(|ordering| match ordering.cmp(&0) {
        Ordering::Less => Ordering::Less,
        Ordering::Greater => Ordering::Greater,
        Ordering::Equal => Ordering::Equal,
    })
}
