#![allow(
    clippy::pattern_type_mismatch,
    reason = "Match ergonomics on &self.0 are intentional; explicit & + ref conflicts with other clippy lints for Copy backends"
)]
use super::float_ops::{self, FloatType, from_int, to_int};
use super::int_math::{self, IntType};
use super::rational_math::{self, RationalType, is_integer, to_integer};
use super::traits::Number;
use core::cmp::Ordering;
use core::fmt::{Debug, Display, Formatter, Result};
use core::ops::{Add, Div, Mul, Neg, Sub};

/// The internal representation of a Scalar.
/// We use a three-tier system: Int -> Rational -> Float
/// Operations that are exact stay in Int/Rational. Operations that lose precision
/// (like sin, exp, sqrt of non-perfect squares) promote to Float.
#[derive(Clone, Debug)]
pub(in crate::number) enum ScalarRepr {
    Int(IntType),
    Rational(RationalType),
    Float(FloatType),
}

/// A mathematical scalar that automatically manages its representation
/// (Integer, Rational, or Float) to maximize precision based on the active backend.
#[derive(Clone)]
pub struct Scalar(pub(in crate::number) ScalarRepr);

impl Scalar {
    /// Create a new Scalar from an integer representation (internal).
    #[inline]
    #[must_use]
    pub(in crate::number) const fn from_int(value: IntType) -> Self {
        Self(ScalarRepr::Int(value))
    }

    /// Create a new Scalar from a rational representation (internal).
    #[inline]
    #[must_use]
    pub(in crate::number) fn from_rational(value: RationalType) -> Self {
        if rational_math::is_integer(&value) {
            Self(ScalarRepr::Int(rational_math::to_integer(&value)))
        } else {
            Self(ScalarRepr::Rational(value))
        }
    }

    /// Create a new Scalar from a float representation (internal).
    #[inline]
    #[must_use]
    pub(in crate::number) fn from_float(value: FloatType) -> Self {
        if let Some(int_repr) = float_ops::to_int(&value) {
            return Self::from_int(int_repr);
        }
        if let Some(rat_repr) = float_ops::to_rational(&value) {
            return Self::from_rational(rat_repr);
        }
        Self(ScalarRepr::Float(value))
    }

    /// Wrap a float value directly, without normalization (internal — used by
    /// math operations whose results are inherently approximate).
    #[inline]
    #[must_use]
    pub(crate) const fn from_float_raw(value: FloatType) -> Self {
        Self(ScalarRepr::Float(value))
    }

    /// Wrap float, promote to Int if exact integer. Used for `sqrt`/`cbrt`/
    /// `round`/`floor`/`ceil` where integer results are plausible.
    #[inline]
    #[must_use]
    pub(crate) fn from_float_maybe_int(value: FloatType) -> Self {
        if let Some(i) = float_ops::to_int(&value) {
            return Self::from_int(i);
        }
        Self(ScalarRepr::Float(value))
    }

    /// Convert to the highest-precision float representation (internal).
    #[must_use]
    pub(in crate::number) fn to_float_repr(&self) -> FloatType {
        match &self.0 {
            ScalarRepr::Int(i) => from_int(i),
            ScalarRepr::Rational(r) => {
                let num = from_int(&rational_math::numer(r));
                let den = from_int(&rational_math::denom(r));
                float_ops::div(&num, &den)
            }
            ScalarRepr::Float(f) => float_ops::clone(f),
        }
    }

    /// Set working precision for arbitrary-precision backends (e.g. rug).
    /// Returns `true` on success. Fixed backends (f32/f64) return `false`.
    #[must_use]
    pub fn set_precision(bits: u32) -> bool {
        float_ops::set_precision(bits)
    }

    /// Get current working precision in bits.
    /// Fixed backends return their mantissa width (53 for f64, 24 for f32).
    #[must_use]
    pub fn get_precision() -> u32 {
        float_ops::get_precision()
    }

    /// Returns `Some(IntType)` if this value is exactly an integer, `None` otherwise.
    #[must_use]
    pub fn to_int(&self) -> Option<IntType> {
        match &self.0 {
            ScalarRepr::Int(i) => Some(int_math::clone(i)),
            ScalarRepr::Rational(r) => {
                rational_math::is_integer(r).then(|| rational_math::to_integer(r))
            }
            ScalarRepr::Float(f) => float_ops::to_int(f),
        }
    }

    /// Greatest common divisor of `self` and `other`.
    /// Only valid when both values are integers. Returns [`Scalar`].
    #[must_use]
    pub fn gcd(&self, other: &Self) -> Self {
        match (&self.0, &other.0) {
            (ScalarRepr::Int(a), ScalarRepr::Int(b)) => Self::from_int(int_math::gcd(a, b)),
            _ => Self::from_int(int_math::zero()),
        }
    }

