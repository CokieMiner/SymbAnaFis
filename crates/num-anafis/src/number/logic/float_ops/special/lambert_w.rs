//! Lambert W function via Halley iteration.
//!
//! Reference: [Corless96], [DLMF, §4.13]

use super::SpecFloat;

/// Halley iteration core shared by W₀ and W₋₁.
#[allow(clippy::many_single_char_names, reason = "Mathematical variables")]
fn halley_iter<T: SpecFloat>(x: T, mut w: T) -> T {
    let one = T::one();
    let tolerance = T::eps();
    let two = T::two();
    let half = T::half();

    for iter in 0..T::LAMBERT_MAX_ITERATIONS {
        if (w + one).abs() < T::eps() {
            break;
        }
        if iter > 2 && (w + one).abs() < T::eps().sqrt() {
            break;
        }
        let ew = w.exp();
        let wew = w * ew;
        let f = wew - x;
        let w1 = w + one;
        let fp = ew * w1;
        let fpp = ew * (w + two);
        let d = f * fp / (fp * fp - half * f * fpp);
        w = w - d;
        if d.abs() < tolerance * (one + w.abs()) {
            break;
        }
    }
    w
}

/// W(x) solves W·e^W = x (principal branch W₀).
pub fn lambertw0<T: SpecFloat>(x: T) -> T {
    let one = T::one();
    let e = T::e();
    let e_inv = one / e;

    if x < -e_inv {
        return T::nan();
    }
    if x == T::zero() {
        return T::zero();
    }
    if (x + e_inv).abs() < T::eps() {
        return T::neg_one();
    }

    // Polynomial and threshold constants for the piecewise initial guesses
    let m_three_tenths = -T::from_usize(3) / T::from_usize(10);
    let one_point_five = T::from_usize(3) / T::from_usize(2);
    // 11/72 is the coefficient of the cubic term in the branch point expansion
    let c1 = T::from_usize(11) / T::from_usize(72);

    // Initial guesses: Corless et al. (1996)
    // The domain is partitioned to provide an initial guess within the rapid
    // convergence basin of Halley's method.
    // 1. x < -0.3: Branch point expansion (Eq. 4.22)
    // 2. x < 0: Truncated branch point expansion
    // 3. x < 1: Padé-like rational approximation
    // 4. x < 3: Clamped log-log approximation (Eq. 4.19)
    // 5. x >= 3: Full log-log asymptotic expansion
    let w = if x < m_three_tenths {
        let two = T::two();
        let arg = (two * (e * x + one)).max(T::zero());
        let p = arg.sqrt();
        let third = T::from_usize(3);
        -one + p - p * p / third + c1 * p * p * p
    } else if x < T::zero() {
        let two = T::two();
        let p = (two * (e * x + one)).sqrt();
        -one + p
    } else if x < one {
        x * (one - x * (one - x * one_point_five))
    } else if x < T::from_usize(3) {
        // Log-log initial guess: Corless et al. (1996), Eq. (4.19)
        let l = x.ln();
        let l_ln = l.ln();
        // Clamp ln(ln(x)) to 0 when x < e to keep initial guess non-negative
        let log_log_x_clamped = if l_ln > T::zero() { l_ln } else { T::zero() };
        l - log_log_x_clamped
    } else {
        let l1 = x.ln();
        let l2 = l1.ln();
        l1 - l2 + l2 / l1
    };

    halley_iter(x, w)
}

/// W₋₁(x) — the lower real branch, defined for x ∈ [-1/e, 0).
/// Returns W ≤ -1; NaN outside the domain.
pub fn lambertwm1<T: SpecFloat>(x: T) -> T {
    let one = T::one();
    let e = T::e();
    let e_inv = one / e;

    if x < -e_inv || x >= T::zero() {
        return T::nan();
    }
    if (x + e_inv).abs() < T::eps() {
        return T::neg_one();
    }

    // Initial guess: use log-log approximation for most of [-1/e, 0);
    // switch to branch-point expansion very near -1/e.
    // Branch-point expansion: Corless et al. (1996), Eq. (4.23)
    // Width 0.1 balances the accuracy of the two-term branch-point expansion
    // (p + p²/3) against the log-log form, keeping both within Halley's
    // convergence basin (~0.1 ULP after 2‑3 iterations for f64).
    let branch_width = T::from_usize(1) / T::from_usize(10);
    let w = if (x + e_inv).abs() < T::eps().sqrt() {
        // Very near branch point: W₋₁ ≈ -1 - p - p²/3 - 11p³/72
        // where p = √(2(ex+1))
        let two = T::two();
        let p = (two * (e * x + one)).sqrt();
        let third = one / T::from_usize(3);
        let c3 = T::from_usize(11) / T::from_usize(72);
        -one - p - third * p * p - c3 * p * p * p
    } else if (x + e_inv).abs() < branch_width {
        // Intermediate region: blend branch-point and log approaches
        let two = T::two();
        let p = (two * (e * x + one)).sqrt();
        let third = one / T::from_usize(3);
        -one - p - third * p * p
    } else {
        // Log-log approximation good away from the branch point
        let l1 = (-x).ln();
        let l2 = (-l1).ln();
        l1 - l2 + l2 / l1
    };

    let w_refined = halley_iter(x, w);

    // Post-Halley refinement: one Newton step on the residual to clean up
    // the last ULP of error. This is cheap and eliminates rounding from
    // the Halley iteration's more complex update formula.
    let ew = w_refined.exp();
    let wew = w_refined * ew;
    let f = wew - x;
    let fp = ew * (w_refined + one);
    if fp.abs() > T::eps() {
        w_refined - f / fp
    } else {
        w_refined
    }
}
