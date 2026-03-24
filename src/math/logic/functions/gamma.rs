use super::helpers::{gamma_pole_sign, sign_from_delta, signed_infinity};
use crate::core::traits::MathScalar;

#[inline]
fn lanczos_ag<T: MathScalar>(x_minus_one: T) -> T {
    let c = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    let mut ag = T::from_f64(c[0]).expect("Failed to convert gamma series coefficient");
    for (i, &coeff) in c.iter().enumerate().skip(1) {
        ag += T::from_f64(coeff).expect("Failed to convert gamma coefficient")
            / (x_minus_one + T::from_usize(i).expect("Failed to convert array index"));
    }
    ag
}

/// Gamma function Γ(x) using Lanczos approximation with g=7
///
/// Γ(z+1) ≈ √(2π) (z + g + 1/2)^(z+1/2) e^(-(z+g+1/2)) Aₘ(z)
/// Uses reflection formula for x < 0.5: Γ(z)Γ(1-z) = π/sin(πz)
///
/// Reference: Lanczos (1964) "A Precision Approximation of the Gamma Function"
/// SIAM J. Numerical Analysis, Ser. B, Vol. 1, pp. 86-96
/// See also: DLMF §5.10 <https://dlmf.nist.gov/5.10>
pub fn eval_gamma<T: MathScalar>(x: T) -> T {
    if x <= T::zero() && x.fract() == T::zero() {
        let delta = x - x.round();
        let sign = gamma_pole_sign(x) * sign_from_delta(delta);
        return signed_infinity(sign);
    }
    let half = T::from_f64(0.5).expect("Failed to convert mathematical constant 0.5");
    let one = T::one();
    let pi = T::PI();

    if x < half {
        // Consider adding Stirling's series for large negative x
        if x < T::from_f64(-10.0).expect("Failed to convert constant -10.0") {
            // Use reflection + Gamma(1-x)
            // For large negative x, 1-x is large positive, so Lanczos works well.
            // We return directly to avoid stack depth
            return pi / ((pi * x).sin() * eval_gamma(one - x));
        }
        pi / ((pi * x).sin() * eval_gamma(one - x))
    } else {
        let x_minus_one = x - one;
        let ag = lanczos_ag(x_minus_one);
        let g = T::from_f64(7.0).expect("Failed to convert mathematical constant 7.0");
        let t = x_minus_one + g + half;
        let two_pi_sqrt = (T::from_f64(2.0).expect("Failed to convert constant 2.0") * pi).sqrt();
        two_pi_sqrt * t.powf(x_minus_one + half) * (-t).exp() * ag
    }
}

/// Sign of the Gamma function Γ(x)
///
/// Positive for x > 0 and x ∈ (-2n-1, -2n) for integer n ≥ 0.
/// Negative for x ∈ (-2n, -2n+1) for integer n > 0.
pub(super) fn gamma_sign<T: MathScalar>(x: T) -> T {
    if x > T::zero() {
        T::one()
    } else {
        let n = (-x).to_i64().unwrap_or(0);
        if (n & 1) == 0 { -T::one() } else { T::one() }
    }
}

/// Log-Gamma function ln|Γ(x)|
///
/// Computes the natural logarithm of the absolute value of the Gamma function.
/// Highly accurate and avoids overflow for large x where Γ(x) would exceed `f64::MAX`.
///
/// Reference: Lanczos (1964) "A Precision Approximation of the Gamma Function"
pub fn eval_lgamma<T: MathScalar>(x: T) -> T {
    if x <= T::zero() && x.fract() == T::zero() {
        return T::infinity(); // Pole
    }
    let half = T::from_f64(0.5).expect("Failed to convert mathematical constant 0.5");
    let one = T::one();
    let pi = T::PI();

    if x < half {
        // Reflection formula: ln|Γ(z)| = ln(π) - ln(|sin(πz)|) - ln(|Γ(1-z)|)
        let sin_term = (pi * x).sin().abs();
        pi.ln() - sin_term.ln() - eval_lgamma(one - x)
    } else {
        let x_minus_one = x - one;
        let ag = lanczos_ag(x_minus_one);
        let g = T::from_f64(7.0).expect("Failed to convert mathematical constant 7.0");
        let t = x_minus_one + g + half;
        let two_pi_sqrt_ln = (T::from_f64(2.0).expect("Failed to convert constant 2.0") * pi)
            .sqrt()
            .ln();
        two_pi_sqrt_ln + (x_minus_one + half) * t.ln() - t + ag.abs().ln()
    }
}