    /// Returns `true` if this value is an integer and even.
    #[must_use]
    pub fn is_even(&self) -> bool {
        match &self.0 {
            ScalarRepr::Int(i) => int_math::is_even(i),
            ScalarRepr::Rational(_) | ScalarRepr::Float(_) => false,
        }
    }

    /// Returns `true` if this value is an integer and odd.
    #[must_use]
    pub fn is_odd(&self) -> bool {
        match &self.0 {
            ScalarRepr::Int(i) => !int_math::is_even(i),
            ScalarRepr::Rational(_) | ScalarRepr::Float(_) => false,
        }
    }

    pub(in crate::number) fn is_nan_internal(&self) -> bool {
        if let ScalarRepr::Float(f) = &self.0 {
            float_ops::is_nan(f)
        } else {
            false
        }
    }

    fn round_toward(&self, toward_neg_inf: bool) -> Self {
        match &self.0 {
            ScalarRepr::Int(_) => self.clone(),
            ScalarRepr::Rational(r) => {
                let int_part = to_integer(r);
                if is_integer(r) {
                    return Self::from_int(int_part);
                }
                let zero = rational_math::from_integer(int_math::zero());
                let is_neg = rational_math::cmp(r, &zero) == Ordering::Less;
                if toward_neg_inf == is_neg {
                    let one = int_math::from_i64(1).expect("1 fits");
                    let adjusted = if toward_neg_inf {
                        int_math::sub(&int_part, &one)
                    } else {
                        int_math::add(&int_part, &one)
                    };
                    adjusted.map_or_else(
                        || {
                            let f = self.to_float_repr();
                            if toward_neg_inf {
                                Self::from_float_maybe_int(float_ops::floor(&f))
                            } else {
                                Self::from_float_maybe_int(float_ops::ceil(&f))
                            }
                        },
                        Self::from_int,
                    )
                } else {
                    Self::from_int(int_part)
                }
            }
            ScalarRepr::Float(f) => {
                if toward_neg_inf {
                    Self::from_float_maybe_int(float_ops::floor(f))
                } else {
                    Self::from_float_maybe_int(float_ops::ceil(f))
                }
            }
        }
    }

    /// Returns the machine epsilon for the current precision.
    #[must_use]
    pub fn epsilon() -> Self {
        let two = float_ops::from_i64(2);
        let prec = Self::get_precision();
        let neg_prec = float_ops::from_i64(-i64::from(prec));
        Self::from_float_raw(float_ops::pow(&two, &neg_prec))
    }
}

// ============================================================================
// Core Trait Implementations
// ============================================================================

impl Debug for Scalar {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match &self.0 {
            ScalarRepr::Int(i) => write!(f, "Int({})", int_math::to_string(i)),
            ScalarRepr::Rational(r) => write!(f, "Rational({})", rational_math::to_string(r)),
            ScalarRepr::Float(fl) => write!(f, "Float({})", float_ops::to_string(fl)),
        }
    }
}

impl Display for Scalar {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match &self.0 {
            ScalarRepr::Int(i) => write!(f, "{}", int_math::to_string(i)),
            ScalarRepr::Rational(r) => write!(f, "{}", rational_math::to_string(r)),
            ScalarRepr::Float(fl) => write!(f, "{}", float_ops::to_string(fl)),
        }
    }
}

impl PartialEq for Scalar {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (ScalarRepr::Int(a), ScalarRepr::Int(b)) => int_math::cmp(a, b) == Ordering::Equal,
            (ScalarRepr::Rational(a), ScalarRepr::Rational(b)) => {
                rational_math::cmp(a, b) == Ordering::Equal
            }
            (ScalarRepr::Int(a), ScalarRepr::Rational(b)) => {
                rational_math::cmp(&rational_math::from_integer(int_math::clone(a)), b)
                    == Ordering::Equal
            }
            (ScalarRepr::Rational(a), ScalarRepr::Int(b)) => {
                rational_math::cmp(a, &rational_math::from_integer(int_math::clone(b)))
                    == Ordering::Equal
            }
            _ => {
                // IEEE 754: NaN != NaN
                let fl = self.to_float_repr();
                let fr = other.to_float_repr();
                float_ops::cmp(&fl, &fr) == Some(Ordering::Equal)
            }
        }
    }
}

impl Eq for Scalar {}

