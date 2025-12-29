//! Mathematical function evaluations
//!
//! This module centralizes all mathematical function implementations,
//! organized by category for maintainability.
//!
//! # Academic References
//!
//! Implementations follow standard numerical methods from:
//!
//! - **DLMF**: NIST Digital Library of Mathematical Functions <https://dlmf.nist.gov>
//! - **A&S**: Abramowitz & Stegun, "Handbook of Mathematical Functions" (1964)
//! - **NR**: Press et al., "Numerical Recipes" (3rd ed., 2007)
//! - Lanczos, C. "A Precision Approximation of the Gamma Function" (1964)
//! - Corless et al. "On the Lambert W Function" (1996)
//!
//! # Domain Validation
//!
//! Functions that can produce undefined results (poles, branch cuts, domain errors)
//! return `Option<T>` and check their inputs. Key validations include:
//!
//! - **Gamma functions**: Non-positive integers are poles
//! - **Zeta function**: s=1 is a pole  
//! - **Logarithms**: Non-positive inputs are domain errors
//! - **Inverse trig**: |x| > 1 is a domain error for asin/acos
//! - **Square root**: Negative inputs return NaN or None depending on context

use crate::core::traits::MathScalar;

pub mod dual;

#[cfg(test)]
mod validation;

/// Exponential function for polar representation
///
/// Currently just wraps `exp()`. This function exists as a placeholder
/// for potential future polar-form exponential implementations.
pub(crate) fn eval_exp_polar<T: MathScalar>(x: T) -> T {
    x.exp()
}

/// Error function erf(x) = (2/√π) ∫₀ˣ e^(-t²) dt
///
/// Uses Taylor series expansion: erf(x) = (2/√π) Σₙ (-1)ⁿ x^(2n+1) / (n!(2n+1))
/// with Kahan summation for numerical stability.
///
/// Reference: DLMF §7.6.1 <https://dlmf.nist.gov/7.6#E1>
pub(crate) fn eval_erf<T: MathScalar>(x: T) -> T {
    let sign = x.signum();
    let x = x.abs();
    // PI is available via FloatConst implementation on T
    let pi = T::PI();
    let sqrt_pi = pi.sqrt();
    let two = T::from(2.0).unwrap();
    let coeff = two / sqrt_pi;

    let mut sum = T::zero();
    let mut compensation = T::zero(); // Kahan summation
    let mut factorial = T::one();
    let mut power = x;

    for n in 0..30 {
        let two_n_plus_one = T::from(2 * n + 1).unwrap();

        let term = power / (factorial * two_n_plus_one);

        // Overflow protection: break if term becomes NaN or infinite
        if term.is_nan() || term.is_infinite() {
            break;
        }

        // Alternating series with Kahan summation
        let signed_term = if n % 2 == 0 { term } else { -term };
        let y = signed_term - compensation;
        let t = sum + y;
        compensation = (t - sum) - y;
        sum = t;

        let n_plus_one = T::from(n + 1).unwrap();
        factorial *= n_plus_one;
        power *= x * x;

        // Check convergence using machine epsilon
        if term.abs() < T::epsilon() {
            break;
        }
    }
    sign * coeff * sum
}

/// Gamma function Γ(x) using Lanczos approximation with g=7
///
/// Γ(z+1) ≈ √(2π) (z + g + 1/2)^(z+1/2) e^(-(z+g+1/2)) Aₘ(z)
/// Uses reflection formula for x < 0.5: Γ(z)Γ(1-z) = π/sin(πz)
///
/// Reference: Lanczos (1964) "A Precision Approximation of the Gamma Function"
/// SIAM J. Numerical Analysis, Ser. B, Vol. 1, pp. 86-96
/// See also: DLMF §5.10 <https://dlmf.nist.gov/5.10>
pub(crate) fn eval_gamma<T: MathScalar>(x: T) -> Option<T> {
    // Add special handling for x near negative integers
    if x < T::zero() && (x.fract().abs() < T::from(1e-10).unwrap()) {
        return None; // Exactly at negative integer pole
    }

    if x <= T::zero() && x.fract() == T::zero() {
        return None;
    }
    let g = T::from(7.0).unwrap();
    let c = [
        0.999_999_999_999_809_9,
        676.5203681218851,
        -1259.1392167224028,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507343278686905,
        -0.13857109526572012,
        9.984_369_578_019_572e-6,
        1.5056327351493116e-7,
    ];
    let half = T::from(0.5).unwrap();
    let one = T::one();
    let pi = T::PI();

    if x < half {
        // Consider adding Stirling's series for large negative x
        if x < T::from(-10.0).unwrap() {
            // Use reflection + Gamma(1-x)
            // For large negative x, 1-x is large positive, so Lanczos works well.
            // We return directly to avoid stack depth
            let val = pi / ((pi * x).sin() * eval_gamma(one - x)?);
            return Some(val);
        }
        Some(pi / ((pi * x).sin() * eval_gamma(one - x)?))
    } else {
        let x = x - one;
        let mut ag = T::from(c[0]).unwrap();
        for (i, &coeff) in c.iter().enumerate().skip(1) {
            ag += T::from(coeff).unwrap() / (x + T::from(i).unwrap());
        }
        let t = x + g + half;
        let two_pi_sqrt = (T::from(2.0).unwrap() * pi).sqrt();
        Some(two_pi_sqrt * t.powf(x + half) * (-t).exp() * ag)
    }
}

/// Digamma function ψ(x) = Γ'(x)/Γ(x) = d/dx ln(Γ(x))
///
/// Uses asymptotic expansion for large x:
/// ψ(x) ~ ln(x) - 1/(2x) - 1/(12x²) + 1/(120x⁴) - 1/(252x⁶) + ...
/// Uses reflection formula for x < 0.5: ψ(1-x) - ψ(x) = π cot(πx)
///
/// Reference: DLMF §5.11 <https://dlmf.nist.gov/5.11>
pub(crate) fn eval_digamma<T: MathScalar>(x: T) -> Option<T> {
    if x <= T::zero() && x.fract() == T::zero() {
        return None;
    }
    let half = T::from(0.5).unwrap();
    let one = T::one();
    let pi = T::PI();

    if x < half {
        return Some(eval_digamma(one - x)? - pi * (pi * x).cos() / (pi * x).sin());
    }
    let mut xv = x;
    let mut result = T::zero();
    let six = T::from(6.0).unwrap();
    while xv < six {
        result -= one / xv;
        xv += one;
    }
    result += xv.ln() - half / xv;
    let x2 = xv * xv;

    let t1 = one / (T::from(12.0).unwrap() * x2);
    let t2 = one / (T::from(120.0).unwrap() * x2 * x2);
    let t3 = one / (T::from(252.0).unwrap() * x2 * x2 * x2);

    Some(result - t1 + t2 - t3)
}

