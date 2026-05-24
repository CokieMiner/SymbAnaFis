//! Associated Legendre functions and spherical harmonics.
//!
//! References: [DLMF, §14.3], [DLMF, §14.9], [DLMF, §14.10], [DLMF, §14.30]
//! - DLMF §14 (Legendre and associated Legendre functions)
//! - DLMF §14.30 (spherical harmonics)

use super::helpers::{kahan_add, legendre_factorial_ratio};
use super::{SpecFloat, SpecInt};

/// Associated Legendre function `P_l^m(x)` for `|x| ≤ 1`.
///
/// Uses forward recurrence from the sectoral value `P_m^m`, which is stable
/// for the moderate degrees used by the scalar API when `|x| ≤ 1`.
#[allow(
    clippy::many_single_char_names,
    reason = "Standard mathematical notation: l, m, x, s, k"
)]
pub fn assoc_legendre<T: SpecFloat, I: SpecInt>(l: I, m: I, x: T) -> T {
    assoc_legendre_core(l, m, x, None)
}

/// Core Legendre with optional pre-computed sin(θ).
///
/// When `sin_x` is provided, it replaces `sqrt(1-x²)` for the
/// seed `P_m^m`. This avoids catastrophic cancellation when `x ≈ ±1`
/// (i.e., θ ≈ 0 or π), where `1 - cos²θ` rounds to zero in finite precision.
#[allow(
    clippy::many_single_char_names,
    reason = "Standard mathematical notation: l, m, x, s, k"
)]
#[allow(
    clippy::too_many_lines,
    reason = "Recurrence logic is kept in one function for clarity"
)]
fn assoc_legendre_core<T: SpecFloat, I: SpecInt>(l: I, m: I, x: T, sin_x: Option<T>) -> T {
    if l.is_negative() {
        return T::nan();
    }
    let m_abs = m.abs();
    if m_abs > l {
        return T::nan();
    }
    let x_abs = x.abs();
    if x.is_nan() || x_abs > T::one() {
        return T::nan();
    }
    let one = T::one();
    let two = T::two();

    let result = 'blk: {
        // -- m ≠ 0: start from sectoral P_m^m ---
        if !m_abs.is_zero() {
            let sqx = sin_x.map_or_else(
                || {
                    let one_minus_x2 = (-x).mul_add(x, one);
                    one_minus_x2.max(T::zero()).sqrt()
                },
                SpecFloat::abs,
            );
            let mut pmm = T::one();
            let mut fact = T::one();
            let mut cnt = I::zero();
            while cnt < m_abs {
                pmm = pmm * (-fact) * sqx;
                fact = fact + two;
                cnt = cnt + I::one();
            }
            if l == m_abs {
                break 'blk pmm;
            }

            let two_m_plus_1 = T::from_int(m_abs + m_abs + I::one());
            let pmmp1 = x * two_m_plus_1 * pmm;

            if l == m_abs + I::one() {
                break 'blk pmmp1;
            }

            let mut pmm_prev = pmm;
            let mut pmm_curr = pmmp1;
            let mut e_prev = T::zero();
            let mut e_curr = T::zero();
            let mut ll = m_abs + I::from_usize(2);

            while ll <= l {
                let f_ll = T::from_int(ll);
                let f_m_abs = T::from_int(m_abs);
                let denom = f_ll - f_m_abs;
                let c_a = x * T::from_int(ll + ll - I::one());
                let c_b = T::from_int(ll + m_abs - I::one());

                // Compensated recurrence: track exact round-off error using FMA
                let prod1 = c_a * pmm_curr;
                let prod2 = c_b * pmm_prev;
                let diff = prod1 - prod2;
                let pll_main = diff / denom;

                let res1 = c_a.mul_add(pmm_curr, -prod1);
                let res2 = c_b.mul_add(pmm_prev, -prod2);
                let diff_err = (prod1 - diff) - prod2;
                let div_err = pll_main.mul_add(-denom, diff) / denom;

                let err_next =
                    (c_a * e_curr - c_b * e_prev + res1 - res2 + diff_err) / denom + div_err;
                let pll = pll_main + err_next;

                if pll.is_infinite() {
                    break 'blk pll;
                }
                e_prev = e_curr;
                e_curr = (pll_main - pll) + err_next;
                pmm_prev = pmm_curr;
                pmm_curr = pll;
                ll = ll + I::one();
            }
            break 'blk pmm_curr;
        }

        // -- m = 0: standard Legendre recurrence ---
        if l.is_zero() {
            break 'blk T::one();
        }
        if l == I::one() {
            break 'blk x;
        }

        let mut p0 = T::one();
        let mut p1 = x;
        let mut ll = I::from_usize(2);
        while ll <= l {
            let f_ll = T::from_int(ll);
            let p2 =
                (T::from_int(ll + ll - I::one()) * x * p1 - T::from_int(ll - I::one()) * p0) / f_ll;
            if p2.is_infinite() {
                break 'blk p2;
            }
            p0 = p1;
            p1 = p2;
            ll = ll + I::one();
        }
        p1
    };

    if m.is_negative() && !result.is_infinite() && !result.is_nan() {
        result * neg_m_factor::<T, I>(l, m_abs)
    } else {
        result
    }
}

