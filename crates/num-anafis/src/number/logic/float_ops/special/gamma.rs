//! Gamma and log-gamma functions via Lanczos approximation.
//!
//! Reference: [Lanczos64], [DLMF, §5.10]

use super::SpecFloat;
use super::helpers::{gamma_pole_sign, is_non_pos_int, kahan_add, signed_infinity, sin_pi_x};

/// Lanczos `A_g(x)` sum with Kahan-compensated accumulation.
///
/// The Lanczos sum coefficients alternate in sign and span many orders of
/// magnitude, so naive summation loses significant bits to cancellation.
/// Kahan compensation recovers most of those lost bits.
///
/// Uses the full coefficient table with the standard `g = 7` parameter.
#[inline]
fn lanczos_ag<T: SpecFloat>(x_minus_one: T) -> T {
    let coeffs = T::lanczos_coeffs();
    let mut lanczos_sum = coeffs.first().copied().unwrap_or_else(T::zero);
    let mut compensation = T::zero();
    // The table is the common 9-coefficient Lanczos fit: a0 plus eight
    // correction terms. Omitting the final small term shifts lgamma by
    // roughly 1e-10 in the reflected negative domain.
    for (i, &coeff) in coeffs.iter().enumerate().skip(1) {
        let term = coeff / (x_minus_one + T::from_usize(i));
        kahan_add(&mut lanczos_sum, &mut compensation, term);
    }
    lanczos_sum
}

/// Gamma function Γ(x) — Lanczos approximation with reflection.
pub fn gamma<T: SpecFloat>(x: T) -> T {
    // Negative integer poles: the sign convention is direction-agnostic
    // (sign depends only on the parity of the integer, not on the approach direction).
    if is_non_pos_int(x) {
        let sign = gamma_pole_sign(x);
        return signed_infinity(sign);
    }
    let half = T::half();
    let one = T::one();
    let pi = T::pi();

    if x < half {
        // Reflection formula: Γ(x) Γ(1-x) = π / sin(πx)
        // DLMF §5.5.3
        pi / (sin_pi_x(x) * gamma(one - x))
    } else if x <= T::from_usize(4) {
        // Small x: direct Lanczos is safe (the log→exp error amplification
        // is negligible when gamma(x) is small).
        let xm1 = x - one;
        let lanczos_sum = lanczos_ag(xm1);
        let lanczos_g = T::from_usize(T::lanczos_coeffs().len() - 2);
        let t = xm1 + lanczos_g + half;
        let sqrt_2pi = (T::two() * pi).sqrt();
        let exponent = xm1 + half;
        let ln_t = t.ln();
        let log_power = exponent.mul_add(ln_t, -t);
        let power_term = log_power.exp();
        sqrt_2pi * power_term * lanczos_sum
    } else {
        // Recurrence: Γ(x) = Γ(f) · f · (f+1) · … · (x−1)
        //
        // Shift x down to f ∈ [2.5, 3.5), compute Γ(f) via the direct
        // Lanczos path above (which is accurate for small arguments),
        // then multiply back up.
        //
        // This avoids the log→exp amplification that dominates the error
        // in the direct Lanczos/Stirling path for large x.
        let target = T::from_usize(3);
        let x_floor = x.floor();
        let frac = x - x_floor;
        let mut f = frac + target;
        if f > T::from_usize(4) {
            f = f - one;
        }
        // f ∈ [2.0, 4.0), compute gamma(f) via the direct small-x path
        let gamma_f = {
            let xm1 = f - one;
            let lanczos_sum = lanczos_ag(xm1);
            let lanczos_g = T::from_usize(T::lanczos_coeffs().len() - 2);
            let t = xm1 + lanczos_g + half;
            let sqrt_2pi = (T::two() * pi).sqrt();
            let exponent = xm1 + half;
            let ln_t = t.ln();
            let log_power = exponent.mul_add(ln_t, -t);
            let power_term = log_power.exp();
            sqrt_2pi * power_term * lanczos_sum
        };
        let n_steps_approx = (x - f).round();
        if n_steps_approx > T::from_usize(8) {
            // Use log-space: ln Γ(x) = ln Γ(f) + Σ ln(k)
            let mut log_sum = gamma_f.abs().ln();
            let mut log_comp = T::zero();
            let mut k = f;
            while k + half < x {
                kahan_add(&mut log_sum, &mut log_comp, k.abs().ln());
                k = k + one;
            }
            let sign = gamma_f.signum();
            sign * (log_sum + log_comp).exp()
        } else {
            let mut result = gamma_f;
            let mut k = f;
            while k + half < x {
                result = result * k;
                k = k + one;
                if result.is_infinite() {
                    return result;
                }
            }
            result
        }
    }
}

/// Stirling's series for ln|Γ(x)|, x ≥ threshold.
///
/// Uses the asymptotic expansion:
///   ln Γ(x) = (x-½)ln(x) - x + ½ln(2π) + Σ B_{2k}/(2k(2k-1)·x^{2k-1})
fn stirling<T: SpecFloat>(x: T) -> T {
    let one = T::one();
    let inv_x = one / x;
    let inv_x2 = inv_x * inv_x;

    let coeffs = T::stirling_coeffs();
    let mut series = T::zero();
    for &c in coeffs.iter().rev() {
        series = series.mul_add(inv_x2, c);
    }

    let sqrt_2pi_ln = (T::two() * T::pi()).sqrt().ln();
    (x - T::half()).mul_add(x.ln(), -x) + sqrt_2pi_ln + series * inv_x
}

