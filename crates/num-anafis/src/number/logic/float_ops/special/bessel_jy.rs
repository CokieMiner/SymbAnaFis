//! Bessel functions `J_n` and `Y_n` of the first and second kind (integer order).
//!
//! Algorithms adapted from:
//! - Cephes Mathematical Library (S. L. Moshier): `j0.c`, `j1.c`, `y0.c`, `y1.c`
//! - Miller, J. C. P. (1952). "Bessel Functions." *British Association
//!   Mathematical Tables*, Vol. X.
//! - DLMF (NIST): §§10.6, 10.8, 10.17, 3.6(iii)

use super::helpers::kahan_add;
use super::{SpecFloat, SpecInt};

// =========================================================================
// J_n — Bessel function of the first kind
// =========================================================================

pub fn bessel_j<T: SpecFloat, I: SpecInt>(n: I, x: T) -> T {
    if x.is_nan() {
        return T::nan();
    }
    if n.is_negative() {
        let val = bessel_j(n.abs(), x);
        return if (n.abs() % I::from_usize(2)) == I::one() {
            -val
        } else {
            val
        };
    }
    let n_float = T::from_int(n);
    if n_float.is_infinite() {
        return T::nan();
    }
    let ax = x.abs();

    if ax < T::eps() {
        return if n.is_zero() { T::one() } else { T::zero() };
    }

    // Small-x power series avoids Miller overflow for f32.
    // J_n(x) ≈ (x/2)^n / n! * [1 - (x/2)²/(n+1) + ...]
    if !n.is_zero() && ax < T::from_usize(1) {
        let sign = j_sign(n, n, x);
        let half_x = T::half() * ax;
        let half_x2 = half_x * half_x;
        let mut term = T::one();
        let mut k = I::one();
        while k <= n {
            term = term * half_x / T::from_int(k);
            k = k + I::one();
        }
        let mut result = term;
        if term == T::zero() {
            return sign * result;
        }
        let n_f = T::from_int(n);
        let mut s = I::one();
        loop {
            let s_f = T::from_int(s);
            term = -term * half_x2 / (s_f * (n_f + s_f));
            if term == T::zero() {
                break;
            }
            if term.abs() < result.abs() * T::eps() {
                break;
            }
            result = result + term;
            s = s + I::one();
        }
        return sign * result;
    }

    // Hankel asymptotic expansion for large x where the series converges well.
    // Condition: x > n²/(2x) regime, i.e., x is large relative to n.
    // The Hankel expansion diverges eventually, but the initial terms converge
    // when 8x >> 4n² - 1. DLMF §10.17.
    let hankel_threshold = n_float * n_float / T::two() + T::from_usize(20);
    if ax > hankel_threshold && ax > T::from_usize(10) {
        let sign = j_sign(n, n, x);
        return sign * bessel_j_asymptotic(n, ax);
    }

    if ax > n_float {
        forward_recurrence_j(n, x)
    } else {
        miller_backward_j(n, x)
    }
}

// =========================================================================
// Y_n — Bessel function of the second kind
// =========================================================================

/// Bessel `Y_n` with compensated forward recurrence.
pub fn bessel_y<T: SpecFloat, I: SpecInt>(n: I, x: T) -> T {
    if x.is_nan() || x <= T::zero() {
        return T::nan();
    }
    let n_abs = n.abs();
    let n_float = T::from_int(n_abs);
    if n_float.is_infinite() {
        return T::nan();
    }
    let y0 = bessel_y0(x);
    if n_abs.is_zero() {
        return y0;
    }
    let y1 = bessel_y1(x);
    if n_abs == I::one() {
        return if n.is_negative() { -y1 } else { y1 };
    }

    // Compensated forward recurrence: Y_{n+1} = (2n/x)·Y_n - Y_{n-1}
    // Uses FMA to track exact rounding errors from the multiplication.
    let (mut y_prev, mut y_curr) = (y0, y1);
    let mut e_prev = T::zero();
    let mut e_curr = T::zero();
    let two = T::two();
    let mut k_t = T::one();
    let mut k = I::one();

    while k < n_abs {
        let ratio = two * k_t / x;
        // Exact product error via FMA: prod + prod_err = ratio * y_curr exactly
        let prod = ratio * y_curr;
        if prod.is_infinite() {
            return if n.is_negative() && (n_abs % I::from_usize(2)) == I::one() {
                -prod.signum() * T::infinity()
            } else {
                prod.signum() * T::infinity()
            };
        }
        let prod_err = ratio.mul_add(y_curr, -prod);
        // Exact subtraction error
        let diff = prod - y_prev;
        let diff_err = (prod - diff) - y_prev;
        let y_main = diff;
        // Propagated compensation including FMA-tracked errors
        let y_comp = ratio * e_curr - e_prev + prod_err + diff_err;
        let y_next = y_main + y_comp;

        e_prev = e_curr;
        e_curr = (y_main - y_next) + y_comp;
        y_prev = y_curr;
        y_curr = y_next;
        k_t = k_t + T::one();
        k = k + I::one();
    }
    if n.is_negative() && (n_abs % I::from_usize(2)) == I::one() {
        -y_curr
    } else {
        y_curr
    }
}

