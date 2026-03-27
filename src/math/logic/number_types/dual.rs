//! Dual number implementation for automatic differentiation
//!
//! Dual numbers extend real numbers with an infinitesimal ε where ε² = 0.
//! A dual number a + bε carries both a value and its derivative.
//!
//! This enables exact derivatives through function composition:
//! f(a + ε) = f(a) + f'(a)ε

use crate::core::traits::MathScalar;
use crate::math::{
    bessel_j, eval_digamma, eval_elliptic_e, eval_elliptic_k, eval_erf, eval_erfc, eval_gamma,
    eval_lambert_w, eval_lgamma, eval_polygamma, eval_tetragamma, eval_trigamma, eval_zeta,
    eval_zeta_deriv,
};
use num_traits::{
    Bounded, Float, FloatConst, FromPrimitive, Num, NumCast, One, Signed, ToPrimitive, Zero,
};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::num::FpCategory;
use std::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd)]
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
    pub const fn new(val: T, eps: T) -> Self {
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

impl<T: MathScalar> Display for Dual<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} + {}\u{3b5}", self.val, self.eps)
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
impl<T: MathScalar> Signed for Dual<T> {
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
    fn classify(self) -> FpCategory {
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
        let n_t = T::from(n).expect("T::from conversion failed");
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
        Self::new(
            sqrt_val,
            self.eps / (T::from(2.0).expect("T::from conversion failed") * sqrt_val),
        )
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

    #[allow(
        clippy::suboptimal_flops,
        reason = "Dual number logarithm requires division for clarity"
    )]
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
        let three = T::from(3.0).expect("T::from conversion failed");
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
    ///
    /// # Panics
    /// Panics if `MathScalar::from(2.0)` returns `None` (violates `MathScalar` invariant).
    #[must_use]
    pub fn erf(self) -> Self {
        let val = eval_erf(self.val);
        #[allow(
            clippy::unwrap_used,
            clippy::panic,
            reason = "T::from(2.0) is a required invariant for MathScalar implementations"
        )]
        let two = T::from(2.0).expect("MathScalar invariant violated: T::from(2.0) failed");
        let pi = T::PI();
        let deriv = two / pi.sqrt() * (-self.val * self.val).exp();
        Self::new(val, self.eps * deriv)
    }

    /// Complementary error function: erfc(x) = 1 - erf(x)
    /// d/dx erfc(x) = -d/dx erf(x) = -(2/√π) * e^(-x²)
    ///
    /// # Panics
    /// Panics if `MathScalar::from(2.0)` returns `None` (violates `MathScalar` invariant).
    #[must_use]
    pub fn erfc(self) -> Self {
        let val = eval_erfc(self.val);
        #[allow(
            clippy::unwrap_used,
            clippy::panic,
            reason = "T::from(2.0) is a required invariant for MathScalar implementations"
        )]
        let two = T::from(2.0).expect("MathScalar invariant violated: T::from(2.0) failed");
        let pi = T::PI();
        let deriv = -two / pi.sqrt() * (-self.val * self.val).exp();
        Self::new(val, self.eps * deriv)
    }

    /// Gamma function: Γ(x)
    /// d/dx Γ(x) = Γ(x) * ψ(x) where ψ is the digamma function
    #[must_use]
    pub fn gamma(self) -> Self {
        let val = eval_gamma(self.val);
        let digamma_val = eval_digamma(self.val);
        let deriv = val * digamma_val;
        Self::new(val, self.eps * deriv)
    }

    /// Log-Gamma function: ln(Γ(x))
    /// d/dx ln(Γ(x)) = ψ(x) where ψ is the digamma function
    #[must_use]
    pub fn lgamma(self) -> Self {
        let val = eval_lgamma(self.val);
        let deriv = eval_digamma(self.val);
        Self::new(val, self.eps * deriv)
    }

    /// Digamma function: ψ(x) = d/dx ln(Γ(x)) = Γ'(x)/Γ(x)
    /// d/dx ψ(x) = ψ₁(x) (trigamma)
    #[must_use]
    pub fn digamma(self) -> Self {
        let val = eval_digamma(self.val);
        let deriv = eval_trigamma(self.val);
        Self::new(val, self.eps * deriv)
    }

    /// Trigamma function: ψ₁(x) = d/dx ψ(x)
    /// d/dx ψ₁(x) = ψ₂(x) (tetragamma)
    #[must_use]
    pub fn trigamma(self) -> Self {
        let val = eval_trigamma(self.val);
        let deriv = eval_tetragamma(self.val);
        Self::new(val, self.eps * deriv)
    }

    /// Polygamma function: ψₙ(x)
    /// d/dx ψₙ(x) = ψₙ₊₁(x)
    #[must_use]
    pub fn polygamma(self, n: i32) -> Self {
        if n < 0 {
            let nan = T::nan();
            return Self::new(nan, nan);
        }
        let val = eval_polygamma(n, self.val);
        let deriv = eval_polygamma(n + 1, self.val);
        Self::new(val, self.eps * deriv)
    }

    /// Riemann zeta function: ζ(x)
    /// d/dx ζ(x) = ζ'(x) (computed via `eval_zeta_deriv`)
    #[must_use]
    pub fn zeta(self) -> Self {
        let val = eval_zeta(self.val);
        let deriv = eval_zeta_deriv(1, self.val);
        Self::new(val, self.eps * deriv)
    }

    /// Lambert W function: W(x) where W(x) * e^W(x) = x
    /// d/dx W(x) = W(x) / (x * (1 + W(x)))
    ///
    /// # Panics
    /// Panics only if internal invariants are violated (never in normal use with f64).
    #[must_use]
    pub fn lambert_w(self) -> Self {
        let w = eval_lambert_w(self.val);
        // d/dx W(x) = W(x) / (x * (1 + W(x))) = 1 / (e^W(x) * (1 + W(x)))
        let one = T::one();
        let deriv = if self.val.abs() < T::from(1e-15).expect("T::from conversion failed") {
            one // W'(0) = 1
        } else {
            w / (self.val * (one + w))
        };
        Self::new(w, self.eps * deriv)
    }

    /// Bessel function of the first kind: `J_n(x)`
    /// d/dx `J_n(x)` = (J_{n-1}(x) - J_{n+1}(x)) / 2
    ///
    /// # Panics
    /// Panics only if internal invariants are violated (never in normal use with f64).
    #[must_use]
    pub fn bessel_j(self, n: i32) -> Self {
        let val = bessel_j(n, self.val);
        let jn_minus_1 = bessel_j(n - 1, self.val);
        let jn_plus_1 = bessel_j(n + 1, self.val);
        let two = T::from(2.0).expect("T::from conversion failed");
        let deriv = (jn_minus_1 - jn_plus_1) / two;
        Self::new(val, self.eps * deriv)
    }

    /// Sinc function: sinc(x) = sin(x)/x (with sinc(0) = 1)
    /// d/dx sinc(x) = (x*cos(x) - sin(x)) / x²
    ///
    /// # Panics
    /// Panics only if internal invariants are violated (never in normal use with f64).
    #[must_use]
    pub fn sinc(self) -> Self {
        let threshold = T::from(1e-10).expect("T::from conversion failed");
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
    #[must_use]
    pub fn sign(self) -> Self {
        Self::new(self.val.signum(), T::zero())
    }

    /// Elliptic integral of the first kind: K(k)
    /// Uses numerical differentiation since analytical is complex
    ///
    /// # Panics
    /// Panics only if internal invariants are violated (never in normal use with f64).
    #[must_use]
    pub fn elliptic_k(self) -> Self {
        let val = eval_elliptic_k(self.val);
        // Numerical derivative via finite difference
        let h = T::from(1e-8).expect("T::from conversion failed");
        let val_plus = eval_elliptic_k(self.val + h);
        let val_minus = eval_elliptic_k(self.val - h);
        let two = T::from(2.0).expect("T::from conversion failed");
        let deriv = (val_plus - val_minus) / (two * h);
        Self::new(val, self.eps * deriv)
    }

    /// Elliptic integral of the second kind: E(k)
    /// Uses numerical differentiation since analytical is complex
    ///
    /// # Panics
    /// Panics only if internal invariants are violated (never in normal use with f64).
    #[must_use]
    pub fn elliptic_e(self) -> Self {
        let val = eval_elliptic_e(self.val);
        // Numerical derivative via finite difference
        let h = T::from(1e-8).expect("T::from conversion failed");
        let val_plus = eval_elliptic_e(self.val + h);
        let val_minus = eval_elliptic_e(self.val - h);
        let two = T::from(2.0).expect("T::from conversion failed");
        let deriv = (val_plus - val_minus) / (two * h);
        Self::new(val, self.eps * deriv)
    }

    /// Beta function: B(a, b) = Γ(a)Γ(b)/Γ(a+b)
    /// d/da B(a, b) = B(a, b) * (ψ(a) - ψ(a+b))
    #[must_use]
    pub fn beta(self, b: Self) -> Self {
        let gamma_a = eval_gamma(self.val);
        let gamma_b = eval_gamma(b.val);
        let gamma_sum = eval_gamma(self.val + b.val);
        let val = gamma_a * gamma_b / gamma_sum;

        let psi_a = eval_digamma(self.val);
        let psi_b = eval_digamma(b.val);
        let psi_sum = eval_digamma(self.val + b.val);

        // d/da B(a,b) = B(a,b) * (ψ(a) - ψ(a+b))
        // d/db B(a,b) = B(a,b) * (ψ(b) - ψ(a+b))
        let deriv_a = val * (psi_a - psi_sum);
        let deriv_b = val * (psi_b - psi_sum);

        Self::new(val, self.eps * deriv_a + b.eps * deriv_b)
    }
}