impl PartialOrd for Scalar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (&self.0, &other.0) {
            (ScalarRepr::Int(a), ScalarRepr::Int(b)) => Some(int_math::cmp(a, b)),
            (ScalarRepr::Rational(a), ScalarRepr::Rational(b)) => Some(rational_math::cmp(a, b)),
            (ScalarRepr::Int(a), ScalarRepr::Rational(b)) => Some(rational_math::cmp(
                &rational_math::from_integer(int_math::clone(a)),
                b,
            )),
            (ScalarRepr::Rational(a), ScalarRepr::Int(b)) => Some(rational_math::cmp(
                a,
                &rational_math::from_integer(int_math::clone(b)),
            )),
            _ => {
                // IEEE 754: NaN is unordered — partial_cmp returns None
                let fl = self.to_float_repr();
                let fr = other.to_float_repr();
                float_ops::cmp(&fl, &fr)
            }
        }
    }
}

// ============================================================================
// Comparison helpers — explicit methods with total-order semantics.
// ============================================================================

impl Scalar {
    /// Total-order maximum. NaN > everything.
    #[must_use]
    pub fn max(self, other: Self) -> Self {
        if self.total_cmp(&other) == Ordering::Less {
            other
        } else {
            self
        }
    }

    /// Total-order minimum. NaN > everything.
    #[must_use]
    pub fn min(self, other: Self) -> Self {
        if self.total_cmp(&other) == Ordering::Greater {
            other
        } else {
            self
        }
    }

    /// Clamp to [lo, hi] using total ordering.
    #[must_use]
    pub fn clamp(self, lo: Self, hi: Self) -> Self {
        if self.total_cmp(&lo) == Ordering::Less {
            lo
        } else if self.total_cmp(&hi) == Ordering::Greater {
            hi
        } else {
            self
        }
    }
}

impl core::hash::Hash for Scalar {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        if let Some(i) = self.to_int() {
            0_u8.hash(state);
            i.hash(state);
        } else if let ScalarRepr::Rational(r) = &self.0 {
            1_u8.hash(state);
            r.hash(state);
        } else if let ScalarRepr::Float(f) = &self.0 {
            if let Some(r) = float_ops::to_rational(f) {
                1_u8.hash(state);
                r.hash(state);
            } else {
                2_u8.hash(state);
                float_ops::to_string(f).hash(state);
            }
        }
    }

    fn hash_slice<H: core::hash::Hasher>(data: &[Self], state: &mut H) {
        for item in data {
            item.hash(state);
        }
    }
}

// ============================================================================
// Arithmetic Implementations
// ============================================================================

impl Add<&Scalar> for &Scalar {
    type Output = Scalar;
    fn add(self, rhs: &Scalar) -> Scalar {
        match (&self.0, &rhs.0) {
            (ScalarRepr::Int(a), ScalarRepr::Int(b)) => int_math::add(a, b).map_or_else(
                || {
                    Scalar::from_float_raw(float_ops::add(
                        &self.to_float_repr(),
                        &rhs.to_float_repr(),
                    ))
                },
                Scalar::from_int,
            ),
            (ScalarRepr::Rational(a), ScalarRepr::Rational(b)) => {
                Scalar::from_rational(rational_math::add(a, b))
            }
            (ScalarRepr::Int(a), ScalarRepr::Rational(b)) => Scalar::from_rational(
                rational_math::add(&rational_math::from_integer(int_math::clone(a)), b),
            ),
            (ScalarRepr::Rational(a), ScalarRepr::Int(b)) => Scalar::from_rational(
                rational_math::add(a, &rational_math::from_integer(int_math::clone(b))),
            ),
            _ => {
                Scalar::from_float_raw(float_ops::add(&self.to_float_repr(), &rhs.to_float_repr()))
            }
        }
    }
}

impl Sub<&Scalar> for &Scalar {
    type Output = Scalar;
    fn sub(self, rhs: &Scalar) -> Scalar {
        match (&self.0, &rhs.0) {
            (ScalarRepr::Int(a), ScalarRepr::Int(b)) => int_math::sub(a, b).map_or_else(
                || {
                    Scalar::from_float_raw(float_ops::sub(
                        &self.to_float_repr(),
                        &rhs.to_float_repr(),
                    ))
                },
                Scalar::from_int,
            ),
            (ScalarRepr::Rational(a), ScalarRepr::Rational(b)) => {
                Scalar::from_rational(rational_math::sub(a, b))
            }
            (ScalarRepr::Int(a), ScalarRepr::Rational(b)) => Scalar::from_rational(
                rational_math::sub(&rational_math::from_integer(int_math::clone(a)), b),
            ),
            (ScalarRepr::Rational(a), ScalarRepr::Int(b)) => Scalar::from_rational(
                rational_math::sub(a, &rational_math::from_integer(int_math::clone(b))),
            ),
            _ => {
                Scalar::from_float_raw(float_ops::sub(&self.to_float_repr(), &rhs.to_float_repr()))
            }
        }
    }
}

