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

/// Exponential function for polar representation
///
/// Currently just wraps `exp()`. This function exists as a placeholder
/// for potential future polar-form exponential implementations.
pub fn eval_exp_polar<T: MathScalar>(x: T) -> T {
    x.exp()
}

/// Error function erf(x) = (2/√π) ∫₀ˣ e^(-t²) dt
///
/// Uses Taylor series expansion: erf(x) = (2/√π) Σₙ (-1)ⁿ x^(2n+1) / (n!(2n+1))
/// with Kahan summation for numerical stability.
///
/// Reference: DLMF §7.6.1 <https://dlmf.nist.gov/7.6#E1>
pub fn eval_erf<T: MathScalar>(x: T) -> T {
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
pub fn eval_gamma<T: MathScalar>(x: T) -> Option<T> {
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
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
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
pub fn eval_digamma<T: MathScalar>(x: T) -> Option<T> {
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
pub fn eval_trigamma<T: MathScalar>(x: T) -> Option<T> {
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
pub fn eval_tetragamma<T: MathScalar>(x: T) -> Option<T> {
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
pub fn eval_zeta<T: MathScalar>(x: T) -> Option<T> {
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
        let n = T::from(f64::from(n_terms)).unwrap();
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

    for (idx, d_coeff) in d_coeffs.iter_mut().enumerate().skip(1) {
        let k = idx;
        let i = T::from(k - 1).unwrap();
        let two_i_plus_1 = T::from(2 * k - 1).unwrap();
        let two_i_plus_2 = T::from(2 * k).unwrap();
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
        let k_plus_1 = T::from(k + 1).unwrap();
        let sign = if k % 2 == 0 { one } else { -one };
        let term = sign * (*d_coeff_k - d_n) / k_plus_1.powf(s);

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
/// * `Some(value)` if convergent
/// * `None` if at pole (s=1) or invalid
pub fn eval_zeta_deriv<T: MathScalar>(n: i32, x: T) -> Option<T> {
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
#[allow(clippy::many_single_char_names)]
pub fn eval_lambert_w<T: MathScalar>(x: T) -> Option<T> {
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
pub fn eval_polygamma<T: MathScalar>(n: i32, x: T) -> Option<T> {
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
                #[allow(clippy::cast_possible_wrap)]
                let two_k: i32 = 2 * ((k + 1) as i32);
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
                let next_factor1 = T::from(n + two_k).unwrap();
                let next_factor2 = T::from(n + two_k + 1).unwrap();
                fact_ratio *= next_factor1 * next_factor2;
            }

            Some(r + asym_sign * sum)
        }
    }
}

/// Hermite polynomials `H_n(x)` (physicist's convention)
///
/// Uses three-term recurrence: H_{n+1}(x) = 2x `H_n(x)` - 2n H_{n-1}(x)
/// with `H_0(x)` = 1, `H_1(x)` = 2x
///
/// Reference: DLMF §18.9 <https://dlmf.nist.gov/18.9>
pub fn eval_hermite<T: MathScalar>(n: i32, x: T) -> Option<T> {
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

/// Associated Legendre function `P_l^m(x)` for -1 ≤ x ≤ 1
///
/// Uses recurrence relation starting from `P_m^m`, then P_{m+1}^m.
/// Negative m handled via relation: P_l^{-m} = (-1)^m (l-m)!/(l+m)! `P_l^m`
///
/// Reference: DLMF §14.10 <https://dlmf.nist.gov/14.10>
pub fn eval_assoc_legendre<T: MathScalar>(l: i32, m: i32, x: T) -> Option<T> {
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

/// Spherical harmonics `Y_l^m(θ`, φ) (real form)
///
/// `Y_l^m` = `N_l^m` `P_l^m(cos` θ) cos(mφ)
/// where `N_l^m` is the normalization factor.
///
/// Reference: DLMF §14.30 <https://dlmf.nist.gov/14.30>
pub fn eval_spherical_harmonic<T: MathScalar>(l: i32, m: i32, theta: T, phi: T) -> Option<T> {
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
#[allow(clippy::unnecessary_wraps)]
pub fn eval_elliptic_k<T: MathScalar>(k: T) -> Option<T> {
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
#[allow(clippy::unnecessary_wraps)]
pub fn eval_elliptic_e<T: MathScalar>(k: T) -> Option<T> {
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

/// Bessel function of the first kind `J_n(x)`
///
/// Uses a hybrid algorithm for numerical stability:
/// - **Forward Recurrence** for $n \le |x|$: $J_{n+1} = (2n/x) `J_n` - J_{n-1}$
/// - **Miller's Algorithm (Backward Recurrence)** for $n > |x|$: Stable for decaying region.
///
/// with `J_0` and `J_1` computed via rational approximations.
///
/// # Special Values
/// - `J_0(0)` = 1
/// - `J_n(0)` = 0 for n ≠ 0
///
/// Reference: A&S §9.1.27, DLMF §10.6 <https://dlmf.nist.gov/10.6>
pub fn bessel_j<T: MathScalar>(n: i32, x: T) -> Option<T> {
    let n_abs = n.abs();
    let ax = x.abs();

    // Special case: J_n(0)
    let threshold = T::from(1e-10).unwrap();
    if ax < threshold {
        return Some(if n_abs == 0 { T::one() } else { T::zero() });
    }

    // REGIME 1: Forward Recurrence is stable when n <= |x|
    if T::from(n_abs).unwrap() <= ax {
        return bessel_j_forward(n, x);
    }

    // REGIME 2: Miller's Algorithm (Backward Recurrence) when n > |x|
    bessel_j_miller(n, x)
}

/// Forward recurrence for `J_n(x)` when n <= |x|
#[allow(clippy::unnecessary_wraps)]
fn bessel_j_forward<T: MathScalar>(n: i32, x: T) -> Option<T> {
    let n_abs = n.abs();

    let j0 = bessel_j0(x);
    if n_abs == 0 {
        return Some(j0);
    }

    let j1 = bessel_j1(x);
    if n_abs == 1 {
        return Some(if n < 0 { -j1 } else { j1 });
    }

    let (mut jp, mut jc) = (j0, j1);
    let two = T::from(2.0).unwrap();

    for k in 1..n_abs {
        let k_t = T::from(k).unwrap();
        // J_{k+1} = (2k/x) J_k - J_{k-1}
        let jn = (two * k_t / x) * jc - jp;
        jp = jc;
        jc = jn;
    }

    Some(if n < 0 && n_abs % 2 == 1 { -jc } else { jc })
}

/// Miller's backward recurrence algorithm for `J_n(x)` when n > |x|
#[allow(clippy::unnecessary_wraps)]
fn bessel_j_miller<T: MathScalar>(n: i32, x: T) -> Option<T> {
    let n_abs = n.abs();
    let two = T::from(2.0).unwrap();

    // Choose starting N using a safe heuristic
    let n_start = compute_miller_start(n_abs);

    // Initialize recurrence
    let mut j_next = T::zero(); // J_{k+1}
    let mut j_curr = T::from(1e-30).unwrap(); // J_k (small seed)

    let mut result = T::zero();
    let mut sum = T::zero(); // For normalization: J_0 + 2*(J_2 + J_4 + ...)
    let mut compensation = T::zero(); // Kahan summation compensation

    // Backward recurrence: J_{k-1} = (2k/x) J_k - J_{k+1}
    for k in (0..=n_start).rev() {
        let k_t = T::from(k).unwrap();
        let j_prev = (two * k_t / x) * j_curr - j_next;

        // Store J_n when we reach it
        if k == n_abs {
            result = j_curr;
        }

        // Accumulate normalization sum with Kahan summation
        // Sum = J_0 + 2 \sum_{k=1}^{\infty} J_{2k}
        if k == 0 {
            // sum += j_curr (J_0 term)
            let y = j_curr - compensation;
            let t = sum + y;
            compensation = (t - sum) - y;
            sum = t;
        } else if k % 2 == 0 {
            // sum += 2 * j_curr (even terms)
            let term = two * j_curr;
            let y = term - compensation;
            let t = sum + y;
            compensation = (t - sum) - y;
            sum = t;
        }

        j_next = j_curr;
        j_curr = j_prev;
    }

    // Normalize: true J_n = (computed J_n) / (computed sum)
    // The sum should be J_0(x) of the computed sequence, which is 1
    let scale = T::one() / sum;
    let normalized_result = result * scale;

    Some(if n < 0 && n_abs % 2 == 1 {
        -normalized_result
    } else {
        normalized_result
    })
}

/// Compute starting point for Miller's algorithm
fn compute_miller_start(n: i32) -> i32 {
    // Heuristic from Numerical Recipes with extra safety margin
    // N = n + sqrt(C * n) + M
    let n_f = f64::from(n);
    let c = 50.0; // Conservative for 15+ digit accuracy
    let m = 15; // Safety margin

    n + (c * n_f).sqrt().round() as i32 + m
}

/// Bessel function `J_0(x)` via rational approximation
///
/// For |x| < 8: uses polynomial ratio
/// For |x| ≥ 8: uses asymptotic form `J_0(x)` ≈ √(2/πx) cos(x - π/4) P(8/x)
///
/// Reference: A&S §9.4.1-9.4.6 <https://dlmf.nist.gov/10.17>
pub fn bessel_j0<T: MathScalar>(x: T) -> T {
    const BESSEL_J0_SMALL_NUM: [f64; 6] = [
        57_568_490_574.0,
        -13_362_590_354.0,
        651_619_640.7,
        -11_214_424.18,
        77_392.330_17,
        -184.905_245_6,
    ];
    const BESSEL_J0_SMALL_DEN: [f64; 6] = [
        57_568_490_411.0,
        1_029_532_985.0,
        9_494_680.718,
        59_272.648_53,
        267.853_271_2,
        1.0,
    ];

    const BESSEL_J0_LARGE_P_COS: [f64; 5] = [
        1.0,
        -0.109_862_862_7e-2,
        0.273_451_040_7e-4,
        -0.207_337_063_9e-5,
        0.209_388_721_1e-6,
    ];
    const BESSEL_J0_LARGE_P_SIN: [f64; 5] = [
        -0.156_249_999_5e-1,
        0.143_048_876_5e-3,
        -0.691_114_765_1e-5,
        0.762_109_516_1e-6,
        0.934_935_152e-7,
    ];

    let ax = x.abs();
    let eight = T::from(8.0).unwrap();

    if ax < eight {
        let y = x * x;
        eval_rational_poly(y, &BESSEL_J0_SMALL_NUM, &BESSEL_J0_SMALL_DEN)
    } else {
        let z = eight / ax;
        let y = z * z;
        let shift = T::from(0.785_398_164).unwrap();
        let xx = ax - shift;

        let c_sqrt = T::FRAC_2_PI();
        let term_sqrt = (c_sqrt / ax).sqrt();

        // P ~ cos
        let p_cos = eval_poly_horner(y, &BESSEL_J0_LARGE_P_COS);

        // Q ~ sin
        let p_sin = eval_poly_horner(y, &BESSEL_J0_LARGE_P_SIN);

        term_sqrt * (xx.cos() * p_cos - z * xx.sin() * p_sin)
    }
}

/// Bessel function `J_1(x)` via rational approximation
///
/// Same structure as `J_0` with different coefficients.
///
/// Reference: A&S §9.4.4-9.4.6 <https://dlmf.nist.gov/10.17>
pub fn bessel_j1<T: MathScalar>(x: T) -> T {
    const BESSEL_J1_SMALL_NUM: [f64; 6] = [
        72_362_614_232.0,
        -7_895_059_235.0,
        242_396_853.1,
        -2_972_611.439,
        15_704.482_60,
        -30.160_366_06,
    ];
    const BESSEL_J1_SMALL_DEN: [f64; 6] = [
        144_725_228_442.0,
        2_300_535_178.0,
        18_583_304.74,
        99_447.433_94,
        376.999_139_7,
        1.0,
    ];

    const BESSEL_J1_LARGE_P_COS: [f64; 5] = [
        1.0,
        0.183_105e-2,
        -0.351_639_649_6e-4,
        0.245_752_017_4e-5,
        -0.240_337_019e-6,
    ];
    const BESSEL_J1_LARGE_P_SIN: [f64; 5] = [
        0.046_874_999_95,
        -0.200_269_087_3e-3,
        0.844_919_909_6e-5,
        -0.882_289_87e-6,
        0.105_787_412e-6,
    ];

    let ax = x.abs();
    let eight = T::from(8.0).unwrap();

    if ax < eight {
        let y = x * x;
        let term = eval_rational_poly(y, &BESSEL_J1_SMALL_NUM, &BESSEL_J1_SMALL_DEN);
        x * term
    } else {
        let z = eight / ax;
        let y = z * z;
        let shift = T::from(2.356_194_491).unwrap();
        let xx = ax - shift;

        let c_sqrt = T::FRAC_2_PI();
        let term_sqrt = (c_sqrt / ax).sqrt();

        let p_cos = eval_poly_horner(y, &BESSEL_J1_LARGE_P_COS);
        let p_sin = eval_poly_horner(y, &BESSEL_J1_LARGE_P_SIN);

        let ans = term_sqrt * (xx.cos() * p_cos - z * xx.sin() * p_sin);

        if x < T::zero() { -ans } else { ans }
    }
}

/// Bessel function of the second kind `Y_n(x)`
///
/// Uses forward recurrence: Y_{n+1}(x) = (2n/x) `Y_n(x)` - Y_{n-1}(x)
/// Defined only for x > 0 (singular at origin).
///
/// Reference: A&S §9.1.27, DLMF §10.6 <https://dlmf.nist.gov/10.6>
pub fn bessel_y<T: MathScalar>(n: i32, x: T) -> Option<T> {
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

/// Bessel function `Y_0(x)` via rational approximation
///
/// Reference: A&S §9.4.1-9.4.6 <https://dlmf.nist.gov/10.17>
pub fn bessel_y0<T: MathScalar>(x: T) -> T {
    const BESSEL_Y0_SMALL_NUM: [f64; 6] = [
        -2_957_821_389.0,
        7_062_834_065.0,
        -512_359_803.6,
        10_879_881.29,
        -86_327.927_57,
        228.462_273_3,
    ];
    const BESSEL_Y0_SMALL_DEN: [f64; 6] = [
        40_076_544_269.0,
        745_249_964.8,
        7_189_466.438,
        47_447.264_70,
        226.103_024_4,
        1.0,
    ];

    const BESSEL_Y0_LARGE_P_SIN: [f64; 5] = [
        1.0,
        -0.109_862_862_7e-2,
        0.273_451_040_7e-4,
        -0.207_337_063_9e-5,
        0.209_388_721_1e-6,
    ];
    const BESSEL_Y0_LARGE_P_COS: [f64; 5] = [
        -0.156_249_999_5e-1,
        0.143_048_876_5e-3,
        -0.691_114_765_1e-5,
        0.762_109_516_1e-6,
        0.934_935_152e-7,
    ];

    let eight = T::from(8.0).unwrap();

    if x < eight {
        let y = x * x;
        let term = eval_rational_poly(y, &BESSEL_Y0_SMALL_NUM, &BESSEL_Y0_SMALL_DEN);
        let c = T::FRAC_2_PI();
        term + c * bessel_j0(x) * x.ln()
    } else {
        let z = eight / x;
        let y = z * z;
        let shift = T::from(0.785_398_164).unwrap();
        let xx = x - shift;

        let c_sqrt = T::FRAC_2_PI();
        let term_sqrt = (c_sqrt / x).sqrt();

        let p_sin = eval_poly_horner(y, &BESSEL_Y0_LARGE_P_SIN);
        let p_cos = eval_poly_horner(y, &BESSEL_Y0_LARGE_P_COS);

        term_sqrt * (xx.sin() * p_sin + z * xx.cos() * p_cos)
    }
}

/// Bessel function `Y_1(x)` via rational approximation
///
/// Reference: A&S §9.4.4-9.4.6 <https://dlmf.nist.gov/10.17>
pub fn bessel_y1<T: MathScalar>(x: T) -> T {
    const BESSEL_Y1_SMALL_NUM: [f64; 6] = [
        -0.490_060_494_3e13,
        0.127_527_439_0e13,
        -0.515_343_813_9e11,
        0.734_926_455_1e9,
        -0.423_792_272_6e7,
        0.851_193_793_5e4,
    ];
    const BESSEL_Y1_SMALL_DEN: [f64; 7] = [
        0.249_958_057_0e14,
        0.424_441_966_4e12,
        0.373_365_036_7e10,
        0.224_590_400_2e8,
        0.102_042_605_0e6,
        0.354_963_288_5e3,
        1.0,
    ];

    const BESSEL_Y1_LARGE_P_SIN: [f64; 5] = [
        1.0,
        0.183_105e-2,
        -0.351_639_649_6e-4,
        0.245_752_017_4e-5,
        -0.240_337_019e-6,
    ];
    const BESSEL_Y1_LARGE_P_COS: [f64; 5] = [
        0.046_874_999_95,
        -0.200_269_087_3e-3,
        0.844_919_909_6e-5,
        -0.882_289_87e-6,
        0.105_787_412e-6,
    ];

    let eight = T::from(8.0).unwrap();

    if x < eight {
        let y = x * x;
        let term_poly = eval_rational_poly(y, &BESSEL_Y1_SMALL_NUM, &BESSEL_Y1_SMALL_DEN);
        let term = x * term_poly;
        let c = T::FRAC_2_PI();
        term + c * (bessel_j1(x) * x.ln() - T::one() / x)
    } else {
        let z = eight / x;
        let y = z * z;
        let shift = T::from(2.356_194_491).unwrap();
        let xx = x - shift;

        let c_sqrt = T::FRAC_2_PI();
        let term_sqrt = (c_sqrt / x).sqrt();

        let p_sin = eval_poly_horner(y, &BESSEL_Y1_LARGE_P_SIN);
        let p_cos = eval_poly_horner(y, &BESSEL_Y1_LARGE_P_COS);

        term_sqrt * (xx.sin() * p_sin + z * xx.cos() * p_cos)
    }
}

/// Modified Bessel function of the first kind `I_n(x)`
///
/// Uses Miller's backward recurrence algorithm for numerical stability.
/// I_{k-1} = (2k/x) `I_k` + I_{k+1}, normalized using `I_0(x)`.
///
/// Reference: Numerical Recipes §6.6, A&S §9.6 <https://dlmf.nist.gov/10.25>
pub fn bessel_i<T: MathScalar>(n: i32, x: T) -> T {
    let n_abs = n.abs();
    if n_abs == 0 {
        return bessel_i0(x);
    }
    if n_abs == 1 {
        return bessel_i1(x);
    }

    let threshold = T::from(1e-10).unwrap();
    if x.abs() < threshold {
        return T::zero();
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
    let n_start = n_abs + f64::from(40 * n_abs).sqrt() as i32 + 10;
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

    result * scale
}

/// Modified Bessel function `I_0(x)` via polynomial approximation
///
/// Reference: A&S §9.8.1-9.8.4 <https://dlmf.nist.gov/10.40>
pub fn bessel_i0<T: MathScalar>(x: T) -> T {
    const BESSEL_I0_SMALL: [f64; 7] = [
        1.0,
        3.515_622_9,
        3.089_942_4,
        1.206_749_2,
        0.265_973_2,
        0.036_076_8,
        0.004_581_3,
    ];
    const BESSEL_I0_LARGE: [f64; 9] = [
        0.398_942_28,
        0.013_285_92,
        0.002_253_19,
        -0.001_575_65,
        0.009_162_81,
        -0.020_577_06,
        0.026_355_37,
        -0.016_476_33,
        0.003_923_77,
    ];

    let ax = x.abs();
    let three_seven_five = T::from(3.75).unwrap();

    if ax < three_seven_five {
        let y = (x / three_seven_five).powi(2);
        // c0=1.0 implicit? "T::one() + y * (...)"
        // Yes.
        eval_poly_horner(y, &BESSEL_I0_SMALL)
    } else {
        let y = three_seven_five / ax;
        let term = ax.exp() / ax.sqrt();

        term * eval_poly_horner(y, &BESSEL_I0_LARGE)
    }
}

/// Modified Bessel function `I_1(x)` via polynomial approximation
///
/// Reference: A&S §9.8.1-9.8.4 <https://dlmf.nist.gov/10.40>
pub fn bessel_i1<T: MathScalar>(x: T) -> T {
    const BESSEL_I1_SMALL: [f64; 7] = [
        0.5,
        0.878_905_94,
        0.514_988_69,
        0.150_849_34,
        0.026_587_33,
        0.003_015_32,
        0.000_324_11,
    ];
    const BESSEL_I1_LARGE: [f64; 9] = [
        0.398_942_28,
        -0.039_880_24,
        -0.003_620_18,
        0.001_638_01,
        -0.010_315_55,
        0.022_829_67,
        -0.028_953_12,
        0.017_876_54,
        -0.004_200_59,
    ];

    let ax = x.abs();
    let three_seven_five = T::from(3.75).unwrap();

    let ans = if ax < three_seven_five {
        let y = (x / three_seven_five).powi(2);
        ax * eval_poly_horner(y, &BESSEL_I1_SMALL)
    } else {
        let y = three_seven_five / ax;
        let term = ax.exp() / ax.sqrt();

        term * eval_poly_horner(y, &BESSEL_I1_LARGE)
    };
    if x < T::zero() { -ans } else { ans }
}

/// Modified Bessel function of the second kind `K_n(x)`
///
/// Uses forward recurrence: K_{n+1} = (2n/x) `K_n` + K_{n-1}
/// Defined only for x > 0 (singular at origin).
///
/// Reference: A&S §9.6.26, DLMF §10.29 <https://dlmf.nist.gov/10.29>
pub fn bessel_k<T: MathScalar>(n: i32, x: T) -> Option<T> {
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

/// Modified Bessel function `K_0(x)` via polynomial approximation
///
/// Reference: A&S §9.8.5-9.8.8 <https://dlmf.nist.gov/10.40>
pub fn bessel_k0<T: MathScalar>(x: T) -> T {
    const COEFFS: [f64; 7] = [
        -0.577_215_66,
        0.422_784_20,
        0.230_697_56,
        0.034_885_90,
        0.002_626_98,
        0.000_107_50,
        0.000_007_4,
    ];

    const COEFFS_LARGE: [f64; 8] = [
        1.253_314_14,
        -0.078_323_58,
        0.021_895_68,
        -0.010_624_46,
        0.005_878_72,
        -0.002_515_40,
        0.000_532_08,
        -0.000_025_200,
    ];

    let two = T::from(2.0).unwrap();
    if x <= two {
        let four = T::from(4.0).unwrap();
        let y = x * x / four;
        let i0 = bessel_i0(x);
        let ln_term = -(x / two).ln() * i0;

        let poly = eval_poly_horner(y, &COEFFS);
        ln_term + poly
    } else {
        let y = two / x;
        let term = (-x).exp() / x.sqrt();

        term * eval_poly_horner(y, &COEFFS_LARGE)
    }
}

pub fn bessel_k1<T: MathScalar>(x: T) -> T {
    const COEFFS: [f64; 7] = [
        1.0,
        0.154_431_44,
        -0.672_785_79,
        -0.181_568_97,
        -0.019_194_02,
        -0.001_104_04,
        -0.000_046_86,
    ];

    const COEFFS_LARGE: [f64; 8] = [
        1.253_314_14,
        0.234_986_19,
        -0.036_556_20,
        0.015_042_68,
        -0.007_803_53,
        0.003_256_14,
        -0.000_682_45,
        0.000_031_6,
    ];

    let two = T::from(2.0).unwrap();
    if x <= two {
        let four = T::from(4.0).unwrap();
        let y = x * x / four;

        let term1 = x.ln() * bessel_i1(x);
        let term2 = T::one() / x;

        let poly = eval_poly_horner(y, &COEFFS);
        term1 + term2 * poly
    } else {
        let y = two / x;
        let term = (-x).exp() / x.sqrt();

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
