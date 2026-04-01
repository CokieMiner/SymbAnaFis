#![allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "Maintain abstraction for non-Copy high-precision types"
)]
#![allow(clippy::option_if_let_else, reason = "if let is more readable here")]
#![allow(
    clippy::same_name_method,
    reason = "Trait and inherent methods deliberately share the same name for ergonomics"
)]
use super::float_ops::{
    FloatRepr, float_abs, float_add, float_atan, float_cbrt, float_clone, float_cmp, float_cos,
    float_div, float_exp, float_fract, float_from_f64, float_from_i64, float_from_int,
    float_is_finite, float_is_integer, float_is_neg_one, float_is_negative, float_is_one,
    float_is_positive, float_is_zero, float_ln, float_mul, float_neg, float_pow, float_round,
    float_sin, float_sqrt, float_sub, float_to_f64, float_to_i64_exact,
};
use super::int_math::{
    IntRepr, int_checked_abs, int_checked_add, int_checked_mul, int_checked_neg, int_clone,
    int_cmp, int_div_exact, int_from_i64_exact, int_from_i128_exact, int_from_u64_exact,
    int_from_u128_exact, int_gcd, int_is_even, int_is_neg_one, int_is_negative, int_is_one,
    int_is_positive, int_is_zero, int_mod, int_perfect_cube, int_perfect_square, int_to_f64_lossy,
    int_to_i64_exact, int_zero,
};
use core::cmp::Ordering;
use core::fmt::{Debug, Display, Formatter, Result as FmtResult};
use core::hash::{Hash, Hasher};
use core::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Debug, Clone)]
enum NumberRepr {
    Int(IntRepr),
    Rational { num: IntRepr, den: IntRepr },
    Float(FloatRepr),
}

/// Core scalar number used by symbolic expressions.
#[derive(Clone)]
pub struct Number(NumberRepr);

impl Number {
    fn from_float_repr(value: FloatRepr) -> Self {
        if !float_is_finite(&value) {
            return Self(NumberRepr::Float(value));
        }

        if let Some(i64_value) = float_to_i64_exact(&value)
            && let Some(int_value) = int_from_i64_exact(i64_value)
        {
            return Self(NumberRepr::Int(int_value));
        }

        Self(NumberRepr::Float(value))
    }

    #[cfg(test)]
    pub(in crate::number::logic) const fn from_backend_float_unchecked(value: FloatRepr) -> Self {
        Self(NumberRepr::Float(value))
    }

    fn int_to_float_repr(value: &IntRepr) -> FloatRepr {
        float_from_int(value)
    }

    fn to_float_repr(&self) -> FloatRepr {
        match &self.0 {
            NumberRepr::Int(value) => Self::int_to_float_repr(value),
            NumberRepr::Rational { num, den } => {
                float_div(&Self::int_to_float_repr(num), &Self::int_to_float_repr(den))
            }
            NumberRepr::Float(value) => float_clone(value),
        }
    }

    fn from_i64_internal(value: i64) -> Self {
        if let Some(int_value) = int_from_i64_exact(value) {
            Self(NumberRepr::Int(int_value))
        } else {
            Self::from_float_repr(float_from_i64(value))
        }
    }

    /// Construct an integer value.
    #[must_use]
    pub fn int(value: i64) -> Self {
        Self::from_i64_internal(value)
    }

    fn int_eq_i64(value: &IntRepr, expected: i64) -> bool {
        int_from_i64_exact(expected)
            .is_some_and(|target| int_cmp(value, &target) == Ordering::Equal)
    }

    fn rational_from_int(num: IntRepr, den: IntRepr) -> Self {
        if int_is_zero(&den) {
            return Self::from_float_repr(float_from_f64(f64::NAN));
        }

        let mut num = num;
        let mut den = den;

        if int_is_negative(&den) {
            let Some(next_num) = int_checked_neg(&num) else {
                return Self::from_float_repr(float_div(
                    &float_from_int(&num),
                    &float_from_int(&den),
                ));
            };
            let Some(next_den) = int_checked_neg(&den) else {
                return Self::from_float_repr(float_div(
                    &float_from_int(&num),
                    &float_from_int(&den),
                ));
            };
            num = next_num;
            den = next_den;
        }

        let gcd = int_gcd(&num, &den);
        let num = int_div_exact(&num, &gcd);
        let den = int_div_exact(&den, &gcd);

        if int_is_one(&den) {
            Self(NumberRepr::Int(num))
        } else {
            Self(NumberRepr::Rational { num, den })
        }
    }