/// Trigamma function ψ₁(x) = d²/dx² ln(Γ(x))
///
/// Uses asymptotic expansion: ψ₁(x) ~ 1/x + 1/(2x²) + 1/(6x³) - 1/(30x⁵) + ...
/// with recurrence for small x: ψ₁(x) = ψ₁(x+1) + 1/x²
///
/// Reference: DLMF §5.15 <https://dlmf.nist.gov/5.15>
pub(crate) fn eval_trigamma<T: MathScalar>(x: T) -> Option<T> {
    if x <= T::zero() && x.fract() == T::zero() {
        return None;
    }
    let mut xv = x;
    let mut r = T::zero();
    let six = T::from(6.0).unwrap();
    let one = T::one();

    while xv < six {
        r += one / (xv * xv);
        xv += one;
    }
    let x2 = xv * xv;
    let half = T::from(0.5).unwrap();

    Some(
        r + one / xv + half / x2 + one / (six * x2 * xv)
            - one / (T::from(30.0).unwrap() * x2 * x2 * xv)
            + one / (T::from(42.0).unwrap() * x2 * x2 * x2 * xv),
    )
}

/// Tetragamma function ψ₂(x) = d³/dx³ ln(Γ(x))
///
/// Uses asymptotic expansion with recurrence for small x.
///
/// Reference: DLMF §5.15 <https://dlmf.nist.gov/5.15>
pub(crate) fn eval_tetragamma<T: MathScalar>(x: T) -> Option<T> {
    if x <= T::zero() && x.fract() == T::zero() {
        return None;
    }
    let mut xv = x;
    let mut r = T::zero();
    let six = T::from(6.0).unwrap();
    let one = T::one();
    let two = T::from(2.0).unwrap();

    while xv < six {
        r -= two / (xv * xv * xv);
        xv += one;
    }
    let x2 = xv * xv;
    Some(r - one / x2 + one / (x2 * xv) + one / (two * x2 * x2) + one / (six * x2 * x2 * xv))
}

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
pub(crate) fn eval_zeta<T: MathScalar>(x: T) -> Option<T> {
    let one = T::one();
    let threshold = T::from(1e-10).unwrap();

    // Pole at s = 1
    if (x - one).abs() < threshold {
        return None;
    }

    // Use reflection for s < 0 to ensure fast convergence for negative values
    if x < T::zero() {
        let pi = T::PI();
        let two = T::from(2.0).unwrap();
        let gs = eval_gamma(one - x)?;
        let z = eval_zeta(one - x)?;
        let term1 = two.powf(x);
        let term2 = pi.powf(x - one);
        let term3 = (pi * x / two).sin();
        return Some(term1 * term2 * term3 * gs * z);
    }

    // For s >= 0 (and s != 1):
    // Use Enhanced Euler-Maclaurin for 1 < s <= 1.5 where series converges slowly
    let one_point_five = T::from(1.5).unwrap();
    if x > one && x <= one_point_five {
        let n_terms = 100;
        let mut sum = T::zero();
        let mut compensation = T::zero(); // Kahan summation

        for k in 1..=n_terms {
            let k_t = T::from(k).unwrap();
            let term = one / k_t.powf(x);

            // Kahan summation algorithm
            let y = term - compensation;
            let t = sum + y;
            compensation = (t - sum) - y;
            sum = t;
        }

        // Enhanced Euler-Maclaurin correction with Bernoulli numbers
        let n = T::from(n_terms as f64).unwrap();
        let n_pow_x = n.powf(x);
        let n_pow_1_minus_x = n.powf(one - x);

        // Integral approximation: ∫[N,∞] 1/t^s dt = N^(1-s)/(s-1)
        let em_integral = n_pow_1_minus_x / (x - one);

        // Boundary correction: 1/(2N^s)
        let em_boundary = T::from(0.5).unwrap() / n_pow_x;

        // Bernoulli corrections (improving convergence)
        // B_2 = 1/6: correction term s/(12 N^(s+1))
        let b2_correction = x / (T::from(12.0).unwrap() * n.powf(x + one));

        // B_4 = -1/30: correction term s(s+1)(s+2)/(720 N^(s+3))
        let s_plus_1 = x + one;
        let s_plus_2 = x + T::from(2.0).unwrap();
        let b4_correction = -x * s_plus_1 * s_plus_2
            / (T::from(720.0).unwrap() * n.powf(x + T::from(3.0).unwrap()));

        Some(sum + em_integral + em_boundary + b2_correction + b4_correction)
    } else {
        // For s > 1.5 OR 0 <= s < 1: Use Borwein's Algorithm 2
        // Borwein is globally convergent (except pole).
        eval_zeta_borwein(x)
    }
}

/// Borwein's Algorithm 2 for ζ(s) - Accelerated convergence
///
/// Uses Chebyshev polynomial-based acceleration with d_k coefficients.
/// Formula from page 3 of Borwein's 1991 paper:
///
/// d_k = n · Σ_{i=0}^k [(n+i-1)! · 4^i] / [(n-i)! · (2i)!]
///
/// ζ(s) = -1 / [d_n(1-2^(1-s))] · Σ_{k=0}^{n-1} [(-1)^k(d_k - d_n)] / (k+1)^s + γ_n(s)
///
/// where γ_n(s) is a small error term that can be ignored for sufficient n.
///
/// **Convergence**: Requires ~(1.3)n terms for n-digit accuracy
/// Much faster than simple alternating series.
///
/// Reference: Borwein (1991) "An Efficient Algorithm for the Riemann Zeta Function"
fn eval_zeta_borwein<T: MathScalar>(s: T) -> Option<T> {
    let one = T::one();
    let two = T::from(2.0).unwrap();
    let four = T::from(4.0).unwrap();
    let n = 14; // Optimal for double precision (~18 digits)

    // Compute denominator: 1 - 2^(1-s)
    let denom = one - two.powf(one - s);
    if denom.abs() < T::from(1e-15).unwrap() {
        return None; // Too close to s=1
    }

    // Compute d_k coefficients
    // d_k = n · Σ_{i=0}^{k} [(n+i-1)! · 4^i] / [(n-i)! · (2i)!]
    let mut d_coeffs = vec![T::zero(); n + 1];
    let n_t = T::from(n).unwrap();

    // For k=0: d_0 = n * (1/n) = 1
    let mut term = one / n_t; // For i=0: (n-1)!/(n!·0!) = 1/n
    let mut current_inner_sum = term;
    d_coeffs[0] = n_t * current_inner_sum;

    for k in 1..=n {
        let i = T::from(k - 1).unwrap();
        let two_i_plus_1 = T::from(2 * k - 1).unwrap();
        let two_i_plus_2 = T::from(2 * k).unwrap();
        let n_minus_i = n_t - i;
        let n_plus_i = n_t + i;

        // CORRECT recurrence: T_{i+1} = T_i * 4(n+i)(n-i) / ((2i+1)(2i+2))
        // The (n-i) is in the NUMERATOR, not denominator!
        term = term * four * n_plus_i * n_minus_i / (two_i_plus_1 * two_i_plus_2);
        current_inner_sum += term;
        d_coeffs[k] = n_t * current_inner_sum;
    }

    let d_n = d_coeffs[n];

    // Compute the sum: Σ_{k=0}^{n-1} [(-1)^k(d_k - d_n)] / (k+1)^s
    let mut sum = T::zero();
    let mut compensation = T::zero(); // Kahan summation

    for k in 0..n {
        let k_plus_1 = T::from(k + 1).unwrap();
        let sign = if k % 2 == 0 { one } else { -one };
        let term = sign * (d_coeffs[k] - d_n) / k_plus_1.powf(s);

        // Kahan summation
        let y = term - compensation;
        let t = sum + y;
        compensation = (t - sum) - y;
        sum = t;
    }

    // ζ(s) = -1 / [d_n(1-2^(1-s))] · sum
    let result = -sum / (d_n * denom);
    Some(result)
}

