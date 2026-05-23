//! Modified Bessel functions `I_n` and `K_n` of integer order.
//!
//! Algorithms adapted from:
//! - Cephes Mathematical Library (S. L. Moshier): `i0.c`, `k0.c`, `k1.c`, `iv.c`
//! - Boost C++ Libraries: `bessel_i1.hpp`
//! - Clenshaw, C. W. (1955). "A note on the summation of Chebyshev series."
//! - DLMF (NIST): §§10.29, 10.74

use super::bessel_jy::{compute_miller_start, horner_eval};
use super::helpers::kahan_add;
use super::{SpecFloat, SpecInt};

// =========================================================================
// I_n — Modified Bessel function of the first kind
// =========================================================================

pub fn bessel_i<T: SpecFloat, I: SpecInt>(n: I, x: T) -> T {
    if x.is_nan() {
        return T::nan();
    }
    let n_abs = n.abs();
    let ax = x.abs();
    // I_n(-x) = (-1)^n I_n(x)
    let sign = if n_abs % I::from_usize(2) == I::one() && x < T::zero() {
        T::neg_one()
    } else {
        T::one()
    };
    // I_{-n}(x) = I_n(x) for integer n.
    if n_abs.is_zero() {
        return sign * bessel_i0(ax);
    }
    if n_abs == I::one() {
        return sign * bessel_i1_unsigned(ax);
    }

    if ax < T::eps() {
        return T::zero();
    }
    if bessel_i_definitely_overflows(n_abs, ax) {
        return sign * T::infinity();
    }

    // The power series is stable for small and moderate x. It also avoids
    // Miller overflow for f32 when x is small and n >= 2.
    if ax < T::from_usize(40) {
        return sign * power_series_i(n_abs, ax);
    }

    let n_float = T::from_int(n_abs);
    if ax > T::from_usize(18).max(n_float * n_float / T::two()) {
        return sign * bessel_i_asymptotic(n_abs, ax);
    }

    // Forward recurrence for I_n is stable only while the order is small
    // compared with sqrt(x). Larger orders use Miller normalization.
    let threshold = if ax < T::from_usize(9) {
        T::from_usize(2)
    } else {
        (ax / T::from_usize(3)).sqrt()
    };
    if n_float < threshold {
        return sign * forward_recurrence_i(n_abs, ax);
    }

    // Miller backward recurrence with normalization: e^x = I₀ + 2·Σ I_k
    sign * miller_backward_i(n_abs, ax)
}

// =========================================================================
// K_n — Modified Bessel function of the second kind
// =========================================================================

pub fn bessel_k<T: SpecFloat, I: SpecInt>(n: I, x: T) -> T {
    if x <= T::zero() {
        return T::nan();
    }
    let n_abs = n.abs();
    let k0 = bessel_k0(x);
    if n_abs.is_zero() {
        return k0;
    }
    let k1 = bessel_k1(x);
    if n_abs == I::one() {
        return k1;
    }
    let (mut k_prev, mut k_curr) = (k0, k1);
    let two = T::two();
    let mut k_t = T::one();
    let mut k = I::one();

    while k < n_abs {
        let k_next = k_prev + (two * k_t / x) * k_curr;
        k_prev = k_curr;
        k_curr = k_next;
        k_t = k_t + T::one();
        k = k + I::one();
    }
    k_curr
}

// =========================================================================
// I_n strategies
// =========================================================================

fn forward_recurrence_i<T: SpecFloat, I: SpecInt>(n_abs: I, ax: T) -> T {
    let i0 = bessel_i0(ax);
    if n_abs.is_zero() {
        return i0;
    }
    let i1 = bessel_i1_unsigned(ax);
    if n_abs == I::one() {
        return i1;
    }
    let (mut i_prev, mut i_curr) = (i0, i1);
    let two = T::two();
    let mut k_t = T::one();
    let mut k = I::one();
    while k < n_abs {
        let ratio = two * k_t / ax;
        if ratio.is_infinite() {
            break;
        }
        let i_next = i_prev - ratio * i_curr;
        if i_next.is_infinite() {
            return i_next;
        }
        i_prev = i_curr;
        i_curr = i_next;
        k_t = k_t + T::one();
        k = k + I::one();
    }
    i_curr
}

fn power_series_i<T: SpecFloat, I: SpecInt>(n_abs: I, ax: T) -> T {
    let half_x = T::half() * ax;
    let half_x2 = half_x * half_x;
    let mut term = T::one();
    let mut k = I::one();
    while k <= n_abs {
        term = term * half_x / T::from_int(k);
        k = k + I::one();
    }

    let mut result = term;
    for s in 1..=256 {
        let denom = T::from_usize(s) * (T::from_int(n_abs) + T::from_usize(s));
        term = term * half_x2 / denom;
        if term.is_nan() || term.is_infinite() {
            break;
        }
        result = result + term;
        if term.abs() < result.abs() * T::eps() {
            break;
        }
    }
    result
}

