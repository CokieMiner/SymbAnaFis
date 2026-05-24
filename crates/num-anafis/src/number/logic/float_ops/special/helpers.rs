//! Real scalar helper functions for special function implementations.

use super::{SpecFloat, SpecInt};

/// Returns `−1` if `δ` has a negative fractional part (or negative and
/// integer-valued), `+1` otherwise. Used to determine the sign of pole
/// infinities in the gamma and polygamma functions.
#[inline]
pub(super) fn sign_from_delta<T: SpecFloat>(delta: T) -> T {
    if delta.is_nan() {
        return T::nan();
    }
    let frac = delta.fract();
    if frac.is_sign_negative() || (frac == T::zero() && delta.is_sign_negative()) {
        T::neg_one()
    } else {
        T::one()
    }
}

/// Returns ±∞ according to `sign`'s sign bit.
#[inline]
pub(super) fn signed_infinity<T: SpecFloat>(sign: T) -> T {
    if sign.is_nan() {
        return T::nan();
    }
    if sign.is_sign_negative() {
        T::neg_infinity()
    } else {
        T::infinity()
    }
}

/// Convenience: combine [`sign_from_delta`] and [`signed_infinity`].
#[inline]
pub(super) fn signed_infinity_from_delta<T: SpecFloat>(delta: T) -> T {
    signed_infinity(sign_from_delta(delta))
}

/// Kahan compensated summation: `sum += term` with error tracking.
#[inline]
pub(super) fn kahan_add<T: SpecFloat>(sum: &mut T, comp: &mut T, term: T) {
    let y = term - *comp;
    let t = *sum + y;
    *comp = (t - *sum) - y;
    *sum = t;
}

/// True when `x` is a non-positive integer (poles of `Γ(x)` and `ψ(x)`).
#[inline]
pub(super) fn is_non_pos_int<T: SpecFloat>(x: T) -> bool {
    x <= T::zero() && x == x.round()
}

/// Computes `(l−|m|)! / (l+|m|)!` directly or in log-space.
///
/// For `l+|m| < 120` the product `1 / ∏_{j=l−|m|+1}^{l+|m|} j` is evaluated
/// directly. Beyond that the log-space path avoids underflow from repeated
/// multiplication of tiny factors. The threshold 120 is a safe guard: the
/// product of 120 terms at magnitude ~O(1) has no risk of underflow in f64.
#[inline]
pub(super) fn legendre_factorial_ratio<T: SpecFloat, I: SpecInt>(l: I, m_abs: I) -> T {
    if l + m_abs < I::from_usize(120) {
        let mut ratio = T::one();
        let start = l - m_abs + I::one();
        let end = l + m_abs;
        let mut j = start;
        while j <= end {
            ratio = ratio / T::from_int(j);
            j = j + I::one();
        }
        ratio
    } else {
        let mut log_ratio = T::zero();
        let mut log_comp = T::zero();
        let start = l - m_abs + I::one();
        let end = l + m_abs;
        let mut j = start;
        while j <= end {
            kahan_add(&mut log_ratio, &mut log_comp, -T::from_int(j).ln());
            j = j + I::one();
        }
        (log_ratio + log_comp).exp()
    }
}

/// sin(πx) with accurate range reduction for large |x|.
#[inline]
pub(super) fn sin_pi_x<T: SpecFloat>(x: T) -> T {
    let n = x.round();
    let f = x - n;
    let two = T::two();
    let sign = if (n / two).fract() == T::zero() {
        T::one()
    } else {
        -T::one()
    };
    sign * (T::pi() * f).sin()
}

/// π·cot(πx) with accurate range reduction for large |x|.
#[inline]
pub(super) fn pi_cot_pi_x<T: SpecFloat>(x: T) -> T {
    let f = x - x.round();
    let pi = T::pi();
    pi * (pi * f).cos() / (pi * f).sin()
}

/// Sign of `Γ(x)` at a non-positive integer pole, using parity of `⌊−x⌋`.
///
/// `Γ(−n)` has pole with sign `(−1)ⁿ` approaching from the positive side.
/// Returns `+1` for even `n`, `−1` for odd `n`, with overflow protection
/// for very large `|x|` via float parity fallback.
#[inline]
pub(super) fn gamma_pole_sign<T: SpecFloat>(x: T) -> T {
    let neg_x = -x;
    let r = neg_x.round();
    // Upper bound to prevent to_int() overflow for extremely large integers.
    // usize::MAX >> 1 equals i64::MAX (on 64-bit) or i32::MAX (on 32-bit).
    let int_max_f = T::from_usize(usize::MAX >> 1);
    if r >= T::zero() && r <= int_max_f {
        if (r / T::two()).fract() == T::zero() {
            T::one()
        } else {
            T::neg_one()
        }
    } else {
        // Fallback: parity via float floor to avoid integer overflow.
        // For very large values that overflow Int, they are exact integers
        // so floor() and round() coincide.
        let f = neg_x.floor();
        let two = T::two();
        let half = f / two;
        if half.floor() == half {
            T::one()
        } else {
            T::neg_one()
        }
    }
}