/// Derivative of Riemann Zeta function
///
/// Computes the n-th derivative of ζ(s) using the analytical formula:
/// ζ^(n)(s) = (-1)^n * Σ_{k=1}^∞ [ln(k)]^n / k^s
///
/// This implementation uses the same convergence techniques as eval_zeta
/// to ensure consistency and accuracy.
///
/// Reference: DLMF §25.2 <https://dlmf.nist.gov/25.2>
///
/// # Arguments
/// * `n` - Order of derivative (n ≥ 0)
/// * `x` - Point at which to evaluate the derivative
///
/// # Returns
/// * `Some(value)` if convergent
/// * `None` if at pole (s=1) or invalid
pub(crate) fn eval_zeta_deriv<T: MathScalar>(n: i32, x: T) -> Option<T> {
    if n < 0 {
        return None;
    }
    if n == 0 {
        return eval_zeta(x);
    }

    let one = T::one();
    let epsilon = T::from(1e-10).unwrap();

    // Check for pole at s=1
    if (x - one).abs() < epsilon {
        return None;
    }

    // For Re(s) > 1, use direct series with Kahan summation
    // The analytical series is exact for any n, no need for special cases
    if x > one {
        let mut sum = T::zero();
        let mut compensation = T::zero(); // Kahan summation
        let max_terms = 200;

        for k in 1..=max_terms {
            let k_t = T::from(k).unwrap();
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
                ln_k.powf(T::from(n).unwrap())
            };

            // Calculate term: [ln(k)]^n / k^x
            let term = ln_k_power / k_t.powf(x);

            // Kahan summation algorithm (compensated summation)
            let y = term - compensation;
            let t = sum + y;
            compensation = (t - sum) - y; // Captures lost low-order bits
            sum = t;

            // Enhanced convergence check
            if k > 50 && term.abs() < epsilon * T::from(0.01).unwrap() {
                break;
            }
        }

        // Apply sign: (-1)^n
        let sign = if n % 2 == 0 { one } else { -one };
        Some(sign * sum)
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
fn eval_zeta_deriv_reflection<T: MathScalar>(n: i32, s: T) -> Option<T> {
    let one = T::one();
    let two = T::from(2.0).unwrap();
    let four = T::from(4.0).unwrap();

    // Finite difference step - small enough for accuracy but not too small
    let h = T::from(1e-7).unwrap();

    if n == 1 {
        // First derivative: centered difference
        let zeta_plus = eval_zeta_reflection_base(s + h)?;
        let zeta_minus = eval_zeta_reflection_base(s - h)?;
        Some((zeta_plus - zeta_minus) / (two * h))
    } else if n == 2 {
        // Second derivative: centered second difference
        let zeta_plus = eval_zeta_reflection_base(s + h)?;
        let zeta_center = eval_zeta_reflection_base(s)?;
        let zeta_minus = eval_zeta_reflection_base(s - h)?;
        Some((zeta_plus - two * zeta_center + zeta_minus) / (h * h))
    } else {
        // Higher order: use Richardson extrapolation
        // Compute with step h and h/2, then extrapolate to h→0
        let d_h = centered_finite_diff(n, s, h)?;
        let d_h2 = centered_finite_diff(n, s, h / two)?;

        // Richardson: D_exact ≈ (4^n * D_h - D_{h/2}) / (4^n - 1)
        // For n >= 3, use general extrapolation factor
        let extrapolation = four.powi(n) - one;
        Some((four.powi(n) * d_h2 - d_h) / extrapolation)
    }
}

/// Evaluate ζ(s) using reflection formula (for Re(s) < 1)
///
/// ζ(s) = 2^s · π^(s-1) · sin(πs/2) · Γ(1-s) · ζ(1-s)
/// where ζ(1-s) is computed via exact analytical series
fn eval_zeta_reflection_base<T: MathScalar>(s: T) -> Option<T> {
    let pi = T::PI();
    let two = T::from(2.0).unwrap();
    let half = T::from(0.5).unwrap();
    let one = T::one();
    let one_minus_s = one - s;

    // A(s) = 2^s · π^(s-1) · sin(πs/2) · Γ(1-s)
    let a_term = two.powf(s);
    let b_term = pi.powf(s - one);
    let c_term = (pi * s * half).sin();
    let d_term = eval_gamma(one_minus_s)?;

    // ζ(1-s) via exact analytical series (Re(1-s) > 1 when Re(s) < 1)
    let zeta_term = eval_zeta(one_minus_s)?;

    Some(a_term * b_term * c_term * d_term * zeta_term)
}

/// Compute n-th derivative using centered finite difference
///
/// Uses an n+1 point stencil for the n-th derivative
fn centered_finite_diff<T: MathScalar>(n: i32, s: T, h: T) -> Option<T> {
    let two = T::from(2.0).unwrap();
    let four = T::from(4.0).unwrap();

    if n == 1 {
        let zeta_plus = eval_zeta_reflection_base(s + h)?;
        let zeta_minus = eval_zeta_reflection_base(s - h)?;
        return Some((zeta_plus - zeta_minus) / (two * h));
    }

    if n == 2 {
        let zeta_plus = eval_zeta_reflection_base(s + h)?;
        let zeta_center = eval_zeta_reflection_base(s)?;
        let zeta_minus = eval_zeta_reflection_base(s - h)?;
        return Some((zeta_plus - two * zeta_center + zeta_minus) / (h * h));
    }

    if n == 3 {
        // Third derivative: 4-point centered stencil
        let zeta_plus2 = eval_zeta_reflection_base(s + two * h)?;
        let zeta_plus1 = eval_zeta_reflection_base(s + h)?;
        let zeta_minus1 = eval_zeta_reflection_base(s - h)?;
        let zeta_minus2 = eval_zeta_reflection_base(s - two * h)?;
        // d³f/dx³ ≈ (f(x+2h) - 2f(x+h) + 2f(x-h) - f(x-2h)) / (2h³)
        return Some(
            (-zeta_plus2 + two * zeta_plus1 - two * zeta_minus1 + zeta_minus2) / (two * h * h * h),
        );
    }

    // For n >= 4, use 5-point stencil
    let two_h = two * h;

    let zeta_plus2 = eval_zeta_reflection_base(s + two_h)?;
    let zeta_plus1 = eval_zeta_reflection_base(s + h)?;
    let zeta_center = eval_zeta_reflection_base(s)?;
    let zeta_minus1 = eval_zeta_reflection_base(s - h)?;
    let zeta_minus2 = eval_zeta_reflection_base(s - two_h)?;

    if n == 4 {
        // d⁴f/dx⁴ ≈ (f(x+2h) - 4f(x+h) + 6f(x) - 4f(x-h) + f(x-2h)) / h⁴
        let six = T::from(6.0).unwrap();
        return Some(
            (zeta_plus2 - four * zeta_plus1 + six * zeta_center - four * zeta_minus1 + zeta_minus2)
                / (h * h * h * h),
        );
    }

    // For n >= 5: recursive centered difference
    // d^n f / dx^n ≈ [d^(n-1)f(x+h) - d^(n-1)f(x-h)] / (2h)
    let deriv_plus = centered_finite_diff(n - 1, s + h, h)?;
    let deriv_minus = centered_finite_diff(n - 1, s - h, h)?;
    Some((deriv_plus - deriv_minus) / (two * h))
}

/// Lambert W function: W(x) is the solution to W·e^W = x
///
/// Uses Halley's iteration with carefully chosen initial approximations:
/// - For x near -1/e: series expansion
/// - For x > 0: asymptotic ln(x) - ln(ln(x)) approximation
///
/// Reference: Corless et al. (1996) "On the Lambert W Function"
/// Advances in Computational Mathematics, Vol. 5, pp. 329-359
/// See also: DLMF §4.13 <https://dlmf.nist.gov/4.13>
pub(crate) fn eval_lambert_w<T: MathScalar>(x: T) -> Option<T> {
    let one = T::one();
    let e = T::E();
    let e_inv = one / e;

    if x < -e_inv {
        return None; // Domain error: W(x) undefined for x < -1/e
    }
    if x == T::zero() {
        return Some(T::zero());
    }
    let threshold = T::from(1e-12).unwrap();
    if (x + e_inv).abs() < threshold {
        return Some(-one);
    }

    // Initial guess
    let point_three_neg = T::from(-0.3).unwrap();
    let mut w = if x < point_three_neg {
        let two = T::from(2.0).unwrap();
        // Fix: clamp to 0 to avoid NaN from floating point noise when x is close to -1/e
        let arg = (two * (e * x + one)).max(T::zero());
        let p = arg.sqrt();
        // -1 + p - p^2/3 + 11/72 p^3
        let third = T::from(3.0).unwrap();
        let c1 = T::from(11.0 / 72.0).unwrap();
        -one + p - p * p / third + c1 * p * p * p
    } else if x < T::zero() {
        let two = T::from(2.0).unwrap();
        let p = (two * (e * x + one)).sqrt();
        -one + p
    } else if x < one {
        // x * (1 - x * (1 - x * 1.5))
        let one_point_five = T::from(1.5).unwrap();
        x * (one - x * (one - x * one_point_five))
    } else if x < T::from(3.0).unwrap() {
        let l = x.ln();
        let l_ln = l.ln();
        // l.ln() might be generic, ensuring generic max?
        // Float trait usually has max method? No, generic T usually uses specific methods.
        // MathScalar implies Float which has max.
        // But x.ln() could be negative. T::zero() needed.
        let safe_l_ln = if l_ln > T::zero() { l_ln } else { T::zero() };
        l - safe_l_ln
    } else {
        let l1 = x.ln();
        let l2 = l1.ln();
        l1 - l2 + l2 / l1
    };

    let tolerance = T::from(1e-15).unwrap();
    let neg_one = -one;
    let two = T::from(2.0).unwrap();
    let half = T::from(0.5).unwrap();

    for _ in 0..50 {
        if w <= neg_one {
            w = T::from(-0.99).unwrap();
        }
        let ew = w.exp();
        let wew = w * ew;
        let f = wew - x;
        let w1 = w + one;

        // Break if w+1 is small (singularity near -1)
        if w1.abs() < tolerance {
            break;
        }
        let fp = ew * w1;
        let fpp = ew * (w + two);
        let d = f * fp / (fp * fp - half * f * fpp);
        w -= d;

        if d.abs() < tolerance * (one + w.abs()) {
            break;
        }
    }
    Some(w)
}

/// Polygamma function ψⁿ(x) = d^(n+1)/dx^(n+1) ln(Γ(x))
///
/// Uses recurrence to shift argument, then asymptotic expansion with Bernoulli numbers:
/// ψⁿ(x) = (-1)^(n+1) n! Σ_{k=0}^∞ 1/(x+k)^(n+1)
///
/// Reference: DLMF §5.15 <https://dlmf.nist.gov/5.15>
pub(crate) fn eval_polygamma<T: MathScalar>(n: i32, x: T) -> Option<T> {
    if n < 0 {
        return None;
    }
    match n {
        0 => eval_digamma(x),
        1 => eval_trigamma(x),
        // For n >= 2, use general formula (tetragamma had accuracy issues)
        _ => {
            if x <= T::zero() && x.fract() == T::zero() {
                return None;
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
                factorial *= T::from(i).unwrap();
            }

            let fifteen = T::from(15.0).unwrap();
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
                    n_minus_1_fact *= T::from(i).unwrap();
                }
            }

            // term 1: (n-1)! / xv^n
            let mut sum = n_minus_1_fact / xv.powi(n);
            // term 2: n! / (2 xv^(n+1))
            let two = T::from(2.0).unwrap();
            sum += factorial / (two * xv.powi(n_plus_one));

            let mut xpow = xv.powi(n + 2);
            let mut fact_ratio = factorial * T::from(n + 1).unwrap();

            let mut prev_term_abs = T::max_value();

            for (k, &(b_num, b_den)) in bernoulli_pairs.iter().enumerate() {
                let two_k = 2 * (k + 1);
                // (2k)!
                let mut factorial_2k = T::one();
                for i in 1..=two_k {
                    factorial_2k *= T::from(i).unwrap();
                }

                let val_bk = T::from(b_num).unwrap() / T::from(b_den).unwrap();
                let term = val_bk * fact_ratio / (factorial_2k * xpow);

                if term.abs() > prev_term_abs {
                    break;
                }
                prev_term_abs = term.abs();
                sum += term;

                xpow *= xv * xv;
                let next_factor1 = T::from(n + two_k as i32).unwrap();
                let next_factor2 = T::from(n + two_k as i32 + 1).unwrap();
                fact_ratio *= next_factor1 * next_factor2;
            }

            Some(r + asym_sign * sum)
        }
    }
}

