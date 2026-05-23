//! Special mathematical functions — generic over float and integer types.
//!
//! Each sub-module works via the `SpecFloat` and `SpecInt` traits so that
//! algorithms are backend-switchable with precision-specific constants
//! behind compilation walls.

mod bessel_ik;
mod bessel_jy;
mod beta;
mod complex;
mod elliptic;
mod erf;
mod gamma;
mod helpers;
mod hermite;
mod lambert_w;
mod legendre;
mod polygamma;
mod zeta;
mod zeta_deriv;

pub use bessel_ik::{bessel_i, bessel_k};
pub use bessel_jy::{bessel_j, bessel_y};
pub use beta::beta;
pub use elliptic::{elliptic_e, elliptic_k};
pub use erf::{erf, erfc};
pub use gamma::{gamma, lgamma};
pub use hermite::hermite;
pub use lambert_w::{lambert_w0, lambert_wm1};
pub use legendre::{assoc_legendre, spherical_harmonic};
pub use polygamma::{digamma, polygamma_n, tetragamma, trigamma};
pub use zeta::zeta;
pub use zeta_deriv::zeta_deriv;

// ============================================================================
// SpecInt — integer abstraction
// ============================================================================

pub trait SpecInt:
    Copy
    + PartialEq
    + PartialOrd
    + core::ops::Add<Output = Self>
    + core::ops::Sub<Output = Self>
    + core::ops::Mul<Output = Self>
    + core::ops::Div<Output = Self>
    + core::ops::Rem<Output = Self>
    + core::fmt::Debug
    + 'static
{
    #[allow(
        dead_code,
        reason = "Used via macro-generated from_usize on concrete types"
    )]
    const MAX: Self;

    fn zero() -> Self;
    fn one() -> Self;
    fn abs(self) -> Self;
    fn is_zero(self) -> bool;
    fn is_negative(self) -> bool;
    fn to_usize(self) -> usize;
    fn from_usize(v: usize) -> Self;
}

// ============================================================================
// SpecFloat — float abstraction
// ============================================================================

