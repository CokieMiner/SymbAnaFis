//! Riemann zeta function ζ(s) — real arguments.
//!
//! Strategies:
//! 1. Laurent expansion near s=1 (Stieltjes constants)
//! 2. Euler-Maclaurin summation for 1 < s ≤ 2
//! 3. Borwein's alternating series for s > 2
//! 4. Functional equation (reflection) for s < 0
//!
//! Reference: [DLMF, §25.2], [Borwein00]

use alloc::vec;

use super::SpecFloat;
use super::gamma::{gamma, gamma_sign, lgamma};
use super::helpers::{kahan_add, signed_infinity_from_delta};

/// Riemann zeta function ζ(s) = Σ_{n=1}^∞ 1/n^s
pub fn zeta<T: SpecFloat>(x: T) -> T {
    let one = T::one();

    let delta = x - one;
    if delta == T::zero() {
        return signed_infinity_from_delta(delta);
    }

    if x < T::zero() {
        return reflection(x);
    }

    // Laurent expansion radius 0.1 ≈ f64 truncation accuracy of
    // the 15‑term Stieltjes series (truncation error ∼ (s−1)¹⁶ / 16!
    // ≈ 10⁻¹⁹ at |s−1| = 0.1, well below f64 ε).
    if delta.abs() < T::from_usize(1) / T::from_usize(10) {
        let stieltjes = T::stieltjes_coeffs();
        let mut sum = T::zero();
        let mut compensation = T::zero();
        let mut fact = T::one();
        let mut delta_pow = T::one();
        for (k, &gamma) in stieltjes.iter().enumerate() {
            let sign = if k % 2 == 0 { T::one() } else { T::neg_one() };
            let term = sign * gamma * delta_pow / fact;
            kahan_add(&mut sum, &mut compensation, term);
            fact = fact * T::from_usize(k + 1);
            delta_pow = delta_pow * delta;
        }
        let mut total = one / delta;
        kahan_add(&mut total, &mut compensation, sum);
        return total;
    }

    let two = T::two();
    if x > one && x <= two {
        euler_maclaurin(x)
    } else {
        borwein(x)
    }
}

// =========================================================================
// Borwein's Algorithm 2
//
// Computes ζ(s) for s > 2 using an alternating series based on Chebyshev
// polynomials. This provides robust numerical stability and rapid convergence
// without relying on Bernoulli numbers.
//
// Reference: [DLMF, §25.2.3], [Borwein00, Algorithm 2]
// =========================================================================

/// Borwein's Algorithm 2 alternating series for `ζ(s)`, `s > 2`.
///
/// Uses Chebyshev‑polynomial‑based `d_k` coefficients (Borwein 2000,
/// Theorem 2) to achieve `O(3⁻ⁿ)` convergence with `n = ZETA_BORWEIN_N`
/// terms. The alternating sign gives robust numerical stability without
/// Bernoulli numbers.
///
/// Reference: [Borwein00, Algorithm 2]
fn borwein<T: SpecFloat>(s: T) -> T {
    let one = T::one();
    let two = T::two();
    let four = T::from_usize(4);
    let n = T::ZETA_BORWEIN_N;

    let denom = one - two.powf(one - s);
    if denom.abs() < T::eps() {
        let delta = s - one;
        return signed_infinity_from_delta(delta);
    }

    let mut d_coeffs = vec![T::zero(); n + 1];
    let n_t = T::from_usize(n);

    let mut term = one / n_t;
    let mut current_inner_sum = term;
    let Some(first) = d_coeffs.get_mut(0) else {
        return T::nan();
    };
    *first = n_t * current_inner_sum;

    for (idx, d_coeff) in d_coeffs.iter_mut().enumerate().skip(1) {
        let k = idx;
        let i = T::from_usize(k - 1);
        let two_i_plus_1 = T::from_usize(2 * k - 1);
        let two_i_plus_2 = T::from_usize(2 * k);
        let n_minus_i = n_t - i;
        let n_plus_i = n_t + i;

        term = term * four * n_plus_i * n_minus_i / (two_i_plus_1 * two_i_plus_2);
        current_inner_sum = current_inner_sum + term;
        *d_coeff = n_t * current_inner_sum;
    }

    let d_n = d_coeffs.get(n).copied().unwrap_or_else(T::zero);

    let mut sum = T::zero();
    let mut compensation = T::zero();

    for (k, d_coeff_k) in d_coeffs.iter().enumerate().take(n) {
        let k_plus_1 = T::from_usize(k + 1);
        let sign = if k % 2 == 0 { one } else { -one };
        let current_term = sign * (*d_coeff_k - d_n) / k_plus_1.powf(s);

        let y = current_term - compensation;
        let t = sum + y;
        compensation = (t - sum) - y;
        sum = t;
    }

    -sum / (d_n * denom)
}

// =========================================================================
// Euler-Maclaurin summation for 1 < s ≤ 2
//
// Since Borwein's algorithm converges slower near s=1 and the Stieltjes
// expansion is only strictly valid very close to s=1, the Euler-Maclaurin
// formula bridges the gap.
//
// Reference: [DLMF, §25.2.2], [Borwein00]
// =========================================================================