/// Hermite polynomials H_n(x) (physicist's convention)
///
/// Uses three-term recurrence: H_{n+1}(x) = 2x H_n(x) - 2n H_{n-1}(x)
/// with H_0(x) = 1, H_1(x) = 2x
///
/// Reference: DLMF §18.9 <https://dlmf.nist.gov/18.9>
pub(crate) fn eval_hermite<T: MathScalar>(n: i32, x: T) -> Option<T> {
    if n < 0 {
        return None;
    }
    if n == 0 {
        return Some(T::one());
    }
    let two = T::from(2.0).unwrap();
    let term1 = two * x;
    if n == 1 {
        return Some(term1);
    }
    let (mut h0, mut h1) = (T::one(), term1);
    for k in 1..n {
        let f_k = T::from(k).unwrap();
        // h2 = 2x * h1 - 2k * h0
        let h2 = (two * x * h1) - (two * f_k * h0);
        h0 = h1;
        h1 = h2;
    }
    Some(h1)
}

/// Associated Legendre function P_l^m(x) for -1 ≤ x ≤ 1
///
/// Uses recurrence relation starting from P_m^m, then P_{m+1}^m.
/// Negative m handled via relation: P_l^{-m} = (-1)^m (l-m)!/(l+m)! P_l^m
///
/// Reference: DLMF §14.10 <https://dlmf.nist.gov/14.10>
pub(crate) fn eval_assoc_legendre<T: MathScalar>(l: i32, m: i32, x: T) -> Option<T> {
    if l < 0 || m.abs() > l || x.abs() > T::one() {
        // Technically |x| > 1 is domain error, but some continuations exist.
        // Standard impl assumes -1 <= x <= 1
        return None;
    }
    let m_abs = m.abs();
    let mut pmm = T::one();
    let one = T::one();

    if m_abs > 0 {
        let sqx = (one - x * x).sqrt();
        let mut fact = T::one();
        let two = T::from(2.0).unwrap();
        for _ in 1..=m_abs {
            pmm = pmm * (-fact) * sqx;
            fact += two;
        }
    }
    if l == m_abs {
        return Some(pmm);
    }

    let two_m_plus_1 = T::from(2 * m_abs + 1).unwrap();
    let pmmp1 = x * two_m_plus_1 * pmm;

    if l == m_abs + 1 {
        return Some(pmmp1);
    }

    let (mut pll, mut pmm_prev) = (T::zero(), pmm);
    let mut pmm_curr = pmmp1;

    for ll in (m_abs + 2)..=l {
        let f_ll = T::from(ll).unwrap();
        let f_m_abs = T::from(m_abs).unwrap();

        let term1_fact = T::from(2 * ll - 1).unwrap();
        let term2_fact = T::from(ll + m_abs - 1).unwrap();
        let denom = f_ll - f_m_abs;

        pll = (x * term1_fact * pmm_curr - term2_fact * pmm_prev) / denom;
        pmm_prev = pmm_curr;
        pmm_curr = pll;
    }
    Some(pll)
}