    /// Construct a normalized rational number from `i64` inputs.
    #[must_use]
    pub fn rational(num: i64, den: i64) -> Self {
        match (int_from_i64_exact(num), int_from_i64_exact(den)) {
            (Some(num), Some(den)) => Self::rational_from_int(num, den),
            _ if den == 0 => Self::from_float_repr(float_from_f64(f64::NAN)),
            _ => {
                let n = float_from_i64(num);
                let d = float_from_i64(den);
                Self::from_float_repr(float_div(&n, &d))
            }
        }
    }

    /// Construct a floating value.
    #[must_use]
    pub fn float(value: f64) -> Self {
        Self::from_float_repr(float_from_f64(value))
    }

    /// Return a lossy `f64` approximation.
    #[must_use]
    pub fn to_f64_lossy(&self) -> f64 {
        match &self.0 {
            NumberRepr::Int(value) => int_to_f64_lossy(value),
            NumberRepr::Rational { num, den } => int_to_f64_lossy(num) / int_to_f64_lossy(den),
            NumberRepr::Float(value) => float_to_f64(value),
        }
    }

    /// Return the absolute value, preserving exact representation when possible.
    #[must_use]
    pub fn abs(&self) -> Self {
        match &self.0 {
            NumberRepr::Int(value) => {
                if let Some(abs) = int_checked_abs(value) {
                    Self(NumberRepr::Int(abs))
                } else {
                    Self::from_float_repr(float_abs(&self.to_float_repr()))
                }
            }
            NumberRepr::Rational { num, den } => {
                if let Some(abs) = int_checked_abs(num) {
                    Self(NumberRepr::Rational {
                        num: abs,
                        den: int_clone(den),
                    })
                } else {
                    Self::from_float_repr(float_abs(&self.to_float_repr()))
                }
            }
            NumberRepr::Float(value) => Self::from_float_repr(float_abs(value)),
        }
    }

    /// Return the fractional component.
    #[must_use]
    pub fn fract(&self) -> Self {
        match &self.0 {
            NumberRepr::Int(_) => Self(NumberRepr::Int(int_zero())),
            NumberRepr::Rational { num, den } => {
                Self::rational_from_int(int_mod(num, den), int_clone(den))
            }
            NumberRepr::Float(value) => Self::from_float_repr(float_fract(value)),
        }
    }

    /// Check whether the value is mathematically an integer.
    #[must_use]
    pub fn is_integer(&self) -> bool {
        match &self.0 {
            NumberRepr::Int(_) => true,
            NumberRepr::Rational { den, .. } => int_is_one(den),
            NumberRepr::Float(value) => float_is_integer(value),
        }
    }

    /// Return an exact `i64` if representable.
    #[must_use]
    pub fn to_i64_exact(&self) -> Option<i64> {
        match &self.0 {
            NumberRepr::Int(value) => int_to_i64_exact(value),
            NumberRepr::Rational { num, den } if int_is_one(den) => int_to_i64_exact(num),
            NumberRepr::Float(value) => float_to_i64_exact(value),
            NumberRepr::Rational { .. } => None,
        }
    }

    /// Return an exact `u32` if representable.
    #[must_use]
    pub fn to_u32_exact(&self) -> Option<u32> {
        self.to_i64_exact()
            .and_then(|value| u32::try_from(value).ok())
    }