/// Real spherical harmonic `Y_l^m(θ, φ)`.
///
/// Passes sin(θ) directly to the Legendre function to avoid the catastrophic
/// cancellation in sqrt(1 − cos²θ) when θ ≈ 0 or θ ≈ π.
pub fn spherical_harmonic<T: SpecFloat, I: SpecInt>(l: I, m: I, theta: T, phi: T) -> T {
    if l.is_negative() {
        return T::nan();
    }
    let m_abs = m.abs();
    if m_abs > l {
        return T::nan();
    }
    let cos_theta = theta.cos();
    let sin_theta = theta.sin();

    // Always compute P_l^{|m|} (positive order) to avoid the neg_m_factor
    // double-application issue. The Condon-Shortley phase for negative m
    // is handled explicitly below.
    let plm = assoc_legendre_core(l, m_abs, cos_theta, Some(sin_theta));

    // Normalization sqrt((2l+1)/(4π) · (l-|m|)!/(l+|m|)!)
    // Compute the factorial ratio directly to avoid log-space precision loss.
    // (l-|m|)!/(l+|m|)! = 1/((l-|m|+1)·(l-|m|+2)·…·(l+|m|))
    // Reference: [DLMF, §14.30.1]
    let two_l_plus_1 = T::from_int(l + l + I::one());
    let four_pi = T::from_usize(4) * T::pi();

    // 120 is the threshold where linear factorial accumulation risks overflowing
    // typical f64 precision limits, prompting a switch to log-space.
    let norm = if l + m_abs < I::from_usize(120) {
        let mut ratio = T::one();
        let start = l - m_abs + I::one();
        let end = l + m_abs;
        let mut j = start;
        while j <= end {
            ratio = ratio / T::from_int(j);
            j = j + I::one();
        }
        ((two_l_plus_1 / four_pi) * ratio).sqrt()
    } else {
        // Log-space normalization to avoid underflow from cascaded divisions
        let mut log_ratio = T::zero();
        let mut log_comp = T::zero();
        let start = l - m_abs + I::one();
        let end = l + m_abs;
        let mut j = start;
        while j <= end {
            kahan_add(&mut log_ratio, &mut log_comp, -T::from_int(j).ln());
            j = j + I::one();
        }
        let log_norm_sq = (two_l_plus_1 / four_pi).ln() + log_ratio + log_comp;
        (log_norm_sq * T::half()).exp()
    };

    // Condon-Shortley phase: (-1)^|m| for negative m
    let cs_phase = if m.is_negative() && (m_abs % I::from_usize(2)) == I::one() {
        T::neg_one()
    } else {
        T::one()
    };

    let m_abs_phi = T::from_int(m_abs) * phi;
    cs_phase * norm * plm * m_abs_phi.cos()
}

// =========================================================================
// Internal helpers
// =========================================================================

/// Factor for negative `m` in the associated Legendre function.
///
/// `P_l^{−m}(x) = (−1)^m · (l−m)!/(l+m)! · P_l^m(x)`
///
/// The sign `(−1)^m` comes from the Condon–Shortley phase convention.
/// The factorial ratio is computed via [`legendre_factorial_ratio`].
///
/// Reference: [DLMF, §14.9.3]
#[allow(
    clippy::many_single_char_names,
    reason = "Standard mathematical notation: l, m, x, s, k"
)]
fn neg_m_factor<T: SpecFloat, I: SpecInt>(l: I, m_abs: I) -> T {
    let sign = if (m_abs % I::from_usize(2)).is_zero() {
        T::one()
    } else {
        T::neg_one()
    };
    sign * legendre_factorial_ratio::<T, I>(l, m_abs)
}
