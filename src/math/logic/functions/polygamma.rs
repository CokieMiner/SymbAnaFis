use super::helpers::{sign_from_delta, signed_infinity};
use crate::core::traits::MathScalar;

/// Digamma function ψ(x) = Γ'(x)/Γ(x) = d/dx ln(Γ(x))
///
/// Uses asymptotic expansion for large x:
/// ψ(x) ~ ln(x) - 1/(2x) - 1/(12x²) + 1/(120x⁴) - 1/(252x⁶) + ...
/// Uses reflection formula for x < 0.5: ψ(1-x) - ψ(x) = π cot(πx)
///
/// Reference: DLMF §5.11 <https://dlmf.nist.gov/5.11>
pub fn eval_digamma<T: MathScalar>(x: T) -> T {
    if x <= T::zero() && x.fract() == T::zero() {
        let delta = x - x.round();
        let sign = -sign_from_delta(delta);
        return signed_infinity(sign);
    }
    let half = T::from_f64(0.5).expect("Failed to convert constant 0.5");
    let one = T::one();
    let pi = T::PI();

    if x < half {
        return eval_digamma(one - x) - pi * (pi * x).cos() / (pi * x).sin();
    }
    let mut xv = x;
    let mut result = T::zero();
    let six = T::from_f64(6.0).expect("Failed to convert constant 6.0");
    while xv < six {
        result -= one / xv;
        xv += one;
    }
    result += xv.ln() - half / xv;
    let x2 = xv * xv;

    let t1 = one / (T::from_f64(12.0).expect("Failed to convert constant 12.0") * x2);
    let t2 = one / (T::from_f64(120.0).expect("Failed to convert constant 120.0") * x2 * x2);
    let t3 = one / (T::from_f64(252.0).expect("Failed to convert constant 252.0") * x2 * x2 * x2);

    result - t1 + t2 - t3
}

/// Trigamma function ψ₁(x) = d²/dx² ln(Γ(x))
///
/// Uses asymptotic expansion: ψ₁(x) ~ 1/x + 1/(2x²) + 1/(6x³) - 1/(30x⁵) + ...
/// with recurrence for small x: ψ₁(x) = ψ₁(x+1) + 1/x²
///
/// Reference: DLMF §5.15 <https://dlmf.nist.gov/5.15>
pub fn eval_trigamma<T: MathScalar>(x: T) -> T {
    if x <= T::zero() && x.fract() == T::zero() {
        return T::infinity();
    }
    let mut xv = x;
    let mut r = T::zero();
    let six = T::from_f64(6.0).expect("Failed to convert constant 6.0");
    let one = T::one();

    while xv < six {
        r += one / (xv * xv);
        xv += one;
    }
    let x2 = xv * xv;
    let half = T::from_f64(0.5).expect("Failed to convert constant 0.5");

    r + one / xv + half / x2 + one / (six * x2 * xv)
        - one / (T::from_f64(30.0).expect("Failed to convert constant 30.0") * x2 * x2 * xv)
        + one / (T::from_f64(42.0).expect("Failed to convert constant 42.0") * x2 * x2 * x2 * xv)
}

/// Tetragamma function ψ₂(x) = d³/dx³ ln(Γ(x))
///
/// Uses asymptotic expansion with recurrence for small x.
///
/// Reference: DLMF §5.15 <https://dlmf.nist.gov/5.15>
pub fn eval_tetragamma<T: MathScalar>(x: T) -> T {
    if x <= T::zero() && x.fract() == T::zero() {
        let delta = x - x.round();
        let sign = -sign_from_delta(delta);
        return signed_infinity(sign);
    }
    let mut xv = x;
    let mut r = T::zero();
    let six = T::from(6.0).expect("Failed to convert mathematical constant");
    let one = T::one();
    let two = T::from(2.0).expect("Failed to convert mathematical constant");

    while xv < six {
        r -= two / (xv * xv * xv);
        xv += one;
    }
    let x2 = xv * xv;
    r - one / x2 + one / (x2 * xv) + one / (two * x2 * x2) + one / (six * x2 * x2 * xv)
}

