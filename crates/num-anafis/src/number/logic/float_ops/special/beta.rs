//! Beta function B(x,y) = Γ(x)Γ(y)/Γ(x+y).
//!
//! Reference: [DLMF, §5.12]

use super::SpecFloat;
use super::gamma::{gamma, gamma_sign, lgamma};
use super::helpers::{is_non_pos_int, kahan_add};

/// B(x,y) = Γ(x)Γ(y)/Γ(x+y).
///
/// Strategy:
/// 1. Shift arguments below 1 upward with the coupled beta recurrence.
/// 2. For small positive args (all < 40), use direct Γ(x)Γ(y)/Γ(x+y).
///    40 is a safe threshold since Γ(40) ≈ 2·10⁴⁷ fits comfortably within f64,
///    avoiding the cancellation error intrinsic to lgamma(x)+lgamma(y)−lgamma(x+y).
/// 3. Otherwise, use the log-gamma path on positive arguments only.
pub fn beta<T: SpecFloat>(x: T, y: T) -> T {
    if is_non_pos_int(x) {
        let sign = gamma_sign(x);
        return if sign.is_sign_negative() {
            T::neg_infinity()
        } else {
            T::infinity()
        };
    }
    if is_non_pos_int(y) {
        let sign = gamma_sign(y);
        return if sign.is_sign_negative() {
            T::neg_infinity()
        } else {
            T::infinity()
        };
    }
    let xy = x + y;
    if is_non_pos_int(xy) {
        return T::zero();
    }

    let one = T::one();

    // Coupled recurrence:
    // B(x,y) = ((x+y)/x) B(x+1,y) = ((x+y)/y) B(x,y+1).
    //
    // Shift negative and small positive arguments into the positive
    // well-conditioned region before evaluating the gamma ratio. This avoids
    // subtracting two reflected lgamma values that are nearly equal.
    let mut xs = x;
    let mut ys = y;
    let mut log_factor = T::zero();
    let mut log_factor_comp = T::zero();
    let mut factor_sign = T::one();
    while xs < one {
        let term = (xs + ys) / xs;
        factor_sign = factor_sign * term.signum();
        kahan_add(&mut log_factor, &mut log_factor_comp, term.abs().ln());
        xs = xs + one;
    }
    while ys < one {
        let term = (xs + ys) / ys;
        factor_sign = factor_sign * term.signum();
        kahan_add(&mut log_factor, &mut log_factor_comp, term.abs().ln());
        ys = ys + one;
    }
    let factor = factor_sign * (log_factor + log_factor_comp).exp();

    let xys = xs + ys;

    // Use direct gamma only for small arguments where all three gamma values
    // are finite and the division gx·gy/gxy is well-conditioned.
    // 40 is a safe threshold since Γ(40) ≈ 2e47 fits comfortably within f64,
    // avoiding the cancellation error intrinsic to lgamma(x) + lgamma(y) - lgamma(x+y).
    let small = T::from_usize(40);
    if xs < small && ys < small && xys < small {
        let gx = gamma(xs);
        let gy = gamma(ys);
        let gxy = gamma(xys);
        if !gx.is_infinite() && !gy.is_infinite() && !gxy.is_infinite() && gxy != T::zero() {
            return factor * gx * gy / gxy;
        }
    }

    factor * (lgamma(xs) + lgamma(ys) - lgamma(xys)).exp()
}