fn bessel_i_definitely_overflows<T: SpecFloat, I: SpecInt>(n_abs: I, ax: T) -> bool {
    if ax <= T::one() {
        return false;
    }

    let n_t = T::from_int(n_abs);
    let mu = T::from_usize(4) * n_t * n_t;
    let leading_log = ax - T::half() * (T::two() * T::pi() * ax).ln();
    let conservative_correction = (mu + T::one()) / (T::from_usize(8) * ax);
    leading_log - conservative_correction > T::max_value().ln()
}

fn bessel_i_asymptotic<T: SpecFloat, I: SpecInt>(n_abs: I, ax: T) -> T {
    let n_t = T::from_int(n_abs);
    let mu = T::from_usize(4) * n_t * n_t;
    let mut term = T::one();
    let mut sum = T::one();
    let mut prev_abs = T::max_value();

    for k in 1..=18 {
        let odd = 2 * k - 1;
        let factor = mu - T::from_usize(odd * odd);
        term = -term * factor / (T::from_usize(k) * T::from_usize(8) * ax);
        let term_abs = term.abs();
        if k > 6 && term_abs > prev_abs {
            break;
        }
        sum = sum + term;
        prev_abs = term_abs;
    }

    if sum == T::zero() {
        return T::zero();
    }
    let denom = (T::two() * T::pi() * ax).sqrt();
    let term1 = sum / denom;
    if ax < T::max_value().ln() {
        return ax.exp() * term1;
    }
    let half_ax = ax * T::half();
    (half_ax.exp() * term1) * half_ax.exp()
}

// Miller's Backward Recurrence for I_n(x)
//
// Like J_n(x), I_n(x) can be computed stably using backward recurrence:
// I_{n-1}(x) = I_{n+1}(x) + (2n/x) I_n(x)
//
// The normalization uses the relation: e^x = I_0(x) + 2 \sum_{k=1}^infty I_k(x)
//
// Reference: DLMF §10.29 (Modified Bessel Functions, Recurrence Relations).
fn miller_backward_i<T: SpecFloat, I: SpecInt>(n_abs: I, ax: T) -> T {
    let two = T::two();
    let n_start = compute_i_start(n_abs, ax);
    let scale_threshold = T::max_value().sqrt();

    let mut i_next = T::zero();
    let mut i_curr = T::bessel_miller_seed();
    let mut result = T::zero();
    let mut result_scale = T::zero();
    let mut i0_unscaled = T::zero();
    let mut scale_power = T::zero();

    // Normalization sum: I₀ + 2·Σ_{k≥1} I_k = e^x (Kahan-compensated)
    let mut norm_sum = T::zero();
    let mut norm_comp = T::zero();

    let mut k = n_start;
    let mut k_t = T::from_int(k);

    loop {
        if i_curr.abs() > scale_threshold || i_next.abs() > scale_threshold {
            i_curr = i_curr / scale_threshold;
            i_next = i_next / scale_threshold;
            i0_unscaled = i0_unscaled / scale_threshold;
            norm_sum = norm_sum / scale_threshold;
            norm_comp = norm_comp / scale_threshold;
            scale_power = scale_power + T::one();
        }

        let mut i_prev = (two * k_t / ax) * i_curr + i_next;

        if i_prev.abs() > scale_threshold {
            i_prev = i_prev / scale_threshold;
            i_curr = i_curr / scale_threshold;
            i0_unscaled = i0_unscaled / scale_threshold;
            norm_sum = norm_sum / scale_threshold;
            norm_comp = norm_comp / scale_threshold;
            scale_power = scale_power + T::one();
        }

        if k == n_abs {
            result = i_curr;
            result_scale = scale_power;
        }

        if k.is_zero() {
            i0_unscaled = i_curr;
            kahan_add(&mut norm_sum, &mut norm_comp, i_curr);
        } else {
            kahan_add(&mut norm_sum, &mut norm_comp, two * i_curr);
        }

        i_next = i_curr;
        i_curr = i_prev;
        if k.is_zero() {
            break;
        }
        k = k - I::one();
        k_t = k_t - T::one();
    }

    // Direct ratio: bring result and norm_sum to the same scale, then divide.
    // I_n(x) = (result/norm_sum) · e^x
    let mut res = result;
    let mut nrm = norm_sum;
    let mut diff = result_scale - scale_power;
    while diff > T::zero() {
        nrm = nrm / scale_threshold;
        diff = diff - T::one();
    }
    while diff < T::zero() {
        res = res / scale_threshold;
        diff = diff + T::one();
    }
    if nrm == T::zero() {
        return T::zero();
    }
    let ratio = res / nrm;
    // Multiply by e^x, handling potential overflow
    let max_ln = T::max_value().ln();
    if ax < max_ln {
        ratio * ax.exp()
    } else {
        // Split: e^x = e^(x/2) · e^(x/2) to avoid overflow in single exp
        let half_ax = ax * T::half();
        (ratio * half_ax.exp()) * half_ax.exp()
    }
}

