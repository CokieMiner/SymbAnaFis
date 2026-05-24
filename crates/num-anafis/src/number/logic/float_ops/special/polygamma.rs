//! Digamma, trigamma, tetragamma, and general polygamma functions.
//!
//! Reference: [DLMF, §5.11], [DLMF, §5.15]

use alloc::vec;
use alloc::vec::Vec;

use super::helpers::{
    is_non_pos_int, kahan_add, pi_cot_pi_x, sign_from_delta, signed_infinity, sin_pi_x,
};
use super::{SpecFloat, SpecInt};

/// Digamma ψ(x) — asymptotic expansion with recurrence.
pub fn digamma<T: SpecFloat>(x: T) -> T {
    if is_non_pos_int(x) {
        let delta = x - x.round();
        let sign = -sign_from_delta(delta);
        return signed_infinity(sign);
    }
    let half = T::half();
    let one = T::one();
    // Use the trait-defined shift (e.g., 20 for f64) to move x into the
    // asymptotic region where the Bernoulli series converges rapidly.
    let shift = T::from_usize(T::DIGAMMA_SHIFT);

    let mut xv = x;
    let mut result = T::zero();
    let mut compensation = T::zero();
    if xv >= T::zero() && xv < half {
        // Reflection formula: DLMF §5.5.4 (preferred for 0 < x < 1/2)
        return digamma(one - xv) - pi_cot_pi_x(xv);
    }

    // Recurrence shift: DLMF §5.5.2
    // For negative x, recurrence avoids reflection cancellation when ψ(1-x) ≈ π·cot(πx).
    while xv < shift {
        kahan_add(&mut result, &mut compensation, -one / xv);
        xv = xv + one;
    }
    kahan_add(&mut result, &mut compensation, xv.ln());
    kahan_add(&mut result, &mut compensation, -half / xv);
    let x2 = xv * xv;

    let mut x_pow = x2;
    let mut prev_term_abs = T::max_value();
    // Asymptotic expansion with Bernoulli: DLMF §5.11.2
    for (k, &(bn, bd)) in T::bernoulli_pairs().iter().enumerate() {
        let n = 2 * (k + 1);
        let term = (bn / bd) / (T::from_usize(n) * x_pow);
        if term.abs() > prev_term_abs {
            break;
        }
        prev_term_abs = term.abs();
        kahan_add(&mut result, &mut compensation, -term);
        x_pow = x_pow * x2;
    }

    result + compensation
}

/// Trigamma ψ₁(x).
pub fn trigamma<T: SpecFloat>(x: T) -> T {
    if is_non_pos_int(x) {
        let delta = x - x.round();
        let sign = -sign_from_delta(delta);
        return signed_infinity(sign);
    }
    let half = T::half();
    let one = T::one();
    // Reflection: ψ'(x) = π²/sin²(πx) - ψ'(1-x) for x < 0
    if x < T::zero() {
        let pi = T::pi();
        let f = x - x.round();
        let sin_val = (pi * f).sin();
        pi * pi / (sin_val * sin_val) - trigamma(one - x)
    } else if x < half {
        T::pi() * T::pi() / (sin_pi_x(x) * sin_pi_x(x)) - trigamma(one - x)
    } else {
        polygamma_asymptotic(T::Int::from_usize(1), x)
    }
}

/// Tetragamma ψ₂(x).
pub fn tetragamma<T: SpecFloat>(x: T) -> T {
    if is_non_pos_int(x) {
        let delta = x - x.round();
        let sign = sign_from_delta(delta);
        return signed_infinity(sign);
    }
    let half = T::half();
    let one = T::one();
    // Reflection: ψ''(x) = ψ''(1-x) - 2π³·cot(πx)/sin²(πx) for x < 1/2
    if x < half {
        let pi = T::pi();
        let f = x - x.round();
        let pf = pi * f;
        let sin_pf = pf.sin();
        if sin_pf.abs() < T::eps() {
            return tetragamma(one - x);
        }
        let f_half = (T::two() * f).round() / T::two();
        let cot_pf = if f_half.abs() == half {
            let delta = f - f_half;
            let arg = pi * delta;
            -arg.sin() / arg.cos()
        } else {
            pf.cos() / sin_pf
        };
        let csc_sq = one + cot_pf * cot_pf;
        let two = T::two();
        let refl = tetragamma(one - x);
        refl - two * pi * pi * pi * cot_pf * csc_sq
    } else {
        polygamma_asymptotic(T::Int::from_usize(2), x)
    }
}

/// General polygamma ψⁿ(x) for arbitrary order n.
#[allow(clippy::many_single_char_names, reason = "Standard math notation")]
pub fn polygamma_n<T: SpecFloat, I: SpecInt>(n: I, x: T) -> T {
    if n.is_negative() {
        return T::nan();
    }
    if n.is_zero() {
        digamma(x)
    } else {
        polygamma_asymptotic(n, x)
    }
}

