//! Extension trait exposing all math operations directly on the active
//! compile-time `FloatType`, so callers that don't need the full `Scalar`
//! wrapper can access every function with zero overhead.

use crate::number::logic::float_ops::FloatType;
use crate::number::logic::int_math::IntType;

/// Extension trait providing every mathematical operation directly on the
/// compile-time active float backend (`f32`, `f64`, or `rug::Float`).
///
/// Integer parameters (Bessel orders, Lambert W branch, Polygamma order, etc.)
/// use [`IntType`] — the compile-time active integer backend — so there is
/// never a precision-losing cast from a fixed-width type.
pub trait AnafisMathExt {
    // --- Arithmetic ---
    /// Returns a copy of the value.
    #[must_use]
    fn clone(&self) -> Self;
    /// Adds `rhs` to `self`.
    #[must_use]
    fn add(&self, rhs: &Self) -> Self;
    /// Subtracts `rhs` from `self`.
    #[must_use]
    fn sub(&self, rhs: &Self) -> Self;
    /// Multiplies `self` by `rhs`.
    #[must_use]
    fn mul(&self, rhs: &Self) -> Self;
    /// Divides `self` by `rhs`.
    #[must_use]
    fn div(&self, rhs: &Self) -> Self;
    /// Negates `self`.
    #[must_use]
    fn neg(&self) -> Self;

    // --- Basic math ---
    /// Absolute value.
    #[must_use]
    fn abs(&self) -> Self;
    /// Sign of `self`: −1, 0, or +1.
    #[must_use]
    fn signum(&self) -> Self;
    /// Largest integer not greater than `self`.
    #[must_use]
    fn floor(&self) -> Self;
    /// Smallest integer not less than `self`.
    #[must_use]
    fn ceil(&self) -> Self;
    /// Rounds to the nearest integer.
    #[must_use]
    fn round(&self) -> Self;
    /// Fractional part of `self`.
    #[must_use]
    fn fract(&self) -> Self;
    /// `self` raised to the power `rhs`.
    #[must_use]
    fn pow(&self, rhs: &Self) -> Self;
    /// Square root.
    #[must_use]
    fn sqrt(&self) -> Self;
    /// Cube root.
    #[must_use]
    fn cbrt(&self) -> Self;

    // --- Trigonometric ---
    /// Sine.
    #[must_use]
    fn sin(&self) -> Self;
    /// Cosine.
    #[must_use]
    fn cos(&self) -> Self;
    /// Tangent.
    #[must_use]
    fn tan(&self) -> Self;
    /// Arcsine.
    #[must_use]
    fn asin(&self) -> Self;
    /// Arccosine.
    #[must_use]
    fn acos(&self) -> Self;
    /// Arctangent.
    #[must_use]
    fn atan(&self) -> Self;
    /// Two-argument arctangent `atan2(self, x)`.
    #[must_use]
    fn atan2(&self, x: &Self) -> Self;

    // --- Hyperbolic ---
    /// Hyperbolic sine.
    #[must_use]
    fn sinh(&self) -> Self;
    /// Hyperbolic cosine.
    #[must_use]
    fn cosh(&self) -> Self;
    /// Hyperbolic tangent.
    #[must_use]
    fn tanh(&self) -> Self;
    /// Inverse hyperbolic sine.
    #[must_use]
    fn asinh(&self) -> Self;
    /// Inverse hyperbolic cosine.
    #[must_use]
    fn acosh(&self) -> Self;
    /// Inverse hyperbolic tangent.
    #[must_use]
    fn atanh(&self) -> Self;

    // --- Exponential & logarithmic ---
    /// `e^self`.
    #[must_use]
    fn exp(&self) -> Self;
    /// `e^self − 1` (accurate near zero).
    #[must_use]
    fn expm1(&self) -> Self;
    /// Natural logarithm.
    #[must_use]
    fn ln(&self) -> Self;
    /// `ln(1 + self)` (accurate near zero).
    #[must_use]
    fn log1p(&self) -> Self;

