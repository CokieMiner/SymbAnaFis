//! Derivative of the Riemann zeta function ζ^(n)(s).
//!
//! Strategies:
//! 1. Laurent expansion near s=1 (Stieltjes constants)
//! 2. Euler-Maclaurin summation evaluated as a Taylor series in `s`
//! 3. Cauchy integral formula for `s < 0`
//!
//! Reference: [DLMF, §25.2], [DLMF, §25.4]

use alloc::vec;
use alloc::vec::Vec;

use super::complex::{C, cadd, cdiv, cexp, cgamma, cmul, csin};
use super::helpers::{kahan_add, sign_from_delta, signed_infinity};
use super::zeta::zeta;
use super::{SpecFloat, SpecInt};

/// ζ^(n)(s) — n-th derivative of the Riemann zeta function.
pub fn zeta_deriv<T: SpecFloat, I: SpecInt>(n: I, x: T) -> T {
    if n.is_negative() {
        return T::nan();
    }
    if n.is_zero() {
        return zeta(x);
    }

    let one = T::one();
    let n_usize = n.to_usize();

    let delta = x - one;
    if delta == T::zero() {
        let sign = if (n % I::from_usize(2)).is_zero() {
            sign_from_delta(delta)
        } else {
            -T::one()
        };
        return signed_infinity(sign);
    }

    // Laurent expansion near s=1 using Stieltjes constants
    let stieltjes = T::stieltjes_coeffs();
    // Radius 1/(2+n) shrinks with derivative order because the factorial
    // prefactor n!/(s−1)^{n+1} amplifies the truncation error.
    // The Stieltjes series length ≲ 15 terms; for n ≤ 8 the radius is
    // ≥ 0.1 and the series converges within f64 ε at that distance.
    // A small margin is added so boundary cases (e.g. n=2, δ=0.25) are
    // not rejected by floating-point rounding.
    let laurent_radius = T::one() / T::from_usize(2 + n_usize) + T::eps();
    if delta.abs() <= laurent_radius {
        let n_fact = factorial::<T>(n_usize);
        let pole_sign = if n_usize.is_multiple_of(2) { one } else { -one };
        let pole_term = pole_sign * n_fact / delta.powf(T::from_usize(n_usize + 1));

        let mut series_sum = T::zero();
        if n_usize < stieltjes.len() {
            for k in 0..(stieltjes.len() - n_usize) {
                let sign = if (n_usize + k).is_multiple_of(2) {
                    one
                } else {
                    -one
                };
                let Some(&st) = stieltjes.get(n_usize + k) else {
                    break;
                };
                let term = sign * st * delta.powf(T::from_usize(k)) / factorial::<T>(k);
                if term.abs() < series_sum.abs() * T::eps() {
                    break;
                }
                series_sum = series_sum + term;
            }
        }
        return pole_term + series_sum;
    }

    if x < T::zero() {
        via_cauchy(n, x)
    } else {
        direct(n, x)
    }
}

// =========================================================================
// Euler-Maclaurin direct summation for s > 0
// =========================================================================

/// Direct evaluation of `ζ^(n)(s)` for `s > 0` via Taylor expansion.
///
/// For `s < 1` the Borwein series is used (faster than EM near the critical
/// strip); for `s ≥ 1` the Euler–Maclaurin series is used.
fn direct<T: SpecFloat, I: SpecInt>(n: I, s: T) -> T {
    let n_usize = n.to_usize();
    if s < T::one() {
        return borwein_taylor_derivative(n_usize, s);
    }
    let coeffs = euler_maclaurin_taylor(n_usize, s);
    coeffs.get(n_usize).copied().unwrap_or_else(T::nan) * factorial::<T>(n_usize)
}

fn borwein_taylor_derivative<T: SpecFloat>(order: usize, s: T) -> T {
    let coeffs = borwein_taylor(order, s);
    coeffs.get(order).copied().unwrap_or_else(T::nan) * factorial::<T>(order)
}

