//! Dual number implementation for automatic differentiation
//!
//! Dual numbers extend real numbers with an infinitesimal ε where ε² = 0.
//! A dual number a + bε carries both a value and its derivative.
//!
//! This enables exact derivatives through function composition:
//! f(a + ε) = f(a) + f'(a)ε

use crate::core::traits::MathScalar;
use num_traits::{Bounded, Float, FloatConst, FromPrimitive, Num, NumCast, One, ToPrimitive, Zero};
use std::fmt;
use std::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
/// Dual number for automatic differentiation
///
/// A dual number `a + bε` represents both a value (`a`) and its derivative (`b`),
/// where `ε` is an infinitesimal satisfying `ε² = 0`.
///
/// # Automatic Differentiation
/// Dual numbers enable exact derivative computation through algebraic manipulation:
/// ```
/// use symb_anafis::Dual;
///
/// // f(x) = x², f'(x) = 2x
/// let x = Dual::new(3.0, 1.0);  // x + 1ε (derivative w.r.t. x is 1)
/// let fx = x * x;               // (x + ε)² = x² + 2xε
/// assert_eq!(fx.val, 9.0);      // f(3) = 9
/// assert_eq!(fx.eps, 6.0);      // f'(3) = 6
/// ```
///
/// # Type Parameters
/// * `T`: The scalar type (typically `f64`), must implement [`MathScalar`]
///
/// # Fields
/// * `val`: The real value component
/// * `eps`: The infinitesimal derivative component
pub struct Dual<T: MathScalar> {
    /// The real value component
    pub val: T,
    /// The infinitesimal derivative component
    pub eps: T,
}

impl<T: MathScalar> Dual<T> {
    /// Create a new dual number
    ///
    /// # Arguments
    /// * `val` - The real value component
    /// * `eps` - The infinitesimal derivative component
    ///
    /// # Example
    /// ```
    /// use symb_anafis::Dual;
    /// let x = Dual::new(2.0, 1.0);  // Represents x + ε
    /// ```
    #[inline]
    pub fn new(val: T, eps: T) -> Self {
        Self { val, eps }
    }

    /// Create a constant dual number (derivative = 0)
    ///
    /// # Arguments
    /// * `val` - The constant value
    ///
    /// # Example
    /// ```
    /// use symb_anafis::Dual;
    /// let c = Dual::constant(5.0);  // Represents 5 + 0ε
    /// assert_eq!(c.eps, 0.0);
    /// ```
    #[inline]
    pub fn constant(val: T) -> Self {
        Self {
            val,
            eps: T::zero(),
        }
    }
}

impl<T: MathScalar> fmt::Display for Dual<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} + {}ε", self.val, self.eps)
    }
}

// Basic Arithmetic

impl<T: MathScalar> Add for Dual<T> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.val + rhs.val, self.eps + rhs.eps)
    }
}

impl<T: MathScalar> Sub for Dual<T> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.val - rhs.val, self.eps - rhs.eps)
    }
}

impl<T: MathScalar> Mul for Dual<T> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        // Product rule
        Self::new(self.val * rhs.val, self.val * rhs.eps + self.eps * rhs.val)
    }
}

impl<T: MathScalar> Div for Dual<T> {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        // Quotient rule
        let val = self.val / rhs.val;
        let eps = (self.eps * rhs.val - self.val * rhs.eps) / (rhs.val * rhs.val);
        Self::new(val, eps)
    }
}

impl<T: MathScalar> Neg for Dual<T> {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.val, -self.eps)
    }
}

impl<T: MathScalar> Rem for Dual<T> {
    type Output = Self;
    fn rem(self, rhs: Self) -> Self {
        // IMPORTANT: The remainder operation is NOT differentiable!
        // It's discontinuous at integer multiples of the divisor.
        // The derivative is technically 0 almost everywhere, but undefined at jumps.
        // For AD purposes, we set eps = 0 to indicate non-differentiability.
        Self::new(self.val % rhs.val, T::zero())
    }
}