#[allow(clippy::many_single_char_names, reason = "Standard math notation")]
#[allow(
    clippy::too_many_lines,
    reason = "mathematical expansion requires many lines"
)]
fn polygamma_asymptotic<T: SpecFloat, I: SpecInt>(n: I, x: T) -> T {
    if is_non_pos_int(x) {
        let delta = x - x.round();
        let sign = if (n % I::from_usize(2)).is_zero() {
            -sign_from_delta(delta)
        } else {
            T::one()
        };
        return signed_infinity(sign);
    }
    let half = T::half();
    let one = T::one();
    if x < half {
        let n_usize = n.to_usize();
        let reflected = polygamma_asymptotic(n, one - x);
        let reflected_sign = if n_usize.is_multiple_of(2) {
            one
        } else {
            T::neg_one()
        };
        let pi = T::pi();
        let reduced = x - x.round();
        let arg = pi * reduced;
        let cot = arg.cos() / arg.sin();
        let poly = cot_derivative_poly(n_usize, cot);
        let mut pi_pow = pi;
        for _ in 0..n_usize {
            pi_pow = pi_pow * pi;
        }
        return reflected_sign * reflected - pi_pow * poly;
    }
    let mut xv = x;
    let mut recurrence_sum = T::zero();
    let n_plus_one = n + I::one();
    let sign = if (n_plus_one % I::from_usize(2)).is_zero() {
        T::one()
    } else {
        T::neg_one()
    };
    let two = I::from_usize(2);

    let mut factorial = T::one();
    let mut i = I::one();
    while i <= n {
        factorial = factorial * T::from_int(i);
        i = i + I::one();
    }

    // Minimum shift to reach asymptotic region; needs to grow with n
    // to ensure convergence of the Bernoulli expansion.
    // The empirical heuristic `max(60, 6n + 30)` ensures the truncation error
    // of the asymptotic series falls below machine precision for f64.
    let n_usize = n.to_usize();
    let min_shift = 60_usize.max(6 * n_usize + 30);
    let shift = T::from_usize(min_shift);

    let mut compensation = T::zero();
    while xv < shift {
        kahan_add(
            &mut recurrence_sum,
            &mut compensation,
            sign * factorial / xv.pow_int(n_plus_one),
        );
        xv = xv + one;
    }

    let asym_sign = if (n % two).is_zero() {
        T::neg_one()
    } else {
        T::one()
    };

    let n_minus_1_fact = if n > I::one() {
        factorial / T::from_int(n)
    } else {
        T::one()
    };

    let mut sum = n_minus_1_fact / xv.pow_int(n);
    let mut sum_comp = T::zero();
    let two_t = T::two();
    kahan_add(
        &mut sum,
        &mut sum_comp,
        factorial / (two_t * xv.pow_int(n_plus_one)),
    );

    let mut x_pow = xv.pow_int(n + two);
    let mut fact_ratio = factorial * T::from_int(n_plus_one);
    let mut prev_term_abs = T::max_value();
    let mut term_count = 0;

    let pairs = T::bernoulli_pairs();

    let mut factorial_2k = T::one();
    let mut last_2k = 0;
    for (k, pair) in pairs.iter().enumerate() {
        let &(b_num, b_den) = pair;
        let two_k = 2 * (k + 1);
        for idx in (last_2k + 1)..=two_k {
            factorial_2k = factorial_2k * T::from_usize(idx);
        }
        last_2k = two_k;
        let val_bk = b_num / b_den;
        let term = val_bk * fact_ratio / (factorial_2k * x_pow);

        term_count += 1;
        if term_count > 3 && term.abs() > prev_term_abs {
            break;
        }
        prev_term_abs = term.abs();
        kahan_add(&mut sum, &mut sum_comp, term);

        x_pow = x_pow * xv * xv;
        let poch_factor1 = T::from_int(n + I::from_usize(two_k));
        let poch_factor2 = T::from_int(n + I::from_usize(two_k + 1));
        fact_ratio = fact_ratio * poch_factor1 * poch_factor2;
    }

    recurrence_sum + asym_sign * (sum + sum_comp)
}

/// Evaluates `dⁿ/dxⁿ cot(x)` as a polynomial in `cot(x)`.
///
/// Uses the recurrence: if `P_n(cot) = dⁿ/dxⁿ cot(x)` then
/// `P_{n+1}(c) = −(c²+1)·P'_n(c)`. The coefficients are built by repeated
/// differentiation: `P_0 = c`, and at each step the derivative of
/// `c^k` gives `k·c^{k−1}·(−c²−1) = −k·c^{k+1} − k·c^{k−1}`.
fn cot_derivative_poly<T: SpecFloat>(n: usize, cot: T) -> T {
    let mut coeffs: Vec<T> = vec![T::zero(); n + 2];
    if let Some(slot) = coeffs.get_mut(1) {
        *slot = T::one();
    }

    for degree in 0..n {
        let mut next: Vec<T> = vec![T::zero(); n + 2];
        for power in 1..=(degree + 1) {
            let coeff = coeffs.get(power).copied().unwrap_or_else(T::zero);
            let deriv = coeff * T::from_usize(power);
            add_coeff(&mut next, power - 1, -deriv);
            add_coeff(&mut next, power + 1, -deriv);
        }
        coeffs = next;
    }

    let mut value = T::zero();
    for coeff in coeffs.iter().rev() {
        value = value * cot + *coeff;
    }
    value
}

/// Adds `delta` to `coeffs[index]` if the index is in bounds.
fn add_coeff<T: SpecFloat>(coeffs: &mut [T], index: usize, delta: T) {
    if let Some(slot) = coeffs.get_mut(index) {
        *slot = *slot + delta;
    }
}
