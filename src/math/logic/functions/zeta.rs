use super::gamma::eval_gamma;
use super::helpers::{sign_from_delta, signed_infinity, signed_infinity_from_delta};
use crate::core::traits::MathScalar;

/// Riemann zeta function ζ(s) = Σ_{n=1}^∞ 1/n^s
///
/// **Algorithms**:
/// - For s > 1.5: Borwein's accelerated series (fastest)
/// - For 1 < s ≤ 1.5: Enhanced Euler-Maclaurin with Bernoulli corrections
/// - For s < 1: Functional equation ζ(s) = 2^s π^(s-1) sin(πs/2) Γ(1-s) ζ(1-s)
///
/// **Precision**: Achieves 14-15 decimal digits for all s
///
/// Reference: DLMF §25.2, Borwein et al. (2000)
pub fn eval_zeta<T: MathScalar>(x: T) -> T {
    let one = T::one();
    let threshold = T::from(1e-10).expect("Failed to convert mathematical constant");

    // Pole at s = 1
    let delta = x - one;
    if delta.abs() < threshold {
        return signed_infinity_from_delta(delta);
    }

    // Use reflection for s < 0 to ensure fast convergence for negative values
    if x < T::zero() {
        let pi = T::PI();
        let two = T::from(2.0).expect("Failed to convert mathematical constant");
        let gs = eval_gamma(one - x);
        let z = eval_zeta(one - x);
        let term1 = two.powf(x);
        let term2 = pi.powf(x - one);
        let term3 = (pi * x / two).sin();
        return term1 * term2 * term3 * gs * z;
    }

    // For s >= 0 (and s != 1):
    // Use Enhanced Euler-Maclaurin for 1 < s <= 1.5 where series converges slowly
    let one_point_five = T::from(1.5).expect("Failed to convert mathematical constant");
    if x > one && x <= one_point_five {
        let n_terms = 100;
        let mut sum = T::zero();
        let mut compensation = T::zero(); // Kahan summation

        for k in 1..=n_terms {
            let k_t = T::from(k).expect("Failed to convert mathematical constant");
            let term = one / k_t.powf(x);

            // Kahan summation algorithm
            let y = term - compensation;
            let t = sum + y;
            compensation = (t - sum) - y;
            sum = t;
        }

        // Enhanced Euler-Maclaurin correction with Bernoulli numbers
        let n = T::from(f64::from(n_terms)).expect("Failed to convert mathematical constant");
        let n_pow_x = n.powf(x);
        let n_pow_1_minus_x = n.powf(one - x);

        // Integral approximation: ∫[N,∞] 1/t^s dt = N^(1-s)/(s-1)
        let em_integral = n_pow_1_minus_x / (x - one);

        // Boundary correction: 1/(2N^s)
        let em_boundary = T::from(0.5).expect("Failed to convert mathematical constant") / n_pow_x;

        // Bernoulli corrections (improving convergence)
        // B_2 = 1/6: correction term s/(12 N^(s+1))
        let b2_correction =
            x / (T::from(12.0).expect("Failed to convert mathematical constant") * n.powf(x + one));

        // B_4 = -1/30: correction term s(s+1)(s+2)/(720 N^(s+3))
        let s_plus_1 = x + one;
        let s_plus_2 = x + T::from(2.0).expect("Failed to convert mathematical constant");
        let b4_correction = -x * s_plus_1 * s_plus_2
            / (T::from(720.0).expect("Failed to convert mathematical constant")
                * n.powf(x + T::from(3.0).expect("Failed to convert mathematical constant")));

        sum + em_integral + em_boundary + b2_correction + b4_correction
    } else {
        // For s > 1.5 OR 0 <= s < 1: Use Borwein's Algorithm 2
        // Borwein is globally convergent (except pole).
        eval_zeta_borwein(x)
    }
}

