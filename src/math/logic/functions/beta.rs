use super::gamma::{eval_lgamma, gamma_sign};
use super::helpers::signed_infinity;
use crate::core::traits::MathScalar;

/// Beta function B(x, y) = Γ(x)Γ(y) / Γ(x+y)
///
/// Computed via the Log-Gamma function to avoid numerical overflow:
/// B(x, y) = sign * exp(ln|Γ(x)| + ln|Γ(y)| - ln|Γ(x+y)|)
///
/// Reference: DLMF §5.12 <https://dlmf.nist.gov/5.12>
pub fn eval_beta<T: MathScalar>(x: T, y: T) -> T {
    if x <= T::zero() && x.fract() == T::zero() {
        return signed_infinity(gamma_sign(x));
    }
    if y <= T::zero() && y.fract() == T::zero() {
        return signed_infinity(gamma_sign(y));
    }
    let xy = x + y;
    if xy <= T::zero() && xy.fract() == T::zero() {
        // Gamma pole in denominator -> Beta = 0
        return T::zero();
    }

    let sign_x = gamma_sign(x);
    let sign_y = gamma_sign(y);
    let sign_sum = gamma_sign(xy);

    // Because signs are strictly ±1, mult and div are equivalent
    let sign = sign_x * sign_y * sign_sum;
    let lbeta = eval_lgamma(x) + eval_lgamma(y) - eval_lgamma(xy);

    sign * lbeta.exp()
}