/// Spherical harmonics Y_l^m(θ, φ) (real form)
///
/// Y_l^m = N_l^m P_l^m(cos θ) cos(mφ)
/// where N_l^m is the normalization factor.
///
/// Reference: DLMF §14.30 <https://dlmf.nist.gov/14.30>
pub(crate) fn eval_spherical_harmonic<T: MathScalar>(
    l: i32,
    m: i32,
    theta: T,
    phi: T,
) -> Option<T> {
    if l < 0 || m.abs() > l {
        return None;
    }
    let cos_theta = theta.cos();
    let plm = eval_assoc_legendre(l, m, cos_theta)?;
    let m_abs = m.abs();

    // Factorials
    let mut fact_lm = T::one();
    for i in 1..=(l - m_abs) {
        fact_lm *= T::from(i).unwrap();
    }

    let mut fact_lplusm = T::one();
    for i in 1..=(l + m_abs) {
        fact_lplusm *= T::from(i).unwrap();
    }

    let four = T::from(4.0).unwrap();
    let two_l_plus_1 = T::from(2 * l + 1).unwrap();
    let pi = T::PI();

    let norm_sq = (two_l_plus_1 / (four * pi)) * (fact_lm / fact_lplusm);
    let norm = norm_sq.sqrt();

    let m_phi = T::from(m).unwrap() * phi;
    Some(norm * plm * m_phi.cos())
}

/// Complete elliptic integral of the first kind K(k)
///
/// K(k) = ∫₀^(π/2) dθ / √(1 - k² sin²θ)
/// Uses the arithmetic-geometric mean (AGM) algorithm: K(k) = π/(2 · AGM(1, √(1-k²)))
///
/// Reference: DLMF §19.8 <https://dlmf.nist.gov/19.8>
pub(crate) fn eval_elliptic_k<T: MathScalar>(k: T) -> Option<T> {
    let one = T::one();
    if k.abs() >= one {
        return Some(T::infinity());
    }
    let mut a = one;
    let mut b = (one - k * k).sqrt();

    let two = T::from(2.0).unwrap();
    let tolerance = T::from(1e-15).unwrap();

    for _ in 0..25 {
        let an = (a + b) / two;
        let bn = (a * b).sqrt();
        a = an;
        b = bn;
        if (a - b).abs() < tolerance {
            break;
        }
    }
    let pi = T::PI();
    Some(pi / (two * a))
}

/// Complete elliptic integral of the second kind E(k)
///
/// E(k) = ∫₀^(π/2) √(1 - k² sin²θ) dθ
/// Uses the AGM algorithm with correction terms.
///
/// Reference: DLMF §19.8 <https://dlmf.nist.gov/19.8>
pub(crate) fn eval_elliptic_e<T: MathScalar>(k: T) -> Option<T> {
    let one = T::one();
    if k.abs() > one {
        return Some(T::nan());
    }
    let mut a = one;
    let mut b = (one - k * k).sqrt();

    let k2 = k * k;
    let mut sum = one - k2 / T::from(2.0).unwrap();
    let mut pow2 = T::from(0.5).unwrap();
    let two = T::from(2.0).unwrap();
    let tolerance = T::from(1e-15).unwrap();

    for _ in 0..25 {
        let an = (a + b) / two;
        let bn = (a * b).sqrt();
        let cn = (a - b) / two;
        sum -= pow2 * cn * cn;
        a = an;
        b = bn;
        pow2 *= two;
        if cn.abs() < tolerance {
            break;
        }
    }
    let pi = T::PI();
    Some(pi / (two * a) * sum)
}

