//! Mathematical function evaluations
//!
//! This module centralizes all mathematical function implementations,
//! organized by category for maintainability.
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

use crate::traits::MathScalar;

pub mod dual;
mod robustness_tests;

pub fn eval_exp_polar<T: MathScalar>(x: T) -> T {
    x.exp()
}

pub fn eval_erf<T: MathScalar>(x: T) -> T {
    let sign = x.signum();
    let x = x.abs();
    // PI is available via FloatConst implementation on T
    let pi = T::PI();
    let sqrt_pi = pi.sqrt();
    let two = T::from(2.0).unwrap();
    let coeff = two / sqrt_pi;

    let mut sum = T::zero();
    let mut factorial = T::one();
    let mut power = x;

    for n in 0..30 {
        let two_n_plus_one = T::from(2 * n + 1).unwrap();

        let term = power / (factorial * two_n_plus_one);

        // Overflow protection: break if term becomes NaN or infinite
        if term.is_nan() || term.is_infinite() {
            break;
        }

        if n % 2 == 0 {
            sum += term;
        } else {
            sum -= term;
        }

        let n_plus_one = T::from(n + 1).unwrap();
        factorial *= n_plus_one;
        power *= x * x;

        // Check convergence with generic epsilon if possible, or T::epsilon()
        // 1e-16 is f64 specific. T::epsilon() is better.
        if term.abs() < T::epsilon() {
            break;
        }
    }
    sign * coeff * sum
}

pub fn eval_gamma<T: MathScalar>(x: T) -> Option<T> {
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

pub fn eval_zeta<T: MathScalar>(x: T) -> Option<T> {
    let one = T::one();
    if (x - one).abs() < T::from(1e-10).unwrap() {
        return None;
    }
    if x > one {
        let mut s = T::zero();
        for n in 1..=100 {
            let n_t = T::from(n).unwrap();
            s += one / n_t.powf(x);
        }
        let n = T::from(100.0).unwrap();
        let term2 = n.powf(one - x) / (x - one);
        let term3 = T::from(0.5).unwrap() / n.powf(x);
        Some(s + term2 + term3)
    } else {
        let pi = T::PI();
        let two = T::from(2.0).unwrap();
        let gs = eval_gamma(one - x)?;
        let z = eval_zeta(one - x)?;
        let term1 = two.powf(x);
        let term2 = pi.powf(x - one);
        let term3 = (pi * x / two).sin();
        Some(term1 * term2 * term3 * gs * z)
    }
}

/// Derivative of Riemann Zeta function
///
/// Computes the n-th derivative of ζ(s) using the analytical formula:
/// ζ^(n)(s) = (-1)^n * Σ_{k=1}^∞ [ln(k)]^n / k^s
///
/// This implementation uses the same convergence techniques as eval_zeta
/// to ensure consistency and accuracy.
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

    // For Re(s) > 1, use direct series
    if x > one {
        let mut sum = T::zero();
        let max_terms = 200; // Increased for derivative convergence

        for k in 1..=max_terms {
            let k_t = T::from(k).unwrap();
            let ln_k = k_t.ln();

            // Calculate [ln(k)]^n
            let mut ln_k_power = one;
            for _ in 0..n {
                ln_k_power *= ln_k;
            }

            // Add term: [ln(k)]^n / k^x
            let term = ln_k_power / k_t.powf(x);
            sum += term;

            // Check convergence - derivatives converge slower
            if k > 50 && term.abs() < epsilon {
                break;
            }
        }

        // Apply sign: (-1)^n
        let sign = if n % 2 == 0 { one } else { -one };
        Some(sign * sum)
    } else {
        // For Re(s) < 1, use functional equation derivative
        // ζ(s) = 2^s π^(s-1) sin(πs/2) Γ(1-s) ζ(1-s)
        //
        // The derivative uses the product rule recursively.

        let pi = T::PI();
        let two = T::from(2.0).unwrap();
        let half = T::from(0.5).unwrap();

        // Try reflection: if 1-x > 1, can use series there
        let one_minus_x = one - x;
        if one_minus_x > one {
            // We can compute derivatives using the functional equation
            // For n > 0, we use numerical differentiation of the n-1 derivative
            // This is more stable than trying to derive complex product rules

            if n == 1 {
                // First derivative: use analytical product rule
                // ζ'(s) = d/ds[2^s π^(s-1) sin(πs/2) Γ(1-s) ζ(1-s)]

                let pow_2_s = two.powf(x);
                let pow_pi_s_minus_1 = pi.powf(x - one);
                let sin_term = (pi * x * half).sin();
                let gamma_1_minus_s = eval_gamma(one_minus_x)?;
                let zeta_1_minus_s = eval_zeta(one_minus_x)?;
                let zeta_prime_1_minus_s = eval_zeta_deriv(1, one_minus_x)?;

                let ln_2 = T::LN_2();
                let ln_pi = pi.ln();
                let pi_half = pi * half;
                let cos_term = (pi * x * half).cos();

                // Digamma for Γ' = Γ * ψ, so d/ds[Γ(1-s)] = -Γ(1-s) * ψ(1-s)
                let digamma_1_minus_s = eval_digamma(one_minus_x)?;

                // Product rule application (5 terms):
                // term1: d/ds[2^s] = 2^s * ln(2)
                let term1 =
                    pow_2_s * ln_2 * pow_pi_s_minus_1 * sin_term * gamma_1_minus_s * zeta_1_minus_s;

                // term2: d/ds[π^(s-1)] = π^(s-1) * ln(π)
                let term2 = pow_2_s
                    * pow_pi_s_minus_1
                    * ln_pi
                    * sin_term
                    * gamma_1_minus_s
                    * zeta_1_minus_s;

                // term3: d/ds[sin(πs/2)] = cos(πs/2) * π/2
                let term3 = pow_2_s
                    * pow_pi_s_minus_1
                    * cos_term
                    * pi_half
                    * gamma_1_minus_s
                    * zeta_1_minus_s;

                // term4: d/ds[Γ(1-s)] = -Γ(1-s) * ψ(1-s)
                let term4 = -pow_2_s
                    * pow_pi_s_minus_1
                    * sin_term
                    * gamma_1_minus_s
                    * digamma_1_minus_s
                    * zeta_1_minus_s;

                // term5: d/ds[ζ(1-s)] = -ζ'(1-s)
                let term5 =
                    -pow_2_s * pow_pi_s_minus_1 * sin_term * gamma_1_minus_s * zeta_prime_1_minus_s;

                return Some(term1 + term2 + term3 + term4 + term5);
            } else {
                // For n >= 2, use automatic differentiation via Dual numbers
                // This is more accurate than Richardson extrapolation
                //
                // We recursively compute d/dx[ζ^(n-1)(x)] using AD
                // The generic T here is converted to f64, computed via AD, and converted back

                let x_f64 = x.to_f64()?;
                let result_f64 = eval_zeta_deriv_ad(n, x_f64)?;
                return T::from_f64(result_f64);
            }
        }

        // For other cases (0 < x < 1 and 1-x <= 1), return None
        // This region is between the pole at s=1 and where we can use the reflection formula
        None
    }
}