    /// Check whether this value is numerically zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        match &self.0 {
            NumberRepr::Int(value) => int_is_zero(value),
            NumberRepr::Rational { num, .. } => int_is_zero(num),
            NumberRepr::Float(value) => float_is_zero(value),
        }
    }

    /// Check whether this value is numerically one.
    #[must_use]
    pub fn is_one(&self) -> bool {
        match &self.0 {
            NumberRepr::Int(value) => int_is_one(value),
            NumberRepr::Rational { num, den } => int_is_one(num) && int_is_one(den),
            NumberRepr::Float(value) => float_is_one(value),
        }
    }

    /// Check whether this value is numerically negative one.
    #[must_use]
    pub fn is_neg_one(&self) -> bool {
        match &self.0 {
            NumberRepr::Int(value) => int_is_neg_one(value),
            NumberRepr::Rational { num, den } => int_is_neg_one(num) && int_is_one(den),
            NumberRepr::Float(value) => float_is_neg_one(value),
        }
    }

    /// Check whether this value is negative.
    #[must_use]
    pub fn is_negative(&self) -> bool {
        match &self.0 {
            NumberRepr::Int(value) => int_is_negative(value),
            NumberRepr::Rational { num, .. } => int_is_negative(num),
            NumberRepr::Float(value) => float_is_negative(value),
        }
    }

    /// Check whether this value is positive.
    #[must_use]
    pub fn is_positive(&self) -> bool {
        match &self.0 {
            NumberRepr::Int(value) => int_is_positive(value),
            NumberRepr::Rational { num, .. } => int_is_positive(num),
            NumberRepr::Float(value) => float_is_positive(value),
        }
    }

    /// Check exact integer equality.
    #[must_use]
    pub fn is_exact_i64(&self, value: i64) -> bool {
        let Some(expected) = int_from_i64_exact(value) else {
            return false;
        };

        match &self.0 {
            NumberRepr::Int(current) => int_cmp(current, &expected) == Ordering::Equal,
            NumberRepr::Rational { num, den } => {
                int_cmp(num, &expected) == Ordering::Equal && int_is_one(den)
            }
            NumberRepr::Float(_) => false,
        }
    }

    /// Check exact rational equality.
    #[must_use]
    pub fn is_exact_rational(&self, num: i64, den: i64) -> bool {
        match (int_from_i64_exact(num), int_from_i64_exact(den)) {
            (Some(num), Some(den)) => self == &Self::rational_from_int(num, den),
            _ => false,
        }
    }

    /// Check whether this value is an even integer.
    #[must_use]
    pub fn is_even_integer(&self) -> bool {
        match &self.0 {
            NumberRepr::Int(value) => int_is_even(value),
            NumberRepr::Rational { num, den } if int_is_one(den) => int_is_even(num),
            NumberRepr::Rational { .. } => false,
            NumberRepr::Float(_) => self.to_i64_exact().is_some_and(|value| value % 2 == 0),
        }
    }

    /// Check whether this value is divisible by another integer-valued number.
    #[must_use]
    pub fn is_divisible_by(&self, other: &Self) -> bool {
        let lhs = match &self.0 {
            NumberRepr::Int(value) => Some(int_clone(value)),
            NumberRepr::Rational { num, den } if int_is_one(den) => Some(int_clone(num)),
            _ => None,
        };
        let rhs = match &other.0 {
            NumberRepr::Int(value) => Some(int_clone(value)),
            NumberRepr::Rational { num, den } if int_is_one(den) => Some(int_clone(num)),
            _ => None,
        };

        match (lhs, rhs) {
            (Some(_), Some(rhs)) if int_is_zero(&rhs) => false,
            (Some(lhs), Some(rhs)) => int_is_zero(&int_mod(&lhs, &rhs)),
            _ => false,
        }
    }

    /// Approximate comparison against a plain `f64`.
    #[must_use]
    pub fn approx_eq_f64(&self, other: f64, tolerance: f64) -> bool {
        let other = float_from_f64(other);
        let tolerance = float_from_f64(tolerance);
        self.approx_eq_float_values(&other, &tolerance)
    }

    /// Approximate comparison against another [`Number`] with explicit tolerance.
    ///
    /// Returns `false` when `tolerance` is negative.
    #[must_use]
    pub fn approx_eq_number(&self, other: &Self, tolerance: &Self) -> bool {
        self.approx_eq_float_values(&other.to_float_repr(), &tolerance.to_float_repr())
    }

    /// Check floating finiteness.
    #[must_use]
    pub fn is_finite(&self) -> bool {
        match &self.0 {
            NumberRepr::Int(_) | NumberRepr::Rational { .. } => true,
            NumberRepr::Float(value) => float_is_finite(value),
        }
    }

    /// Add two numbers while preserving exactness when possible.
    #[must_use]
    pub fn add(&self, other: &Self) -> Self {
        match (&self.0, &other.0) {
            (NumberRepr::Int(a), NumberRepr::Int(b)) => int_checked_add(a, b).map_or_else(
                || Self::from_float_repr(float_add(&self.to_float_repr(), &other.to_float_repr())),
                |sum| Self(NumberRepr::Int(sum)),
            ),
            (NumberRepr::Int(a), NumberRepr::Rational { num, den })
            | (NumberRepr::Rational { num, den }, NumberRepr::Int(a)) => {
                if let Some(p) = int_checked_mul(a, den)
                    && let Some(sum) = int_checked_add(&p, num)
                {
                    Self::rational_from_int(sum, int_clone(den))
                } else {
                    Self::from_float_repr(float_add(&self.to_float_repr(), &other.to_float_repr()))
                }
            }
            (
                NumberRepr::Rational {
                    num: a_num,
                    den: a_den,
                },
                NumberRepr::Rational {
                    num: b_num,
                    den: b_den,
                },
            ) => {
                if let Some(t1) = int_checked_mul(a_num, b_den)
                    && let Some(t2) = int_checked_mul(b_num, a_den)
                    && let Some(n) = int_checked_add(&t1, &t2)
                    && let Some(d) = int_checked_mul(a_den, b_den)
                {
                    Self::rational_from_int(n, d)
                } else {
                    Self::from_float_repr(float_add(&self.to_float_repr(), &other.to_float_repr()))
                }
            }
            _ => Self::from_float_repr(float_add(&self.to_float_repr(), &other.to_float_repr())),
        }
    }

    /// Multiply two numbers while preserving exactness when possible.
    #[must_use]
    pub fn mul(&self, other: &Self) -> Self {
        match (&self.0, &other.0) {
            (NumberRepr::Int(a), NumberRepr::Int(b)) => int_checked_mul(a, b).map_or_else(
                || Self::from_float_repr(float_mul(&self.to_float_repr(), &other.to_float_repr())),
                |prod| Self(NumberRepr::Int(prod)),
            ),
            (NumberRepr::Int(a), NumberRepr::Rational { num, den })
            | (NumberRepr::Rational { num, den }, NumberRepr::Int(a)) => int_checked_mul(a, num)
                .map_or_else(
                    || {
                        Self::from_float_repr(float_mul(
                            &self.to_float_repr(),
                            &other.to_float_repr(),
                        ))
                    },
                    |n| Self::rational_from_int(n, int_clone(den)),
                ),
            (
                NumberRepr::Rational {
                    num: a_num,
                    den: a_den,
                },
                NumberRepr::Rational {
                    num: b_num,
                    den: b_den,
                },
            ) => {
                if let Some(n) = int_checked_mul(a_num, b_num)
                    && let Some(d) = int_checked_mul(a_den, b_den)
                {
                    Self::rational_from_int(n, d)
                } else {
                    Self::from_float_repr(float_mul(&self.to_float_repr(), &other.to_float_repr()))
                }
            }
            _ => Self::from_float_repr(float_mul(&self.to_float_repr(), &other.to_float_repr())),
        }
    }

    /// Divide two numbers while preserving exactness when possible.
    ///
    /// Returns `None` for division by zero. Prefer this method when callers
    /// need explicit error handling.
    #[must_use]
    pub fn div(&self, other: &Self) -> Option<Self> {
        if other.is_zero() {
            return None;
        }

        Some(match (&self.0, &other.0) {
            (NumberRepr::Int(a), NumberRepr::Int(b)) => {
                Self::rational_from_int(int_clone(a), int_clone(b))
            }
            (NumberRepr::Int(a), NumberRepr::Rational { num, den }) => int_checked_mul(a, den)
                .map_or_else(
                    || {
                        Self::from_float_repr(float_div(
                            &self.to_float_repr(),
                            &other.to_float_repr(),
                        ))
                    },
                    |n| Self::rational_from_int(n, int_clone(num)),
                ),
            (NumberRepr::Rational { num, den }, NumberRepr::Int(b)) => int_checked_mul(den, b)
                .map_or_else(
                    || {
                        Self::from_float_repr(float_div(
                            &self.to_float_repr(),
                            &other.to_float_repr(),
                        ))
                    },
                    |d| Self::rational_from_int(int_clone(num), d),
                ),
            (
                NumberRepr::Rational {
                    num: a_num,
                    den: a_den,
                },
                NumberRepr::Rational {
                    num: b_num,
                    den: b_den,
                },
            ) => {
                if let Some(n) = int_checked_mul(a_num, b_den)
                    && let Some(d) = int_checked_mul(a_den, b_num)
                {
                    Self::rational_from_int(n, d)
                } else {
                    Self::from_float_repr(float_div(&self.to_float_repr(), &other.to_float_repr()))
                }
            }
            _ => Self::from_float_repr(float_div(&self.to_float_repr(), &other.to_float_repr())),
        })
    }

    /// Raise to an integer power with exponentiation by squaring.
    #[must_use]
    pub fn pow_i64(&self, exp: i64) -> Option<Self> {
        if exp == 0 {
            return Some(Self::from(1_i64));
        }

        if exp < 0 {
            let positive = self.pow_i64(exp.checked_neg()?)?;
            return Self::div(&Self::from(1_i64), &positive);
        }

        let mut base = self.clone();
        let mut exp = exp;
        let mut result = Self::from(1_i64);

        while exp > 0 {
            if exp % 2 == 1 {
                result = Self::mul(&result, &base);
            }
            if exp > 1 {
                base = Self::mul(&base, &base);
            }
            exp /= 2;
        }

        Some(result)
    }

    /// Return the exact square root for perfect squares.
    #[must_use]
    pub fn perfect_square_root(&self) -> Option<Self> {
        match &self.0 {
            NumberRepr::Int(value) if !int_is_negative(value) => {
                int_perfect_square(value).map(|root| Self(NumberRepr::Int(root)))
            }
            NumberRepr::Rational { num, den } if !int_is_negative(num) => {
                let n_root = int_perfect_square(num)?;
                let d_root = int_perfect_square(den)?;
                Some(Self::rational_from_int(n_root, d_root))
            }
            NumberRepr::Int(_) | NumberRepr::Rational { .. } | NumberRepr::Float(_) => None,
        }
    }

    /// Return the exact cube root for perfect cubes.
    #[must_use]
    pub fn perfect_cube_root(&self) -> Option<Self> {
        match &self.0 {
            NumberRepr::Int(value) => {
                int_perfect_cube(value).map(|root| Self(NumberRepr::Int(root)))
            }
            NumberRepr::Rational { num, den } => {
                let n_root = int_perfect_cube(num)?;
                let d_root = int_perfect_cube(den)?;
                Some(Self::rational_from_int(n_root, d_root))
            }
            NumberRepr::Float(_) => None,
        }
    }

    /// Square root with exact fast path.
    #[must_use]
    pub fn sqrt(&self) -> Self {
        self.perfect_square_root()
            .unwrap_or_else(|| Self::from_float_repr(float_sqrt(&self.to_float_repr())))
    }

    /// Cube root with exact fast path.
    #[must_use]
    pub fn cbrt(&self) -> Self {
        self.perfect_cube_root()
            .unwrap_or_else(|| Self::from_float_repr(float_cbrt(&self.to_float_repr())))
    }

    /// Round to nearest integer using backend rounding mode.
    #[must_use]
    pub fn round(&self) -> Self {
        match &self.0 {
            NumberRepr::Int(_) => self.clone(),
            NumberRepr::Rational { den, .. } if int_is_one(den) => self.clone(),
            _ => Self::from_float_repr(float_round(&self.to_float_repr())),
        }
    }

    /// Sine of this number.
    #[must_use]
    pub fn sin(&self) -> Self {
        Self::from_float_repr(float_sin(&self.to_float_repr()))
    }

    /// Cosine of this number.
    #[must_use]
    pub fn cos(&self) -> Self {
        Self::from_float_repr(float_cos(&self.to_float_repr()))
    }

    /// Exponential of this number.
    #[must_use]
    pub fn exp(&self) -> Self {
        Self::from_float_repr(float_exp(&self.to_float_repr()))
    }

    /// Natural logarithm of this number.
    #[must_use]
    pub fn ln(&self) -> Self {
        Self::from_float_repr(float_ln(&self.to_float_repr()))
    }

    /// Arctangent of this number.
    ///
    /// This operation is evaluated in the floating domain; there is no exact
    /// rational/integer fast path.
    #[must_use]
    pub fn atan(&self) -> Self {
        Self::from_float_repr(float_atan(&self.to_float_repr()))
    }

    /// Quadrant-correct arctangent of y/x with high-precision backend arithmetic.
    ///
    /// This operation is evaluated in the floating domain; there is no exact
    /// rational/integer fast path.
    #[must_use]
    pub fn atan2(&self, x: &Self) -> Self {
        let y = self;

        if x.is_positive() {
            return (y / x).atan();
        }

        let pi = Self::from(4_i64) * Self::from(1_i64).atan();

        if x.is_negative() {
            let base = (y / x).atan();
            if y.is_negative() {
                return base - pi;
            }
            return base + pi;
        }

        let half_pi = pi / Self::from(2_i64);
        if y.is_positive() {
            return half_pi;
        }
        if y.is_negative() {
            return -half_pi;
        }

        Self::float(f64::NAN)
    }

    /// Power operation with exact fast paths.
    #[must_use]
    pub fn pow(&self, exp: &Self) -> Self {
        if let Some(i) = exp.to_i64_exact()
            && let Some(exact) = self.pow_i64(i)
        {
            return exact;
        }

        if let NumberRepr::Rational { num, den } = &exp.0 {
            if int_is_one(num)
                && Self::int_eq_i64(den, 2)
                && let Some(exact_sqrt) = self.perfect_square_root()
            {
                return exact_sqrt;
            }
            if int_is_one(num)
                && Self::int_eq_i64(den, 3)
                && let Some(exact_cbrt) = self.perfect_cube_root()
            {
                return exact_cbrt;
            }
        }

        Self::from_float_repr(float_pow(&self.to_float_repr(), &exp.to_float_repr()))
    }

    /// Negate this number.
    #[must_use]
    pub fn negate(&self) -> Self {
        match &self.0 {
            NumberRepr::Int(value) => {
                if let Some(neg) = int_checked_neg(value) {
                    Self(NumberRepr::Int(neg))
                } else {
                    Self::from_float_repr(float_neg(&self.to_float_repr()))
                }
            }
            NumberRepr::Rational { num, den } => {
                if let Some(neg) = int_checked_neg(num) {
                    Self(NumberRepr::Rational {
                        num: neg,
                        den: int_clone(den),
                    })
                } else {
                    Self::from_float_repr(float_neg(&self.to_float_repr()))
                }
            }
            NumberRepr::Float(value) => Self::from_float_repr(float_neg(value)),
        }
    }

    fn approx_eq_float_values(&self, other: &FloatRepr, tolerance: &FloatRepr) -> bool {
        if float_is_negative(tolerance) {
            return false;
        }

        let diff = float_abs(&float_sub(&self.to_float_repr(), other));
        float_cmp(&diff, tolerance).is_some_and(|ordering| ordering != Ordering::Greater)
    }

    fn float_is_nan(value: &FloatRepr) -> bool {
        float_cmp(value, value).is_none()
    }

    fn compare_float_total(lhs: &FloatRepr, rhs: &FloatRepr) -> Ordering {
        match (Self::float_is_nan(lhs), Self::float_is_nan(rhs)) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => float_cmp(lhs, rhs).unwrap_or(Ordering::Equal),
        }
    }

    const fn variant_rank(&self) -> u8 {
        match &self.0 {
            NumberRepr::Int(_) => 0,
            NumberRepr::Rational { .. } => 1,
            NumberRepr::Float(_) => 2,
        }
    }

    fn compare_total(&self, other: &Self) -> Ordering {
        // Total-order convention for canonical ordering:
        // NaN compares greater than any non-NaN value.
        let ordering = match (&self.0, &other.0) {
            (NumberRepr::Int(a), NumberRepr::Int(b)) => int_cmp(a, b),
            (NumberRepr::Int(a), NumberRepr::Rational { num, den }) => int_checked_mul(a, den)
                .map_or_else(
                    || Self::compare_float_total(&self.to_float_repr(), &other.to_float_repr()),
                    |a_den| int_cmp(&a_den, num),
                ),
            (NumberRepr::Rational { num, den }, NumberRepr::Int(b)) => int_checked_mul(b, den)
                .map_or_else(
                    || Self::compare_float_total(&self.to_float_repr(), &other.to_float_repr()),
                    |b_den| int_cmp(num, &b_den),
                ),
            (
                NumberRepr::Rational {
                    num: a_num,
                    den: a_den,
                },
                NumberRepr::Rational {
                    num: b_num,
                    den: b_den,
                },
            ) => {
                if let (Some(a_num_b_den), Some(b_num_a_den)) =
                    (int_checked_mul(a_num, b_den), int_checked_mul(b_num, a_den))
                {
                    int_cmp(&a_num_b_den, &b_num_a_den)
                } else {
                    Self::compare_float_total(&self.to_float_repr(), &other.to_float_repr())
                }
            }
            _ => Self::compare_float_total(&self.to_float_repr(), &other.to_float_repr()),
        };

        if ordering == Ordering::Equal {
            self.variant_rank().cmp(&other.variant_rank())
        } else {
            ordering
        }
    }
}