/// Euler–Maclaurin summation formula for `ζ(s)`, `1 < s ≤ 2`.
///
/// `ζ(s) = Σ_{k=1}^{N−1} 1/kˢ + N^{1−s}/(s−1) − ½N^{−s}
///         + Σ_{k=1}^{∞} B_{2k}/(2k)! · s^{[2k−1]} · N^{−s−2k+1}`
///
/// where `s^{[m]} = s·(s+1)·…·(s+m−1)` is the rising factorial.
/// The Bernoulli corrections accelerate convergence so that `N = 200`
/// gives full f64 precision for `s ∈ (1, 2]`.
///
/// Reference: [DLMF, §25.2.2]
fn euler_maclaurin<T: SpecFloat>(x: T) -> T {
    let one = T::one();
    // Σ_{k=1}^{200} 1/k^s for s ∈ (1,2] ≈ 100 ± 1; the tail integral
    // ∫_{200}^{∞} x^{-s} dx = 200^{1−s}/(s−1) ≈ 200^{−0.5}/(0.5) ≈ 0.07
    // at worst (s ≈ 1.5). The Bernoulli corrections then handle the
    // O(1/200^{2k+s−1}) terms, bringing the total error below f64 ε.
    let n_terms = 200;
    let mut sum = T::zero();
    let mut compensation = T::zero();

    for k in 1_usize..n_terms {
        let k_t = T::from_usize(k);
        let term = one / k_t.powf(x);
        kahan_add(&mut sum, &mut compensation, term);
    }

    let n = T::from_usize(n_terms);
    let n_pow_x = n.powf(x);
    let n_pow_1_minus_x = n.powf(one - x);
    let em_integral = n_pow_1_minus_x / (x - one);
    let em_boundary = T::half() / n_pow_x;

    let n_sq = n * n;
    let mut n_pow = n_pow_x * n;
    let mut factorial = T::from_usize(2);
    let mut rising_power_prod = x;
    let mut em_correction = T::zero();

    for (k, &(bn, bd)) in T::bernoulli_pairs().iter().enumerate() {
        let term = (bn / bd) * rising_power_prod / (factorial * n_pow);
        if term.abs() < T::eps() * sum.abs() {
            break;
        }
        em_correction = em_correction + term;

        let j = k + 1;
        factorial = factorial * T::from_usize(2 * j + 1) * T::from_usize(2 * j + 2);
        rising_power_prod =
            rising_power_prod * (x + T::from_usize(2 * j - 1)) * (x + T::from_usize(2 * j));
        n_pow = n_pow * n_sq;
    }

    sum + em_integral + em_boundary + em_correction
}

// =========================================================================
// Functional equation (reflection) for s < 0
//
// Uses the reflection formula:
// ζ(s) = 2^s · π^{s-1} · sin(πs/2) · Γ(1-s) · ζ(1-s)
//
// Reference: [DLMF, §25.4.1]
// =========================================================================

#[allow(
    clippy::many_single_char_names,
    reason = "Mathematical formulas use standard notation (s, f, n, a, b, c, d)"
)]
/// Reflection formula: `ζ(s) = 2ˢ·π^{s−1}·sin(πs/2)·Γ(1−s)·ζ(1−s)`.
///
/// Handles the `0·∞` cancellation at `s = 0` (where `sin(πs/2)·ζ(1−s)`
/// has a removable singularity) via a Laurent–Taylor hybrid expansion
/// using Stieltjes constants.
///
/// Reference: [DLMF, §25.4.1]
fn reflection<T: SpecFloat>(s: T) -> T {
    let pi = T::pi();
    let two = T::two();
    let half = T::half();
    let one = T::one();
    let one_minus_s = one - s;

    // Near s=0: sin(πs/2)·ζ(1-s) has 0×∞ cancellation because ζ(1) is a pole.
    // Laurent+Taylor expansion avoids this singularity.
    // Threshold is set to √ε to balance truncation error (O(s^2)) with
    // cancellation error (ε/s).
    let near_zero_threshold = T::eps().sqrt();
    if s.abs() < near_zero_threshold {
        let a_term = two.powf(s);
        let b_term = pi.powf(s - one);
        let d_term = gamma(one_minus_s);
        let stieltjes = T::stieltjes_coeffs();
        let gamma0 = stieltjes.first().copied().unwrap_or_else(T::zero);
        let gamma1 = stieltjes.get(1).copied().unwrap_or_else(T::zero);
        let pi_half = pi * half;
        let c2 = pi_half * gamma1 + pi * pi * pi / T::from_usize(48);
        let h = -pi_half + (pi_half * gamma0 + c2 * s) * s;
        return a_term * b_term * d_term * h;
    }

    // Direct reflection: ζ(s) = 2^s · π^{s-1} · sin(πs/2) · Γ(1-s) · ζ(1-s)
    // Multiply terms in order of decreasing magnitude to minimize rounding.
    let t = s * half;
    let n = t.round();
    let f = t - n;
    let n_int = n.abs();
    let n_mod2 = n_int - (n_int / two).floor() * two;
    let sin_sign = if n_mod2 < half { one } else { -one };
    let c_term = sin_sign * (pi * f).sin();

    let zeta_term = zeta(one_minus_s);
    if c_term == T::zero() || zeta_term == T::zero() {
        return T::zero();
    }

    let gamma_val = gamma(one_minus_s);
    if gamma_val.is_infinite() {
        let two_pi = two * pi;
        let ln_2pi = two_pi.ln();
        let log_abs =
            s * ln_2pi - pi.ln() + c_term.abs().ln() + lgamma(one_minus_s) + zeta_term.abs().ln();
        let sign = c_term.signum() * gamma_sign(one_minus_s) * zeta_term.signum();
        return sign * log_abs.exp();
    }

    let two_pi = two * pi;
    (two_pi.powf(s) / pi) * c_term * gamma_val * zeta_term
}