// =========================================================================
// J_n strategies
// =========================================================================

/// Hankel asymptotic expansion for `J_n(x)` when x is large.
///
/// `J_n(x) ≈ sqrt(2/(πx)) · [P·cos(χ) − Q·sin(χ)]`
/// where `χ = x − (2n+1)π/4`
///
/// P and Q are series in `1/(8x)` that converge when `8x >> 4n²−1`.
/// Terms are accumulated until they start growing (divergent tail).
#[allow(
    clippy::many_single_char_names,
    reason = "Mathematical formula uses standard notation (n, x, k, mu, chi)"
)]
fn bessel_j_asymptotic<T: SpecFloat, I: SpecInt>(n_abs: I, ax: T) -> T {
    let n_t = T::from_int(n_abs);
    let mu = T::from_usize(4) * n_t * n_t;
    let one = T::one();
    let eight_x = T::from_usize(8) * ax;

    // DLMF §10.17.3: P and Q via the term recurrence
    //   a_0 = 1
    //   a_k = a_{k-1} · (mu - (2k-1)²) / (k · 8x)
    // P = a_0 - a_2 + a_4 - ...   (even-indexed terms, alternating sign)
    // Q = a_1 - a_3 + a_5 - ...   (odd-indexed terms, alternating sign)
    let mut p_sum = one;
    let mut q_sum = T::zero();
    let mut a_k = one;
    let mut prev_abs = T::max_value();

    for k in 1..=30_usize {
        let odd = T::from_usize(2 * k - 1);
        a_k = a_k * (mu - odd * odd) / (T::from_usize(k) * eight_x);
        let a_abs = a_k.abs();
        // Stop if terms start growing (divergent tail of asymptotic series)
        if k > 2 && a_abs > prev_abs {
            break;
        }
        prev_abs = a_abs;
        if k % 2 == 0 {
            // Even index: contributes to P with sign (-1)^(k/2)
            let sign = if (k >> 1) % 2 == 0 { one } else { -one };
            p_sum = p_sum + sign * a_k;
        } else {
            // Odd index: contributes to Q with sign (-1)^((k-1)/2)
            let sign = if ((k - 1) >> 1) % 2 == 0 { one } else { -one };
            q_sum = q_sum + sign * a_k;
        }
    }

    // χ = x − (2n+1)·π/4 with Cody-Waite reduction for π/4
    let two_n_plus_1 = T::two() * n_t + one;
    let chi = (ax - two_n_plus_1 * T::pio4_hi()) - two_n_plus_1 * T::pio4_lo();

    let prefactor = (T::two() / (T::pi() * ax)).sqrt();
    prefactor * (p_sum * chi.cos() - q_sum * chi.sin())
}

fn forward_recurrence_j<T: SpecFloat, I: SpecInt>(n: I, x: T) -> T {
    let n_abs = n.abs();
    let ax = x.abs();
    let sign = j_sign(n, n_abs, x);

    let j0 = bessel_j0(ax);
    if n_abs.is_zero() {
        return j0;
    }

    let j1 = bessel_j1(ax);
    if n_abs == I::one() {
        return sign * j1;
    }

    // Compensated forward recurrence: J_{n+1} = (2n/x)·J_n - J_{n-1}
    let (mut j_prev, mut j_curr) = (j0, j1);
    let mut e_prev = T::zero();
    let mut e_curr = T::zero();
    let two = T::two();
    let mut k_t = T::one();
    let mut k = I::one();

    while k < n_abs {
        let ratio = two * k_t / ax;
        if ratio.is_infinite() {
            return miller_backward_j(n, x);
        }
        let prod = ratio * j_curr;
        let prod_err = ratio.mul_add(j_curr, -prod);
        let diff = prod - j_prev;
        let diff_err = (prod - diff) - j_prev;
        let j_main = diff;
        let j_comp = ratio * e_curr - e_prev + prod_err + diff_err;
        let j_next = j_main + j_comp;

        e_prev = e_curr;
        e_curr = (j_main - j_next) + j_comp;
        j_prev = j_curr;
        j_curr = j_next;
        k_t = k_t + T::one();
        k = k + I::one();
    }

    sign * j_curr
}