impl From<f64> for Number {
    fn from(value: f64) -> Self {
        Self::float(value)
    }
}

impl From<f32> for Number {
    fn from(value: f32) -> Self {
        Self::float(f64::from(value))
    }
}

impl From<i64> for Number {
    fn from(value: i64) -> Self {
        Self::from_i64_internal(value)
    }
}

impl From<i32> for Number {
    fn from(value: i32) -> Self {
        Self::from_i64_internal(i64::from(value))
    }
}

impl From<i16> for Number {
    fn from(value: i16) -> Self {
        Self::from_i64_internal(i64::from(value))
    }
}

impl From<i8> for Number {
    fn from(value: i8) -> Self {
        Self::from_i64_internal(i64::from(value))
    }
}

impl From<u32> for Number {
    fn from(value: u32) -> Self {
        Self::from_i64_internal(i64::from(value))
    }
}

impl From<u16> for Number {
    fn from(value: u16) -> Self {
        Self::from_i64_internal(i64::from(value))
    }
}

impl From<u8> for Number {
    fn from(value: u8) -> Self {
        Self::from_i64_internal(i64::from(value))
    }
}

fn u64_to_f64_lossy(value: u64) -> f64 {
    // Build an approximation without lossy integer casts to keep clippy pedantic clean.
    let Ok(high) = u32::try_from(value >> 32) else {
        return f64::INFINITY;
    };
    let Ok(low) = u32::try_from(value & 0xFFFF_FFFF) else {
        return f64::INFINITY;
    };

    f64::from(high).mul_add(4_294_967_296.0, f64::from(low))
}