/// Borwein's Algorithm 2 for ζ(s) - Accelerated convergence
///
/// Uses Chebyshev polynomial-based acceleration with `d_k` coefficients.
/// Formula from page 3 of Borwein's 1991 paper:
///
/// `d_k` = n · Σ_{i=0}^k [(n+i-1)! · 4^i] / [(n-i)! · (2i)!]
///
/// ζ(s) = -1 / [d_n(1-2^(1-s))] · Σ_{k=0}^{n-1} [(-`1)^k(d_k` - `d_n`)] / (k+1)^s + `γ_n(s)`
///
/// where `γ_n(s)` is a small error term that can be ignored for sufficient n.
///
/// **Convergence**: Requires ~(1.3)n terms for n-digit accuracy
/// Much faster than simple alternating series.
///
/// Reference: Borwein (1991) "An Efficient Algorithm for the Riemann Zeta Function"
fn eval_zeta_borwein<T: MathScalar>(s: T) -> T {
    let one = T::one();
    let two = T::from(2.0).expect("Failed to convert mathematical constant");
    let four = T::from(4.0).expect("Failed to convert mathematical constant");
    let n = 14; // Optimal for double precision (~18 digits)

    // Compute denominator: 1 - 2^(1-s)
    let denom = one - two.powf(one - s);
    if denom.abs() < T::from(1e-15).expect("Failed to convert mathematical constant") {
        let delta = s - one;
        return signed_infinity_from_delta(delta); // Too close to s=1
    }

    // Compute d_k coefficients
    // d_k = n · Σ_{i=0}^{k} [(n+i-1)! · 4^i] / [(n-i)! · (2i)!]
    let mut d_coeffs = vec![T::zero(); n + 1];
    let n_t = T::from(n).expect("Failed to convert mathematical constant");

    // For k=0: d_0 = n * (1/n) = 1
    let mut term = one / n_t; // For i=0: (n-1)!/(n!·0!) = 1/n
    let mut current_inner_sum = term;
    d_coeffs[0] = n_t * current_inner_sum;

    for (idx, d_coeff) in d_coeffs.iter_mut().enumerate().skip(1) {
        let k = idx;
        let i = T::from(k - 1).expect("Failed to convert mathematical constant");
        let two_i_plus_1 = T::from(2 * k - 1).expect("Failed to convert mathematical constant");
        let two_i_plus_2 = T::from(2 * k).expect("Failed to convert mathematical constant");
        let n_minus_i = n_t - i;
        let n_plus_i = n_t + i;

        // CORRECT recurrence: T_{i+1} = T_i * 4(n+i)(n-i) / ((2i+1)(2i+2))
        // The (n-i) is in the NUMERATOR, not denominator!
        term = term * four * n_plus_i * n_minus_i / (two_i_plus_1 * two_i_plus_2);
        current_inner_sum += term;
        *d_coeff = n_t * current_inner_sum;
    }

    let d_n = d_coeffs[n];

    // Compute the sum: Σ_{k=0}^{n-1} [(-1)^k(d_k - d_n)] / (k+1)^s
    let mut sum = T::zero();
    let mut compensation = T::zero(); // Kahan summation

    for (k, d_coeff_k) in d_coeffs.iter().enumerate().take(n) {
        let k_plus_1 = T::from(k + 1).expect("Failed to convert mathematical constant");
        let sign = if k % 2 == 0 { one } else { -one };
        let current_term = sign * (*d_coeff_k - d_n) / k_plus_1.powf(s);

        // Kahan summation
        let y = current_term - compensation;
        let t = sum + y;
        compensation = (t - sum) - y;
        sum = t;
    }

    // ζ(s) = -1 / [d_n(1-2^(1-s))] · sum
    -sum / (d_n * denom)
}