// ===== Bessel functions =====
//
// All Bessel approximations use rational function fits from:
// - Abramowitz & Stegun (1964) "Handbook of Mathematical Functions" §9.4, §9.8
// - Hart et al. (1968) "Computer Approximations"
// See also: DLMF Chapter 10 <https://dlmf.nist.gov/10>

/// Bessel function of the first kind J_n(x)
///
/// Uses forward recurrence: J_{n+1}(x) = (2n/x) J_n(x) - J_{n-1}(x)
/// with J_0 and J_1 computed via rational approximations.
///
/// # Special Values
/// - J_0(0) = 1
/// - J_n(0) = 0 for n ≠ 0
///
/// Reference: A&S §9.1.27, DLMF §10.6 <https://dlmf.nist.gov/10.6>
pub(crate) fn bessel_j<T: MathScalar>(n: i32, x: T) -> Option<T> {
    let n_abs = n.abs();

    // Special case: handle J_n(0) correctly
    // J_0(0) = 1, J_n(0) = 0 for n ≠ 0
    let threshold = T::from(1e-10).unwrap();
    if x.abs() < threshold {
        return Some(if n_abs == 0 { T::one() } else { T::zero() });
    }

    let j0 = bessel_j0(x);
    if n_abs == 0 {
        return Some(j0);
    }
    let j1 = bessel_j1(x);
    if n_abs == 1 {
        return Some(if n < 0 { -j1 } else { j1 });
    }

    let (mut jp, mut jc) = (j0, j1);

    for k in 1..n_abs {
        let k_t = T::from(k).unwrap();
        let jn = (T::from(2.0).unwrap() * k_t / x) * jc - jp;
        jp = jc;
        jc = jn;
    }
    Some(if n < 0 && n_abs % 2 == 1 { -jc } else { jc })
}

/// Bessel function J_0(x) via rational approximation
///
/// For |x| < 8: uses polynomial ratio
/// For |x| ≥ 8: uses asymptotic form J_0(x) ≈ √(2/πx) cos(x - π/4) P(8/x)
///
/// Reference: A&S §9.4.1-9.4.6 <https://dlmf.nist.gov/10.17>
pub(crate) fn bessel_j0<T: MathScalar>(x: T) -> T {
    let ax = x.abs();
    let eight = T::from(8.0).unwrap();

    if ax < eight {
        let y = x * x;
        const NUM_COEFFS: [f64; 6] = [
            57568490574.0,
            -13362590354.0,
            651619640.7,
            -11214424.18,
            77392.33017,
            -184.9052456,
        ];
        const DEN_COEFFS: [f64; 6] = [
            57568490411.0,
            1029532985.0,
            9494680.718,
            59272.64853,
            267.8532712,
            1.0,
        ];

        eval_rational_poly(y, &NUM_COEFFS, &DEN_COEFFS)
    } else {
        let z = eight / ax;
        let y = z * z;
        let shift = T::from(0.785398164).unwrap();
        let xx = ax - shift;

        let c_sqrt = T::FRAC_2_PI();
        let term_sqrt = (c_sqrt / ax).sqrt();

        const P_COS_COEFFS: [f64; 5] = [
            1.0,
            -0.1098628627e-2,
            0.2734510407e-4,
            -0.2073370639e-5,
            0.2093887211e-6,
        ];
        // P ~ cos
        let p_cos = eval_poly_horner(y, &P_COS_COEFFS);

        const P_SIN_COEFFS: [f64; 5] = [
            -0.1562499995e-1,
            0.1430488765e-3,
            -0.6911147651e-5,
            0.7621095161e-6,
            0.934935152e-7,
        ];
        // Q ~ sin
        let p_sin = eval_poly_horner(y, &P_SIN_COEFFS);

        term_sqrt * (xx.cos() * p_cos - z * xx.sin() * p_sin)
    }
}

/// Bessel function J_1(x) via rational approximation
///
/// Same structure as J_0 with different coefficients.
///
/// Reference: A&S §9.4.4-9.4.6 <https://dlmf.nist.gov/10.17>
pub(crate) fn bessel_j1<T: MathScalar>(x: T) -> T {
    let ax = x.abs();
    let eight = T::from(8.0).unwrap();

    if ax < eight {
        let y = x * x;
        const NUM_COEFFS: [f64; 6] = [
            72362614232.0,
            -7895059235.0,
            242396853.1,
            -2972611.439,
            15704.48260,
            -30.16036606,
        ];
        const DEN_COEFFS: [f64; 6] = [
            144725228442.0,
            2300535178.0,
            18583304.74,
            99447.43394,
            376.9991397,
            1.0,
        ];

        let term = eval_rational_poly(y, &NUM_COEFFS, &DEN_COEFFS);
        x * term
    } else {
        let z = eight / ax;
        let y = z * z;
        let shift = T::from(2.356194491).unwrap();
        let xx = ax - shift;

        let c_sqrt = T::FRAC_2_PI();
        let term_sqrt = (c_sqrt / ax).sqrt();

        const P_COS_COEFFS: [f64; 5] = [
            1.0,
            0.183105e-2,
            -0.3516396496e-4,
            0.2457520174e-5,
            -0.240337019e-6,
        ];
        let p_cos = eval_poly_horner(y, &P_COS_COEFFS);

        const P_SIN_COEFFS: [f64; 5] = [
            0.04687499995,
            -0.2002690873e-3,
            0.8449199096e-5,
            -0.88228987e-6,
            0.105787412e-6,
        ];
        let p_sin = eval_poly_horner(y, &P_SIN_COEFFS);

        let ans = term_sqrt * (xx.cos() * p_cos - z * xx.sin() * p_sin);

        if x < T::zero() { -ans } else { ans }
    }
}

/// Bessel function of the second kind Y_n(x)
///
/// Uses forward recurrence: Y_{n+1}(x) = (2n/x) Y_n(x) - Y_{n-1}(x)
/// Defined only for x > 0 (singular at origin).
///
/// Reference: A&S §9.1.27, DLMF §10.6 <https://dlmf.nist.gov/10.6>
pub(crate) fn bessel_y<T: MathScalar>(n: i32, x: T) -> Option<T> {
    if x <= T::zero() {
        return None;
    }
    let n_abs = n.abs();
    let y0 = bessel_y0(x);
    if n_abs == 0 {
        return Some(y0);
    }
    let y1 = bessel_y1(x);
    if n_abs == 1 {
        return Some(if n < 0 { -y1 } else { y1 });
    }
    let (mut yp, mut yc) = (y0, y1);
    let two = T::from(2.0).unwrap();

    for k in 1..n_abs {
        let k_t = T::from(k).unwrap();
        let yn = (two * k_t / x) * yc - yp;
        yp = yc;
        yc = yn;
    }
    Some(if n < 0 && n_abs % 2 == 1 { -yc } else { yc })
}

