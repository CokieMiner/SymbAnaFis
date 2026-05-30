//! Shared function implementations used by both `f64_ops` and `f32_ops`.
//!
//! This module is `#[macro_use]`d by each backend so the macro below is
//! expanded in the calling module's scope, picking up the local
//! `BackingFloat`, `IntType`, `math`, `rational`, and `int_math` bindings.

macro_rules! impl_shared_float_ops {
    () => {
        #[inline]
        pub(super) const fn clone(value: &BackingFloat) -> BackingFloat {
            *value
        }

        #[inline]
        pub(super) fn set_precision(_bits: u32) -> bool {
            false
        }

        // --- Construction & conversion ---

        pub(super) fn to_rational(value: &BackingFloat) -> Option<rational::RationalType> {
            use num_traits::Float;
            if !value.is_finite() {
                return None;
            }
            let (mantissa, exponent, sign) = Float::integer_decode(*value);
            if mantissa == 0 {
                return Some(rational::from_integer(int_math::zero()));
            }

            #[allow(clippy::cast_possible_wrap, reason = "Float mantissa fits in i64")]
            let mut num = int_math::from_i64(mantissa as i64)?;
            if sign < 0 {
                num = int_math::neg(&num)?;
            }

            let result = if exponent >= 0 {
                let mut power = int_math::from_i64(1)?;
                let base_2 = int_math::from_i64(2)?;
                for _ in 0..exponent {
                    power = int_math::mul(&power, &base_2)?;
                }
                rational::from_integer(int_math::mul(&num, &power)?)
            } else {
                let mut den = int_math::from_i64(1)?;
                let base_2 = int_math::from_i64(2)?;
                for _ in 0..(-exponent) {
                    den = int_math::mul(&den, &base_2)?;
                }
                rational::new(num, den)
            };
            Some(result)
        }

        pub(super) fn to_string(value: &BackingFloat) -> String {
            alloc::format!("{value}")
        }

        #[cfg(feature = "serde")]
        pub(super) fn from_str(value: &str) -> Option<BackingFloat> {
            value.parse().ok()
        }

        // --- Arithmetic ---

        #[inline]
        pub(super) fn add(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
            *lhs + *rhs
        }

        #[inline]
        pub(super) fn sub(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
            *lhs - *rhs
        }

        #[inline]
        pub(super) fn mul(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
            *lhs * *rhs
        }

        #[inline]
        pub(super) fn div(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
            *lhs / *rhs
        }

        #[inline]
        pub(super) fn neg(value: &BackingFloat) -> BackingFloat {
            -*value
        }

        // --- Comparison ---

        #[inline]
        pub(super) fn cmp(lhs: &BackingFloat, rhs: &BackingFloat) -> Option<Ordering> {
            lhs.partial_cmp(rhs)
        }

        // --- Properties ---

        #[inline]
        pub(super) fn is_zero(value: &BackingFloat) -> bool {
            *value == 0.0
        }

        #[inline]
        pub(super) fn is_one(value: &BackingFloat) -> bool {
            *value == 1.0
        }

        #[inline]
        pub(super) fn is_neg_one(value: &BackingFloat) -> bool {
            *value == -1.0
        }

        #[inline]
        pub(super) fn is_integer(value: &BackingFloat) -> bool {
            value.fract() == 0.0
        }

        #[inline]
        pub(super) const fn is_finite(value: &BackingFloat) -> bool {
            value.is_finite()
        }

        #[inline]
        pub(super) fn is_negative(value: &BackingFloat) -> bool {
            *value < 0.0
        }

        #[inline]
        pub(super) fn is_positive(value: &BackingFloat) -> bool {
            *value > 0.0
        }

        #[inline]
        pub(super) fn is_nan(value: &BackingFloat) -> bool {
            value.is_nan()
        }

        // --- Basic math ---

        #[inline]
        pub(super) fn abs(value: &BackingFloat) -> BackingFloat {
            math::fabs(*value)
        }

        #[inline]
        pub(super) fn signum(value: &BackingFloat) -> BackingFloat {
            value.signum()
        }

        #[inline]
        pub(super) fn floor(value: &BackingFloat) -> BackingFloat {
            math::floor(*value)
        }

        #[inline]
        pub(super) fn ceil(value: &BackingFloat) -> BackingFloat {
            math::ceil(*value)
        }

        #[inline]
        pub(super) fn round(value: &BackingFloat) -> BackingFloat {
            math::round(*value)
        }

        #[inline]
        pub(super) fn fract(value: &BackingFloat) -> BackingFloat {
            value.fract()
        }

        #[inline]
        pub(super) fn pow(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
            math::pow(*lhs, *rhs)
        }

        #[inline]
        pub(super) fn sqrt(value: &BackingFloat) -> BackingFloat {
            math::sqrt(*value)
        }

        #[inline]
        pub(super) fn cbrt(value: &BackingFloat) -> BackingFloat {
            math::cbrt(*value)
        }

        // --- Trigonometric ---

        #[inline]
        pub(super) fn sin(value: &BackingFloat) -> BackingFloat {
            math::sin(*value)
        }

        #[inline]
        pub(super) fn cos(value: &BackingFloat) -> BackingFloat {
            math::cos(*value)
        }

        #[inline]
        pub(super) fn tan(value: &BackingFloat) -> BackingFloat {
            math::tan(*value)
        }

        #[inline]
        pub(super) fn asin(value: &BackingFloat) -> BackingFloat {
            math::asin(*value)
        }

        #[inline]
        pub(super) fn acos(value: &BackingFloat) -> BackingFloat {
            math::acos(*value)
        }

        #[inline]
        pub(super) fn atan(value: &BackingFloat) -> BackingFloat {
            math::atan(*value)
        }

        #[inline]
        pub(super) fn atan2(y: &BackingFloat, x: &BackingFloat) -> BackingFloat {
            math::atan2(*y, *x)
        }

        // --- Hyperbolic ---

        #[inline]
        pub(super) fn sinh(value: &BackingFloat) -> BackingFloat {
            math::sinh(*value)
        }

        #[inline]
        pub(super) fn cosh(value: &BackingFloat) -> BackingFloat {
            math::cosh(*value)
        }

        #[inline]
        pub(super) fn tanh(value: &BackingFloat) -> BackingFloat {
            math::tanh(*value)
        }

        #[inline]
        pub(super) fn asinh(value: &BackingFloat) -> BackingFloat {
            math::asinh(*value)
        }

        #[inline]
        pub(super) fn acosh(value: &BackingFloat) -> BackingFloat {
            math::acosh(*value)
        }

        #[inline]
        pub(super) fn atanh(value: &BackingFloat) -> BackingFloat {
            math::atanh(*value)
        }

        // --- Exponential & logarithmic ---

        #[inline]
        pub(super) fn exp(value: &BackingFloat) -> BackingFloat {
            math::exp(*value)
        }

        #[inline]
        pub(super) fn expm1(value: &BackingFloat) -> BackingFloat {
            math::expm1(*value)
        }

        #[inline]
        pub(super) fn ln(value: &BackingFloat) -> BackingFloat {
            math::ln(*value)
        }

        #[inline]
        pub(super) fn log1p(value: &BackingFloat) -> BackingFloat {
            math::log1p(*value)
        }

        // --- Bessel functions ---

        #[inline]
        pub(super) fn besselj(n: &IntType, value: &BackingFloat) -> BackingFloat {
            super::special::besselj(*n, *value)
        }

        #[inline]
        pub(super) fn bessely(n: &IntType, value: &BackingFloat) -> BackingFloat {
            super::special::bessely(*n, *value)
        }

        #[inline]
        pub(super) fn besseli(n: &IntType, value: &BackingFloat) -> BackingFloat {
            super::special::besseli(*n, *value)
        }

        #[inline]
        pub(super) fn besselk(n: &IntType, value: &BackingFloat) -> BackingFloat {
            super::special::besselk(*n, *value)
        }

        // --- Special functions ---

        #[inline]
        pub(super) fn erf(value: &BackingFloat) -> BackingFloat {
            super::special::erf(*value)
        }

        #[inline]
        pub(super) fn erfc(value: &BackingFloat) -> BackingFloat {
            super::special::erfc(*value)
        }

        #[inline]
        pub(super) fn gamma(value: &BackingFloat) -> BackingFloat {
            super::special::gamma(*value)
        }

        #[inline]
        pub(super) fn lgamma(value: &BackingFloat) -> BackingFloat {
            super::special::lgamma(*value)
        }

        #[inline]
        pub(super) fn digamma(value: &BackingFloat) -> BackingFloat {
            super::special::digamma(*value)
        }

        #[inline]
        pub(super) fn trigamma(value: &BackingFloat) -> BackingFloat {
            super::special::trigamma(*value)
        }

        #[inline]
        pub(super) fn tetragamma(value: &BackingFloat) -> BackingFloat {
            super::special::tetragamma(*value)
        }

        #[inline]
        pub(super) fn polygamma(n: &IntType, value: &BackingFloat) -> BackingFloat {
            super::special::polygamma_n(*n, *value)
        }

        #[inline]
        pub(super) fn zeta(value: &BackingFloat) -> BackingFloat {
            super::special::zeta(*value)
        }

        #[inline]
        pub(super) fn zeta_deriv(n: &IntType, value: &BackingFloat) -> BackingFloat {
            super::special::zeta_deriv(*n, *value)
        }

        #[inline]
        pub(super) fn lambertw(order: &IntType, value: &BackingFloat) -> BackingFloat {
            if int_math::is_zero(order) {
                super::special::lambertw0(*value)
            } else if int_math::is_neg_one(order) {
                super::special::lambertwm1(*value)
            } else {
                super::special::SpecFloat::nan()
            }
        }

        #[inline]
        pub(super) fn beta(a: &BackingFloat, b: &BackingFloat) -> BackingFloat {
            super::special::beta(*a, *b)
        }

        #[inline]
        pub(super) fn elliptic_k(value: &BackingFloat) -> BackingFloat {
            super::special::elliptic_k(*value)
        }

        #[inline]
        pub(super) fn elliptic_e(value: &BackingFloat) -> BackingFloat {
            super::special::elliptic_e(*value)
        }

        #[inline]
        pub(super) fn hermite(n: &IntType, value: &BackingFloat) -> BackingFloat {
            super::special::hermite(*n, *value)
        }

        #[inline]
        pub(super) fn assoc_legendre(
            l: &IntType,
            m: &IntType,
            value: &BackingFloat,
        ) -> BackingFloat {
            super::special::assoc_legendre(*l, *m, *value)
        }

        #[inline]
        pub(super) fn spherical_harmonic(
            l: &IntType,
            m: &IntType,
            theta: &BackingFloat,
            phi: &BackingFloat,
        ) -> BackingFloat {
            super::special::spherical_harmonic(*l, *m, *theta, *phi)
        }
    };
}
