use crate::core::traits::MathScalar;

/// Lambert W function: W(x) is the solution to W·e^W = x
///
/// Uses Halley's iteration with carefully chosen initial approximations:
/// - For x near -1/e: series expansion
/// - For x > 0: asymptotic ln(x) - ln(ln(x)) approximation
///
/// Reference: Corless et al. (1996) "On the Lambert W Function"
/// Advances in Computational Mathematics, Vol. 5, pp. 329-359
/// See also: DLMF §4.13 <https://dlmf.nist.gov/4.13>
#[allow(clippy::many_single_char_names, reason = "Mathematical variables")]
pub fn eval_lambert_w<T: MathScalar>(x: T) -> T {
    let one = T::one();
    let e = T::E();
    let e_inv = one / e;

    if x < -e_inv {
        return T::nan(); // Domain error: W(x) undefined for x < -1/e
    }
    if x == T::zero() {
        return T::zero();
    }
    let threshold = T::from_f64(1e-12).expect("Failed to convert threshold 1e-12");
    if (x + e_inv).abs() < threshold {
        return -one;
    }

    // Initial guess
    let point_three_neg = T::from_f64(-0.3).expect("Failed to convert constant -0.3");
    let mut w = if x < point_three_neg {
        let two = T::from(2.0).expect("Failed to convert mathematical constant");
        // Fix: clamp to 0 to avoid NaN from floating point noise when x is close to -1/e
        let arg = (two * (e * x + one)).max(T::zero());
        let p = arg.sqrt();
        // -1 + p - p^2/3 + 11/72 p^3
        let third = T::from(3.0).expect("Failed to convert mathematical constant");
        let c1 = T::from(11.0 / 72.0).expect("Failed to convert mathematical constant");
        -one + p - p * p / third + c1 * p * p * p
    } else if x < T::zero() {
        let two = T::from(2.0).expect("Failed to convert mathematical constant");
        let p = (two * (e * x + one)).sqrt();
        -one + p
    } else if x < one {
        // x * (1 - x * (1 - x * 1.5))
        let one_point_five = T::from(1.5).expect("Failed to convert mathematical constant");
        x * (one - x * (one - x * one_point_five))
    } else if x < T::from_f64(3.0).expect("Failed to convert constant 3.0") {
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

    let tolerance = T::from_f64(1e-15).expect("Failed to convert tolerance 1e-15");
    let neg_one = -one;
    let two = T::from_f64(2.0).expect("Failed to convert constant 2.0");
    let half = T::from_f64(0.5).expect("Failed to convert constant 0.5");

    for _ in 0..50 {
        if w <= neg_one {
            w = T::from_f64(-0.99).expect("Failed to convert constant -0.99");
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
    w
}