// Assignments

impl<T: MathScalar> AddAssign for Dual<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.val += rhs.val;
        self.eps += rhs.eps;
    }
}

impl<T: MathScalar> SubAssign for Dual<T> {
    fn sub_assign(&mut self, rhs: Self) {
        self.val -= rhs.val;
        self.eps -= rhs.eps;
    }
}

impl<T: MathScalar> MulAssign for Dual<T> {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl<T: MathScalar> DivAssign for Dual<T> {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl<T: MathScalar> RemAssign for Dual<T> {
    fn rem_assign(&mut self, rhs: Self) {
        *self = *self % rhs;
    }
}

// Traits for MathScalar

impl<T: MathScalar> Zero for Dual<T> {
    fn zero() -> Self {
        Self::constant(T::zero())
    }
    fn is_zero(&self) -> bool {
        self.val.is_zero() && self.eps.is_zero()
    }
}

impl<T: MathScalar> One for Dual<T> {
    fn one() -> Self {
        Self::constant(T::one())
    }
}

impl<T: MathScalar> Num for Dual<T> {
    type FromStrRadixErr = T::FromStrRadixErr;
    fn from_str_radix(str: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        Ok(Self::constant(T::from_str_radix(str, radix)?))
    }
}

impl<T: MathScalar> ToPrimitive for Dual<T> {
    fn to_i64(&self) -> Option<i64> {
        self.val.to_i64()
    }
    fn to_u64(&self) -> Option<u64> {
        self.val.to_u64()
    }
    fn to_f64(&self) -> Option<f64> {
        self.val.to_f64()
    }
}

impl<T: MathScalar> FromPrimitive for Dual<T> {
    fn from_i64(n: i64) -> Option<Self> {
        T::from_i64(n).map(Self::constant)
    }
    fn from_u64(n: u64) -> Option<Self> {
        T::from_u64(n).map(Self::constant)
    }
    fn from_f64(n: f64) -> Option<Self> {
        T::from_f64(n).map(Self::constant)
    }
}

impl<T: MathScalar> NumCast for Dual<T> {
    fn from<N: ToPrimitive>(n: N) -> Option<Self> {
        T::from(n).map(Self::constant)
    }
}

// Signed trait implementation using the correct abs from Float trait
impl<T: MathScalar> num_traits::Signed for Dual<T> {
    fn abs(&self) -> Self {
        // Use Float::abs which correctly implements d/dx|x| = sign(x)
        Float::abs(*self)
    }

    fn abs_sub(&self, other: &Self) -> Self {
        Float::abs_sub(*self, *other)
    }

    fn signum(&self) -> Self {
        Float::signum(*self)
    }

    fn is_positive(&self) -> bool {
        self.val > T::zero()
    }