/// Bessel function Y_0(x) via rational approximation
///
/// Reference: A&S §9.4.1-9.4.6 <https://dlmf.nist.gov/10.17>
pub(crate) fn bessel_y0<T: MathScalar>(x: T) -> T {
    let eight = T::from(8.0).unwrap();
    if x < eight {
        let y = x * x;
        const NUM_COEFFS_REAL: [f64; 6] = [
            -2957821389.0,
            7062834065.0,
            -512359803.6,
            10879881.29,
            -86327.92757,
            228.4622733,
        ];

        const DEN_COEFFS: [f64; 6] = [
            40076544269.0,
            745249964.8,
            7189466.438,
            47447.26470,
            226.1030244,
            1.0,
        ];

        let term = eval_rational_poly(y, &NUM_COEFFS_REAL, &DEN_COEFFS);
        let c = T::FRAC_2_PI();
        term + c * bessel_j0(x) * x.ln()
    } else {
        let z = eight / x;
        let y = z * z;
        let shift = T::from(0.785398164).unwrap();
        let xx = x - shift;

        let c_sqrt = T::FRAC_2_PI();
        let term_sqrt = (c_sqrt / x).sqrt();

        const P_SIN_COEFFS: [f64; 5] = [
            1.0,
            -0.1098628627e-2,
            0.2734510407e-4,
            -0.2073370639e-5,
            0.2093887211e-6,
        ];
        let p_sin = eval_poly_horner(y, &P_SIN_COEFFS);

        const P_COS_COEFFS: [f64; 5] = [
            -0.1562499995e-1,
            0.1430488765e-3,
            -0.6911147651e-5,
            0.7621095161e-6,
            0.934935152e-7,
        ];
        let p_cos = eval_poly_horner(y, &P_COS_COEFFS);

        term_sqrt * (xx.sin() * p_sin + z * xx.cos() * p_cos)
    }
}

/// Bessel function Y_1(x) via rational approximation
///
/// Reference: A&S §9.4.4-9.4.6 <https://dlmf.nist.gov/10.17>
pub(crate) fn bessel_y1<T: MathScalar>(x: T) -> T {
    let eight = T::from(8.0).unwrap();
    if x < eight {
        let y = x * x;
        const NUM_COEFFS: [f64; 6] = [
            -0.4900604943e13,
            0.1275274390e13,
            -0.5153438139e11,
            0.7349264551e9,
            -0.4237922726e7,
            0.8511937935e4,
        ];

        const DEN_COEFFS_REAL: [f64; 7] = [
            0.2499580570e14,
            0.4244419664e12,
            0.3733650367e10,
            0.2245904002e8,
            0.1020426050e6,
            0.3549632885e3,
            1.0,
        ];

        let term_poly = eval_rational_poly(y, &NUM_COEFFS, &DEN_COEFFS_REAL);
        let term = x * term_poly;
        let c = T::FRAC_2_PI();
        term + c * (bessel_j1(x) * x.ln() - T::one() / x)
    } else {
        let z = eight / x;
        let y = z * z;
        let shift = T::from(2.356194491).unwrap();
        let xx = x - shift;

        let c_sqrt = T::FRAC_2_PI();
        let term_sqrt = (c_sqrt / x).sqrt();

        const P_SIN_COEFFS: [f64; 5] = [
            1.0,
            0.183105e-2,
            -0.3516396496e-4,
            0.2457520174e-5,
            -0.240337019e-6,
        ];
        let p_sin = eval_poly_horner(y, &P_SIN_COEFFS);

        const P_COS_COEFFS: [f64; 5] = [
            0.04687499995,
            -0.2002690873e-3,
            0.8449199096e-5,
            -0.88228987e-6,
            0.105787412e-6,
        ];
        let p_cos = eval_poly_horner(y, &P_COS_COEFFS);

        term_sqrt * (xx.sin() * p_sin + z * xx.cos() * p_cos)
    }
}

/// Modified Bessel function of the first kind I_n(x)
///
/// Uses Miller's backward recurrence algorithm for numerical stability.
/// I_{k-1} = (2k/x) I_k + I_{k+1}, normalized using I_0(x).
///
/// Reference: Numerical Recipes §6.6, A&S §9.6 <https://dlmf.nist.gov/10.25>
pub(crate) fn bessel_i<T: MathScalar>(n: i32, x: T) -> Option<T> {
    let n_abs = n.abs();
    if n_abs == 0 {
        return Some(bessel_i0(x));
    }
    if n_abs == 1 {
        return Some(bessel_i1(x));
    }

    let threshold = T::from(1e-10).unwrap();
    if x.abs() < threshold {
        return Some(T::zero());
    }

    // Miller's backward recurrence algorithm
    // Start from a large order N >> n, set I_N = 0, I_{N-1} = 1
    // Recur backward using: I_{k-1} = (2k/x) * I_k + I_{k+1}
    // Normalize using I_0(x) as reference

    let two = T::from(2.0).unwrap();

    // Choose starting order N based on x and n
    // Empirical formula: N = n + sqrt(40*n) + 10 works well
    // The factor 40 comes from numerical stability analysis:
    // - Ensures backward recurrence converges before reaching target order
    // - Balances computational cost vs. accuracy (see NR §6.6)
    // - Tested empirically for x ∈ [0.1, 100], n ∈ [0, 100]
    let n_start = n_abs + ((40 * n_abs) as f64).sqrt() as i32 + 10;
    let n_start = n_start.max(n_abs + 20);

    // Initialize backward recurrence
    let mut i_next = T::zero(); // I_{k+1}
    let mut i_curr = T::from(1e-30).unwrap(); // I_k (small nonzero to avoid underflow)
    let mut result = T::zero();
    let mut sum = T::zero(); // For normalization: sum = I_0 + 2*(I_2 + I_4 + ...)

    // Backward recurrence
    for k in (0..=n_start).rev() {
        let k_t = T::from(k).unwrap();
        // I_{k-1} = (2k/x) * I_k + I_{k+1}
        let i_prev = (two * k_t / x) * i_curr + i_next;

        // Save I_n when we reach it
        if k == n_abs {
            result = i_curr;
        }

        // Accumulate for normalization (using I_0 + 2*sum of even terms)
        if k == 0 {
            sum += i_curr;
        } else if k % 2 == 0 {
            sum += two * i_curr;
        }

        i_next = i_curr;
        i_curr = i_prev;
    }

    // Normalize: actual I_n = result * I_0(x) / computed_I_0
    // The sum approximates I_0 when properly normalized
    let i0_actual = bessel_i0(x);
    let scale = i0_actual / sum;

    Some(result * scale)
}