fn u128_to_f64_lossy(value: u128) -> f64 {
    let Ok(part3) = u32::try_from(value >> 96) else {
        return f64::INFINITY;
    };
    let Ok(part2) = u32::try_from((value >> 64) & 0xFFFF_FFFF) else {
        return f64::INFINITY;
    };
    let Ok(part1) = u32::try_from((value >> 32) & 0xFFFF_FFFF) else {
        return f64::INFINITY;
    };
    let Ok(part0) = u32::try_from(value & 0xFFFF_FFFF) else {
        return f64::INFINITY;
    };

    f64::from(part3)
        .mul_add(4_294_967_296.0, f64::from(part2))
        .mul_add(4_294_967_296.0, f64::from(part1))
        .mul_add(4_294_967_296.0, f64::from(part0))
}

impl From<u64> for Number {
    fn from(value: u64) -> Self {
        if let Some(value_int) = int_from_u64_exact(value) {
            Self(NumberRepr::Int(value_int))
        } else {
            Self::from_float_repr(float_from_f64(u64_to_f64_lossy(value)))
        }
    }
}

impl From<usize> for Number {
    fn from(value: usize) -> Self {
        match u64::try_from(value) {
            Ok(as_u64) => Self::from(as_u64),
            Err(_) => Self::from_float_repr(float_from_f64(f64::INFINITY)),
        }
    }
}