/// Borwein series expanded as Taylor coefficients in `δ` around `s`.
///
/// Returns `[c₀, c₁, …, c_ord]` where
/// `ζ(s+δ) ≈ Σ c_j·δ^j`.
///
/// The Borwein coefficients `d_k` are computed once, then each `1/k^{s+δ}`
/// is Taylor-expanded via `k^{−δ} = exp(−δ·ln k)`. The result is divided
/// by the denominator series `1 − 2^{1−s−δ}` via series reciprocal.
fn borwein_taylor<T: SpecFloat>(order: usize, s: T) -> Vec<T> {
    let one = T::one();
    let two = T::two();
    let four = T::from_usize(4);
    let n = T::ZETA_BORWEIN_N;
    let n_t = T::from_usize(n);

    let mut d_coeffs = vec![T::zero(); n + 1];
    let mut term = one / n_t;
    let mut current_inner_sum = term;
    if let Some(first) = d_coeffs.get_mut(0) {
        *first = n_t * current_inner_sum;
    }

    for k in 1..=n {
        let i = T::from_usize(k - 1);
        let two_i_plus_1 = T::from_usize(2 * k - 1);
        let two_i_plus_2 = T::from_usize(2 * k);
        term = term * four * (n_t + i) * (n_t - i) / (two_i_plus_1 * two_i_plus_2);
        current_inner_sum = current_inner_sum + term;
        if let Some(d_coeff) = d_coeffs.get_mut(k) {
            *d_coeff = n_t * current_inner_sum;
        }
    }

    let d_n = d_coeffs.get(n).copied().unwrap_or_else(T::zero);
    let mut numerator = vec![T::zero(); order + 1];
    let mut num_comp = vec![T::zero(); order + 1];
    for k in 0..n {
        let k_plus_1 = T::from_usize(k + 1);
        let sign = if k % 2 == 0 { one } else { -one };
        let coeff = sign * (d_coeffs.get(k).copied().unwrap_or_else(T::zero) - d_n);
        let ln_k = k_plus_1.ln();
        let mut series_term = coeff / k_plus_1.powf(s);
        kahan_add_coeff(&mut numerator, &mut num_comp, 0, series_term);
        for degree in 1..=order {
            series_term = series_term * (-ln_k) / T::from_usize(degree);
            kahan_add_coeff(&mut numerator, &mut num_comp, degree, series_term);
        }
    }

    let ln2 = two.ln();
    let base = two.powf(one - s);
    let mut denom = vec![T::zero(); order + 1];
    add_coeff(&mut denom, 0, one - base);
    let mut exp_term = base;
    for degree in 1..=order {
        exp_term = exp_term * (-ln2) / T::from_usize(degree);
        add_coeff(&mut denom, degree, -exp_term);
    }

    let reciprocal = reciprocal_series(&denom, order);
    let product = mul_series(&numerator, &reciprocal, order);
    product.into_iter().map(|value| -value / d_n).collect()
}