impl Mul<&Scalar> for &Scalar {
    type Output = Scalar;
    fn mul(self, rhs: &Scalar) -> Scalar {
        match (&self.0, &rhs.0) {
            (ScalarRepr::Int(a), ScalarRepr::Int(b)) => int_math::mul(a, b).map_or_else(
                || {
                    Scalar::from_float_raw(float_ops::mul(
                        &self.to_float_repr(),
                        &rhs.to_float_repr(),
                    ))
                },
                Scalar::from_int,
            ),
            (ScalarRepr::Rational(a), ScalarRepr::Rational(b)) => {
                Scalar::from_rational(rational_math::mul(a, b))
            }
            (ScalarRepr::Int(a), ScalarRepr::Rational(b)) => Scalar::from_rational(
                rational_math::mul(&rational_math::from_integer(int_math::clone(a)), b),
            ),
            (ScalarRepr::Rational(a), ScalarRepr::Int(b)) => Scalar::from_rational(
                rational_math::mul(a, &rational_math::from_integer(int_math::clone(b))),
            ),
            _ => {
                Scalar::from_float_raw(float_ops::mul(&self.to_float_repr(), &rhs.to_float_repr()))
            }
        }
    }
}

impl Div<&Scalar> for &Scalar {
    type Output = Scalar;
    fn div(self, rhs: &Scalar) -> Scalar {
        if rhs.is_zero() {
            return Scalar::from_float_raw(float_ops::div(
                &self.to_float_repr(),
                &rhs.to_float_repr(),
            ));
        }
        match (&self.0, &rhs.0) {
            (ScalarRepr::Int(a), ScalarRepr::Int(b)) => {
                if int_math::modulo(a, b) == int_math::zero() {
                    Scalar::from_int(int_math::div_exact(a, b))
                } else {
                    Scalar::from_rational(rational_math::new(
                        int_math::clone(a),
                        int_math::clone(b),
                    ))
                }
            }
            (ScalarRepr::Rational(a), ScalarRepr::Rational(b)) => {
                Scalar::from_rational(rational_math::div(a, b))
            }
            (ScalarRepr::Int(a), ScalarRepr::Rational(b)) => Scalar::from_rational(
                rational_math::div(&rational_math::from_integer(int_math::clone(a)), b),
            ),
            (ScalarRepr::Rational(a), ScalarRepr::Int(b)) => Scalar::from_rational(
                rational_math::div(a, &rational_math::from_integer(int_math::clone(b))),
            ),
            _ => {
                Scalar::from_float_raw(float_ops::div(&self.to_float_repr(), &rhs.to_float_repr()))
            }
        }
    }
}

impl Neg for &Scalar {
    type Output = Scalar;
    fn neg(self) -> Scalar {
        match &self.0 {
            ScalarRepr::Int(i) => int_math::neg(i).map_or_else(
                || Scalar::from_float_raw(float_ops::neg(&self.to_float_repr())),
                Scalar::from_int,
            ),
            ScalarRepr::Rational(r) => Scalar::from_rational(rational_math::neg(r)),
            ScalarRepr::Float(f) => Scalar::from_float_raw(float_ops::neg(f)),
        }
    }
}

// Forward value-based ops to ref-based ops
impl Add for Scalar {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        &self + &rhs
    }
}
impl Sub for Scalar {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        &self - &rhs
    }
}
impl Mul for Scalar {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        &self * &rhs
    }
}
impl Div for Scalar {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        &self / &rhs
    }
}
impl Neg for Scalar {
    type Output = Self;
    fn neg(self) -> Self {
        -&self
    }
}
impl Add<&Self> for Scalar {
    type Output = Self;
    fn add(self, rhs: &Self) -> Self {
        &self + rhs
    }
}
impl Sub<&Self> for Scalar {
    type Output = Self;
    fn sub(self, rhs: &Self) -> Self {
        &self - rhs
    }
}
impl Mul<&Self> for Scalar {
    type Output = Self;
    fn mul(self, rhs: &Self) -> Self {
        &self * rhs
    }
}
impl Div<&Self> for Scalar {
    type Output = Self;
    fn div(self, rhs: &Self) -> Self {
        &self / rhs
    }
}

// ============================================================================
// Number Trait Helpers
// ============================================================================

/// Extract an `IntType` from a [`Scalar`] used as an integer order parameter.
/// Returns None if extraction fails (e.g., Float that doesn't round to int).
fn extract_int_order(s: &Scalar) -> Option<IntType> {
    match &s.0 {
        ScalarRepr::Int(n) => Some(int_math::clone(n)),
        ScalarRepr::Rational(r) => Some(to_integer(r)),
        ScalarRepr::Float(f) => to_int(&float_ops::round(f)),
    }
}