/// Sign of `Γ(x)`: `+1` for `x > 0`, `(−1)^{⌊|x|⌋+1}` for `x < 0`.
///
/// For negative non‑integer `x`, `Γ(x)` alternates sign each time it crosses
/// a pole (DLMF §5.5.3). The sign is `(−1)^{⌈|x|⌉}`.
pub(super) fn gamma_sign<T: SpecFloat>(x: T) -> T {
    if x > T::zero() {
        T::one()
    } else {
        // Sign is (-1)^n where n = floor(|x|) + 1
        let ix = x.abs().floor();
        let two = T::two();
        if (ix / two).fract() == T::zero() {
            T::neg_one()
        } else {
            T::one()
        }
    }
}

/// Log absolute gamma `ln|Γ(x)|` — avoids overflow for large x.
///
/// Returns the natural logarithm of the absolute value of the gamma function.
/// Does **not** return the sign of Γ(x); use [`gamma_sign`] separately if needed.
///
/// The Stirling threshold is set at 10 (not 15) because Stirling's asymptotic
/// expansion converges faster and with fewer accumulated rounding errors than
/// the Lanczos approach for moderate-to-large x.
pub fn lgamma<T: SpecFloat>(x: T) -> T {
    if is_non_pos_int(x) {
        return T::infinity();
    }
    let half = T::half();
    let one = T::one();
    let pi = T::pi();

    if x < T::zero() {
        if x > -T::from_usize(10) {
            // Shift up using recurrence to avoid reflection formula cancellation
            let mut xv = x;
            let mut shift_correction = T::zero();
            let mut shift_comp = T::zero();
            while xv < T::from_usize(5) / T::two() {
                kahan_add(&mut shift_correction, &mut shift_comp, -xv.abs().ln());
                xv = xv + one;
            }
            lgamma(xv) + shift_correction + shift_comp
        } else {
            // Reflection formula for log-gamma:
            // ln Γ(x) = ln(π) - ln(|sin(πx)|) - ln Γ(1-x)
            // DLMF §5.5.3
            let sin_term = sin_pi_x(x).abs();
            if sin_term == T::zero() {
                return T::infinity();
            }
            pi.ln() - sin_term.ln() - lgamma(one - x)
        }
    } else if x > T::from_usize(10) {
        stirling(x)
    } else {
        // Taylor expansions near x=1 and x=2 to avoid catastrophic cancellation.
        // DLMF §5.7.1 (Taylor series expansion of ln Γ(x))
        let dist1 = (x - one).abs();
        let dist2 = (x - T::two()).abs();
        // 0.4 threshold bounds the maximum evaluated argument for the Taylor
        // series (up to k=80) ensuring truncation error < machine epsilon.
        let threshold = T::from_usize(2) / T::from_usize(5);
        if dist1 <= threshold {
            let z = x - one;
            let gamma0 = T::stieltjes_coeffs()
                .first()
                .copied()
                .unwrap_or_else(T::zero);
            let mut sum = -gamma0 * z;
            let mut comp = T::zero();
            let mut z_pow = z;
            let eps = T::eps();
            // Up to 80 terms guarantees full precision convergence within |z| <= 0.4
            for k in 2..=80 {
                z_pow = z_pow * z;
                let zk = T::zeta_ints().get(k - 2).copied().unwrap_or(one);
                let sign = if k % 2 == 0 { T::one() } else { -T::one() };
                let term = sign * zk * z_pow / T::from_usize(k);
                kahan_add(&mut sum, &mut comp, term);
                if term.abs() <= eps * (sum.abs() + T::one()) {
                    break;
                }
            }
            return sum + comp;
        } else if dist2 <= threshold {
            let z = x - T::two();
            let gamma0 = T::stieltjes_coeffs()
                .first()
                .copied()
                .unwrap_or_else(T::zero);
            let mut sum = (one - gamma0) * z;
            let mut comp = T::zero();
            let mut z_pow = z;
            let eps = T::eps();
            // Up to 80 terms guarantees full precision convergence within |z| <= 0.4
            for k in 2..=80 {
                z_pow = z_pow * z;
                let zk_minus_1 = T::zeta_ints().get(k - 2).copied().unwrap_or(one) - one;
                let sign = if k % 2 == 0 { T::one() } else { -T::one() };
                let term = sign * zk_minus_1 * z_pow / T::from_usize(k);
                kahan_add(&mut sum, &mut comp, term);
                if term.abs() <= eps * (sum.abs() + T::one()) {
                    break;
                }
            }
            return sum + comp;
        }

        // Near x=1 or x=2, lgamma(x) ≈ 0 and the Lanczos formula suffers
        // catastrophic cancellation. Use recurrence to shift x into [2.5, 3.5]
        // where lgamma is well-conditioned (~0.5–1.0).
        //
        // lgamma(x) = lgamma(x+1) - ln(x)    [shift up]
        // lgamma(x) = lgamma(x-1) + ln(x-1)  [shift down]
        let mut xv = x;
        let mut shift_correction = T::zero();
        let mut shift_comp = T::zero();

        // Shift up to at least 2.5
        while xv < T::from_usize(5) / T::two() {
            kahan_add(&mut shift_correction, &mut shift_comp, -xv.abs().ln());
            xv = xv + one;
        }
        // Shift down from above 3.5
        while xv > T::from_usize(7) / T::two() {
            xv = xv - one;
            kahan_add(&mut shift_correction, &mut shift_comp, xv.ln());
        }

        let xm1 = xv - one;
        let lanczos_sum = lanczos_ag(xm1);
        let lanczos_g = T::from_usize(T::lanczos_coeffs().len() - 2);
        let t = xm1 + lanczos_g + half;
        let sqrt_2pi_ln = (T::two() * pi).sqrt().ln();
        sqrt_2pi_ln
            + (xm1 + half).mul_add(t.ln(), -t)
            + lanczos_sum.abs().ln()
            + shift_correction
            + shift_comp
    }
}
