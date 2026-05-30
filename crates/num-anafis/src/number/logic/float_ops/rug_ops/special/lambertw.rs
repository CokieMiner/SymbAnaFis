#![allow(
    clippy::doc_markdown,
    reason = "LaTeX math notation in $...$ / $$...$$ is not recognized by clippy"
)]
//! Lambert W function (principal and -1 branches) using Halley's method.
use super::super::{BackingFloat, get_precision, nan};
use super::bessel::precision_threshold_with;
use crate::number::logic::int_math::IntType;
use rug::Float;

/// Computes the principal branch $W_0(x)$ of the Lambert W function.
///
/// **Algorithm**: Halley's method (Corless et al., 1996, §4.3):
/// $$ w_{n+1} = w_n - \frac{w_n e^{w_n} - x}
///    { e^{w_n}(w_n+1) - \frac{(w_n+2)(w_n e^{w_n} - x)}{2(w_n+1)} } $$
///
/// **Initial estimate** (piecewise; Corless et al., 1996, §5.2):
/// - $x > 3$: $W_0(x) \approx \ln x - \ln\ln x$ (dominant asymptotic)
/// - $0.5 < x \le 3$: use the approximation $\ln(x+1)/2$
///   (minimax error < 0.15 over this interval, Fritsch et al., 1981)
/// - $-1/e \le x \le 0.5$: start with $W_0(x) \approx x$ (linear series)
///
/// **Convergence criterion**: Halley's method converges cubically, so once the
/// correction $|\Delta w| < 2^{-(prec - 10)}$, the iterate is accurate to the
/// full target precision (Corless et al., 1996, §4.5).  The 10-bit threshold
/// margin compensates for the final rounding step.
///
/// **Reference**: Corless, R. M. et al. (1996). "On the Lambert W Function."
/// *Adv. Comput. Math.* 5, 329–359. [DOI:10.1007/BF02124750]
pub fn lambertw(n: &IntType, value: &BackingFloat) -> BackingFloat {
    let Some(order) = n.to_i32() else {
        return nan();
    };
    if order == 0 {
        lambertw0_internal(value)
    } else if order == -1 {
        lambertwm1_internal(value)
    } else {
        nan()
    }
}

fn lambertw0_internal(value: &BackingFloat) -> BackingFloat {
    let prec = get_precision();
    let work_prec = prec + 40;
    let v_w = Float::with_val(work_prec, value);
    let e = Float::with_val(work_prec, 1).exp();
    let neg_inv_e = BackingFloat::with_val(work_prec, -1) / &e;
    if v_w < neg_inv_e {
        return nan();
    }

    let one = BackingFloat::with_val(work_prec, 1);
    let two = BackingFloat::with_val(work_prec, 2);

    let mut w = if v_w > 3 {
        let ln_v = v_w.clone().ln();
        BackingFloat::with_val(work_prec, ln_v.clone() - ln_v.ln())
    } else if BackingFloat::with_val(work_prec, &v_w * 2) > 1 {
        let log1p_v = BackingFloat::with_val(work_prec, &v_w + &one).ln();
        BackingFloat::with_val(work_prec, &log1p_v / 2)
    } else {
        v_w.clone()
    };
    let thr = precision_threshold_with(work_prec);
    loop {
        let ew = w.clone().exp();
        let wew = BackingFloat::with_val(work_prec, &w * &ew);
        let num = BackingFloat::with_val(work_prec, &wew - &v_w);
        let wp1 = BackingFloat::with_val(work_prec, &w + &one);
        let denom = BackingFloat::with_val(
            work_prec,
            &ew * &wp1
                - BackingFloat::with_val(
                    work_prec,
                    BackingFloat::with_val(work_prec, &wp1 + &one) * &num,
                ) / BackingFloat::with_val(work_prec, &two * &wp1),
        );
        let correction = BackingFloat::with_val(work_prec, &num / &denom);
        let new_w = BackingFloat::with_val(work_prec, &w - &correction);
        if BackingFloat::with_val(work_prec, &new_w - &w).abs() < thr {
            break;
        }
        w = new_w;
    }
    Float::with_val(prec, w)
}

/// W₋₁(x) — lower real branch, defined for x ∈ [-1/e, 0). Returns W ≤ -1.
///
/// **Initial estimate** (Corless et al. 1996 §5.2):
/// - Near 0 ($x > -1/100$): $W_{-1}(x) \approx \ln(-x) - \ln(-\ln(-x)) + \ln(-\ln(-x)) / \ln(-x)$
///   (the iterative log asymptotic; the threshold $-1/100$ was chosen so that
///   $|\ln(-x)| \gtrsim 4.6$, ensuring the correction term $\ln(-\ln(-x))$ is well-defined).
/// - Near $-1/e$: use the branch-point expansion
///   $W_{-1}(x) \approx -1 - \sqrt{2(ex+1)}$ (DLMF §4.13, Corless et al. eq. 5.10).
///
/// The same Halley iteration as $W_0$ yields cubic convergence.
///
/// **Reference**: Corless et al. (1996), *ibid.*, §5.2, §4.3.
fn lambertwm1_internal(value: &BackingFloat) -> BackingFloat {
    let prec = get_precision();
    let work_prec = prec + 40;
    let v_w = Float::with_val(work_prec, value);
    let e = Float::with_val(work_prec, 1).exp();
    let neg_inv_e = BackingFloat::with_val(work_prec, -1) / &e;
    if v_w < neg_inv_e || v_w >= 0 {
        return nan();
    }
    let tol = precision_threshold_with(work_prec);
    let delta = BackingFloat::with_val(work_prec, &v_w + &neg_inv_e);
    if delta.abs() < tol {
        return Float::with_val(prec, -1);
    }

    let one = BackingFloat::with_val(work_prec, 1);
    let two = BackingFloat::with_val(work_prec, 2);
    let mut w =
        if v_w > BackingFloat::with_val(work_prec, BackingFloat::with_val(work_prec, -1) / 100) {
            let neg_v = BackingFloat::with_val(work_prec, -v_w.clone());
            let l1 = neg_v.ln();
            let l2 = BackingFloat::with_val(work_prec, -l1.clone()).ln();
            BackingFloat::with_val(work_prec, l1.clone() - l2.clone() + l2 / l1)
        } else {
            let ev = BackingFloat::with_val(work_prec, &e * &v_w);
            let p = BackingFloat::with_val(
                work_prec,
                two.clone() * BackingFloat::with_val(work_prec, &ev + &one),
            )
            .sqrt();
            BackingFloat::with_val(work_prec, -one.clone() - p)
        };

    let thr = precision_threshold_with(work_prec);
    loop {
        let ew = w.clone().exp();
        let wew = BackingFloat::with_val(work_prec, &w * &ew);
        let num = BackingFloat::with_val(work_prec, &wew - &v_w);
        let wp1 = BackingFloat::with_val(work_prec, &w + &one);
        let denom = BackingFloat::with_val(
            work_prec,
            &ew * &wp1
                - BackingFloat::with_val(
                    work_prec,
                    BackingFloat::with_val(work_prec, &wp1 + &one) * &num,
                ) / BackingFloat::with_val(work_prec, &two * &wp1),
        );
        let correction = BackingFloat::with_val(work_prec, &num / &denom);
        let new_w = BackingFloat::with_val(work_prec, &w - &correction);
        if BackingFloat::with_val(work_prec, &new_w - &w).abs() < thr {
            break;
        }
        w = new_w;
    }
    Float::with_val(prec, w)
}