macro_rules! impl_special_with_int_order_fn {
    ($name:ident, $float_fn:path) => {
        fn $name(&self, order: &Self) -> Self {
            let Some(n) = extract_int_order(order) else {
                return Self::from_float_raw(float_ops::nan());
            };
            Self::from_float_raw($float_fn(&n, &self.to_float_repr()))
        }
    };
}

// ============================================================================
// Number Trait Implementation
// ============================================================================

impl Number for Scalar {
    // Basic math
    fn abs(&self) -> Self {
        match &self.0 {
            ScalarRepr::Int(i) => int_math::abs(i).map_or_else(
                || Self::from_float_raw(float_ops::abs(&self.to_float_repr())),
                Self::from_int,
            ),
            ScalarRepr::Rational(r) => {
                if rational_math::cmp(r, &rational_math::from_integer(int_math::zero()))
                    == Ordering::Less
                {
                    Self::from_rational(rational_math::neg(r))
                } else {
                    self.clone()
                }
            }
            ScalarRepr::Float(f) => Self::from_float_raw(float_ops::abs(f)),
        }
    }

    fn signum(&self) -> Self {
        match &self.0 {
            ScalarRepr::Int(i) => {
                if int_math::is_zero(i) {
                    Self::from_int(int_math::zero())
                } else if int_math::is_negative(i) {
                    Self::from_int(int_math::from_i64(-1).expect("constant -1 always fits"))
                } else {
                    Self::from_int(int_math::from_i64(1).expect("constant 1 always fits"))
                }
            }
            ScalarRepr::Rational(r) => {
                let zero_r = rational_math::from_integer(int_math::zero());
                match rational_math::cmp(r, &zero_r) {
                    Ordering::Less => {
                        Self::from_int(int_math::from_i64(-1).expect("constant -1 always fits"))
                    }
                    Ordering::Equal => Self::from_int(int_math::zero()),
                    Ordering::Greater => {
                        Self::from_int(int_math::from_i64(1).expect("constant 1 always fits"))
                    }
                }
            }
            ScalarRepr::Float(f) => Self::from_float_raw(float_ops::signum(f)),
        }
    }

    fn floor(&self) -> Self {
        self.round_toward(true)
    }

    fn ceil(&self) -> Self {
        self.round_toward(false)
    }

    fn round(&self) -> Self {
        Self::from_float_maybe_int(float_ops::round(&self.to_float_repr()))
    }

    fn fract(&self) -> Self {
        match &self.0 {
            ScalarRepr::Int(_) => Self::from_int(int_math::zero()),
            ScalarRepr::Rational(_) => {
                if self.is_integer() {
                    Self::from_int(int_math::zero())
                } else {
                    self - &self.floor()
                }
            }
            ScalarRepr::Float(f) => Self::from_float_raw(float_ops::fract(f)),
        }
    }

    fn negate(&self) -> Self {
        -self
    }

    // Roots
    fn sqrt(&self) -> Self {
        if let ScalarRepr::Int(i) = &self.0
            && let Some(root) = int_math::perfect_square(i)
        {
            return Self::from_int(root);
        }
        Self::from_float_maybe_int(float_ops::sqrt(&self.to_float_repr()))
    }

    fn cbrt(&self) -> Self {
        if let ScalarRepr::Int(i) = &self.0
            && let Some(root) = int_math::perfect_cube(i)
        {
            return Self::from_int(root);
        }
        Self::from_float_maybe_int(float_ops::cbrt(&self.to_float_repr()))
    }

    // Delegation to float backend for transcendentals
    fn sin(&self) -> Self {
        Self::from_float_raw(float_ops::sin(&self.to_float_repr()))
    }
    fn cos(&self) -> Self {
        Self::from_float_raw(float_ops::cos(&self.to_float_repr()))
    }
    fn tan(&self) -> Self {
        Self::from_float_raw(float_ops::tan(&self.to_float_repr()))
    }
    fn cot(&self) -> Self {
        Self::from_float_raw(float_ops::div(
            &float_ops::from_f64(1.0),
            &float_ops::tan(&self.to_float_repr()),
        ))
    }
    fn sec(&self) -> Self {
        Self::from_float_raw(float_ops::div(
            &float_ops::from_f64(1.0),
            &float_ops::cos(&self.to_float_repr()),
        ))
    }
    fn csc(&self) -> Self {
        Self::from_float_raw(float_ops::div(
            &float_ops::from_f64(1.0),
            &float_ops::sin(&self.to_float_repr()),
        ))
    }