pub trait SpecFloat:
    Copy
    + PartialEq
    + PartialOrd
    + core::ops::Add<Output = Self>
    + core::ops::Sub<Output = Self>
    + core::ops::Mul<Output = Self>
    + core::ops::Div<Output = Self>
    + core::ops::Neg<Output = Self>
    + 'static
{
    type Int: SpecInt;

    fn eps() -> Self;
    const ERF_TERMS: usize;
    const DIGAMMA_SHIFT: usize;
    const LAMBERT_MAX_ITERATIONS: usize;
    const ZETA_BORWEIN_N: usize;
    const ZETA_EM_TERMS: usize;

    fn zero() -> Self;
    fn one() -> Self;
    fn neg_one() -> Self {
        -Self::one()
    }
    fn two() -> Self {
        Self::one() + Self::one()
    }
    fn half() -> Self {
        Self::one() / Self::two()
    }
    fn pi() -> Self;
    fn e() -> Self;
    fn frac_2_pi() -> Self;
    fn infinity() -> Self;
    fn neg_infinity() -> Self {
        -Self::infinity()
    }
    fn nan() -> Self;
    fn max_value() -> Self;

    // Cody-Waite range reduction constants for π/4 and 3π/4.
    fn pio4_hi() -> Self;
    fn pio4_lo() -> Self;
    fn pio34_hi() -> Self;
    fn pio34_lo() -> Self;

    fn from_int<I: SpecInt>(v: I) -> Self {
        let a = v.abs();
        let f = Self::from_usize(a.to_usize());
        if v.is_negative() { -f } else { f }
    }
    fn from_usize(v: usize) -> Self;
    fn to_int(self) -> Option<Self::Int>;

    fn abs(self) -> Self;
    fn signum(self) -> Self;
    fn sqrt(self) -> Self;
    fn sin(self) -> Self;
    fn cos(self) -> Self;
    fn atan2(self, other: Self) -> Self;
    fn max(self, other: Self) -> Self;
    fn mul_add(self, a: Self, b: Self) -> Self;
    fn exp(self) -> Self;
    fn ln(self) -> Self;
    fn fract(self) -> Self;
    fn round(self) -> Self;
    fn floor(self) -> Self;
    fn powf(self, exp: Self) -> Self;
    fn pow_int<I: SpecInt>(self, n: I) -> Self;

    fn is_nan(self) -> bool;
    fn is_infinite(self) -> bool;
    fn is_sign_negative(self) -> bool;
    fn is_sign_positive(self) -> bool {
        !self.is_sign_negative() && !self.is_nan()
    }

    // --- Coefficient arrays for special functions ---
    // Each backend stores these as const arrays in native precision.

    fn lanczos_coeffs() -> &'static [Self];
    fn bernoulli_pairs() -> &'static [(Self, Self)];

    fn stieltjes_coeffs() -> &'static [Self];
    fn stirling_coeffs() -> &'static [Self];
    fn zeta_ints() -> &'static [Self];

    fn bessel_j0_num_coeffs() -> &'static [Self];
    fn bessel_j0_den_coeffs() -> &'static [Self];
    fn bessel_j0_pcos_coeffs() -> &'static [Self];
    fn bessel_j0_psin_coeffs() -> &'static [Self];

    fn bessel_j1_num_coeffs() -> &'static [Self];
    fn bessel_j1_den_coeffs() -> &'static [Self];
    fn bessel_j1_pcos_coeffs() -> &'static [Self];
    fn bessel_j1_psin_coeffs() -> &'static [Self];

    fn bessel_y0_num_coeffs() -> &'static [Self];
    fn bessel_y0_den_coeffs() -> &'static [Self];
    fn bessel_y0_pcos_coeffs() -> &'static [Self];
    fn bessel_y0_psin_coeffs() -> &'static [Self];

    fn bessel_y1_num_coeffs() -> &'static [Self];
    fn bessel_y1_den_coeffs() -> &'static [Self];
    fn bessel_y1_pcos_coeffs() -> &'static [Self];
    fn bessel_y1_psin_coeffs() -> &'static [Self];

    fn bessel_i0_small_coeffs() -> &'static [Self];
    fn bessel_i0_large_coeffs() -> &'static [Self];

    fn bessel_i1_small_coeffs() -> &'static [Self];
    fn bessel_i1_large_coeffs() -> &'static [Self];

    fn bessel_k0_small_coeffs() -> &'static [Self];
    fn bessel_k0_large_coeffs() -> &'static [Self];

    fn bessel_k1_small_coeffs() -> &'static [Self];
    fn bessel_k1_large_coeffs() -> &'static [Self];

    fn bessel_j_split() -> Self;
    fn bessel_miller_seed() -> Self;

    fn bessel_j0_root1() -> Self;
    fn bessel_j0_root2() -> Self;
    fn bessel_j1_root1() -> Self;
    fn bessel_j1_root2() -> Self;
}

// ============================================================================
// Internal Macros — used to implement traits in sibling modules
// ============================================================================

/// Implements the [`SpecInt`] trait for a concrete integer type.
macro_rules! impl_spec_int {
    ($ty:ty) => {
        impl $crate::number::logic::float_ops::special::SpecInt for $ty {
            const MAX: Self = <$ty>::MAX;

            #[inline]
            fn zero() -> Self {
                0
            }
            #[inline]
            fn one() -> Self {
                1
            }
            #[inline]
            fn abs(self) -> Self {
                <$ty>::abs(self)
            }
            #[inline]
            fn is_zero(self) -> bool {
                self == 0
            }
            #[inline]
            fn is_negative(self) -> bool {
                self < 0
            }
            #[inline]
            fn to_usize(self) -> usize {
                debug_assert!(
                    !self.is_negative(),
                    "SpecInt::to_usize called with negative value"
                );
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "Callers guarantee non-negative values within usize range"
                )]
                {
                    self as usize
                }
            }
            #[inline]
            fn from_usize(v: usize) -> Self {
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "MAX checked against usize; only relevant on 32-bit targets"
                )]
                let max_usize = Self::MAX as usize;
                debug_assert!(
                    v <= max_usize,
                    "SpecInt::from_usize: value {v} exceeds target type range"
                );
                #[allow(
                    clippy::cast_possible_wrap,
                    clippy::cast_possible_truncation,
                    reason = "Callers guarantee value fits; debug_assert catches violations"
                )]
                {
                    v as Self
                }
            }
        }
    };
}