    fn is_negative(&self) -> bool {
        self.val < T::zero()
    }
}

impl<T: MathScalar> Bounded for Dual<T> {
    fn min_value() -> Self {
        Self::constant(T::min_value())
    }
    fn max_value() -> Self {
        Self::constant(T::max_value())
    }
}

impl<T: MathScalar> FloatConst for Dual<T> {
    fn E() -> Self {
        Self::constant(T::E())
    }
    fn FRAC_1_PI() -> Self {
        Self::constant(T::FRAC_1_PI())
    }
    fn FRAC_1_SQRT_2() -> Self {
        Self::constant(T::FRAC_1_SQRT_2())
    }
    fn FRAC_2_PI() -> Self {
        Self::constant(T::FRAC_2_PI())
    }
    fn FRAC_2_SQRT_PI() -> Self {
        Self::constant(T::FRAC_2_SQRT_PI())
    }
    fn FRAC_PI_2() -> Self {
        Self::constant(T::FRAC_PI_2())
    }
    fn FRAC_PI_3() -> Self {
        Self::constant(T::FRAC_PI_3())
    }
    fn FRAC_PI_4() -> Self {
        Self::constant(T::FRAC_PI_4())
    }
    fn FRAC_PI_6() -> Self {
        Self::constant(T::FRAC_PI_6())
    }
    fn FRAC_PI_8() -> Self {
        Self::constant(T::FRAC_PI_8())
    }
    fn LN_10() -> Self {
        Self::constant(T::LN_10())
    }
    fn LN_2() -> Self {
        Self::constant(T::LN_2())
    }
    fn LOG10_2() -> Self {
        Self::constant(T::LOG10_2())
    }
    fn LOG10_E() -> Self {
        Self::constant(T::LOG10_E())
    }
    fn LOG2_10() -> Self {
        Self::constant(T::LOG2_10())
    }
    fn LOG2_E() -> Self {
        Self::constant(T::LOG2_E())
    }
    fn PI() -> Self {
        Self::constant(T::PI())
    }
    fn SQRT_2() -> Self {
        Self::constant(T::SQRT_2())
    }
}

impl<T: MathScalar + Float> Float for Dual<T> {
    fn nan() -> Self {
        Self::constant(T::nan())
    }
    fn infinity() -> Self {
        Self::constant(T::infinity())
    }
    fn neg_infinity() -> Self {
        Self::constant(T::neg_infinity())
    }
    fn neg_zero() -> Self {
        Self::constant(T::neg_zero())
    }
    fn min_value() -> Self {
        Self::constant(T::min_value())
    }
    fn max_value() -> Self {
        Self::constant(T::max_value())
    }
    fn min_positive_value() -> Self {
        Self::constant(T::min_positive_value())
    }
    fn is_nan(self) -> bool {
        self.val.is_nan()
    }
    fn is_infinite(self) -> bool {
        self.val.is_infinite()
    }
    fn is_finite(self) -> bool {
        self.val.is_finite()
    }
    fn is_normal(self) -> bool {
        self.val.is_normal()
    }
    fn classify(self) -> std::num::FpCategory {
        self.val.classify()
    }

    fn floor(self) -> Self {
        Self::constant(self.val.floor())
    }
    fn ceil(self) -> Self {
        Self::constant(self.val.ceil())
    }
    fn round(self) -> Self {
        Self::constant(self.val.round())
    }
    fn trunc(self) -> Self {
        Self::constant(self.val.trunc())
    }
    fn fract(self) -> Self {
        Self::new(self.val.fract(), self.eps)
    }

    fn abs(self) -> Self {
        let sign = if self.val >= T::zero() {
            T::one()
        } else {
            -T::one()
        };
        Self::new(self.val.abs(), self.eps * sign)
    }

    fn signum(self) -> Self {
        Self::constant(self.val.signum())
    }

    fn is_sign_positive(self) -> bool {
        self.val.is_sign_positive()
    }
    fn is_sign_negative(self) -> bool {
        self.val.is_sign_negative()
    }

    fn mul_add(self, a: Self, b: Self) -> Self {
        // self * a + b
        self * a + b
    }

    fn recip(self) -> Self {
        Self::one() / self
    }

    fn powi(self, n: i32) -> Self {
        let n_t = T::from(n).unwrap();
        let val_pow = self.val.powi(n);
        let val_pow_minus_1 = self.val.powi(n - 1);
        Self::new(val_pow, self.eps * n_t * val_pow_minus_1)
    }

    fn powf(self, n: Self) -> Self {
        // x^y = exp(y * ln(x))
        (n * self.ln()).exp()
    }

    fn sqrt(self) -> Self {
        let sqrt_val = self.val.sqrt();
        Self::new(sqrt_val, self.eps / (T::from(2.0).unwrap() * sqrt_val))
    }

    fn exp(self) -> Self {
        let exp_val = self.val.exp();
        Self::new(exp_val, self.eps * exp_val)
    }

    fn exp2(self) -> Self {
        let ln2 = T::LN_2();
        let exp2_val = self.val.exp2();
        Self::new(exp2_val, self.eps * exp2_val * ln2)
    }