    fn asin(&self) -> Self {
        Self::from_float_raw(float_ops::asin(&self.to_float_repr()))
    }
    fn acos(&self) -> Self {
        Self::from_float_raw(float_ops::acos(&self.to_float_repr()))
    }
    fn atan(&self) -> Self {
        Self::from_float_raw(float_ops::atan(&self.to_float_repr()))
    }
    fn acot(&self) -> Self {
        Self::from_float_raw(float_ops::atan(&float_ops::div(
            &float_ops::from_f64(1.0),
            &self.to_float_repr(),
        )))
    }
    fn asec(&self) -> Self {
        Self::from_float_raw(float_ops::acos(&float_ops::div(
            &float_ops::from_f64(1.0),
            &self.to_float_repr(),
        )))
    }
    fn acsc(&self) -> Self {
        Self::from_float_raw(float_ops::asin(&float_ops::div(
            &float_ops::from_f64(1.0),
            &self.to_float_repr(),
        )))
    }

    fn sinh(&self) -> Self {
        Self::from_float_raw(float_ops::sinh(&self.to_float_repr()))
    }
    fn cosh(&self) -> Self {
        Self::from_float_raw(float_ops::cosh(&self.to_float_repr()))
    }
    fn tanh(&self) -> Self {
        Self::from_float_raw(float_ops::tanh(&self.to_float_repr()))
    }
    fn coth(&self) -> Self {
        Self::from_float_raw(float_ops::div(
            &float_ops::from_f64(1.0),
            &float_ops::tanh(&self.to_float_repr()),
        ))
    }
    fn sech(&self) -> Self {
        Self::from_float_raw(float_ops::div(
            &float_ops::from_f64(1.0),
            &float_ops::cosh(&self.to_float_repr()),
        ))
    }
    fn csch(&self) -> Self {
        Self::from_float_raw(float_ops::div(
            &float_ops::from_f64(1.0),
            &float_ops::sinh(&self.to_float_repr()),
        ))
    }

    fn asinh(&self) -> Self {
        Self::from_float_raw(float_ops::asinh(&self.to_float_repr()))
    }
    fn acosh(&self) -> Self {
        Self::from_float_raw(float_ops::acosh(&self.to_float_repr()))
    }
    fn atanh(&self) -> Self {
        Self::from_float_raw(float_ops::atanh(&self.to_float_repr()))
    }
    fn acoth(&self) -> Self {
        Self::from_float_raw(float_ops::atanh(&float_ops::div(
            &float_ops::from_f64(1.0),
            &self.to_float_repr(),
        )))
    }
    fn asech(&self) -> Self {
        Self::from_float_raw(float_ops::acosh(&float_ops::div(
            &float_ops::from_f64(1.0),
            &self.to_float_repr(),
        )))
    }
    fn acsch(&self) -> Self {
        Self::from_float_raw(float_ops::asinh(&float_ops::div(
            &float_ops::from_f64(1.0),
            &self.to_float_repr(),
        )))
    }

    fn exp(&self) -> Self {
        Self::from_float_raw(float_ops::exp(&self.to_float_repr()))
    }
    fn expm1(&self) -> Self {
        Self::from_float_raw(float_ops::expm1(&self.to_float_repr()))
    }
    fn exp_neg(&self) -> Self {
        Self::from_float_raw(float_ops::exp(&float_ops::neg(&self.to_float_repr())))
    }
    fn ln(&self) -> Self {
        Self::from_float_raw(float_ops::ln(&self.to_float_repr()))
    }
    fn log1p(&self) -> Self {
        Self::from_float_raw(float_ops::log1p(&self.to_float_repr()))
    }

    // Special functions — delegate to the `special` module via SpecFloat generics
    fn erf(&self) -> Self {
        Self::from_float_raw(float_ops::erf(&self.to_float_repr()))
    }
    fn erfc(&self) -> Self {
        Self::from_float_raw(float_ops::erfc(&self.to_float_repr()))
    }
    fn gamma(&self) -> Self {
        // Exact factorial for positive integers: Γ(n) = (n-1)!
        if let Some(n) = self.to_int()
            && int_math::is_positive(&n)
        {
            let mut acc = int_math::from_i64(1).expect("1 always fits");
            let mut k = int_math::from_i64(1).expect("1 always fits");
            let one = int_math::clone(&k);
            while int_math::cmp(&k, &n) == core::cmp::Ordering::Less {
                if let Some(next) = int_math::mul(&acc, &k) {
                    acc = next;
                } else {
                    return Self::from_float_raw(float_ops::gamma(&self.to_float_repr()));
                }
                if let Some(next) = int_math::add(&k, &one) {
                    k = next;
                } else {
                    return Self::from_float_raw(float_ops::gamma(&self.to_float_repr()));
                }
            }
            return Self::from_int(acc);
        }
        Self::from_float_raw(float_ops::gamma(&self.to_float_repr()))
    }
    fn lgamma(&self) -> Self {
        Self::from_float_raw(float_ops::lgamma(&self.to_float_repr()))
    }
    fn digamma(&self) -> Self {
        Self::from_float_raw(float_ops::digamma(&self.to_float_repr()))
    }
    fn trigamma(&self) -> Self {
        Self::from_float_raw(float_ops::trigamma(&self.to_float_repr()))
    }
    fn tetragamma(&self) -> Self {
        Self::from_float_raw(float_ops::tetragamma(&self.to_float_repr()))
    }
    fn sinc(&self) -> Self {
        // sinc(x) = sin(x)/x, sinc(0) = 1
        if self.is_zero() {
            return Self::from_int(int_math::from_i64(1).expect("constant 1 always fits"));
        }
        &self.sin() / self
    }