#[allow(
    clippy::many_single_char_names,
    reason = "Mathematical formulas use standard single-letter notation (n, x, k, y, t)"
)]
// Miller's Backward Recurrence Algorithm
//
// Computes J_n(x) by iterating the recurrence relation backwards from a
// dynamically computed starting index `N_start > n`. This is unconditionally
// stable for J_n(x), unlike forward recurrence which suffers catastrophic
// cancellation when J_n(x) < J_{n-1}(x).
//
// Reference: Miller, J. C. P. (1952), DLMF §10.74.
fn miller_backward_j<T: SpecFloat, I: SpecInt>(n: I, x: T) -> T {
    let n_abs = n.abs();
    let ax = x.abs();
    let sign = j_sign(n, n_abs, x);
    let two = T::two();
    let scale_threshold = T::max_value().sqrt();
    let inv_scale_threshold = T::one() / scale_threshold;

    let n_start = compute_miller_start(n_abs);

    let mut j_next = T::zero();
    // Seed the recurrence with an arbitrarily small non-zero value.
    // The exact magnitude doesn't matter because the sequence is normalized
    // using the sum relationship: 1 = J_0(x) + 2Σ J_{2k}(x) at the end.
    let mut j_curr = T::bessel_miller_seed();

    let mut result = T::zero();
    let mut result_scale = T::zero();
    let mut sum = T::zero();
    let mut compensation = T::zero();
    let mut scale_power = T::zero();

    let mut k = n_start;
    let mut k_t = T::from_int(k);

    loop {
        if j_curr.abs() > scale_threshold || j_next.abs() > scale_threshold {
            j_curr = j_curr / scale_threshold;
            j_next = j_next / scale_threshold;
            sum = sum / scale_threshold;
            compensation = compensation / scale_threshold;
            scale_power = scale_power + T::one();
        } else if j_curr != T::zero()
            && j_next != T::zero()
            && j_curr.abs() < inv_scale_threshold
            && j_next.abs() < inv_scale_threshold
        {
            j_curr = j_curr * scale_threshold;
            j_next = j_next * scale_threshold;
            sum = sum * scale_threshold;
            compensation = compensation * scale_threshold;
            scale_power = scale_power - T::one();
        }

        let mut j_prev = (two * k_t / ax) * j_curr - j_next;

        if j_prev.abs() > scale_threshold {
            j_prev = j_prev / scale_threshold;
            j_curr = j_curr / scale_threshold;
            sum = sum / scale_threshold;
            compensation = compensation / scale_threshold;
            scale_power = scale_power + T::one();
        }

        if k == n_abs {
            result = j_curr;
            result_scale = scale_power;
        }

        if k.is_zero() {
            kahan_add(&mut sum, &mut compensation, j_curr);
        } else if (k % I::from_usize(2)).is_zero() {
            kahan_add(&mut sum, &mut compensation, two * j_curr);
        }

        j_next = j_curr;
        j_curr = j_prev;
        if k.is_zero() {
            break;
        }
        k = k - I::one();
        k_t = k_t - T::one();
    }

    if sum == T::zero() || sum.is_nan() || sum.is_infinite() {
        return T::zero();
    }
    // Direct ratio: bring result and sum to the same scale, then divide.
    // This avoids the log-space detour exp(ln|a| - ln|b|) which loses
    // ≥2 ULP from ln/exp roundtrip plus cancellation in the subtraction.
    let mut res = result;
    let mut nrm = sum;
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
    sign * res / nrm
}

// =========================================================================
// J₀, J₁, Y₀, Y₁ — rational approximations
// =========================================================================

fn bessel_j0<T: SpecFloat>(x: T) -> T {
    if x.is_nan() {
        return T::nan();
    }
    let ax = x.abs();
    let split = T::bessel_j_split();
    if ax <= split {
        let z = ax * ax;
        let r1 = T::bessel_j0_root1();
        let r2 = T::bessel_j0_root2();
        (z - r1)
            * (z - r2)
            * horner_rational(z, T::bessel_j0_num_coeffs(), T::bessel_j0_den_coeffs())
    } else {
        let z = split / ax;
        let q = z * z;
        let p = horner_rational(q, T::bessel_j0_pcos_coeffs(), T::bessel_j0_psin_coeffs());
        let q_rat = horner_rational(q, T::bessel_y0_pcos_coeffs(), T::bessel_y0_psin_coeffs());
        let xx = (ax - T::pio4_hi()) - T::pio4_lo();
        let term_sqrt = (T::frac_2_pi() / ax).sqrt();
        term_sqrt * (p * xx.cos() - z * q_rat * xx.sin())
    }
}