    fn ln(self) -> Self {
        Self::new(self.val.ln(), self.eps / self.val)
    }

    fn log(self, base: Self) -> Self {
        self.ln() / base.ln()
    }

    fn log2(self) -> Self {
        self.ln() / Self::constant(T::LN_2())
    }

    fn log10(self) -> Self {
        self.ln() / Self::constant(T::LN_10())
    }

    fn max(self, other: Self) -> Self {
        if self.val > other.val { self } else { other }
    }

    fn min(self, other: Self) -> Self {
        if self.val < other.val { self } else { other }
    }

    fn abs_sub(self, other: Self) -> Self {
        if self.val <= other.val {
            Self::zero()
        } else {
            self - other
        }
    }

    fn cbrt(self) -> Self {
        let val_cbrt = self.val.cbrt();
        let three = T::from(3.0).unwrap();
        // d/dx x^(1/3) = 1/3 x^(-2/3) = 1/(3 * cbrt(x)^2)
        Self::new(val_cbrt, self.eps / (three * val_cbrt * val_cbrt))
    }

    fn hypot(self, other: Self) -> Self {
        (self * self + other * other).sqrt()
    }

    // Trig
    fn sin(self) -> Self {
        Self::new(self.val.sin(), self.eps * self.val.cos())
    }

    fn cos(self) -> Self {
        Self::new(self.val.cos(), -self.eps * self.val.sin())
    }

    fn tan(self) -> Self {
        let tan_val = self.val.tan();
        let sec2_val = T::one() + tan_val * tan_val;
        Self::new(tan_val, self.eps * sec2_val)
    }

    fn asin(self) -> Self {
        let one = T::one();
        let deriv = one / (one - self.val * self.val).sqrt();
        Self::new(self.val.asin(), self.eps * deriv)
    }

    fn acos(self) -> Self {
        let one = T::one();
        let deriv = -one / (one - self.val * self.val).sqrt();
        Self::new(self.val.acos(), self.eps * deriv)
    }

    fn atan(self) -> Self {
        let one = T::one();
        let deriv = one / (one + self.val * self.val);
        Self::new(self.val.atan(), self.eps * deriv)
    }

    fn atan2(self, other: Self) -> Self {
        // atan(y/x)
        let val = self.val.atan2(other.val);
        let r2 = self.val * self.val + other.val * other.val;
        let eps = (other.val * self.eps - self.val * other.eps) / r2;
        Self::new(val, eps)
    }

    fn sin_cos(self) -> (Self, Self) {
        (self.sin(), self.cos())
    }

    fn exp_m1(self) -> Self {
        self.exp() - Self::one()
    }

    fn ln_1p(self) -> Self {
        (self + Self::one()).ln()
    }

    fn sinh(self) -> Self {
        Self::new(self.val.sinh(), self.eps * self.val.cosh())
    }

    fn cosh(self) -> Self {
        Self::new(self.val.cosh(), self.eps * self.val.sinh())
    }

    fn tanh(self) -> Self {
        let tanh_val = self.val.tanh();
        let sech2_val = T::one() - tanh_val * tanh_val;
        Self::new(tanh_val, self.eps * sech2_val)
    }

    fn asinh(self) -> Self {
        let one = T::one();
        let deriv = one / (self.val * self.val + one).sqrt();
        Self::new(self.val.asinh(), self.eps * deriv)
    }

    fn acosh(self) -> Self {
        let one = T::one();
        let deriv = one / (self.val * self.val - one).sqrt();
        Self::new(self.val.acosh(), self.eps * deriv)
    }

    fn atanh(self) -> Self {
        let one = T::one();
        let deriv = one / (one - self.val * self.val);
        Self::new(self.val.atanh(), self.eps * deriv)
    }