    fn elliptic_k(&self) -> Self {
        Self::from_float_raw(float_ops::elliptic_k(&self.to_float_repr()))
    }
    fn elliptic_e(&self) -> Self {
        Self::from_float_raw(float_ops::elliptic_e(&self.to_float_repr()))
    }
    fn zeta(&self) -> Self {
        Self::from_float_raw(float_ops::zeta(&self.to_float_repr()))
    }
    fn exp_polar(&self) -> Self {
        // Real scalar: exp_polar(r) = e^r (polar form with zero angle).
        // Full complex polar form exp(r + i*theta) = e^r * (cos(theta) + i*sin(theta))
        // requires a Complex type, which is not yet implemented.
        self.exp()
    }

    // Binary / multi-arg
    fn atan2(&self, x: &Self) -> Self {
        Self::from_float_raw(float_ops::atan2(&self.to_float_repr(), &x.to_float_repr()))
    }
    fn log_base(&self, base: &Self) -> Self {
        &self.ln() / &base.ln()
    }
    fn pow(&self, exp: &Self) -> Self {
        // Integer exponent — preserve Int/Rational via exponentiation by squaring
        if let ScalarRepr::Int(e) = &exp.0 {
            if int_math::is_negative(e) {
                let abs_e = int_math::abs(e).expect("abs of non-zero integer should succeed");
                let pos = self.pow(&Self::from_int(abs_e));
                return &Self::from_int(int_math::from_i64(1).expect("constant 1 always fits"))
                    / &pos;
            }
            if int_math::is_zero(e) {
                return Self::from_int(int_math::from_i64(1).expect("constant 1 always fits"));
            }

            let two = int_math::from_i64(2).expect("2 always fits in IntType");
            let mut result = Self::from_int(int_math::from_i64(1).expect("constant 1 always fits"));
            let mut base = self.clone();
            let mut ec = int_math::clone(e);

            while !int_math::is_zero(&ec) {
                if int_math::modulo(&ec, &two) != int_math::zero() {
                    result = &result * &base;
                }
                base = &base * &base;
                ec = int_math::div_exact(&ec, &two);
            }
            return result;
        }

        // Non-integer exponent → use float pow
        Self::from_float_raw(float_ops::pow(&self.to_float_repr(), &exp.to_float_repr()))
    }
    impl_special_with_int_order_fn!(besselj, float_ops::besselj);
    impl_special_with_int_order_fn!(bessely, float_ops::bessely);
    impl_special_with_int_order_fn!(besseli, float_ops::besseli);
    impl_special_with_int_order_fn!(besselk, float_ops::besselk);
    impl_special_with_int_order_fn!(polygamma, float_ops::polygamma);

    fn beta(&self, other: &Self) -> Self {
        Self::from_float_raw(float_ops::beta(
            &self.to_float_repr(),
            &other.to_float_repr(),
        ))
    }

    impl_special_with_int_order_fn!(zeta_deriv, float_ops::zeta_deriv);
    impl_special_with_int_order_fn!(hermite, float_ops::hermite);
    impl_special_with_int_order_fn!(lambertw, float_ops::lambertw);

    fn assoc_legendre(&self, l: &Self, m: &Self) -> Self {
        let (Some(li), Some(mi)) = (extract_int_order(l), extract_int_order(m)) else {
            return Self::from_float_raw(float_ops::nan());
        };
        Self::from_float_raw(float_ops::assoc_legendre(&li, &mi, &self.to_float_repr()))
    }

    fn spherical_harmonic(&self, l: &Self, m: &Self, phi: &Self) -> Self {
        let (Some(li), Some(mi)) = (extract_int_order(l), extract_int_order(m)) else {
            return Self::from_float_raw(float_ops::nan());
        };
        Self::from_float_raw(float_ops::spherical_harmonic(
            &li,
            &mi,
            &self.to_float_repr(),
            &phi.to_float_repr(),
        ))
    }