fn euler_maclaurin_taylor<T: SpecFloat>(order: usize, s: T) -> Vec<T> {
    let one = T::one();
    // More terms are needed when s < 0.5 because 1/kˢ decays slowly.
    // For s ≥ 0.5: 160 + 24·order terms keeps the Taylor series tail
    //   (∼ Σ (ln k / kˢ)ʲ / j! · δˢ^j) below f64 ε.
    // For s < 0.5: 512 + 64·order compensates for the slower decay,
    //   adding ∼256 terms per unit of s-dependent slowdown.
    let n_terms = if s < T::half() {
        512 + 64 * order
    } else {
        160 + 24 * order
    };
    let mut coeffs: Vec<T> = vec![T::zero(); order + 1];
    let mut comp: Vec<T> = vec![T::zero(); order + 1];

    for k in 1..n_terms {
        let k_f = T::from_usize(k);
        let ln_k = k_f.ln();
        let mut term = one / k_f.powf(s);
        kahan_add_coeff(&mut coeffs, &mut comp, 0, term);
        for j in 1..=order {
            term = term * (-ln_k) / T::from_usize(j);
            kahan_add_coeff(&mut coeffs, &mut comp, j, term);
        }
    }

    let n_val = T::from_usize(n_terms);
    let ln_n = n_val.ln();

    // Integral tail: N^(1-s-h)/(s+h-1).
    let integral_exp = exp_linear_series(n_val.powf(one - s), -ln_n, order);
    let integral_recip = reciprocal_linear_series(s - one, order);
    kahan_add_series_scaled(
        &mut coeffs,
        &mut comp,
        &mul_series(&integral_exp, &integral_recip, order),
        one,
    );

    // Endpoint correction: 1/2 N^(-s-h).
    let boundary = exp_linear_series(T::half() / n_val.powf(s), -ln_n, order);
    kahan_add_series_scaled(&mut coeffs, &mut comp, &boundary, one);

    for (idx, &(bn, bd)) in T::bernoulli_pairs().iter().enumerate() {
        let r = idx + 1;
        let deriv_order = 2 * r - 1;
        let scale = (bn / bd) / factorial::<T>(2 * r);
        let rising = rising_factorial_series(s, deriv_order, order);
        let pow = exp_linear_series(
            one / n_val.powf(s + T::from_usize(deriv_order)),
            -ln_n,
            order,
        );
        let correction = mul_series(&rising, &pow, order);
        kahan_add_series_scaled(&mut coeffs, &mut comp, &correction, scale);
    }

    coeffs
}

// =========================================================================
// Cauchy integral for s < 0
// =========================================================================

fn via_cauchy<T: SpecFloat, I: SpecInt>(n: I, s: T) -> T {
    let n_usize = n.to_usize();
    let two = T::two();
    let pi = T::pi();

    if n_usize == 1 && s == T::zero() {
        return -T::half() * (two * pi).ln();
    }

    cauchy_integral(n_usize, s)
}

/// Cauchy integral formula for ζ^(n)(s).
///
/// `f^(n)(s) = n!/(2π) ∫ f(s + r e^(iθ)) e^(-inθ) dθ / r^n`
#[allow(
    clippy::many_single_char_names,
    reason = "Mathematical formula uses standard notation (n, s, r, m, z)"
)]
fn cauchy_integral<T: SpecFloat>(n: usize, s: T) -> T {
    let pi = T::pi();
    let two = T::two();
    let zero = T::zero();

    let dist_to_pole = (s - T::one()).abs();
    let n_f = T::from_usize(n);
    // Optimal radius that balances the factorial prefactor n!/R^n against
    // the growth of ζ(z) near the pole.  Using R ≈ n/ln(|s|+n) minimises
    // the amplification factor, recovering ~7 bits of effective precision
    // in f64 compared with the naive √n heuristic.
    let optimal_r = n_f / (s.abs() + n_f).ln();
    let target_radius = if optimal_r > T::one() {
        optimal_r
    } else {
        T::one()
    };
    // Cap the radius at 90% of the distance to the pole s=1 to avoid the singularity.
    let cap = dist_to_pole * T::from_usize(9) / T::from_usize(10);
    let radius = if target_radius < cap {
        target_radius
    } else {
        cap
    };

    // Evaluate the contour integral using the trapezoidal rule, which exhibits
    // exponential convergence for periodic analytic functions.
    // 10000 points: the trapezoidal error on a circle of radius R with analytic
    // integrand decays like exp(−2πm·R) for functions analytic in a strip.
    // With R ∼ O(1) and m = 10⁴, the quadrature error is ∼exp(−6·10⁴), far
    // below f64 ε. The dominant error source is the evaluation of ζ(z) itself.
    let m = 10000;
    let m_f = T::from_usize(m);

    let mut sum_re = zero;
    let mut comp_re = zero;
    let mut sum_im = zero;
    let mut comp_im = zero;

    for j in 0..m {
        let theta = two * pi * T::from_usize(j) / m_f;
        let z = (s + radius * theta.cos(), radius * theta.sin());
        let fz = complex_zeta(z);

        let n_theta = T::from_usize(n) * theta;
        let prod = cmul(fz, (n_theta.cos(), -n_theta.sin()));
        kahan_add(&mut sum_re, &mut comp_re, prod.0);
        kahan_add(&mut sum_im, &mut comp_im, prod.1);
    }

    factorial::<T>(n) * sum_re / (m_f * radius.powf(T::from_usize(n)))
}

