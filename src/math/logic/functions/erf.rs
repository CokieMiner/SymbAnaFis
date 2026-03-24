use crate::core::traits::MathScalar;

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
    let two = T::from_f64(2.0).expect("Failed to convert mathematical constant 2.0");
    let coeff = two / sqrt_pi;

    let mut sum = T::zero();
    let mut compensation = T::zero(); // Kahan summation
    let mut factorial = T::one();
    let mut power = x;

    for n in 0..30 {
        let two_n_plus_one =
            T::from_usize(2 * n + 1).expect("Failed to convert small integer to float");

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

        let n_plus_one = T::from_usize(n + 1).expect("Failed to convert loop counter to float");
        factorial *= n_plus_one;
        power *= x * x;

        // Check convergence using machine epsilon
        if term.abs() < T::epsilon() {
            break;
        }
    }
    sign * coeff * sum
}

/// Complementary error function erfc(x) = 1 - erf(x)
///
/// Uses 1 - erf(x) for small x (x < 1.5) and a continued fraction for large x
/// to maintain ~15 digits of relative precision and avoid catastrophic cancellation.
///
/// Reference: DLMF §7.9 <https://dlmf.nist.gov/7.9>
/// Reference: Abramowitz & Stegun §7.1.14
#[allow(
    clippy::many_single_char_names,
    reason = "Standard notation for continued fraction implementation"
)]
pub fn eval_erfc<T: MathScalar>(x: T) -> T {
    if x.is_nan() {
        return x;
    }
    if x.is_infinite() {
        return if x.is_sign_positive() {
            T::zero()
        } else {
            T::from_f64(2.0).expect("Failed to convert 2.0 to T")
        };
    }

    let abs_x = x.abs();

    // Early exit for underflow: erfc(28) < 1e-340, which underflows f64
    if x > T::from_f64(28.0).expect("Failed to convert 28.0 to T") {
        return T::zero();
    }

    if abs_x < T::from_f64(1.5).expect("Failed to convert 1.5 to T") {
        return T::one() - eval_erf(x);
    }
    if x < T::zero() {
        return T::from_f64(2.0).expect("Failed to convert 2.0 to T") - eval_erfc(abs_x);
    }

    // Continued fraction for x >= 1.5:
    // erfc(x) = (e^-x^2 / sqrt(pi)) * [1 / (x + (1/2)/(x + (1)/(x + (3/2)/(x + ...))))]
    // Uses Modified Lentz's Method for stable evaluation.
    let pi = T::PI();
    let one = T::one();
    let tiny = T::from_f64(1e-30).expect("Failed to convert 1e-30 to T");
    let eps = T::epsilon();

    let mut f = abs_x;
    if f.abs() < tiny {
        f = tiny;
    }
    let mut c = f;
    let mut d = T::zero();

    for j in 1..200 {
        let a = T::from_f64(0.5 * f64::from(j)).expect("Failed to convert coefficient");
        // b_j is always abs_x in this form
        d = abs_x + a * d;
        if d.abs() < tiny {
            d = tiny;
        }
        d = one / d;
        c = abs_x + a / c;
        if c.abs() < tiny {
            c = tiny;
        }
        let delta = c * d;
        f *= delta;
        if (delta - one).abs() < eps {
            break;
        }
    }

    let coeff = (-abs_x * abs_x).exp() / pi.sqrt();
    coeff / f
}