    // Core properties
    fn is_zero(&self) -> bool {
        match &self.0 {
            ScalarRepr::Int(i) => int_math::is_zero(i),
            ScalarRepr::Rational(r) => {
                rational_math::cmp(r, &rational_math::from_integer(int_math::zero()))
                    == Ordering::Equal
            }
            ScalarRepr::Float(f) => float_ops::is_zero(f),
        }
    }

    fn is_one(&self) -> bool {
        match &self.0 {
            ScalarRepr::Int(i) => int_math::is_one(i),
            ScalarRepr::Rational(r) => {
                rational_math::cmp(
                    r,
                    &rational_math::from_integer(
                        int_math::from_i64(1).expect("constant 1 always fits"),
                    ),
                ) == Ordering::Equal
            }
            ScalarRepr::Float(f) => float_ops::is_one(f),
        }
    }

    fn is_neg_one(&self) -> bool {
        match &self.0 {
            ScalarRepr::Int(i) => int_math::is_neg_one(i),
            ScalarRepr::Rational(r) => {
                rational_math::cmp(
                    r,
                    &rational_math::from_integer(
                        int_math::from_i64(-1).expect("constant -1 always fits"),
                    ),
                ) == Ordering::Equal
            }
            ScalarRepr::Float(f) => float_ops::is_neg_one(f),
        }
    }

    fn is_integer(&self) -> bool {
        match &self.0 {
            ScalarRepr::Int(_) => true,
            ScalarRepr::Rational(r) => is_integer(r),
            ScalarRepr::Float(f) => float_ops::is_integer(f),
        }
    }

    fn is_negative(&self) -> bool {
        match &self.0 {
            ScalarRepr::Int(i) => int_math::is_negative(i),
            ScalarRepr::Rational(r) => {
                rational_math::cmp(r, &rational_math::from_integer(int_math::zero()))
                    == Ordering::Less
            }
            ScalarRepr::Float(f) => float_ops::is_negative(f),
        }
    }

    fn is_positive(&self) -> bool {
        match &self.0 {
            ScalarRepr::Int(i) => int_math::is_positive(i),
            ScalarRepr::Rational(r) => {
                rational_math::cmp(r, &rational_math::from_integer(int_math::zero()))
                    == Ordering::Greater
            }
            ScalarRepr::Float(f) => float_ops::is_positive(f),
        }
    }

    fn is_finite(&self) -> bool {
        match &self.0 {
            ScalarRepr::Int(_) | ScalarRepr::Rational(_) => true,
            ScalarRepr::Float(f) => float_ops::is_finite(f),
        }
    }

    fn to_float(&self) -> Self {
        Self::from_float_raw(self.to_float_repr())
    }

    fn approx_eq_number(&self, other: &Self, tolerance: &Self) -> bool {
        let diff = (self - other).abs();
        let scale = self.abs().max(other.abs()).max(Self::from_int(
            int_math::from_i64(1).expect("constant 1 always fits"),
        ));
        diff.total_cmp(&(tolerance * &scale)) != Ordering::Greater
    }

    fn total_cmp(&self, other: &Self) -> Ordering {
        match (&self.0, &other.0) {
            (ScalarRepr::Int(a), ScalarRepr::Int(b)) => int_math::cmp(a, b),
            (ScalarRepr::Rational(a), ScalarRepr::Rational(b)) => rational_math::cmp(a, b),
            (ScalarRepr::Int(a), ScalarRepr::Rational(b)) => {
                rational_math::cmp(&rational_math::from_integer(int_math::clone(a)), b)
            }
            (ScalarRepr::Rational(a), ScalarRepr::Int(b)) => {
                rational_math::cmp(a, &rational_math::from_integer(int_math::clone(b)))
            }
            _ => {
                let fl = self.to_float_repr();
                let fr = other.to_float_repr();
                float_ops::cmp(&fl, &fr).unwrap_or_else(|| {
                    // One or both are NaN (since Infinity compares correctly with finite values).
                    // Total order: NaN is greater than everything else.
                    match (self.is_nan_internal(), other.is_nan_internal()) {
                        (false, true) => Ordering::Less,
                        (true, false) => Ordering::Greater,
                        _ => Ordering::Equal, // Both NaN
                    }
                })
            }
        }
    }

    fn num_max(&self, other: &Self) -> Self {
        if self.is_nan_internal() || other.is_nan_internal() {
            return Self::from_float_raw(float_ops::nan());
        }
        if self.total_cmp(other) == Ordering::Less {
            other.clone()
        } else {
            self.clone()
        }
    }

    fn num_min(&self, other: &Self) -> Self {
        if self.is_nan_internal() || other.is_nan_internal() {
            return Self::from_float_raw(float_ops::nan());
        }
        if self.total_cmp(other) == Ordering::Greater {
            other.clone()
        } else {
            self.clone()
        }
    }
}