impl From<i128> for Number {
    fn from(value: i128) -> Self {
        if let Some(value_int) = int_from_i128_exact(value) {
            Self(NumberRepr::Int(value_int))
        } else {
            let magnitude = u128_to_f64_lossy(value.unsigned_abs());
            let approx = if value.is_negative() {
                -magnitude
            } else {
                magnitude
            };
            Self::from_float_repr(float_from_f64(approx))
        }
    }
}

impl From<u128> for Number {
    fn from(value: u128) -> Self {
        if let Some(value_int) = int_from_u128_exact(value) {
            Self(NumberRepr::Int(value_int))
        } else {
            Self::from_float_repr(float_from_f64(u128_to_f64_lossy(value)))
        }
    }
}

impl From<Number> for f64 {
    fn from(value: Number) -> Self {
        value.to_f64_lossy()
    }
}

impl Add for Number {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::add(&self, &rhs)
    }
}

impl Add<&Self> for Number {
    type Output = Self;

    fn add(self, rhs: &Self) -> Self::Output {
        Self::add(&self, rhs)
    }
}

impl Add<Number> for &Number {
    type Output = Number;

    fn add(self, rhs: Number) -> Self::Output {
        Number::add(self, &rhs)
    }
}

impl Add<&Number> for &Number {
    type Output = Number;