    // --- Special functions (scalar) ---
    /// Error function `erf(self)`.
    #[must_use]
    fn erf(&self) -> Self;
    /// Complementary error function `erfc(self) = 1 − erf(self)`.
    #[must_use]
    fn erfc(&self) -> Self;
    /// Euler Gamma function `Γ(self)`.
    #[must_use]
    fn gamma(&self) -> Self;
    /// Natural logarithm of the Gamma function `ln|Γ(self)|`.
    #[must_use]
    fn lgamma(&self) -> Self;
    /// Digamma function `ψ(self) = Γ′(self)/Γ(self)`.
    #[must_use]
    fn digamma(&self) -> Self;
    /// Trigamma function `ψ¹(self)`.
    #[must_use]
    fn trigamma(&self) -> Self;
    /// Tetragamma function `ψ²(self)`.
    #[must_use]
    fn tetragamma(&self) -> Self;
    /// Complete elliptic integral of the first kind `K(self)`.
    #[must_use]
    fn elliptic_k(&self) -> Self;
    /// Complete elliptic integral of the second kind `E(self)`.
    #[must_use]
    fn elliptic_e(&self) -> Self;
    /// Riemann Zeta function `ζ(self)`.
    #[must_use]
    fn zeta(&self) -> Self;
    /// Beta function `B(self, b)`.
    #[must_use]
    fn beta(&self, b: &Self) -> Self;

    // --- Special functions with integer parameters ---
    /// Lambert W function. `order`: 0 = principal W₀, −1 = W₋₁ branch.
    #[must_use]
    fn lambertw(&self, order: IntType) -> Self;
    /// Bessel function of the first kind `J_n(self)`.
    #[must_use]
    fn besselj(&self, n: IntType) -> Self;
    /// Bessel function of the second kind `Y_n(self)`.
    #[must_use]
    fn bessely(&self, n: IntType) -> Self;
    /// Modified Bessel function of the first kind `I_n(self)`.
    #[must_use]
    fn besseli(&self, n: IntType) -> Self;
    /// Modified Bessel function of the second kind `K_n(self)`.
    #[must_use]
    fn besselk(&self, n: IntType) -> Self;
    /// Polygamma function `ψ^(n)(self)`.
    #[must_use]
    fn polygamma(&self, n: IntType) -> Self;
    /// n-th derivative of the Riemann Zeta function at `self`.
    #[must_use]
    fn zeta_deriv(&self, n: IntType) -> Self;
    /// Physicist's Hermite polynomial `H_n(self)`.
    #[must_use]
    fn hermite(&self, n: IntType) -> Self;
    /// Associated Legendre polynomial `P_l^m(self)`.
    #[must_use]
    fn assoc_legendre(&self, l: IntType, m: IntType) -> Self;
    /// Real spherical harmonic `Y_l^m(theta=self, phi)`.
    #[must_use]
    fn spherical_harmonic(&self, l: IntType, m: IntType, phi: &Self) -> Self;
}

impl AnafisMathExt for FloatType {
    // --- Arithmetic ---
    #[inline]
    fn clone(&self) -> Self {
        crate::number::logic::float_ops::clone(self)
    }
    #[inline]
    fn add(&self, rhs: &Self) -> Self {
        crate::number::logic::float_ops::add(self, rhs)
    }
    #[inline]
    fn sub(&self, rhs: &Self) -> Self {
        crate::number::logic::float_ops::sub(self, rhs)
    }
    #[inline]
    fn mul(&self, rhs: &Self) -> Self {
        crate::number::logic::float_ops::mul(self, rhs)
    }
    #[inline]
    fn div(&self, rhs: &Self) -> Self {
        crate::number::logic::float_ops::div(self, rhs)
    }
    #[inline]
    fn neg(&self) -> Self {
        crate::number::logic::float_ops::neg(self)
    }

    // --- Basic math ---
    #[inline]
    fn abs(&self) -> Self {
        crate::number::logic::float_ops::abs(self)
    }
    #[inline]
    fn signum(&self) -> Self {
        crate::number::logic::float_ops::signum(self)
    }
    #[inline]
    fn floor(&self) -> Self {
        crate::number::logic::float_ops::floor(self)
    }
    #[inline]
    fn ceil(&self) -> Self {
        crate::number::logic::float_ops::ceil(self)
    }
    #[inline]
    fn round(&self) -> Self {
        crate::number::logic::float_ops::round(self)
    }
    #[inline]
    fn fract(&self) -> Self {
        crate::number::logic::float_ops::fract(self)
    }
    #[inline]
    fn pow(&self, rhs: &Self) -> Self {
        crate::number::logic::float_ops::pow(self, rhs)
    }
    #[inline]
    fn sqrt(&self) -> Self {
        crate::number::logic::float_ops::sqrt(self)
    }
    #[inline]
    fn cbrt(&self) -> Self {
        crate::number::logic::float_ops::cbrt(self)
    }

    // --- Trigonometric ---
    #[inline]
    fn sin(&self) -> Self {
        crate::number::logic::float_ops::sin(self)
    }
    #[inline]
    fn cos(&self) -> Self {
        crate::number::logic::float_ops::cos(self)
    }
    #[inline]
    fn tan(&self) -> Self {
        crate::number::logic::float_ops::tan(self)
    }
    #[inline]
    fn asin(&self) -> Self {
        crate::number::logic::float_ops::asin(self)
    }
    #[inline]
    fn acos(&self) -> Self {
        crate::number::logic::float_ops::acos(self)
    }
    #[inline]
    fn atan(&self) -> Self {
        crate::number::logic::float_ops::atan(self)
    }
    #[inline]
    fn atan2(&self, x: &Self) -> Self {
        crate::number::logic::float_ops::atan2(self, x)
    }