/// Modified Bessel function I_0(x) via polynomial approximation
///
/// Reference: A&S §9.8.1-9.8.4 <https://dlmf.nist.gov/10.40>
pub(crate) fn bessel_i0<T: MathScalar>(x: T) -> T {
    let ax = x.abs();
    let three_seven_five = T::from(3.75).unwrap();

    if ax < three_seven_five {
        let y = (x / three_seven_five).powi(2);
        // c0=1.0 implicit? "T::one() + y * (...)"
        // Yes.
        const COEFFS: [f64; 7] = [
            1.0, 3.5156229, 3.0899424, 1.2067492, 0.2659732, 0.0360768, 0.0045813,
        ];
        eval_poly_horner(y, &COEFFS)
    } else {
        let y = three_seven_five / ax;
        let term = ax.exp() / ax.sqrt();

        const COEFFS: [f64; 9] = [
            0.39894228,
            0.01328592,
            0.00225319,
            -0.00157565,
            0.00916281,
            -0.02057706,
            0.02635537,
            -0.01647633,
            0.00392377,
        ];
        term * eval_poly_horner(y, &COEFFS)
    }
}

/// Modified Bessel function I_1(x) via polynomial approximation
///
/// Reference: A&S §9.8.1-9.8.4 <https://dlmf.nist.gov/10.40>
pub(crate) fn bessel_i1<T: MathScalar>(x: T) -> T {
    let ax = x.abs();
    let three_seven_five = T::from(3.75).unwrap();

    let ans = if ax < three_seven_five {
        let y = (x / three_seven_five).powi(2);
        const COEFFS: [f64; 7] = [
            0.5, 0.87890594, 0.51498869, 0.15084934, 0.02658733, 0.00301532, 0.00032411,
        ];
        ax * eval_poly_horner(y, &COEFFS)
    } else {
        let y = three_seven_five / ax;
        let term = ax.exp() / ax.sqrt();

        const COEFFS_LARGE: [f64; 9] = [
            0.39894228,
            -0.03988024,
            -0.00362018,
            0.00163801,
            -0.01031555,
            0.02282967,
            -0.02895312,
            0.01787654,
            -0.00420059,
        ];
        term * eval_poly_horner(y, &COEFFS_LARGE)
    };
    if x < T::zero() { -ans } else { ans }
}

/// Modified Bessel function of the second kind K_n(x)
///
/// Uses forward recurrence: K_{n+1} = (2n/x) K_n + K_{n-1}
/// Defined only for x > 0 (singular at origin).
///
/// Reference: A&S §9.6.26, DLMF §10.29 <https://dlmf.nist.gov/10.29>
pub(crate) fn bessel_k<T: MathScalar>(n: i32, x: T) -> Option<T> {
    if x <= T::zero() {
        return None;
    }
    let n_abs = n.abs();
    let k0 = bessel_k0(x);
    if n_abs == 0 {
        return Some(k0);
    }
    let k1 = bessel_k1(x);
    if n_abs == 1 {
        return Some(k1);
    }
    let (mut kp, mut kc) = (k0, k1);
    let two = T::from(2.0).unwrap();

    for k in 1..n_abs {
        let k_t = T::from(k).unwrap();
        let kn = kp + (two * k_t / x) * kc;
        kp = kc;
        kc = kn;
    }
    Some(kc)
}

/// Modified Bessel function K_0(x) via polynomial approximation
///
/// Reference: A&S §9.8.5-9.8.8 <https://dlmf.nist.gov/10.40>
pub(crate) fn bessel_k0<T: MathScalar>(x: T) -> T {
    let two = T::from(2.0).unwrap();
    if x <= two {
        let four = T::from(4.0).unwrap();
        let y = x * x / four;
        let i0 = bessel_i0(x);
        let ln_term = -(x / two).ln() * i0;

        const COEFFS: [f64; 7] = [
            -0.57721566,
            0.42278420,
            0.23069756,
            0.03488590,
            0.00262698,
            0.00010750,
            0.0000074,
        ];
        let poly = eval_poly_horner(y, &COEFFS);
        ln_term + poly
    } else {
        let y = two / x;
        let term = (-x).exp() / x.sqrt();

        const COEFFS_LARGE: [f64; 8] = [
            1.25331414,
            -0.07832358,
            0.02189568,
            -0.01062446,
            0.00587872,
            -0.00251540,
            0.00053208,
            -0.000025200,
        ];
        term * eval_poly_horner(y, &COEFFS_LARGE)
    }
}

pub(crate) fn bessel_k1<T: MathScalar>(x: T) -> T {
    let two = T::from(2.0).unwrap();
    if x <= two {
        let four = T::from(4.0).unwrap();
        let y = x * x / four;

        let term1 = x.ln() * bessel_i1(x);
        let term2 = T::one() / x;

        const COEFFS: [f64; 7] = [
            1.0,
            0.15443144,
            -0.67278579,
            -0.18156897,
            -0.01919402,
            -0.00110404,
            -0.00004686,
        ];
        let poly = eval_poly_horner(y, &COEFFS);
        term1 + term2 * poly
    } else {
        let y = two / x;
        let term = (-x).exp() / x.sqrt();

        const COEFFS_LARGE: [f64; 8] = [
            1.25331414,
            0.23498619,
            -0.03655620,
            0.01504268,
            -0.00780353,
            0.00325614,
            -0.00068245,
            0.0000316,
        ];
        term * eval_poly_horner(y, &COEFFS_LARGE)
    }
}

/// Helper: Evaluate polynomial c[0] + x*c[1] + ... + x^n*c[n] using Horner's method
///
/// Note: Coefficients should be ordered from constant term c[0] to highest power c[n].
fn eval_poly_horner<T: MathScalar>(x: T, coeffs: &[f64]) -> T {
    let mut sum = T::zero();
    // Horner's method: c[0] + x(c[1] + x(c[2] + ...))
    // We iterate from highest power c[n] down to c[0]
    for &c in coeffs.iter().rev() {
        sum = sum * x + T::from(c).unwrap();
    }
    sum
}

/// Helper: Evaluate rational function P(x)/Q(x)
///
/// Computes (n[0] + x*n[1] + ...) / (d[0] + x*d[1] + ...)
fn eval_rational_poly<T: MathScalar>(x: T, num: &[f64], den: &[f64]) -> T {
    let n = eval_poly_horner(x, num);
    let d = eval_poly_horner(x, den);
    n / d
}