/// Complex Riemann ζ(z).
fn complex_zeta<T: SpecFloat>(z: C<T>) -> C<T> {
    let one = T::one();
    let zero = T::zero();

    if z.1.abs() < T::eps() {
        return (zeta(z.0), zero);
    }

    let w = (z.0 - one, z.1);
    let tenth = T::from_usize(1) / T::from_usize(10);
    if cabs_sq(w) < tenth * tenth {
        let stieltjes = T::stieltjes_coeffs();
        let mut sum = (zero, zero);
        let mut fact = T::one();
        let mut w_pow = (one, zero);
        for (k, &gamma) in stieltjes.iter().enumerate() {
            let sign = if k % 2 == 0 { one } else { -one };
            let term_coeff = sign * gamma / fact;
            sum = cadd(sum, (term_coeff * w_pow.0, term_coeff * w_pow.1));
            fact = fact * T::from_usize(k + 1);
            w_pow = cmul(w_pow, w);
        }
        return cadd(cdiv((one, zero), w), sum);
    }

    if z.0 > one {
        return complex_zeta_em(z);
    }
    if z.0 > zero {
        return complex_zeta_borwein(z);
    }

    let pi = T::pi();
    let two = T::two();
    let z_len_sq = cabs_sq(z);
    let threshold = T::from_usize(1) / T::from_usize(10000);
    if z_len_sq < threshold * threshold {
        let ln2 = two.ln();
        let ln_pi = pi.ln();
        let two_to_z = cexp((z.0 * ln2, z.1 * ln2));
        let pi_to_zm1 = cexp(((z.0 - one) * ln_pi, z.1 * ln_pi));
        let one_minus_z = (one - z.0, -z.1);
        let d_term = cgamma(one_minus_z);

        let pi_half = pi * T::half();
        let stieltjes = T::stieltjes_coeffs();
        let gamma0 = stieltjes.first().copied().unwrap_or_else(T::zero);
        let gamma1 = stieltjes.get(1).copied().unwrap_or_else(T::zero);
        let gamma2 = stieltjes.get(2).copied().unwrap_or_else(T::zero);

        let c0 = -pi_half;
        let c1 = pi_half * gamma0;
        let pi_cubed_over_48 = pi * pi * pi / T::from_usize(48);
        let c2 = pi_half * gamma1 + pi_cubed_over_48;
        let c3 = pi_half * gamma2 / T::two() - pi_cubed_over_48 * gamma0;

        let p1 = (c2 + c3 * z.0, c3 * z.1);
        let p2 = cmul(p1, z);
        let p3 = (c1 + p2.0, p2.1);
        let h = cmul(p3, z);
        return cmul(cmul(two_to_z, pi_to_zm1), cmul(d_term, (c0 + h.0, h.1)));
    }

    let ln2 = two.ln();
    let ln_pi = pi.ln();
    let two_to_z = cexp((z.0 * ln2, z.1 * ln2));
    let pi_to_zm1 = cexp(((z.0 - one) * ln_pi, z.1 * ln_pi));
    let c_term = csin((pi * z.0 / two, pi * z.1 / two));
    let d_term = cgamma((one - z.0, -z.1));
    let zeta_1mz = complex_zeta((one - z.0, -z.1));

    cmul(
        cmul(cmul(two_to_z, pi_to_zm1), c_term),
        cmul(d_term, zeta_1mz),
    )
}