    fn add(self, rhs: &Number) -> Self::Output {
        Number::add(self, rhs)
    }
}

impl Sub for Number {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::add(&self, &rhs.negate())
    }
}

impl Sub<&Self> for Number {
    type Output = Self;

    fn sub(self, rhs: &Self) -> Self::Output {
        Self::add(&self, &rhs.negate())
    }
}

impl Sub<Number> for &Number {
    type Output = Number;

    fn sub(self, rhs: Number) -> Self::Output {
        Number::add(self, &rhs.negate())
    }
}

impl Sub<&Number> for &Number {
    type Output = Number;

    fn sub(self, rhs: &Number) -> Self::Output {
        Number::add(self, &rhs.negate())
    }
}

impl Mul for Number {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::mul(&self, &rhs)
    }
}

impl Mul<&Self> for Number {
    type Output = Self;

    fn mul(self, rhs: &Self) -> Self::Output {
        Self::mul(&self, rhs)
    }
}

impl Mul<Number> for &Number {
    type Output = Number;

    fn mul(self, rhs: Number) -> Self::Output {
        Number::mul(self, &rhs)
    }
}

impl Mul<&Number> for &Number {
    type Output = Number;

    fn mul(self, rhs: &Number) -> Self::Output {
        Number::mul(self, rhs)
    }
}