    fn integer_decode(self) -> (u64, i16, i8) {
        self.val.integer_decode()
    }
}

// ============================================================================
// Special Function Implementations for Dual Numbers
// ============================================================================
// These enable automatic differentiation for special mathematical functions.
// Each function implements both the value and its derivative.
impl<T: MathScalar> Dual<T> {
    /// Error function: erf(x) = (2/√π) ∫₀ˣ e^(-t²) dt
    /// d/dx erf(x) = (2/√π) * e^(-x²)
    pub fn erf(self) -> Self {
        let val = crate::math::eval_erf(self.val);
        let two = T::from(2.0).unwrap();
        let pi = T::PI();
        let deriv = two / pi.sqrt() * (-self.val * self.val).exp();
        Self::new(val, self.eps * deriv)
    }

    /// Complementary error function: erfc(x) = 1 - erf(x)
    /// d/dx erfc(x) = -d/dx erf(x) = -(2/√π) * e^(-x²)
    pub fn erfc(self) -> Self {
        let erf_result = self.erf();
        Self::new(T::one() - erf_result.val, -erf_result.eps)
    }

    /// Gamma function: Γ(x)
    /// d/dx Γ(x) = Γ(x) * ψ(x) where ψ is the digamma function
    pub fn gamma(self) -> Option<Self> {
        let val = crate::math::eval_gamma(self.val)?;
        let digamma_val = crate::math::eval_digamma(self.val)?;
        let deriv = val * digamma_val;
        Some(Self::new(val, self.eps * deriv))
    }

    /// Digamma function: ψ(x) = d/dx ln(Γ(x)) = Γ'(x)/Γ(x)
    /// d/dx ψ(x) = ψ₁(x) (trigamma)
    pub fn digamma(self) -> Option<Self> {
        let val = crate::math::eval_digamma(self.val)?;
        let deriv = crate::math::eval_trigamma(self.val)?;
        Some(Self::new(val, self.eps * deriv))
    }

    /// Trigamma function: ψ₁(x) = d/dx ψ(x)
    /// d/dx ψ₁(x) = ψ₂(x) (tetragamma)
    pub fn trigamma(self) -> Option<Self> {
        let val = crate::math::eval_trigamma(self.val)?;
        let deriv = crate::math::eval_tetragamma(self.val)?;
        Some(Self::new(val, self.eps * deriv))
    }

    /// Polygamma function: ψₙ(x)
    /// d/dx ψₙ(x) = ψₙ₊₁(x)
    pub fn polygamma(self, n: i32) -> Option<Self> {
        let val = crate::math::eval_polygamma(n, self.val)?;
        let deriv = crate::math::eval_polygamma(n + 1, self.val)?;
        Some(Self::new(val, self.eps * deriv))
    }

    /// Riemann zeta function: ζ(x)
    /// d/dx ζ(x) = ζ'(x) (computed via eval_zeta_deriv)
    pub fn zeta(self) -> Option<Self> {
        let val = crate::math::eval_zeta(self.val)?;
        let deriv = crate::math::eval_zeta_deriv(1, self.val)?;
        Some(Self::new(val, self.eps * deriv))
    }

    /// Lambert W function: W(x) where W(x) * e^W(x) = x
    /// d/dx W(x) = W(x) / (x * (1 + W(x)))
    pub fn lambert_w(self) -> Option<Self> {
        let w = crate::math::eval_lambert_w(self.val)?;
        // d/dx W(x) = W(x) / (x * (1 + W(x))) = 1 / (e^W(x) * (1 + W(x)))
        let one = T::one();
        let deriv = if self.val.abs() < T::from(1e-15).unwrap() {
            one // W'(0) = 1
        } else {
            w / (self.val * (one + w))
        };
        Some(Self::new(w, self.eps * deriv))
    }