fn bessel_j1<T: SpecFloat>(x: T) -> T {
    if x.is_nan() {
        return T::nan();
    }
    let ax = x.abs();
    let split = T::bessel_j_split();
    if ax <= split {
        let z = ax * ax;
        let r1 = T::bessel_j1_root1();
        let r2 = T::bessel_j1_root2();
        let ans = ax
            * (z - r1)
            * (z - r2)
            * horner_rational(z, T::bessel_j1_num_coeffs(), T::bessel_j1_den_coeffs());
        if x < T::zero() { -ans } else { ans }
    } else {
        let z = split / ax;
        let q = z * z;
        let p = horner_rational(q, T::bessel_j1_pcos_coeffs(), T::bessel_j1_psin_coeffs());
        let q_rat = horner_rational(q, T::bessel_y1_pcos_coeffs(), T::bessel_y1_psin_coeffs());
        let xx = (ax - T::pio34_hi()) - T::pio34_lo();
        let term_sqrt = (T::frac_2_pi() / ax).sqrt();
        let ans = term_sqrt * (p * xx.cos() - z * q_rat * xx.sin());
        if x < T::zero() { -ans } else { ans }
    }
}

fn bessel_y0<T: SpecFloat>(x: T) -> T {
    if x.is_nan() || x <= T::zero() {
        return T::nan();
    }
    let split = T::bessel_j_split();
    if x <= split {
        let z = x * x;
        let p = horner_rational(z, T::bessel_y0_num_coeffs(), T::bessel_y0_den_coeffs());
        p + T::frac_2_pi() * bessel_j0(x) * x.ln()
    } else {
        let z = split / x;
        let q = z * z;
        let p = horner_rational(q, T::bessel_j0_pcos_coeffs(), T::bessel_j0_psin_coeffs());
        let q_rat = horner_rational(q, T::bessel_y0_pcos_coeffs(), T::bessel_y0_psin_coeffs());
        let xx = (x - T::pio4_hi()) - T::pio4_lo();
        let term_sqrt = (T::frac_2_pi() / x).sqrt();
        term_sqrt * (p * xx.sin() + z * q_rat * xx.cos())
    }
}

fn bessel_y1<T: SpecFloat>(x: T) -> T {
    if x.is_nan() || x <= T::zero() {
        return T::nan();
    }
    let split = T::bessel_j_split();
    if x <= split {
        let z = x * x;
        let p = x * horner_rational(z, T::bessel_y1_num_coeffs(), T::bessel_y1_den_coeffs());
        p + T::frac_2_pi() * (bessel_j1(x) * x.ln() - T::one() / x)
    } else {
        let z = split / x;
        let q = z * z;
        let p = horner_rational(q, T::bessel_j1_pcos_coeffs(), T::bessel_j1_psin_coeffs());
        let q_rat = horner_rational(q, T::bessel_y1_pcos_coeffs(), T::bessel_y1_psin_coeffs());
        let xx = (x - T::pio34_hi()) - T::pio34_lo();
        let term_sqrt = (T::frac_2_pi() / x).sqrt();
        term_sqrt * (p * xx.sin() + z * q_rat * xx.cos())
    }
}

// =========================================================================
// Shared helpers
// =========================================================================

#[inline]
fn j_sign<T: SpecFloat, I: SpecInt>(n: I, n_abs: I, x: T) -> T {
    let n_odd = (n_abs % I::from_usize(2)) == I::one();
    let sign_opposite = n.is_negative() != (x < T::zero());
    let sign_flip = n_odd && sign_opposite;
    if sign_flip { -T::one() } else { T::one() }
}

pub(super) fn compute_miller_start<I: SpecInt>(n: I) -> I {
    let extra = I::from_usize(approx_sqrt_max(n, 50)) + I::from_usize(15);
    n + extra
}

fn approx_sqrt_max<I: SpecInt>(n: I, factor: usize) -> usize {
    approx_sqrt(factor.saturating_mul(n.to_usize()))
}

fn approx_sqrt(x: usize) -> usize {
    let mut r = x;
    let mut s = 1_usize;
    while s < r {
        s <<= 1;
        r >>= 1;
    }
    s >>= 1;
    r = s;
    for _ in 0..5 {
        r = (r + x.checked_div(r.max(1)).unwrap_or(0)) >> 1;
    }
    r
}

#[inline]
pub(super) fn horner_eval<T: SpecFloat>(x: T, coeffs: &[T]) -> T {
    let mut sum = T::zero();
    for &c in coeffs.iter().rev() {
        sum = x.mul_add(sum, c);
    }
    sum
}

#[inline]
pub(super) fn horner_rational<T: SpecFloat>(x: T, num: &[T], den: &[T]) -> T {
    horner_eval(x, num) / horner_eval(x, den)
}