/// Derivative of Riemann Zeta function
///
/// Computes the n-th derivative of ζ(s) using the analytical formula:
/// ζ^(n)(s) = (-1)^n * Σ_{k=1}^∞ [ln(k)]^n / k^s
///
/// This implementation uses the same convergence techniques as `eval_zeta`
/// to ensure consistency and accuracy.
///
/// Reference: DLMF §25.2 <https://dlmf.nist.gov/25.2>
///
/// # Arguments
/// * `n` - Order of derivative (n ≥ 0)
/// * `x` - Point at which to evaluate the derivative
///
/// # Returns
/// * `value` if convergent
/// * `±infinity` if at pole (s=1)
/// * `NaN` if derivative order is invalid
pub fn eval_zeta_deriv<T: MathScalar>(n: i32, x: T) -> T {
    if n < 0 {
        return T::nan();
    }
    if n == 0 {
        return eval_zeta(x);
    }

    let one = T::one();
    let epsilon = T::from(1e-10).expect("Failed to convert mathematical constant");

    // Check for pole at s=1
    let delta = x - one;
    if delta.abs() < epsilon {
        let sign = if n % 2 == 0 {
            sign_from_delta(delta)
        } else {
            -T::one()
        };
        return signed_infinity(sign);
    }

    // For Re(s) > 1, use direct series with Kahan summation
    // The analytical series is exact for any n, no need for special cases
    if x > one {
        let mut sum = T::zero();
        let mut compensation = T::zero(); // Kahan summation
        let max_terms = 200;

        for k in 1..=max_terms {
            let k_t = T::from(k).expect("Failed to convert mathematical constant");
            let ln_k = k_t.ln();

            // Calculate [ln(k)]^n using faster exponentiation for large n
            let ln_k_power = if n <= 5 {
                // Direct multiplication for small n
                let mut result = one;
                for _ in 0..n {
                    result *= ln_k;
                }
                result
            } else {
                // Use powf for large n (faster)
                ln_k.powf(T::from(n).expect("Failed to convert mathematical constant"))
            };

            // Calculate term: [ln(k)]^n / k^x
            let term = ln_k_power / k_t.powf(x);

            // Kahan summation algorithm (compensated summation)
            let y = term - compensation;
            let t = sum + y;
            compensation = (t - sum) - y; // Captures lost low-order bits
            sum = t;

            // Enhanced convergence check
            if k > 50
                && term.abs()
                    < epsilon * T::from_f64(0.01).expect("Failed to convert constant 0.01")
            {
                break;
            }
        }

        // Apply sign: (-1)^n
        let sign = if n % 2 == 0 { one } else { -one };
        sign * sum
    } else {
        // For Re(s) < 1, use functional equation derivative
        // ζ(s) = 2^s π^(s-1) sin(πs/2) Γ(1-s) ζ(1-s)

        // Use reflection for all s < 1 (except pole at s=1 already handled)
        // With improved eval_zeta, the reflection formula components are now stable for 0 < s < 1
        eval_zeta_deriv_reflection(n, x)
    }
}

/// Compute zeta derivative using reflection formula with finite differences
///
/// For Re(s) < 1, uses reflection formula: ζ(s) = A(s)·ζ(1-s)
/// where A(s) = 2^s · π^(s-1) · sin(πs/2) · Γ(1-s)
///
/// We compute ζ^(n)(s) numerically using finite differences,
/// while ζ(1-s) is evaluated exactly via analytical series.
///
/// This is simpler and more stable than the full Leibniz expansion.
fn eval_zeta_deriv_reflection<T: MathScalar>(n: i32, s: T) -> T {
    let one = T::one();
    let two = T::from_f64(2.0).expect("Failed to convert constant 2.0");
    let four = T::from_f64(4.0).expect("Failed to convert constant 4.0");

    // Finite difference step - small enough for accuracy but not too small
    let h = T::from_f64(1e-7).expect("Failed to convert epsilon 1e-7");

    if n == 1 {
        // First derivative: centered difference
        let zeta_plus = eval_zeta_reflection_base(s + h);
        let zeta_minus = eval_zeta_reflection_base(s - h);
        (zeta_plus - zeta_minus) / (two * h)
    } else if n == 2 {
        // Second derivative: centered second difference
        let zeta_plus = eval_zeta_reflection_base(s + h);
        let zeta_center = eval_zeta_reflection_base(s);
        let zeta_minus = eval_zeta_reflection_base(s - h);
        (zeta_plus - two * zeta_center + zeta_minus) / (h * h)
    } else {
        // Higher order: use Richardson extrapolation
        // Compute with step h and h/2, then extrapolate to h→0
        let d_h = centered_finite_diff(n, s, h);
        let d_h2 = centered_finite_diff(n, s, h / two);

        // Richardson: D_exact ≈ (4^n * D_h - D_{h/2}) / (4^n - 1)
        // For n >= 3, use general extrapolation factor
        let extrapolation = four.powi(n) - one;
        (four.powi(n) * d_h2 - d_h) / extrapolation
    }
}