pub(in crate::number::logic::float_ops) use impl_spec_int;

/// Implements the [`SpecFloat`] trait for a concrete float type,
/// including all coefficient arrays for special functions.
macro_rules! impl_spec_float {
    (
        $ty:ty,
        $int:ty,
        pi_val = $pi:expr,
        e_val = $e:expr,
        f2pi_val = $f2pi:expr,
        erf_terms = $erf:expr,
        digamma_shift = $dshift:expr,
        lambert_max_iterations = $liter:expr,
        zeta_borwein_n = $zbn:expr,
        zeta_em_terms = $zem:expr,
        pio4_hi = $p4hi:expr,
        pio4_lo = $p4lo:expr,
        pio34_hi = $p34hi:expr,
        pio34_lo = $p34lo:expr,
        bessel_j_split = $bjsplit:expr,
        bessel_miller_seed = $seed:expr,
        bessel_j0_root1 = $bj0r1:expr,
        bessel_j0_root2 = $bj0r2:expr,
        bessel_j1_root1 = $bj1r1:expr,
        bessel_j1_root2 = $bj1r2:expr,
        lanczos: [$($lc:expr),* $(,)?],
        bernoulli: [$($bn:expr, $bd:expr);* $(;)?],
        stieltjes: [$($st:expr),* $(,)?],
        zeta_ints: [$($zi:expr),* $(,)?],
        stirling: [$($sc:expr),* $(,)?],
        bessel_j0_num: [$($bjn0:expr),* $(,)?],
        bessel_j0_den: [$($bjd0:expr),* $(,)?],
        bessel_j0_pcos: [$($bjpc0:expr),* $(,)?],
        bessel_j0_psin: [$($bjps0:expr),* $(,)?],
        bessel_j1_num: [$($bjn1:expr),* $(,)?],
        bessel_j1_den: [$($bjd1:expr),* $(,)?],
        bessel_j1_pcos: [$($bjpc1:expr),* $(,)?],
        bessel_j1_psin: [$($bjps1:expr),* $(,)?],
        bessel_y0_num: [$($byn0:expr),* $(,)?],
        bessel_y0_den: [$($byd0:expr),* $(,)?],
        bessel_y0_pcos: [$($bypc0:expr),* $(,)?],
        bessel_y0_psin: [$($byps0:expr),* $(,)?],
        bessel_y1_num: [$($byn1:expr),* $(,)?],
        bessel_y1_den: [$($byd1:expr),* $(,)?],
        bessel_y1_pcos: [$($bypc1:expr),* $(,)?],
        bessel_y1_psin: [$($byps1:expr),* $(,)?],
        bessel_i0_small: [$($bi0s:expr),* $(,)?],
        bessel_i0_large: [$($bi0l:expr),* $(,)?],
        bessel_i1_small: [$($bi1bs:expr),* $(,)?],
        bessel_i1_large: [$($bi1bl:expr),* $(,)?],
        bessel_k0_small: [$($bk0s:expr),* $(,)?],
        bessel_k0_large: [$($bk0l:expr),* $(,)?],
        bessel_k1_small: [$($bk1s:expr),* $(,)?],
        bessel_k1_large: [$($bk1l:expr),* $(,)?],
    ) => {
        impl $crate::number::logic::float_ops::special::SpecFloat for $ty {
            type Int = $int;

            #[inline] fn eps() -> Self { <$ty>::EPSILON }
            const ERF_TERMS: usize = $erf;
            const DIGAMMA_SHIFT: usize = $dshift;
            const LAMBERT_MAX_ITERATIONS: usize = $liter;
            const ZETA_BORWEIN_N: usize = $zbn;
            const ZETA_EM_TERMS: usize = $zem;

            #[inline] fn zero() -> Self { 0.0 }
            #[inline] fn one() -> Self { 1.0 }
            #[inline] fn pi() -> Self { $pi }
            #[inline] fn e() -> Self { $e }
            #[inline] fn frac_2_pi() -> Self { $f2pi }
            #[inline] fn infinity() -> Self { <$ty>::INFINITY }
            #[inline] fn nan() -> Self { <$ty>::NAN }
            #[inline] fn max_value() -> Self { <$ty>::MAX }

            // Cody-Waite range reduction constants for π/4 and 3π/4
            #[inline] fn pio4_hi() -> Self { $p4hi }
            #[inline] fn pio4_lo() -> Self { $p4lo }
            #[inline] fn pio34_hi() -> Self { $p34hi }
            #[inline] fn pio34_lo() -> Self { $p34lo }

            #[inline]
            #[allow(
                clippy::cast_precision_loss,
                trivial_numeric_casts,
                reason = "usize-to-float; cast needed for generic macro support"
            )]
            fn from_usize(v: usize) -> Self { v as Self }

            #[inline]
            #[allow(
                clippy::cast_possible_truncation,
                trivial_numeric_casts,
                reason = "Checked via floor; cast needed for generic macro support"
            )]
            fn to_int(self) -> Option<$int> {
                if (self - self.floor()).abs() < Self::EPSILON {
                    #[allow(
                        trivial_numeric_casts,
                        clippy::cast_precision_loss,
                        reason = "Bounds check ensures value fits; cast needed for generic macro"
                    )]
                    {
                        let max_f = <$int>::MAX as Self;
                        let min_f = <$int>::MIN as Self;
                        if self >= min_f && self < max_f + Self::one() {
                            return Some(self as $int);
                        }
                    }
                }
                None
            }

            #[inline] fn abs(self) -> Self { math::fabs(self) }
            #[inline] fn signum(self) -> Self { self.signum() }
            #[inline] fn sqrt(self) -> Self { math::sqrt(self) }
            #[inline] fn sin(self) -> Self { math::sin(self) }
            #[inline] fn cos(self) -> Self { math::cos(self) }
            #[inline] fn atan2(self, other: Self) -> Self { math::atan2(self, other) }
            #[inline] fn max(self, other: Self) -> Self { <$ty>::max(self, other) }
            #[inline] fn mul_add(self, a: Self, b: Self) -> Self { <$ty>::mul_add(self, a, b) }
            #[inline] fn exp(self) -> Self { math::exp(self) }
            #[inline] fn ln(self) -> Self { math::ln(self) }
            #[inline] fn fract(self) -> Self { self.fract() }
            #[inline] fn round(self) -> Self { math::round(self) }
            #[inline] fn floor(self) -> Self { math::floor(self) }
            #[inline] fn powf(self, exp: Self) -> Self { math::pow(self, exp) }
            #[inline]
            fn pow_int<I: $crate::number::logic::float_ops::special::SpecInt>(self, n: I) -> Self {
                let mut result = Self::one();
                let mut base = self;
                let mut e = n.abs();
                let two = I::from_usize(2);
                while !e.is_zero() {
                    if (e % two) != I::zero() {
                        result *= base;
                    }
                    base *= base;
                    e = e / two;
                }
                if n.is_negative() { Self::one() / result } else { result }
            }
            #[inline] fn is_nan(self) -> bool { <$ty>::is_nan(self) }
            #[inline] fn is_infinite(self) -> bool { <$ty>::is_infinite(self) }
            #[inline] fn is_sign_negative(self) -> bool { <$ty>::is_sign_negative(self) }

            // --- Coefficient arrays — stored as native-precision consts ---

            #[inline]
            fn lanczos_coeffs() -> &'static [Self] { &[$($lc),*] }
            #[inline]
            fn bernoulli_pairs() -> &'static [(Self, Self)] { &[$(($bn,$bd)),*] }
            #[inline]
            fn stieltjes_coeffs() -> &'static [Self] { &[$($st),*] }
            #[inline]
            fn stirling_coeffs() -> &'static [Self] { &[$($sc),*] }
            #[inline]
            fn zeta_ints() -> &'static [Self] { &[$($zi),*] }

            #[inline] fn bessel_j0_num_coeffs() -> &'static [Self] { &[$($bjn0),*] }
            #[inline] fn bessel_j0_den_coeffs() -> &'static [Self] { &[$($bjd0),*] }
            #[inline] fn bessel_j0_pcos_coeffs() -> &'static [Self] { &[$($bjpc0),*] }
            #[inline] fn bessel_j0_psin_coeffs() -> &'static [Self] { &[$($bjps0),*] }

            #[inline] fn bessel_j1_num_coeffs() -> &'static [Self] { &[$($bjn1),*] }
            #[inline] fn bessel_j1_den_coeffs() -> &'static [Self] { &[$($bjd1),*] }
            #[inline] fn bessel_j1_pcos_coeffs() -> &'static [Self] { &[$($bjpc1),*] }
            #[inline] fn bessel_j1_psin_coeffs() -> &'static [Self] { &[$($bjps1),*] }

            #[inline] fn bessel_y0_num_coeffs() -> &'static [Self] { &[$($byn0),*] }
            #[inline] fn bessel_y0_den_coeffs() -> &'static [Self] { &[$($byd0),*] }
            #[inline] fn bessel_y0_pcos_coeffs() -> &'static [Self] { &[$($bypc0),*] }
            #[inline] fn bessel_y0_psin_coeffs() -> &'static [Self] { &[$($byps0),*] }

            #[inline] fn bessel_y1_num_coeffs() -> &'static [Self] { &[$($byn1),*] }
            #[inline] fn bessel_y1_den_coeffs() -> &'static [Self] { &[$($byd1),*] }
            #[inline] fn bessel_y1_pcos_coeffs() -> &'static [Self] { &[$($bypc1),*] }
            #[inline] fn bessel_y1_psin_coeffs() -> &'static [Self] { &[$($byps1),*] }

            #[inline] fn bessel_i0_small_coeffs() -> &'static [Self] { &[$($bi0s),*] }
            #[inline] fn bessel_i0_large_coeffs() -> &'static [Self] { &[$($bi0l),*] }

            #[inline] fn bessel_i1_small_coeffs() -> &'static [Self] { &[$($bi1bs),*] }
            #[inline] fn bessel_i1_large_coeffs() -> &'static [Self] { &[$($bi1bl),*] }

            #[inline] fn bessel_k0_small_coeffs() -> &'static [Self] { &[$($bk0s),*] }
            #[inline] fn bessel_k0_large_coeffs() -> &'static [Self] { &[$($bk0l),*] }

            #[inline] fn bessel_k1_small_coeffs() -> &'static [Self] { &[$($bk1s),*] }
            #[inline] fn bessel_k1_large_coeffs() -> &'static [Self] { &[$($bk1l),*] }

            #[inline] fn bessel_j_split() -> Self { $bjsplit }
            #[inline] fn bessel_miller_seed() -> Self { $seed }

            #[inline] fn bessel_j0_root1() -> Self { $bj0r1 }
            #[inline] fn bessel_j0_root2() -> Self { $bj0r2 }
            #[inline] fn bessel_j1_root1() -> Self { $bj1r1 }
            #[inline] fn bessel_j1_root2() -> Self { $bj1r2 }
        }
    };
}

pub(in crate::number::logic::float_ops) use impl_spec_float;