/// Euler–Maclaurin zeta for `Re(z) > 1` — complex extension.
///
/// Same algorithm as `euler_maclaurin` but in complex arithmetic.
/// The initial sum uses `ZETA_EM_TERMS` terms, then the integral tail
/// and Bernoulli corrections are applied in complex form.
fn complex_zeta_em<T: SpecFloat>(z: C<T>) -> C<T> {
    let one = T::one();
    let zero = T::zero();
    let half = T::half();

    let n_terms = T::ZETA_EM_TERMS;
    let n_t = T::from_usize(n_terms);
    let mut sum = (one, zero);
    for k in 2..n_terms {
        let k_f = T::from_usize(k);
        let ln_k = k_f.ln();
        sum = cadd(sum, cexp((-z.0 * ln_k, -z.1 * ln_k)));
    }

    let ln_n = n_t.ln();
    let n_to_1mz = cexp(((one - z.0) * ln_n, -z.1 * ln_n));
    let em_integral = cdiv(n_to_1mz, (z.0 - one, z.1));
    let n_to_neg_z = cexp((-z.0 * ln_n, -z.1 * ln_n));
    let em_boundary = (half * n_to_neg_z.0, half * n_to_neg_z.1);

    let mut em_correction = (zero, zero);
    let mut factorial_2k = T::from_usize(2);
    let mut rising_power_prod = z;
    let n_sq = n_t * n_t;
    let mut n_pow = cmul(cexp((z.0 * ln_n, z.1 * ln_n)), (n_t, zero));

    for (k, &(bn, bd)) in T::bernoulli_pairs().iter().enumerate() {
        let num = (
            (bn / bd) * rising_power_prod.0,
            (bn / bd) * rising_power_prod.1,
        );
        let term = cdiv(num, cmul((factorial_2k, zero), n_pow));
        if cabs_sq(term) < T::eps() * T::eps() * cabs_sq(sum) {
            break;
        }
        em_correction = cadd(em_correction, term);

        let j = k + 1;
        factorial_2k = factorial_2k * T::from_usize(2 * j + 1) * T::from_usize(2 * j + 2);
        rising_power_prod = cmul(rising_power_prod, (z.0 + T::from_usize(2 * j - 1), z.1));
        rising_power_prod = cmul(rising_power_prod, (z.0 + T::from_usize(2 * j), z.1));
        n_pow = cmul(n_pow, (n_sq, zero));
    }

    cadd(cadd(cadd(sum, em_integral), em_boundary), em_correction)
}

/// Borwein alternating series for `0 < Re(z) ≤ 1` — complex extension.
///
/// Same algorithm as `borwein` in `zeta.rs` but with complex `k^{−z}` terms.
fn complex_zeta_borwein<T: SpecFloat>(z: C<T>) -> C<T> {
    let zero = T::zero();
    let one = T::one();
    let two = T::two();
    let four = T::from_usize(4);
    let n = T::ZETA_BORWEIN_N;

    let mut d_coeffs = vec![T::zero(); n + 1];
    let n_t = T::from_usize(n);
    let mut term = one / n_t;
    let mut current_inner_sum = term;
    if let Some(first) = d_coeffs.get_mut(0) {
        *first = n_t * current_inner_sum;
    }

    for k in 1..=n {
        let i = T::from_usize(k - 1);
        let two_i_plus_1 = T::from_usize(2 * k - 1);
        let two_i_plus_2 = T::from_usize(2 * k);
        term = term * four * (n_t + i) * (n_t - i) / (two_i_plus_1 * two_i_plus_2);
        current_inner_sum = current_inner_sum + term;
        if let Some(d_coeff) = d_coeffs.get_mut(k) {
            *d_coeff = n_t * current_inner_sum;
        }
    }

    let d_n = d_coeffs.get(n).copied().unwrap_or_else(T::zero);
    let mut sum = (zero, zero);
    for k in 0..n {
        let k_plus_1 = T::from_usize(k + 1);
        let sign = if k % 2 == 0 { one } else { -one };
        let coeff = sign * (d_coeffs.get(k).copied().unwrap_or_else(T::zero) - d_n);
        let ln_k = k_plus_1.ln();
        let term_base = cexp((-z.0 * ln_k, -z.1 * ln_k));
        sum = cadd(sum, (coeff * term_base.0, coeff * term_base.1));
    }

    let ln2 = two.ln();
    let two_to_1mz = cexp(((one - z.0) * ln2, z.1 * -ln2));
    let bracket = (one - two_to_1mz.0, -two_to_1mz.1);
    let denom = (d_n * bracket.0, d_n * bracket.1);
    if cabs_sq(denom) < T::eps() * T::eps() {
        return (zeta(z.0), zero);
    }
    let result = cdiv(sum, denom);
    (-result.0, -result.1)
}