    /// Bessel function of the first kind: J_n(x)
    /// d/dx J_n(x) = (J_{n-1}(x) - J_{n+1}(x)) / 2
    pub fn bessel_j(self, n: i32) -> Option<Self> {
        let val = crate::math::bessel_j(n, self.val)?;
        let jn_minus_1 = crate::math::bessel_j(n - 1, self.val)?;
        let jn_plus_1 = crate::math::bessel_j(n + 1, self.val)?;
        let two = T::from(2.0).unwrap();
        let deriv = (jn_minus_1 - jn_plus_1) / two;
        Some(Self::new(val, self.eps * deriv))
    }

    /// Sinc function: sinc(x) = sin(x)/x (with sinc(0) = 1)
    /// d/dx sinc(x) = (x*cos(x) - sin(x)) / x²
    pub fn sinc(self) -> Self {
        let threshold = T::from(1e-10).unwrap();
        if self.val.abs() < threshold {
            // sinc(0) = 1, sinc'(0) = 0
            Self::new(T::one(), T::zero())
        } else {
            let sin_x = self.val.sin();
            let cos_x = self.val.cos();
            let val = sin_x / self.val;
            let deriv = (self.val * cos_x - sin_x) / (self.val * self.val);
            Self::new(val, self.eps * deriv)
        }
    }

    /// Sign function: sign(x) = x/|x| for x ≠ 0
    /// d/dx sign(x) = 0 (almost everywhere, undefined at 0)
    pub fn sign(self) -> Self {
        Self::new(self.val.signum(), T::zero())
    }

    /// Elliptic integral of the first kind: K(k)
    /// Uses numerical differentiation since analytical is complex
    pub fn elliptic_k(self) -> Option<Self> {
        let val = crate::math::eval_elliptic_k(self.val)?;
        // Numerical derivative via finite difference
        let h = T::from(1e-8).unwrap();
        let val_plus = crate::math::eval_elliptic_k(self.val + h)?;
        let val_minus = crate::math::eval_elliptic_k(self.val - h)?;
        let two = T::from(2.0).unwrap();
        let deriv = (val_plus - val_minus) / (two * h);
        Some(Self::new(val, self.eps * deriv))
    }

    /// Elliptic integral of the second kind: E(k)
    /// Uses numerical differentiation since analytical is complex
    pub fn elliptic_e(self) -> Option<Self> {
        let val = crate::math::eval_elliptic_e(self.val)?;
        // Numerical derivative via finite difference
        let h = T::from(1e-8).unwrap();
        let val_plus = crate::math::eval_elliptic_e(self.val + h)?;
        let val_minus = crate::math::eval_elliptic_e(self.val - h)?;
        let two = T::from(2.0).unwrap();
        let deriv = (val_plus - val_minus) / (two * h);
        Some(Self::new(val, self.eps * deriv))
    }

