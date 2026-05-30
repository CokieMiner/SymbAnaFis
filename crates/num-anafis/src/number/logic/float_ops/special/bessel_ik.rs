//! Modified Bessel functions `I_n` and `K_n` of integer order.
//!
//! Algorithms adapted from:
//! - Cephes Mathematical Library (S. L. Moshier): `i0.c`, `k0.c`, `k1.c`, `iv.c`
//! - Boost C++ Libraries: `besseli1.hpp`
//! - Clenshaw, C. W. (1955). "A note on the summation of Chebyshev series."
//! - DLMF (NIST): §§10.29, 10.74

use super::bessel_jy::{compute_miller_start, horner_eval};
use super::helpers::kahan_add;
use super::{SpecFloat, SpecInt};

// =========================================================================
// I_n — Modified Bessel function of the first kind
// =========================================================================

/// Modified Bessel function of the first kind, `I_n(x)`, integer order `n`.
///
/// Dispatch logic:
/// - **Power series** (`|x| < 40`): Direct [`power_series_i`] evaluation.
///   The series `Σ (x/2)^{2k+n} / (k! Γ(n+k+1))` converges for all finite `x`
///   but becomes slow for large arguments. The threshold 40 ensures that the
///   series converges within ~256 terms before overflow of the factorial
///   denominator at f32/f64 precision (Γ(256) ≈ 10⁵⁰⁷ exceeds `f64::MAX`).
/// - **Asymptotic expansion** (`|x| > max(18, n²/2)`): Hankel-type expansion
///   via [`besseli_asymptotic`] using DLMF §10.40.1. The minimum 18 prevents
///   underflow of the `√(2πx)⁻¹` prefactor during intermediate series terms.
/// - **Forward recurrence** (`n < √(|x|/3)` for `|x| ≥ 9`, else `n < 2`):
///   [`forward_recurrence_i`] is stable only when the order `n` is small
///   compared with `√x`; outside this region the exponentially growing
///   solution contaminates the desired one.
/// - **Miller backward recurrence** (default): [`miller_backward_i`] with
///   normalization `eˣ = I₀(x) + 2 Σ_{k≥1} I_k(x)` (DLMF §10.74).
///
/// # Symmetries (integer order)
/// - `I_{-n}(x) = I_n(x)`
/// - `I_n(-x) = (-1)^n I_n(x)`
///
/// Reference: [DLMF, §10.25.2], [DLMF, §10.40.1],
///            [DLMF, §10.29.1], [DLMF, §10.74]
pub fn besseli<T: SpecFloat, I: SpecInt>(n: I, x: T) -> T {
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
        return sign * besseli0(ax);
    }
    if n_abs == I::one() {
        return sign * besseli1_unsigned(ax);
    }

    if ax < T::eps() {
        return T::zero();
    }
    if besseli_definitely_overflows(n_abs, ax) {
        return sign * T::infinity();
    }

    // The power series is stable for small and moderate x. It also avoids
    // Miller overflow for f32 when x is small and n >= 2.
    if ax < T::from_usize(40) {
        return sign * power_series_i(n_abs, ax);
    }

    let n_float = T::from_int(n_abs);
    if ax > T::from_usize(18).max(n_float * n_float / T::two()) {
        return sign * besseli_asymptotic(n_abs, ax);
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

/// Modified Bessel function of the second kind, `K_n(x)`, integer order `n` (x > 0).
///
/// Computed via forward recurrence from `K_0(x)` and `K_1(x)`:
/// `K_{n+1}(x) = K_{n-1}(x) + (2n/x)·K_n(x)`.
///
/// Unlike `I_n`, forward recurrence for `K_n` is **stable** because `K_n` is
/// the dominant solution in the forward direction (it decreases monotonically
/// with increasing order for fixed x).
///
/// # Restrictions
/// `K_n(x)` is defined only for `x > 0`. Returns `NaN` for `x ≤ 0`.
///
/// Reference: [DLMF, §10.29.1]
pub fn besselk<T: SpecFloat, I: SpecInt>(n: I, x: T) -> T {
    if x <= T::zero() {
        return T::nan();
    }
    let n_abs = n.abs();
    let k0 = besselk0(x);
    if n_abs.is_zero() {
        return k0;
    }
    let k1 = besselk1(x);
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

/// Forward recurrence for `I_n` — stable only when `n < √(x/3)`.
///
/// Uses `I_{k+1}(x) = I_{k-1}(x) - (2k/x)·I_k(x)`.
/// The recurrence is the minimal (decaying) solution; forward iteration
/// amplifies the dominant (growing) solution, so it is only safe when `n`
/// is small relative to `√x`.
///
/// Reference: [DLMF, §10.29.1]
fn forward_recurrence_i<T: SpecFloat, I: SpecInt>(n_abs: I, ax: T) -> T {
    let i0 = besseli0(ax);
    if n_abs.is_zero() {
        return i0;
    }
    let i1 = besseli1_unsigned(ax);
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

/// Power series for `I_n(x)` — converges for all finite `x`.
///
/// `I_n(x) = Σ_{k=0}^∞ (x/2)^{2k+n} / (k! Γ(n+k+1))`
///
/// The series is evaluated directly without transformation: each term derives
/// from the previous by multiplication by `(x/2)² / (k·(n+k))`. Convergence
/// is rapid for `x < 40` (256 terms suffice for f64 precision).
///
/// Reference: [DLMF, §10.25.2]
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

/// Conservative overflow check for `I_n(x)` at f64/f32 precision.
///
/// Uses the leading term of the asymptotic expansion (DLMF §10.40.1):
/// `ln I_n(x) ≈ x − ½ln(2πx) − (4n²−1)/(8x)` to estimate whether the result
/// exceeds `T::MAX` before computing it.
fn besseli_definitely_overflows<T: SpecFloat, I: SpecInt>(n_abs: I, ax: T) -> bool {
    if ax <= T::one() {
        return false;
    }

    let n_t = T::from_int(n_abs);
    let mu = T::from_usize(4) * n_t * n_t;
    let leading_log = ax - T::half() * (T::two() * T::pi() * ax).ln();
    let conservative_correction = (mu + T::one()) / (T::from_usize(8) * ax);
    leading_log - conservative_correction > T::max_value().ln()
}

/// Hankel-like asymptotic expansion for `I_n(x)`.
///
/// `I_n(x) ∼ eˣ / √(2πx) · Σ_{k=0}^∞ (-1)^k a_k(ν) / x^k`
/// where `a_k(ν) = (4ν²−1²)(4ν²−3²)⋯(4ν²−(2k−1)²) / (k! 8^k)`
/// and `ν = n`. This is a divergent series but the first ~12 terms give
/// full f64 precision when `x > max(18, n²/2)`.
///
/// Reference: [DLMF, §10.40.1]
fn besseli_asymptotic<T: SpecFloat, I: SpecInt>(n_abs: I, ax: T) -> T {
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

/// Miller backward recurrence for `I_n(x)`.
///
/// Like `J_n(x)`, `I_n(x)` can be computed stably using backward recurrence:
/// `I_{k-1}(x) = I_{k+1}(x) + (2k/x)·I_k(x)`.
///
/// The normalization uses the relation `eˣ = I₀(x) + 2 Σ_{k≥1} I_k(x)`.
/// The sum is accumulated with Kahan compensation, and the result is the
/// ratio of the desired value (at step `k = n`) over the normalization sum,
/// multiplied by `eˣ`.
///
/// Scaling: when intermediate values approach `√(T::MAX)`, they are divided
/// by `√(T::MAX)` and the scale factor is tracked via `scale_power`. This
/// avoids overflow of the growing sequence before normalization.
///
/// Reference: [DLMF, §10.74], [DLMF, §10.29.1]
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

/// Determines the starting index for Miller backward recurrence of `I_n`.
///
/// The start index must satisfy two conditions:
/// 1. It is large enough that `I_k(x)` for `k > start` is negligible relative
///    to the normalization sum `eˣ`.
/// 2. It exceeds `x + 20` to ensure the backward recurrence from a decaying
///    region yields correct relative magnitudes.
///
/// The `n + 80` baseline ensures at least 80 extra backward steps beyond the
/// target order `n`, giving the normalization sum enough terms to converge.
/// The `x + 20` lower bound (with increments of 16) relates to the empirical
/// observation that the series `I_k(x)` peaks near `k ≈ x` and has decayed
/// sufficiently by `k ≈ x + 20` for Miller normalization to be accurate.
fn compute_i_start<T: SpecFloat, I: SpecInt>(n: I, ax: T) -> I {
    let start = compute_miller_start(n);
    // n + 80 extra terms ensures the tail of the sequence beyond `n` has
    // decayed to well below ε relative to the normalization sum eˣ.
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

/// `I₀(x)` via Clenshaw evaluation of Chebyshev approximations.
///
/// Uses piecewise approximations from Cephes / Boost:
/// - **Small x** (`|x| ≤ 8`): Chebyshev series in `y = x/2 − 2`,
///   then `I₀(x) = eˣ · T(y)`.
/// - **Large x** (`|x| > 8`): Asymptotic form `I₀(x) = eˣ · P(32/x − 2) / √x`,
///   where `P` is a Chebyshev series for the scaled amplitude.
///
/// The split point 8 balances the accuracy of the two Chebyshev fits
/// (each with ~12 terms) at f64 precision.
///
/// References: [Cephes, i0.c], [DLMF, §10.25.2], [DLMF, §10.40.1]
fn besseli0<T: SpecFloat>(x: T) -> T {
    if x.is_nan() {
        return T::nan();
    }
    let ax = x.abs();
    if ax <= T::from_int(8) {
        let y = (ax / T::from_int(2)) - T::from_int(2);
        ax.exp() * clenshaw_eval(y, T::besseli0_small_coeffs())
    } else {
        let y = (T::from_int(32) / ax) - T::from_int(2);
        let p = clenshaw_eval(y, T::besseli0_large_coeffs());
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

/// `I₁(x)` for `x ≥ 0` — no sign handling.
///
/// Uses piecewise approximations adapted from Boost `besseli1.hpp`:
/// - **Small x** (`x < 7.75`): Rational approximation via Horner on `(x/2)²`.
///   The split point `31/4 = 7.75` is empirically chosen so that both the
///   small-x and large-x approximations achieve full f64 precision.
/// - **Large x** (`x ≥ 7.75`): Asymptotic expansion `I₁(x) = eˣ · P(1/x) / √x`,
///   where `P` is a polynomial in `1/x`.
///
/// The small-x form `I₁(x) = x/2 · (1 + ½(x/2)² + (x/2)⁴·P((x/2)²))`
/// factors out the leading Taylor terms for better conditioning.
///
/// Reference: [DLMF, §10.25.2], [DLMF, §10.40.1]
fn besseli1_unsigned<T: SpecFloat>(ax: T) -> T {
    if ax.is_nan() {
        return T::nan();
    }
    let boost_split = T::from_usize(31) / T::from_usize(4); // 7.75
    if ax < boost_split {
        let a = (ax / T::two()).powf(T::two());
        let p = horner_eval(a, T::besseli1_small_coeffs());
        ax * (T::one() + T::half() * a + a * a * p) / T::two()
    } else {
        let y = T::one() / ax;
        let p = horner_eval(y, T::besseli1_large_coeffs());
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

/// `K₀(x)` for `x > 0`.
///
/// Piecewise rational approximations (Cephes):
/// - **Small x** (`x ≤ 2`): `K₀(x) = −ln(x/2)·I₀(x) + P(x²/4)`, where `P`
///   is a polynomial in `x²/4`. The `ln·I₀` term captures the logarithmic
///   singularity at `x = 0` (DLMF §10.31.1).
/// - **Large x** (`x > 2`): Asymptotic expansion
///   `K₀(x) = e⁻ˣ · P(8/x − 2) / √x`.
///
/// The split at `x = 2` ensures both approximations are accurate to within
/// ~1 ULP of f64 precision (12-term Chebyshev fits).
///
/// Reference: [DLMF, §10.31.1], [DLMF, §10.40.2]
fn besselk0<T: SpecFloat>(x: T) -> T {
    if x.is_nan() || x <= T::zero() {
        return T::nan();
    }
    let two = T::two();
    if x <= two {
        let four = T::from_usize(4);
        let y = x * x / four;
        let i0 = besseli0(x);
        let ln_term = -(x / two).ln() * i0;
        ln_term + horner_eval(y, T::besselk0_small_coeffs())
    } else {
        let y = (T::from_int(8) / x) - T::two();
        (-x).exp() * clenshaw_eval(y, T::besselk0_large_coeffs()) / x.sqrt()
    }
}

/// `K₁(x)` for `x > 0`.
///
/// Piecewise rational approximations (Cephes):
/// - **Small x** (`x ≤ 2`): `K₁(x) = ln(x/2)·I₁(x) + P(x²/4)/x`, where `P`
///   is a polynomial in `x²/4`. The `ln·I₁` term captures the logarithmic
///   singularity at `x = 0` (DLMF §10.31.1).
/// - **Large x** (`x > 2`): Asymptotic expansion
///   `K₁(x) = e⁻ˣ · P(8/x − 2) / √x`.
///
/// Reference: [DLMF, §10.31.1], [DLMF, §10.40.2]
fn besselk1<T: SpecFloat>(x: T) -> T {
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
        let i1 = sign * besseli1_unsigned(ax);
        let ln_term = (x * T::half()).ln() * i1;
        ln_term + horner_eval(y, T::besselk1_small_coeffs()) / x
    } else {
        let y = (T::from_int(8) / x) - T::two();
        (-x).exp() * clenshaw_eval(y, T::besselk1_large_coeffs()) / x.sqrt()
    }
}

// =========================================================================
// Chebyshev evaluator
// =========================================================================

/// Clenshaw's Algorithm for evaluating Chebyshev series.
///
/// Reference: [Clenshaw55]
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