// =========================================================================
// Internal helpers
// =========================================================================

fn add_coeff<T: SpecFloat>(coeffs: &mut [T], index: usize, term: T) {
    if let Some(slot) = coeffs.get_mut(index) {
        *slot = *slot + term;
    }
}

fn kahan_add_coeff<T: SpecFloat>(coeffs: &mut [T], comp: &mut [T], index: usize, term: T) {
    if let (Some(slot), Some(c)) = (coeffs.get_mut(index), comp.get_mut(index)) {
        kahan_add(slot, c, term);
    }
}

fn kahan_add_series_scaled<T: SpecFloat>(target: &mut [T], comp: &mut [T], series: &[T], scale: T) {
    for (index, coeff) in series.iter().enumerate() {
        kahan_add_coeff(target, comp, index, scale * *coeff);
    }
}

fn exp_linear_series<T: SpecFloat>(base: T, slope: T, order: usize) -> Vec<T> {
    let mut coeffs: Vec<T> = vec![T::zero(); order + 1];
    let mut term = base;
    add_coeff(&mut coeffs, 0, term);
    for j in 1..=order {
        term = term * slope / T::from_usize(j);
        add_coeff(&mut coeffs, j, term);
    }
    coeffs
}

fn reciprocal_linear_series<T: SpecFloat>(delta: T, order: usize) -> Vec<T> {
    let mut coeffs: Vec<T> = vec![T::zero(); order + 1];
    let mut term = T::one() / delta;
    add_coeff(&mut coeffs, 0, term);
    let ratio = -term;
    for j in 1..=order {
        term = term * ratio;
        add_coeff(&mut coeffs, j, term);
    }
    coeffs
}

fn reciprocal_series<T: SpecFloat>(series: &[T], order: usize) -> Vec<T> {
    let mut out: Vec<T> = vec![T::zero(); order + 1];
    let a0 = series.first().copied().unwrap_or_else(T::zero);
    add_coeff(&mut out, 0, T::one() / a0);

    for degree in 1..=order {
        let mut sum = T::zero();
        for k in 1..=degree {
            let a_k = series.get(k).copied().unwrap_or_else(T::zero);
            let prev = out.get(degree - k).copied().unwrap_or_else(T::zero);
            sum = sum + a_k * prev;
        }
        add_coeff(&mut out, degree, -sum / a0);
    }
    out
}

fn mul_series<T: SpecFloat>(lhs: &[T], rhs: &[T], order: usize) -> Vec<T> {
    let mut out: Vec<T> = vec![T::zero(); order + 1];
    for (i, left) in lhs.iter().enumerate() {
        for (j, right) in rhs.iter().enumerate() {
            let degree = i + j;
            if degree > order {
                break;
            }
            add_coeff(&mut out, degree, *left * *right);
        }
    }
    out
}

fn rising_factorial_series<T: SpecFloat>(s: T, factors: usize, order: usize) -> Vec<T> {
    let mut coeffs: Vec<T> = vec![T::zero(); order + 1];
    add_coeff(&mut coeffs, 0, T::one());

    for j in 0..factors {
        let mut next: Vec<T> = vec![T::zero(); order + 1];
        let constant = s + T::from_usize(j);
        for degree in 0..=order {
            let coeff = coeffs.get(degree).copied().unwrap_or_else(T::zero);
            add_coeff(&mut next, degree, coeff * constant);
            if degree < order {
                add_coeff(&mut next, degree + 1, coeff);
            }
        }
        coeffs = next;
    }
    coeffs
}

fn factorial<T: SpecFloat>(n: usize) -> T {
    let mut f = T::one();
    for j in 1..=n {
        f = f * T::from_usize(j);
    }
    f
}

#[inline]
fn cabs_sq<T: SpecFloat>(z: C<T>) -> T {
    z.0.mul_add(z.0, z.1 * z.1)
}