    /// Beta function: B(a, b) = Γ(a)Γ(b)/Γ(a+b)
    /// d/da B(a, b) = B(a, b) * (ψ(a) - ψ(a+b))
    pub fn beta(self, b: Self) -> Option<Self> {
        let gamma_a = crate::math::eval_gamma(self.val)?;
        let gamma_b = crate::math::eval_gamma(b.val)?;
        let gamma_ab = crate::math::eval_gamma(self.val + b.val)?;
        let val = gamma_a * gamma_b / gamma_ab;

        let psi_a = crate::math::eval_digamma(self.val)?;
        let psi_b = crate::math::eval_digamma(b.val)?;
        let psi_ab = crate::math::eval_digamma(self.val + b.val)?;

        // d/da B(a,b) = B(a,b) * (ψ(a) - ψ(a+b))
        // d/db B(a,b) = B(a,b) * (ψ(b) - ψ(a+b))
        let deriv_a = val * (psi_a - psi_ab);
        let deriv_b = val * (psi_b - psi_ab);

        Some(Self::new(val, self.eps * deriv_a + b.eps * deriv_b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }

    #[test]
    fn test_dual_basic_arithmetic() {
        // x = 2, dx = 1 (variable)
        let x = Dual::new(2.0, 1.0);
        // c = 3, dc = 0 (constant)
        let c = Dual::constant(3.0);

        // x + c = 5, derivative = 1
        let sum = x + c;
        assert!(approx_eq(sum.val, 5.0));
        assert!(approx_eq(sum.eps, 1.0));

        // x - c = -1, derivative = 1
        let diff = x - c;
        assert!(approx_eq(diff.val, -1.0));
        assert!(approx_eq(diff.eps, 1.0));

        // x * c = 6, d/dx(3x) = 3
        let prod = x * c;
        assert!(approx_eq(prod.val, 6.0));
        assert!(approx_eq(prod.eps, 3.0));

        // x / c = 2/3, d/dx(x/3) = 1/3
        let quot = x / c;
        assert!(approx_eq(quot.val, 2.0 / 3.0));
        assert!(approx_eq(quot.eps, 1.0 / 3.0));
    }

    #[test]
    fn test_dual_product_rule() {
        // f(x) = x * x = x^2, f'(x) = 2x
        let x = Dual::new(3.0, 1.0);
        let x_squared = x * x;
        assert!(approx_eq(x_squared.val, 9.0));
        assert!(approx_eq(x_squared.eps, 6.0)); // 2 * 3
    }

    #[test]
    fn test_dual_quotient_rule() {
        // f(x) = x / (x + 1), f'(x) = 1/(x+1)^2
        let x = Dual::new(2.0, 1.0);
        let one = Dual::constant(1.0);
        let result = x / (x + one);

        assert!(approx_eq(result.val, 2.0 / 3.0));
        // f'(2) = 1/(3)^2 = 1/9
        assert!(approx_eq(result.eps, 1.0 / 9.0));
    }

    #[test]
    fn test_dual_sin_cos() {
        use std::f64::consts::PI;

        // At x = π/4: sin(π/4) = √2/2, cos(π/4) = √2/2
        let x = Dual::new(PI / 4.0, 1.0);

        let sin_x = x.sin();
        let cos_x = x.cos();

        let sqrt2_2 = 2.0_f64.sqrt() / 2.0;
        assert!(approx_eq(sin_x.val, sqrt2_2));
        assert!(approx_eq(sin_x.eps, sqrt2_2)); // d/dx sin(x) = cos(x)

        assert!(approx_eq(cos_x.val, sqrt2_2));
        assert!(approx_eq(cos_x.eps, -sqrt2_2)); // d/dx cos(x) = -sin(x)
    }

    #[test]
    fn test_dual_exp_ln() {
        // At x = 1: exp(1) = e, d/dx exp(x) = exp(x)
        let x = Dual::new(1.0, 1.0);
        let exp_x = x.exp();

        assert!(approx_eq(exp_x.val, std::f64::consts::E));
        assert!(approx_eq(exp_x.eps, std::f64::consts::E)); // d/dx exp(x) = exp(x)

        // At x = e: ln(e) = 1, d/dx ln(x) = 1/x
        let y = Dual::new(std::f64::consts::E, 1.0);
        let ln_y = y.ln();

        assert!(approx_eq(ln_y.val, 1.0));
        assert!(approx_eq(ln_y.eps, 1.0 / std::f64::consts::E)); // d/dx ln(x) = 1/x
    }

    #[test]
    fn test_dual_sqrt() {
        // At x = 4: sqrt(4) = 2, d/dx sqrt(x) = 1/(2*sqrt(x)) = 1/4
        let x = Dual::new(4.0, 1.0);
        let sqrt_x = x.sqrt();

        assert!(approx_eq(sqrt_x.val, 2.0));
        assert!(approx_eq(sqrt_x.eps, 0.25));
    }

    #[test]
    fn test_dual_powi() {
        // At x = 2: x^3 = 8, d/dx x^3 = 3x^2 = 12
        let x = Dual::new(2.0, 1.0);
        let x_cubed = x.powi(3);

        assert!(approx_eq(x_cubed.val, 8.0));
        assert!(approx_eq(x_cubed.eps, 12.0));
    }

    #[test]
    fn test_dual_chain_rule() {
        // f(x) = sin(x^2), f'(x) = 2x * cos(x^2)
        let x = Dual::new(2.0, 1.0);
        let x_squared = x * x;
        let result = x_squared.sin();

        assert!(approx_eq(result.val, 4.0_f64.sin()));
        // f'(2) = 2*2 * cos(4) = 4 * cos(4)
        assert!(approx_eq(result.eps, 4.0 * 4.0_f64.cos()));
    }

    #[test]
    fn test_dual_tan() {
        use std::f64::consts::PI;

        // At x = π/4: tan(π/4) = 1, d/dx tan(x) = sec^2(x) = 2
        let x = Dual::new(PI / 4.0, 1.0);
        let tan_x = x.tan();

        assert!(approx_eq(tan_x.val, 1.0));
        assert!(approx_eq(tan_x.eps, 2.0)); // sec^2(π/4) = 1/cos^2(π/4) = 2
    }

    #[test]
    fn test_dual_hyperbolic() {
        // At x = 0: sinh(0) = 0, cosh(0) = 1, tanh(0) = 0
        // d/dx sinh(x) = cosh(x) = 1 at x=0
        // d/dx cosh(x) = sinh(x) = 0 at x=0
        // d/dx tanh(x) = sech^2(x) = 1 at x=0
        let x = Dual::new(0.0, 1.0);

        let sinh_x = x.sinh();
        assert!(approx_eq(sinh_x.val, 0.0));
        assert!(approx_eq(sinh_x.eps, 1.0));

        let cosh_x = x.cosh();
        assert!(approx_eq(cosh_x.val, 1.0));
        assert!(approx_eq(cosh_x.eps, 0.0));

        let tanh_x = x.tanh();
        assert!(approx_eq(tanh_x.val, 0.0));
        assert!(approx_eq(tanh_x.eps, 1.0));
    }

    #[test]
    fn test_dual_erf() {
        // d/dx erf(x) = (2/√π) * e^(-x²)
        let x = Dual::new(0.0, 1.0);
        let result = x.erf();

        assert!(approx_eq(result.val, 0.0)); // erf(0) = 0
        // erf'(0) = 2/√π ≈ 1.1284
        let expected_deriv = 2.0 / std::f64::consts::PI.sqrt();
        assert!(approx_eq(result.eps, expected_deriv));
    }

    #[test]
    fn test_dual_gamma() {
        // Γ(1) = 1, Γ'(1) = -γ (Euler-Mascheroni constant, ≈ -0.5772)
        let x = Dual::new(2.0, 1.0);
        let result = x.gamma().unwrap();

        assert!(approx_eq(result.val, 1.0)); // Γ(2) = 1! = 1
        // Γ'(2) = Γ(2) * ψ(2) = 1 * (1 - γ) ≈ 0.4228
        assert!(result.eps > 0.0 && result.eps < 1.0);
    }

    #[test]
    fn test_dual_sinc() {
        // sinc(0) = 1, sinc'(0) = 0
        let x = Dual::new(0.0, 1.0);
        let result = x.sinc();

        assert!(approx_eq(result.val, 1.0));
        assert!(approx_eq(result.eps, 0.0));

        // At x = π: sinc(π) = 0, sinc'(π) = -1/π
        let x_pi = Dual::new(std::f64::consts::PI, 1.0);
        let result_pi = x_pi.sinc();
        assert!(result_pi.val.abs() < 1e-10);
    }

    #[test]
    fn test_dual_lambert_w() {
        // W(0) = 0, W'(0) = 1
        let x = Dual::new(1.0, 1.0);
        let result = x.lambert_w().unwrap();

        // W(1) ≈ 0.5671
        assert!(result.val > 0.5 && result.val < 0.6);
        // W'(1) = W(1) / (1 * (1 + W(1)))
        let expected_deriv = result.val / (1.0 * (1.0 + result.val));
        assert!((result.eps - expected_deriv).abs() < 1e-8);
    }
}