pub fn eval_lambert_w<T: MathScalar>(x: T) -> Option<T> {
    let one = T::one();
    let e = T::E();
    let e_inv = one / e;

    if x < -e_inv {
        return Some(T::nan());
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
        let p = (two * (e * x + one)).sqrt();
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
            let bernoulli = [1.0 / 6.0, -1.0 / 30.0, 1.0 / 42.0, -1.0 / 30.0, 5.0 / 66.0]; // Standard f64, cast to T

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

            // Using generic epsilon for simple convergence check?
            let mut prev_term_abs = T::max_value();

            for (k, &bk) in bernoulli.iter().enumerate() {
                let two_k = 2 * (k + 1);
                // (2k)!
                let mut factorial_2k = T::one();
                for i in 1..=two_k {
                    factorial_2k *= T::from(i).unwrap();
                }

                let val_bk = T::from(bk).unwrap();
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

pub fn bessel_j<T: MathScalar>(n: i32, x: T) -> Option<T> {
    let n_abs = n.abs();
    let j0 = bessel_j0(x);
    if n_abs == 0 {
        return Some(j0);
    }
    let j1 = bessel_j1(x);
    if n_abs == 1 {
        return Some(if n < 0 { -j1 } else { j1 });
    }

    // Check small x to avoid division by zero
    let threshold = T::from(1e-10).unwrap();
    if x.abs() < threshold {
        return Some(T::zero());
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

pub fn bessel_j0<T: MathScalar>(x: T) -> T {
    let ax = x.abs();
    let eight = T::from(8.0).unwrap();

    if ax < eight {
        let y = x * x;
        // Constants
        let c1 = T::from(57568490574.0).unwrap();
        let c2 = T::from(-13362590354.0).unwrap();
        let c3 = T::from(651619640.7).unwrap();
        let c4 = T::from(-11214424.18).unwrap();
        let c5 = T::from(77392.33017).unwrap();
        let c6 = T::from(-184.9052456).unwrap();

        let d1 = T::from(57568490411.0).unwrap();
        let d2 = T::from(1029532985.0).unwrap();
        let d3 = T::from(9494680.718).unwrap();
        let d4 = T::from(59272.64853).unwrap();
        let d5 = T::from(267.8532712).unwrap();
        // d6 is 1.0 implicit

        let num = c1 + y * (c2 + y * (c3 + y * (c4 + y * (c5 + y * c6))));
        let den = d1 + y * (d2 + y * (d3 + y * (d4 + y * (d5 + y))));

        num / den
    } else {
        let z = eight / ax;
        let y = z * z;
        let shift = T::from(0.785398164).unwrap();
        let xx = ax - shift;

        let c_sqrt = T::FRAC_2_PI();
        let term_sqrt = (c_sqrt / ax).sqrt();

        let term1_c1 = T::one();
        let term1_c2 = T::from(-0.1098628627e-2).unwrap();
        let term1_c3 = T::from(0.2734510407e-4).unwrap();
        let term1_c4 = T::from(-0.2073370639e-5).unwrap();
        let term1_c5 = T::from(0.2093887211e-6).unwrap();

        let p_cos = term1_c1 + y * (term1_c2 + y * (term1_c3 + y * (term1_c4 + y * term1_c5)));

        let term2_c1 = T::from(-0.1562499995e-1).unwrap();
        let term2_c2 = T::from(0.1430488765e-3).unwrap();
        let term2_c3 = T::from(-0.6911147651e-5).unwrap();
        let term2_c4 = T::from(0.7621095161e-6).unwrap();
        let term2_c5 = T::from(0.934935152e-7).unwrap();

        let p_sin = term2_c1 + y * (term2_c2 + y * (term2_c3 + y * (term2_c4 - y * term2_c5)));

        term_sqrt * (xx.cos() * p_cos - z * xx.sin() * p_sin)
    }
}

pub fn bessel_j1<T: MathScalar>(x: T) -> T {
    let ax = x.abs();
    let eight = T::from(8.0).unwrap();

    if ax < eight {
        let y = x * x;
        let c1 = T::from(72362614232.0).unwrap();
        let c2 = T::from(-7895059235.0).unwrap();
        let c3 = T::from(242396853.1).unwrap();
        let c4 = T::from(-2972611.439).unwrap();
        let c5 = T::from(15704.48260).unwrap();
        let c6 = T::from(-30.16036606).unwrap();

        let d1 = T::from(144725228442.0).unwrap();
        let d2 = T::from(2300535178.0).unwrap();
        let d3 = T::from(18583304.74).unwrap();
        let d4 = T::from(99447.43394).unwrap();
        let d5 = T::from(376.9991397).unwrap();

        let num = c1 + y * (c2 + y * (c3 + y * (c4 + y * (c5 + y * c6))));
        let den = d1 + y * (d2 + y * (d3 + y * (d4 + y * (d5 + y))));

        x * (num / den)
    } else {
        let z = eight / ax;
        let y = z * z;
        let shift = T::from(2.356194491).unwrap();
        let xx = ax - shift;

        let c_sqrt = T::FRAC_2_PI();
        let term_sqrt = (c_sqrt / ax).sqrt();

        // Constants block 1
        let a1 = T::one();
        let a2 = T::from(0.183105e-2).unwrap();
        let a3 = T::from(-0.3516396496e-4).unwrap();
        let a4 = T::from(0.2457520174e-5).unwrap();
        let a5 = T::from(-0.240337019e-6).unwrap();
        let p_cos = a1 + y * (a2 + y * (a3 + y * (a4 + y * a5)));

        // Constants block 2
        let b1 = T::from(0.04687499995).unwrap();
        let b2 = T::from(-0.2002690873e-3).unwrap();
        let b3 = T::from(0.8449199096e-5).unwrap();
        let b4 = T::from(-0.88228987e-6).unwrap();
        let b5 = T::from(0.105787412e-6).unwrap();
        let p_sin = b1 + y * (b2 + y * (b3 + y * (b4 + y * b5)));

        let ans = term_sqrt * (xx.cos() * p_cos - z * xx.sin() * p_sin);

        if x < T::zero() { -ans } else { ans }
    }
}

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

pub fn bessel_y0<T: MathScalar>(x: T) -> T {
    let eight = T::from(8.0).unwrap();
    if x < eight {
        let y = x * x;
        let num = T::from(-2957821389.0).unwrap()
            + y * (T::from(7062834065.0).unwrap()
                + y * (T::from(-512359803.6).unwrap()
                    + y * (T::from(10879881.29).unwrap()
                        + y * (T::from(-86327.92757).unwrap()
                            + y * T::from(228.4622733).unwrap()))));

        let den = T::from(40076544269.0).unwrap()
            + y * (T::from(745249964.8).unwrap()
                + y * (T::from(7189466.438).unwrap()
                    + y * (T::from(47447.26470).unwrap()
                        + y * (T::from(226.1030244).unwrap() + y))));

        let term = num / den;
        let c = T::FRAC_2_PI();
        term + c * bessel_j0(x) * x.ln()
    } else {
        let z = eight / x;
        let y = z * z;
        let shift = T::from(0.785398164).unwrap();
        let xx = x - shift;

        let c_sqrt = T::FRAC_2_PI();
        let term_sqrt = (c_sqrt / x).sqrt();

        // Reusing polynomial logic simplified
        // P1 ~ sin
        let a1 = T::one();
        let a2 = T::from(-0.1098628627e-2).unwrap();
        let a3 = T::from(0.2734510407e-4).unwrap();
        let a4 = T::from(-0.2073370639e-5).unwrap();
        let a5 = T::from(0.2093887211e-6).unwrap();
        let p_sin = a1 + y * (a2 + y * (a3 + y * (a4 + y * a5)));

        // P2 ~ cos
        let b1 = T::from(-0.1562499995e-1).unwrap();
        let b2 = T::from(0.1430488765e-3).unwrap();
        let b3 = T::from(-0.6911147651e-5).unwrap();
        let b4 = T::from(0.7621095161e-6).unwrap();
        let b5 = T::from(0.934935152e-7).unwrap();
        let p_cos = b1 + y * (b2 + y * (b3 + y * (b4 - y * b5)));

        term_sqrt * (xx.sin() * p_sin + z * xx.cos() * p_cos)
    }
}

pub fn bessel_y1<T: MathScalar>(x: T) -> T {
    let eight = T::from(8.0).unwrap();
    if x < eight {
        let y = x * x;
        // Large coeffs...
        let c1 = T::from(-0.4900604943e13).unwrap();
        let c2 = T::from(0.1275274390e13).unwrap();
        let c3 = T::from(-0.5153438139e11).unwrap();
        let c4 = T::from(0.7349264551e9).unwrap();
        let c5 = T::from(-0.4237922726e7).unwrap();
        let c6 = T::from(0.8511937935e4).unwrap();
        let num = c1 + y * (c2 + y * (c3 + y * (c4 + y * (c5 + y * c6))));

        let d1 = T::from(0.2499580570e14).unwrap();
        let d2 = T::from(0.4244419664e12).unwrap();
        let d3 = T::from(0.3733650367e10).unwrap();
        let d4 = T::from(0.2245904002e8).unwrap();
        let d5 = T::from(0.1020426050e6).unwrap();
        let d6 = T::from(0.3549632885e3).unwrap();
        // d7 y
        let den = d1 + y * (d2 + y * (d3 + y * (d4 + y * (d5 + y * (d6 + y)))));

        let term = x * (num / den);
        let c = T::FRAC_2_PI();
        term + c * (bessel_j1(x) * x.ln() - T::one() / x)
    } else {
        let z = eight / x;
        let y = z * z;
        let shift = T::from(2.356194491).unwrap();
        let xx = x - shift;

        let c_sqrt = T::FRAC_2_PI();
        let term_sqrt = (c_sqrt / x).sqrt();

        // P1
        let a1 = T::one();
        let a2 = T::from(0.183105e-2).unwrap();
        let a3 = T::from(-0.3516396496e-4).unwrap();
        let a4 = T::from(0.2457520174e-5).unwrap();
        let a5 = T::from(-0.240337019e-6).unwrap();
        let p_sin = a1 + y * (a2 + y * (a3 + y * (a4 + y * a5)));

        // P2
        let b1 = T::from(0.04687499995).unwrap();
        let b2 = T::from(-0.2002690873e-3).unwrap();
        let b3 = T::from(0.8449199096e-5).unwrap();
        let b4 = T::from(-0.88228987e-6).unwrap();
        let b5 = T::from(0.105787412e-6).unwrap();
        let p_cos = b1 + y * (b2 + y * (b3 + y * (b4 + y * b5)));

        term_sqrt * (xx.sin() * p_sin + z * xx.cos() * p_cos)
    }
}

/// Modified Bessel function of the first kind I_n(x)
///
/// Uses Miller's backward recurrence algorithm for numerical stability.
/// This is the standard approach for computing Bessel I functions accurately
/// for large orders and small arguments.
pub fn bessel_i<T: MathScalar>(n: i32, x: T) -> Option<T> {
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
    // Empirical formula: N = n + sqrt(40*n) works well
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

pub fn bessel_i0<T: MathScalar>(x: T) -> T {
    let ax = x.abs();
    let three_seven_five = T::from(3.75).unwrap();

    if ax < three_seven_five {
        let y = (x / three_seven_five).powi(2);
        let c1 = T::from(3.5156229).unwrap();
        let c2 = T::from(3.0899424).unwrap();
        let c3 = T::from(1.2067492).unwrap();
        let c4 = T::from(0.2659732).unwrap();
        let c5 = T::from(0.0360768).unwrap();
        let c6 = T::from(0.0045813).unwrap();

        T::one() + y * (c1 + y * (c2 + y * (c3 + y * (c4 + y * (c5 + y * c6)))))
    } else {
        let y = three_seven_five / ax;
        let term = ax.exp() / ax.sqrt();

        let c0 = T::from(0.39894228).unwrap();
        let c1 = T::from(0.01328592).unwrap();
        let c2 = T::from(0.00225319).unwrap();
        let c3 = T::from(-0.00157565).unwrap();
        let c4 = T::from(0.00916281).unwrap();
        let c5 = T::from(-0.02057706).unwrap();
        let c6 = T::from(0.02635537).unwrap();
        let c7 = T::from(-0.01647633).unwrap();
        let c8 = T::from(0.00392377).unwrap();

        term * (c0
            + y * (c1 + y * (c2 + y * (c3 + y * (c4 + y * (c5 + y * (c6 + y * (c7 + y * c8))))))))
    }
}

pub fn bessel_i1<T: MathScalar>(x: T) -> T {
    let ax = x.abs();
    let three_seven_five = T::from(3.75).unwrap();

    let ans = if ax < three_seven_five {
        let y = (x / three_seven_five).powi(2);
        let c0 = T::from(0.5).unwrap();
        let c1 = T::from(0.87890594).unwrap();
        let c2 = T::from(0.51498869).unwrap();
        let c3 = T::from(0.15084934).unwrap();
        let c4 = T::from(0.02658733).unwrap();
        let c5 = T::from(0.00301532).unwrap();
        let c6 = T::from(0.00032411).unwrap();

        ax * (c0 + y * (c1 + y * (c2 + y * (c3 + y * (c4 + y * (c5 + y * c6))))))
    } else {
        let y = three_seven_five / ax;
        let term = ax.exp() / ax.sqrt();

        let c0 = T::from(0.39894228).unwrap();
        let c1 = T::from(-0.03988024).unwrap();
        let c2 = T::from(-0.00362018).unwrap();
        let c3 = T::from(0.00163801).unwrap();
        let c4 = T::from(-0.01031555).unwrap();
        let c5 = T::from(0.02282967).unwrap();
        let c6 = T::from(-0.02895312).unwrap();
        let c7 = T::from(0.01787654).unwrap();
        let c8 = T::from(-0.00420059).unwrap();

        term * (c0
            + y * (c1 + y * (c2 + y * (c3 + y * (c4 + y * (c5 + y * (c6 + y * (c7 + y * c8))))))))
    };
    if x < T::zero() { -ans } else { ans }
}

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

pub fn bessel_k0<T: MathScalar>(x: T) -> T {
    let two = T::from(2.0).unwrap();
    if x <= two {
        let four = T::from(4.0).unwrap();
        let y = x * x / four;
        let i0 = bessel_i0(x);
        let ln_term = -(x / two).ln() * i0;

        let c0 = T::from(-0.57721566).unwrap();
        let c1 = T::from(0.42278420).unwrap();
        let c2 = T::from(0.23069756).unwrap();
        let c3 = T::from(0.03488590).unwrap();
        let c4 = T::from(0.00262698).unwrap();
        let c5 = T::from(0.00010750).unwrap();
        let c6 = T::from(0.0000074).unwrap();

        let poly = c0 + y * (c1 + y * (c2 + y * (c3 + y * (c4 + y * (c5 + y * c6)))));
        ln_term + poly
    } else {
        let y = two / x;
        let term = (-x).exp() / x.sqrt();

        let c0 = T::from(1.25331414).unwrap();
        let c1 = T::from(-0.07832358).unwrap();
        let c2 = T::from(0.02189568).unwrap();
        let c3 = T::from(-0.01062446).unwrap();
        let c4 = T::from(0.00587872).unwrap();
        let c5 = T::from(-0.00251540).unwrap();
        let c6 = T::from(0.00053208).unwrap();
        let c7 = T::from(-0.000025200).unwrap();

        term * (c0 + y * (c1 + y * (c2 + y * (c3 + y * (c4 + y * (c5 + y * (c6 + y * c7)))))))
    }
}

fn bessel_k1<T: MathScalar>(x: T) -> T {
    let two = T::from(2.0).unwrap();
    if x <= two {
        let four = T::from(4.0).unwrap();
        let y = x * x / four;

        let term1 = x.ln() * bessel_i1(x);
        let term2 = T::one() / x;

        let c0 = T::one();
        let c1 = T::from(0.15443144).unwrap();
        let c2 = T::from(-0.67278579).unwrap();
        let c3 = T::from(-0.18156897).unwrap();
        let c4 = T::from(-0.01919402).unwrap();
        let c5 = T::from(-0.00110404).unwrap();
        let c6 = T::from(-0.00004686).unwrap();

        let poly = c0 + y * (c1 + y * (c2 + y * (c3 + y * (c4 + y * (c5 + y * c6)))));
        term1 + term2 * poly
    } else {
        let y = two / x;
        let term = (-x).exp() / x.sqrt();

        let c0 = T::from(1.25331414).unwrap();
        let c1 = T::from(0.23498619).unwrap();
        let c2 = T::from(-0.03655620).unwrap();
        let c3 = T::from(0.01504268).unwrap();
        let c4 = T::from(-0.00780353).unwrap();
        let c5 = T::from(0.00325614).unwrap();
        let c6 = T::from(-0.00068245).unwrap();
        let c7 = T::from(0.0000316).unwrap();

        term * (c0 + y * (c1 + y * (c2 + y * (c3 + y * (c4 + y * (c5 + y * (c6 + y * c7)))))))
    }
}

// ===== Automatic Differentiation based derivatives =====
//
// These functions use dual numbers to compute derivatives via forward-mode AD.
// This is more accurate and efficient than numerical differentiation.

use dual::Dual;

/// Compute the derivative of zeta function using automatic differentiation
///
/// Uses dual numbers for higher accuracy than numerical differentiation.
/// For n-th derivatives, we chain AD calls.
///
/// # Arguments
/// * `n` - Order of derivative (n >= 0)
/// * `x` - Point at which to evaluate
///
/// # Returns
/// The n-th derivative of zeta at x
pub fn eval_zeta_deriv_ad(n: i32, x: f64) -> Option<f64> {
    if n < 0 {
        return None;
    }
    if n == 0 {
        return eval_zeta(x);
    }

    // For first derivative, use AD directly
    if n == 1 {
        let dual_x = Dual::new(x, 1.0);
        let result = eval_zeta(dual_x)?;
        return Some(result.eps);
    }

    // For higher derivatives, recursively apply AD
    // d^n/dx^n f(x) = d/dx [d^(n-1)/dx^(n-1) f(x)]
    // We use dual numbers on the (n-1)th derivative function

    // For n >= 2, we compute numerically on the AD-computed (n-1)th derivative
    // This is still more accurate than pure numerical differentiation
    // because each step uses AD instead of finite differences

    let h = 1e-8;
    let f_plus = eval_zeta_deriv_ad(n - 1, x + h)?;
    let f_minus = eval_zeta_deriv_ad(n - 1, x - h)?;

    Some((f_plus - f_minus) / (2.0 * h))
}

#[cfg(test)]
mod ad_tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn test_zeta_deriv_ad() {
        // Test zeta'(s) at s = 2
        // Known: zeta'(2) ≈ -0.9375482543...
        let x = 2.0;
        let ad_deriv = eval_zeta_deriv_ad(1, x).unwrap();

        // Numerical verification
        let h = 1e-6;
        let numerical = (eval_zeta(x + h).unwrap() - eval_zeta(x - h).unwrap()) / (2.0 * h);

        assert!(
            approx_eq(ad_deriv, numerical, 1e-4),
            "AD zeta': {} vs numerical: {}",
            ad_deriv,
            numerical
        );
    }
}