impl Div for Number {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::div(&self, &rhs).unwrap_or_else(|| Self::from_float_repr(float_from_f64(f64::NAN)))
    }
}

impl Div<&Self> for Number {
    type Output = Self;

    fn div(self, rhs: &Self) -> Self::Output {
        Self::div(&self, rhs).unwrap_or_else(|| Self::from_float_repr(float_from_f64(f64::NAN)))
    }
}

impl Div<Number> for &Number {
    type Output = Number;

    fn div(self, rhs: Number) -> Self::Output {
        Number::div(self, &rhs).unwrap_or_else(|| Number::from_float_repr(float_from_f64(f64::NAN)))
    }
}

impl Div<&Number> for &Number {
    type Output = Number;

    fn div(self, rhs: &Number) -> Self::Output {
        Number::div(self, rhs).unwrap_or_else(|| Number::from_float_repr(float_from_f64(f64::NAN)))
    }
}

impl Neg for Number {
    type Output = Self;

    fn neg(self) -> Self::Output {
        self.negate()
    }
}

impl Neg for &Number {
    type Output = Number;

    fn neg(self) -> Self::Output {
        self.negate()
    }
}

impl Display for Number {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match &self.0 {
            NumberRepr::Int(value) => write!(f, "{value}"),
            NumberRepr::Rational { num, den } => write!(f, "{num}/{den}"),
            NumberRepr::Float(value) => write!(f, "{value}"),
        }
    }
}

impl Debug for Number {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(self, f)
    }
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (NumberRepr::Int(lhs), NumberRepr::Int(rhs)) => int_cmp(lhs, rhs) == Ordering::Equal,
            (
                NumberRepr::Rational {
                    num: lhs_num,
                    den: lhs_den,
                },
                NumberRepr::Rational {
                    num: rhs_num,
                    den: rhs_den,
                },
            ) => {
                int_cmp(lhs_num, rhs_num) == Ordering::Equal
                    && int_cmp(lhs_den, rhs_den) == Ordering::Equal
            }
            (NumberRepr::Float(lhs), NumberRepr::Float(rhs)) => {
                if Self::float_is_nan(lhs) && Self::float_is_nan(rhs) {
                    true
                } else {
                    Self::compare_float_total(lhs, rhs) == Ordering::Equal
                }
            }
            _ => false,
        }
    }
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        self.compare_total(other)
    }
}

impl Eq for Number {}

impl Hash for Number {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match &self.0 {
            NumberRepr::Int(value) => {
                0_u8.hash(state);
                value.hash(state);
            }
            NumberRepr::Rational { num, den } => {
                1_u8.hash(state);
                num.hash(state);
                den.hash(state);
            }
            NumberRepr::Float(value) => {
                2_u8.hash(state);

                if Self::float_is_nan(value) {
                    0_u8.hash(state);
                } else if float_is_zero(value) {
                    1_u8.hash(state);
                } else {
                    2_u8.hash(state);
                    float_to_f64(value).to_bits().hash(state);
                }
            }
        }
    }
}