    // --- Hyperbolic ---
    #[inline]
    fn sinh(&self) -> Self {
        crate::number::logic::float_ops::sinh(self)
    }
    #[inline]
    fn cosh(&self) -> Self {
        crate::number::logic::float_ops::cosh(self)
    }
    #[inline]
    fn tanh(&self) -> Self {
        crate::number::logic::float_ops::tanh(self)
    }
    #[inline]
    fn asinh(&self) -> Self {
        crate::number::logic::float_ops::asinh(self)
    }
    #[inline]
    fn acosh(&self) -> Self {
        crate::number::logic::float_ops::acosh(self)
    }
    #[inline]
    fn atanh(&self) -> Self {
        crate::number::logic::float_ops::atanh(self)
    }

    // --- Exponential & logarithmic ---
    #[inline]
    fn exp(&self) -> Self {
        crate::number::logic::float_ops::exp(self)
    }
    #[inline]
    fn expm1(&self) -> Self {
        crate::number::logic::float_ops::expm1(self)
    }
    #[inline]
    fn ln(&self) -> Self {
        crate::number::logic::float_ops::ln(self)
    }
    #[inline]
    fn log1p(&self) -> Self {
        crate::number::logic::float_ops::log1p(self)
    }

    // --- Special functions (scalar) ---
    #[inline]
    fn erf(&self) -> Self {
        crate::number::logic::float_ops::erf(self)
    }
    #[inline]
    fn erfc(&self) -> Self {
        crate::number::logic::float_ops::erfc(self)
    }
    #[inline]
    fn gamma(&self) -> Self {
        crate::number::logic::float_ops::gamma(self)
    }
    #[inline]
    fn lgamma(&self) -> Self {
        crate::number::logic::float_ops::lgamma(self)
    }
    #[inline]
    fn digamma(&self) -> Self {
        crate::number::logic::float_ops::digamma(self)
    }
    #[inline]
    fn trigamma(&self) -> Self {
        crate::number::logic::float_ops::trigamma(self)
    }
    #[inline]
    fn tetragamma(&self) -> Self {
        crate::number::logic::float_ops::tetragamma(self)
    }
    #[inline]
    fn elliptic_k(&self) -> Self {
        crate::number::logic::float_ops::elliptic_k(self)
    }
    #[inline]
    fn elliptic_e(&self) -> Self {
        crate::number::logic::float_ops::elliptic_e(self)
    }
    #[inline]
    fn zeta(&self) -> Self {
        crate::number::logic::float_ops::zeta(self)
    }
    #[inline]
    fn beta(&self, b: &Self) -> Self {
        crate::number::logic::float_ops::beta(self, b)
    }

    // --- Special functions with integer parameters ---
    #[inline]
    fn lambertw(&self, order: IntType) -> Self {
        crate::number::logic::float_ops::lambertw(&order, self)
    }
    #[inline]
    fn besselj(&self, n: IntType) -> Self {
        crate::number::logic::float_ops::besselj(&n, self)
    }
    #[inline]
    fn bessely(&self, n: IntType) -> Self {
        crate::number::logic::float_ops::bessely(&n, self)
    }
    #[inline]
    fn besseli(&self, n: IntType) -> Self {
        crate::number::logic::float_ops::besseli(&n, self)
    }
    #[inline]
    fn besselk(&self, n: IntType) -> Self {
        crate::number::logic::float_ops::besselk(&n, self)
    }
    #[inline]
    fn polygamma(&self, n: IntType) -> Self {
        crate::number::logic::float_ops::polygamma(&n, self)
    }
    #[inline]
    fn zeta_deriv(&self, n: IntType) -> Self {
        crate::number::logic::float_ops::zeta_deriv(&n, self)
    }
    #[inline]
    fn hermite(&self, n: IntType) -> Self {
        crate::number::logic::float_ops::hermite(&n, self)
    }
    #[inline]
    fn assoc_legendre(&self, l: IntType, m: IntType) -> Self {
        crate::number::logic::float_ops::assoc_legendre(&l, &m, self)
    }
    #[inline]
    fn spherical_harmonic(&self, l: IntType, m: IntType, phi: &Self) -> Self {
        crate::number::logic::float_ops::spherical_harmonic(&l, &m, self, phi)
    }
}