fn compute_i_start<T: SpecFloat, I: SpecInt>(n: I, ax: T) -> I {
    let start = compute_miller_start(n);
    // More backward steps = better normalization precision.
    // 40 extra terms ensures the un-normalized I_k for k > n_start
    // are negligible relative to the normalization sum.
    let mut min_start = I::from_usize(n.to_usize() + 80);
    let target = ax + T::from_usize(20);
    while T::from_int(min_start) < target {
        min_start = min_start + I::from_usize(16);
    }
    if start < min_start { min_start } else { start }
}

// =========================================================================
// I₀, I₁ — Chebyshev / polynomial approximations
// =========================================================================

fn bessel_i0<T: SpecFloat>(x: T) -> T {
    if x.is_nan() {
        return T::nan();
    }
    let ax = x.abs();
    if ax <= T::from_int(8) {
        let y = (ax / T::from_int(2)) - T::from_int(2);
        ax.exp() * clenshaw_eval(y, T::bessel_i0_small_coeffs())
    } else {
        let y = (T::from_int(32) / ax) - T::from_int(2);
        let p = clenshaw_eval(y, T::bessel_i0_large_coeffs());
        if p == T::zero() {
            T::zero()
        } else {
            let p_factor = p / ax.sqrt();
            let max_ln = T::max_value().ln();
            if ax < max_ln {
                ax.exp() * p_factor
            } else {
                let half_ax = ax * T::half();
                (half_ax.exp() * p_factor) * half_ax.exp()
            }
        }
    }
}

/// `I_1(x)` for x >= 0 — no sign handling.
fn bessel_i1_unsigned<T: SpecFloat>(ax: T) -> T {
    if ax.is_nan() {
        return T::nan();
    }
    let boost_split = T::from_usize(31) / T::from_usize(4); // 7.75
    if ax < boost_split {
        let a = (ax / T::two()).powf(T::two());
        let p = horner_eval(a, T::bessel_i1_small_coeffs());
        ax * (T::one() + T::half() * a + a * a * p) / T::two()
    } else {
        let y = T::one() / ax;
        let p = horner_eval(y, T::bessel_i1_large_coeffs());
        if p == T::zero() {
            T::zero()
        } else {
            let p_factor = p / ax.sqrt();
            let max_ln = T::max_value().ln();
            if ax < max_ln {
                ax.exp() * p_factor
            } else {
                let half_ax = ax * T::half();
                (half_ax.exp() * p_factor) * half_ax.exp()
            }
        }
    }
}

// =========================================================================
// K₀, K₁
// =========================================================================

fn bessel_k0<T: SpecFloat>(x: T) -> T {
    if x.is_nan() || x <= T::zero() {
        return T::nan();
    }
    let two = T::two();
    if x <= two {
        let four = T::from_usize(4);
        let y = x * x / four;
        let i0 = bessel_i0(x);
        let ln_term = -(x / two).ln() * i0;
        ln_term + horner_eval(y, T::bessel_k0_small_coeffs())
    } else {
        let y = (T::from_int(8) / x) - T::two();
        (-x).exp() * clenshaw_eval(y, T::bessel_k0_large_coeffs()) / x.sqrt()
    }
}

fn bessel_k1<T: SpecFloat>(x: T) -> T {
    if x.is_nan() || x <= T::zero() {
        return T::nan();
    }
    let two = T::two();
    if x <= two {
        let four = T::from_usize(4);
        let y = x * x / four;
        let ax = x.abs();
        let sign = if x < T::zero() {
            T::neg_one()
        } else {
            T::one()
        };
        let i1 = sign * bessel_i1_unsigned(ax);
        let ln_term = (x * T::half()).ln() * i1;
        ln_term + horner_eval(y, T::bessel_k1_small_coeffs()) / x
    } else {
        let y = (T::from_int(8) / x) - T::two();
        (-x).exp() * clenshaw_eval(y, T::bessel_k1_large_coeffs()) / x.sqrt()
    }
}

// =========================================================================
// Chebyshev evaluator
// =========================================================================

/// Clenshaw's Algorithm for evaluating Chebyshev series.
///
/// Reference: Clenshaw, C. W. (1955), "A note on the summation of Chebyshev series."
#[inline]
fn clenshaw_eval<T: SpecFloat>(x: T, coeffs: &[T]) -> T {
    let mut it = coeffs.iter();
    let Some(&first) = it.next() else {
        return T::zero();
    };
    let mut b0 = first;
    let mut b1 = T::zero();
    let mut b2 = T::zero();
    for &c in it {
        b2 = b1;
        b1 = b0;
        b0 = x * b1 - b2 + c;
    }
    T::half() * (b0 - b2)
}