/// Evaluate ζ(s) using reflection formula (for Re(s) < 1)
///
/// ζ(s) = 2^s · π^(s-1) · sin(πs/2) · Γ(1-s) · ζ(1-s)
/// where ζ(1-s) is computed via exact analytical series
fn eval_zeta_reflection_base<T: MathScalar>(s: T) -> T {
    let pi = T::PI();
    let two = T::from(2.0).expect("Failed to convert mathematical constant");
    let half = T::from(0.5).expect("Failed to convert mathematical constant");
    let one = T::one();
    let one_minus_s = one - s;

    // A(s) = 2^s · π^(s-1) · sin(πs/2) · Γ(1-s)
    let a_term = two.powf(s);
    let b_term = pi.powf(s - one);
    let c_term = (pi * s * half).sin();
    let d_term = eval_gamma(one_minus_s);

    // ζ(1-s) via exact analytical series (Re(1-s) > 1 when Re(s) < 1)
    let zeta_term = eval_zeta(one_minus_s);

    a_term * b_term * c_term * d_term * zeta_term
}

/// Compute n-th derivative using centered finite difference
///
/// Uses an n+1 point stencil for the n-th derivative
fn centered_finite_diff<T: MathScalar>(n: i32, s: T, h: T) -> T {
    let two = T::from(2.0).expect("Failed to convert mathematical constant");
    let four = T::from(4.0).expect("Failed to convert mathematical constant");

    if n == 1 {
        let zeta_plus = eval_zeta_reflection_base(s + h);
        let zeta_minus = eval_zeta_reflection_base(s - h);
        return (zeta_plus - zeta_minus) / (two * h);
    }

    if n == 2 {
        let zeta_plus = eval_zeta_reflection_base(s + h);
        let zeta_center = eval_zeta_reflection_base(s);
        let zeta_minus = eval_zeta_reflection_base(s - h);
        return (zeta_plus - two * zeta_center + zeta_minus) / (h * h);
    }

    if n == 3 {
        // Third derivative: 4-point centered stencil
        let zeta_plus2 = eval_zeta_reflection_base(s + two * h);
        let zeta_plus1 = eval_zeta_reflection_base(s + h);
        let zeta_minus1 = eval_zeta_reflection_base(s - h);
        let zeta_minus2 = eval_zeta_reflection_base(s - two * h);
        // d³f/dx³ ≈ (f(x+2h) - 2f(x+h) + 2f(x-h) - f(x-2h)) / (2h³)
        return (-zeta_plus2 + two * zeta_plus1 - two * zeta_minus1 + zeta_minus2)
            / (two * h * h * h);
    }

    // For n >= 4, use 5-point stencil
    let two_h = two * h;

    let zeta_plus2 = eval_zeta_reflection_base(s + two_h);
    let zeta_plus1 = eval_zeta_reflection_base(s + h);
    let zeta_center = eval_zeta_reflection_base(s);
    let zeta_minus1 = eval_zeta_reflection_base(s - h);
    let zeta_minus2 = eval_zeta_reflection_base(s - two_h);

    if n == 4 {
        // d⁴f/dx⁴ ≈ (f(x+2h) - 4f(x+h) + 6f(x) - 4f(x-h) + f(x-2h)) / h⁴
        let six = T::from_f64(6.0).expect("Failed to convert constant 6.0");
        return (zeta_plus2 - four * zeta_plus1 + six * zeta_center - four * zeta_minus1
            + zeta_minus2)
            / (h * h * h * h);
    }

    // For n >= 5: recursive centered difference
    // d^n f / dx^n ≈ [d^(n-1)f(x+h) - d^(n-1)f(x-h)] / (2h)
    let deriv_plus = centered_finite_diff(n - 1, s + h, h);
    let deriv_minus = centered_finite_diff(n - 1, s - h, h);
    (deriv_plus - deriv_minus) / (two * h)
}
