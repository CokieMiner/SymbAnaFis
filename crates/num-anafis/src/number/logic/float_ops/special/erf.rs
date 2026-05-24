//! Error function erf(x) and complementary error function erfc(x).
//!
//! Reference: [DLMF, §7.6.1], [DLMF, §7.9]

use super::SpecFloat;

/// erf(x) = (2/√π) ∫₀ˣ e^(-t²) dt — Taylor series with Kahan summation.
#[allow(clippy::many_single_char_names, reason = "Standard math notation")]
pub fn erf<T: SpecFloat>(x: T) -> T {
    if x.is_nan() {
        return T::nan();
    }
    let sign = x.signum();
    let abs_x = x.abs();
    let one_point_five = T::from_usize(3) / T::two();

    if abs_x > one_point_five {
        return sign * (T::one() - erfc(abs_x));
    }

    let sqrt_pi = T::pi().sqrt();
    let coeff = T::two() / sqrt_pi;

    let mut sum = T::zero();
    let mut compensation = T::zero();
    let mut factorial = T::one();
    let mut power = abs_x;

    for n in 0..T::ERF_TERMS {
        let two_n_plus_one = T::from_usize(2 * n + 1);
        let term = power / (factorial * two_n_plus_one);

        if term.is_nan() || term.is_infinite() {
            break;
        }

        let signed_term = if n % 2 == 0 { term } else { -term };
        let y = signed_term - compensation;
        let t = sum + y;
        compensation = (t - sum) - y;
        sum = t;

        factorial = factorial * T::from_usize(n + 1);
        power = power * abs_x * abs_x;

        if sum.abs() > T::zero() && term.abs() < sum.abs() * T::eps() {
            break;
        }
    }
    sign * coeff * sum
}

/// erfc(x) = 1 - erf(x) — continued fraction for large x.
#[allow(clippy::many_single_char_names, reason = "Standard math notation")]
pub fn erfc<T: SpecFloat>(x: T) -> T {
    if x.is_nan() {
        return x;
    }
    if x.is_infinite() {
        return if x.is_sign_positive() {
            T::zero()
        } else {
            T::two()
        };
    }

    let abs_x = x.abs();
    // 28 is the threshold where erfc(28) underflows f64::MIN_POSITIVE,
    // since exp(-28²) = exp(-784) ≈ 1e-340.
    let large_x = T::from_usize(28);

    if x > large_x {
        return T::zero();
    }
    if x < -large_x {
        return T::two();
    }
    let one_point_five = T::from_usize(3) / T::two(); // 1.5
    if abs_x < one_point_five {
        return T::one() - erf(x);
    }
    if x < T::zero() {
        return T::two() - erfc(abs_x);
    }

    // Modified Lentz continued fraction: Lentz (1976)
    let one = T::one();
    let tiny = T::eps();
    let eps = T::eps();

    let mut f = abs_x;
    if f < tiny {
        f = tiny;
    }
    let mut c = f;
    let mut d = T::zero();

    let tolerance = eps / T::from_usize(100);

    for j in 1_usize..200 {
        // a_j = j/2 from DLMF §7.9.3
        let a = T::from_usize(j) * T::half();
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
        f = f * delta;
        if (delta - one).abs() < tolerance {
            break;
        }
    }
    let coeff = (-abs_x * abs_x).exp() / T::pi().sqrt();
    coeff / f
}