/// Polygamma function ψⁿ(x) = d^(n+1)/dx^(n+1) ln(Γ(x))
///
/// Uses recurrence to shift argument, then asymptotic expansion with Bernoulli numbers:
/// ψⁿ(x) = (-1)^(n+1) n! Σ_{k=0}^∞ 1/(x+k)^(n+1)
///
/// Reference: DLMF §5.15 <https://dlmf.nist.gov/5.15>
pub fn eval_polygamma<T: MathScalar>(n: i32, x: T) -> T {
    if n < 0 {
        return T::nan();
    }
    match n {
        0 => eval_digamma(x),
        1 => eval_trigamma(x),
        // For n >= 2, use general formula (tetragamma had accuracy issues)
        _ => {
            if x <= T::zero() && x.fract() == T::zero() {
                let delta = x - x.round();
                let sign = if n % 2 == 0 {
                    -sign_from_delta(delta)
                } else {
                    T::one()
                };
                return signed_infinity(sign);
            }
            let mut xv = x;
            let mut r = T::zero();
            // ψ^(n)(x) = (-1)^(n+1) * n! * Σ_{k=0}^∞ 1/(x+k)^(n+1)
            let sign = if (n + 1) % 2 == 0 {
                T::one()
            } else {
                -T::one()
            };

            // Factorial up to n
            let mut factorial = T::one();
            for i in 1..=n {
                factorial *= T::from_i32(i).expect("Failed to convert factorial counter");
            }

            let fifteen = T::from_f64(15.0).expect("Failed to convert constant 15.0");
            let one = T::one();
            let n_plus_one = n + 1;

            while xv < fifteen {
                // r += sign * factorial / xv^(n+1)
                r += sign * factorial / xv.powi(n_plus_one);
                xv += one;
            }

            let asym_sign = if n % 2 == 0 { -T::one() } else { T::one() };
            // Fix: Store as (num, den) tuples to compute exact T values avoiding f64 truncation
            // B2=1/6, B4=-1/30, B6=1/42, B8=-1/30, B10=5/66
            let bernoulli_pairs = [
                (1.0, 6.0),
                (-1.0, 30.0),
                (1.0, 42.0),
                (-1.0, 30.0),
                (5.0, 66.0),
            ];

            // (n-1)!
            let mut n_minus_1_fact = T::one();
            if n > 1 {
                for i in 1..n {
                    n_minus_1_fact *= T::from_i32(i).expect("Failed to convert factorial counter");
                }
            }

            // term 1: (n-1)! / xv^n
            let mut sum = n_minus_1_fact / xv.powi(n);
            // term 2: n! / (2 xv^(n+1))
            let two = T::from(2.0).expect("Failed to convert mathematical constant");
            sum += factorial / (two * xv.powi(n_plus_one));

            let mut xpow = xv.powi(n + 2);
            let mut fact_ratio =
                factorial * T::from(n + 1).expect("Failed to convert mathematical constant");

            let mut prev_term_abs = T::max_value();

            for (k, &(b_num, b_den)) in bernoulli_pairs.iter().enumerate() {
                #[allow(clippy::cast_possible_wrap, reason = "k is small and within i32 range")]
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "k is small and within i32 range"
                )]
                let k_plus_1 = (k + 1) as i32;
                let two_k: i32 = 2 * k_plus_1;
                // (2k)!
                let mut factorial_2k = T::one();
                for i in 1..=two_k {
                    factorial_2k *= T::from(i).expect("Failed to convert mathematical constant");
                }

                let val_bk = T::from_f64(b_num).expect("Failed to convert Bernoulli numerator")
                    / T::from_f64(b_den).expect("Failed to convert Bernoulli denominator");
                let term = val_bk * fact_ratio / (factorial_2k * xpow);

                if term.abs() > prev_term_abs {
                    break;
                }
                prev_term_abs = term.abs();
                sum += term;

                xpow *= xv * xv;
                let next_factor1 =
                    T::from(n + two_k).expect("Failed to convert mathematical constant");
                let next_factor2 =
                    T::from(n + two_k + 1).expect("Failed to convert mathematical constant");
                fact_ratio *= next_factor1 * next_factor2;
            }

            r + asym_sign * sum
        }
    }
}
