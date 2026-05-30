#![allow(
    clippy::doc_markdown,
    reason = "LaTeX math notation in $...$ / $$...$$ is not recognized by clippy"
)]
//! Riemann Zeta function, its derivatives, and related series expansions.
use super::super::{BackingFloat, get_precision};
use crate::number::logic::int_math::IntType;
use alloc::vec::Vec;
use rug::ops::Pow;
use rug::{Float, Integer};

use super::bessel::precision_threshold_with;
use super::gamma::{bernoulli_even_up_to, polygamma};

pub fn zeta(value: &BackingFloat) -> BackingFloat {
    value.clone().zeta()
}

/// Computes the $n$-th derivative of the Riemann zeta function $\zeta^{(n)}(v)$.
///
/// **Strategy**:
/// - $n = 0$: return $\zeta(v)$ via `rug`'s built-in `zeta()`.
/// - $v > 1$ or $|v-1| < 1/10$: use `zeta_deriv_internal` (Euler-Maclaurin or
///   Laurent/Borwein series respectively).
/// - $v \le 0.9$ and $|v-1| \ge 1/10$: use the functional equation (reflection)
///   (DLMF §25.4.2):
///   $$ \zeta(s) = 2 (2\pi)^{s-1} \sin(\pi s / 2) \Gamma(1-s) \zeta(1-s). $$
///   The $n$-th derivative is obtained by Leibniz rule applied to the product of
///   four factors $A(s) B(s) C(s) D(s)$, where:
///   - $A(s) = 2 (2\pi)^{s-1}$ with derivatives $A^{(k)}(s) = A(s) [\ln(2\pi)]^k$,
///   - $B(s) = \sin(\pi s / 2)$ with derivatives obtained by phase-shifted sines,
///   - $C(s) = \Gamma(1-s)$ with derivatives computed via $\psi^{(k)}(1-s)$,
///   - $D(s) = \zeta(1-s)$ with derivatives computed recursively.
///
/// **Magic constants**:
/// - `guard_bits = max(prec/10, 60) + 50 + n * 8`:
///   | Term | Purpose |
///   |---|---|
///   | `prec/10` | base fraction of target precision for series expansion error |
///   | `.max(60)` | ensures at least 60 bits (series need minimum terms) |
///   | `+ 50` | absorbs accumulated rounding from the factorial and binomial coefficient computations in the Leibniz product |
///   | `+ n * 8` | each derivative order introduces ~8 new operations (binomial multiplications, additions), each losing at most ~1 bit; 8 bits per order is conservative |
///
/// **References**:
/// - DLMF §25.2 (Zeta function representations)
/// - DLMF §25.4.2 (Functional equation)
/// - Borwein et al. (2000), *ibid.*
pub fn zeta_deriv(n: &IntType, v: &BackingFloat) -> BackingFloat {
    if n.is_zero() {
        return v.clone().zeta();
    }
    let Some(n_usize) = n.to_usize() else {
        return Float::with_val(get_precision(), rug::float::Special::Nan);
    };
    let prec = get_precision();
    let guard_bits =
        prec.div_ceil(10).max(60) + 50 + u32::try_from(n_usize).unwrap_or(0).saturating_mul(8);
    let work_prec = prec.saturating_add(guard_bits);

    let s = Float::with_val(work_prec, v);
    let one = Float::with_val(work_prec, 1);
    let tenth = Float::with_val(work_prec, Float::with_val(work_prec, 1) / 10);
    let delta = Float::with_val(work_prec, &s - &one);

    if s > one || delta.abs() < tenth {
        let res = zeta_deriv_internal(n_usize, &s, work_prec);
        return Float::with_val(prec, res);
    }

    // Reflection for s <= 0.9 via Leibniz rule on functional equation:
    // \zeta(s) = 2 (2\pi)^{s-1} \sin(\pi s / 2) \Gamma(1-s) \zeta(1-s)
    // (DLMF §25.4.2)
    let two = Float::with_val(work_prec, 2);
    let two_pi = Float::with_val(
        work_prec,
        &two * Float::with_val(work_prec, rug::float::Constant::Pi),
    );
    let ln_2pi = two_pi.clone().ln();

    // A(s) = 2 (2\pi)^{s-1}  —  derivatives are A(s) * [ln(2\pi)]^k
    let mut a_derivs = Vec::with_capacity(n_usize + 1);
    let a_base = Float::with_val(
        work_prec,
        &two * two_pi.pow(Float::with_val(work_prec, &s - &one)),
    );
    let mut cur_factor = Float::with_val(work_prec, 1);
    for _ in 0..=n_usize {
        a_derivs.push(Float::with_val(work_prec, &a_base * &cur_factor));
        cur_factor *= &ln_2pi;
    }

    // B(s) = \sin(\pi s / 2)  —  derivatives via d^k/ds^k sin(s * pi/2)
    let pi_half = Float::with_val(
        work_prec,
        Float::with_val(work_prec, rug::float::Constant::Pi) / 2,
    );
    let mut b_derivs = Vec::with_capacity(n_usize + 1);
    let mut cur_factor_b = Float::with_val(work_prec, 1);
    for k in 0..=n_usize {
        let angle = Float::with_val(
            work_prec,
            &pi_half * &s + Float::with_val(work_prec, &pi_half * &Float::with_val(work_prec, k)),
        );
        b_derivs.push(Float::with_val(work_prec, angle.sin() * &cur_factor_b));
        cur_factor_b *= &pi_half;
    }

    // C(s) = \Gamma(1-s)  —  derivatives via polygamma
    let one_minus_s = Float::with_val(work_prec, &one - &s);
    let mut c_derivs = Vec::with_capacity(n_usize + 1);
    c_derivs.push(one_minus_s.clone().gamma());
    let mut g_derivs = Vec::with_capacity(n_usize + 1);
    g_derivs.push(Float::with_val(work_prec, 0));
    for m in 1..=n_usize {
        let psi_val = polygamma(
            &crate::number::logic::int_math::from_i64(i64::try_from(m - 1).expect("fits"))
                .expect("m-1 fits"),
            &one_minus_s,
        );
        let sign = if m % 2 == 0 { 1 } else { -1 };
        g_derivs.push(Float::with_val(work_prec, sign * psi_val));
    }
    for k in 0..n_usize {
        let mut sum = Float::with_val(work_prec, 0);
        let mut binom = Integer::from(1);
        for j in 0..=k {
            if j > 0 {
                binom *= Integer::from(k + 1 - j);
                binom /= Integer::from(j);
            }
            sum += Float::with_val(
                work_prec,
                c_derivs.get(j).expect("c_derivs has j")
                    * g_derivs.get(k - j + 1).expect("g_derivs has k-j+1"),
            ) * &binom;
        }
        c_derivs.push(sum);
    }

    // D(s) = \zeta(1-s)  —  derivatives recurse via zeta_deriv_internal
    let mut d_derivs = Vec::with_capacity(n_usize + 1);
    for k in 0..=n_usize {
        let z_val = zeta_deriv_internal(k, &one_minus_s, work_prec);
        let res_z = if k.is_multiple_of(2) { z_val } else { -z_val };
        d_derivs.push(Float::with_val(work_prec, res_z));
    }

    // Leibniz product A(s) * B(s) * C(s) * D(s)
    let ab = mul_derivs(&a_derivs, &b_derivs, n_usize, work_prec);
    let abc = mul_derivs(&ab, &c_derivs, n_usize, work_prec);
    let abcd = mul_derivs(&abc, &d_derivs, n_usize, work_prec);

    let res = abcd.get(n_usize).expect("abcd has n elements").clone();
    Float::with_val(prec, res)
}

/// Compute $n!$ as a `Float` at the given precision via direct multiplication.
fn factorial_rug(prec: u32, n: usize) -> Float {
    let mut f = Float::with_val(prec, 1);
    for j in 2..=n {
        f = Float::with_val(prec, &f * Float::with_val(prec, j));
    }
    f
}

fn exp_linear_series_rug(base: &Float, slope: &Float, order: usize, prec: u32) -> Vec<Float> {
    let mut coeffs = alloc::vec![Float::with_val(prec, 0); order + 1];
    let mut term = base.clone();
    if let Some(first) = coeffs.first_mut() {
        first.clone_from(&term);
    }
    for (j, coeff) in coeffs.iter_mut().enumerate().skip(1) {
        let j_f = Float::with_val(prec, j);
        term = Float::with_val(prec, &term * slope);
        term = Float::with_val(prec, &term / &j_f);
        coeff.clone_from(&term);
    }
    coeffs
}

fn reciprocal_linear_series_rug(delta: &Float, order: usize, prec: u32) -> Vec<Float> {
    let mut coeffs = alloc::vec![Float::with_val(prec, 0); order + 1];
    let mut term = Float::with_val(prec, 1) / delta;
    if let Some(first) = coeffs.first_mut() {
        first.clone_from(&term);
    }
    let ratio = Float::with_val(prec, -term.clone());
    for coeff in coeffs.iter_mut().skip(1) {
        term = Float::with_val(prec, &term * &ratio);
        coeff.clone_from(&term);
    }
    coeffs
}

fn reciprocal_series_rug(series: &[Float], order: usize, prec: u32) -> Vec<Float> {
    let mut out = alloc::vec![Float::with_val(prec, 0); order + 1];
    let a0 = series.first().expect("series array must not be empty");
    if let Some(first) = out.first_mut() {
        *first = Float::with_val(prec, 1) / a0;
    }

    for degree in 1..=order {
        let mut sum = Float::with_val(prec, 0);
        for k in 1..=degree {
            if let (Some(a_k), Some(prev_coeff)) = (series.get(k), out.get(degree - k)) {
                sum += Float::with_val(prec, a_k * prev_coeff);
            }
        }
        if let Some(coeff) = out.get_mut(degree) {
            *coeff = Float::with_val(prec, -sum / a0);
        }
    }
    out
}

fn mul_series_rug(lhs: &[Float], rhs: &[Float], order: usize, prec: u32) -> Vec<Float> {
    let mut out = alloc::vec![Float::with_val(prec, 0); order + 1];
    for (i, left) in lhs.iter().enumerate() {
        for (j, right) in rhs.iter().enumerate() {
            let degree = i + j;
            if degree > order {
                break;
            }
            if let Some(slot) = out.get_mut(degree) {
                *slot += Float::with_val(prec, left * right);
            }
        }
    }
    out
}

fn rising_factorial_series_rug(s: &Float, factors: usize, order: usize, prec: u32) -> Vec<Float> {
    let mut coeffs = alloc::vec![Float::with_val(prec, 0); order + 1];
    if let Some(first) = coeffs.first_mut() {
        *first = Float::with_val(prec, 1);
    }

    for j in 0..factors {
        let mut next = alloc::vec![Float::with_val(prec, 0); order + 1];
        let constant = Float::with_val(prec, s + j);
        for degree in 0..=order {
            if let Some(coeff) = coeffs.get(degree) {
                if let Some(slot) = next.get_mut(degree) {
                    *slot += Float::with_val(prec, coeff * &constant);
                }
                if degree < order {
                    let coeff_clone = coeff.clone();
                    if let Some(next_slot) = next.get_mut(degree + 1) {
                        *next_slot += coeff_clone;
                    }
                }
            }
        }
        coeffs = next;
    }
    coeffs
}

/// Computes the Taylor-series coefficients of the Dirichlet eta function
/// $\eta(s) = \sum_{k=1}^\infty (-1)^{k-1} / k^s$ at a point $s$ using the
/// Borwein accelerated series (Borwein, Bradley & Crandall, 2000, §4).
///
/// The algorithm uses the $d_n(k)$ expansion:
/// $$ \eta^{(j)}(s) = \sum_{k=0}^n \frac{(-1)^k (d_k - d_n)}{d_n}
///    \frac{\ln^j(k+1)}{(k+1)^s}, $$
/// where $d_k$ are computed via the forward recurrence:
/// $$ d_k = n \cdot \sum_{i=0}^k \frac{4^i (n+i-1)! (n-i)!}{(2i+1)! (2i)!}. $$
///
/// **Magic constants**:
/// - `n = (prec * 10000) / 25431 + 20`: The Borwein series converges geometrically
///   with ratio $\approx 3 - 2\sqrt{2} \approx 0.1716$ (Borwein et al. eq. 4.5).
///   To obtain `prec` bits, we need $n \ge \lceil prec \cdot \log(2) / \log(1/r) \rceil$.
///   Since $\log(2) / \log(1/(3-2\sqrt{2})) \approx 10000 / 25431 \approx 0.3932$,
///   the formula `prec * 10000 / 25431` is a tight rational approximation of the
///   required series length.  The `+ 20` safety margin ensures adequate coverage
///   even for small `prec`.
///
/// **Reference**: Borwein, J. M., Bradley, D. M. & Crandall, R. E. (2000).
/// "Computational Strategies for the Riemann Zeta Function." *J. Comput. Appl. Math.*
/// 121, 247–285. §4 (The Borwein Algorithm).
fn eta_taylor_rug(order: usize, s: &Float, prec: u32) -> Vec<Float> {
    let one = Float::with_val(prec, 1);
    let four = Float::with_val(prec, 4);

    let prec_usize = usize::try_from(prec).expect("u32 mathematically fits in usize");
    let n = (prec_usize * 10000).div_euclid(25431) + 20;

    let mut d_coeffs = alloc::vec![Float::with_val(prec, 0); n + 1];
    let n_t = Float::with_val(prec, n);
    let mut term = Float::with_val(prec, &one / &n_t);
    let mut current_inner_sum = term.clone();
    if let Some(first) = d_coeffs.first_mut() {
        *first = Float::with_val(prec, &n_t * &current_inner_sum);
    }

    for (k, d_coeff) in d_coeffs.iter_mut().enumerate().skip(1) {
        let i_f = Float::with_val(prec, k - 1);
        let two_i_plus_1 = Float::with_val(prec, 2 * k - 1);
        let two_i_plus_2 = Float::with_val(prec, 2 * k);

        let n_plus_i = Float::with_val(prec, &n_t + &i_f);
        let n_minus_i = Float::with_val(prec, &n_t - &i_f);

        term = Float::with_val(prec, &term * &four);
        term = Float::with_val(prec, &term * &n_plus_i);
        term = Float::with_val(prec, &term * &n_minus_i);
        let denom = Float::with_val(prec, &two_i_plus_1 * &two_i_plus_2);
        term = Float::with_val(prec, &term / &denom);

        current_inner_sum += &term;
        *d_coeff = Float::with_val(prec, &n_t * &current_inner_sum);
    }

    let d_n = d_coeffs.get(n).expect("d_coeffs sized correctly").clone();
    let mut numerator = alloc::vec![Float::with_val(prec, 0); order + 1];

    for (k, d_k) in d_coeffs.iter().enumerate().take(n) {
        let k_plus_1 = Float::with_val(prec, k + 1);
        let sign = if k % 2 == 0 {
            one.clone()
        } else {
            Float::with_val(prec, -1)
        };
        let diff = Float::with_val(prec, d_k - &d_n);
        let coeff = Float::with_val(prec, &sign * &diff);
        let ln_k = k_plus_1.clone().ln();
        let neg_ln_k = Float::with_val(prec, -&ln_k);

        let mut series_term = Float::with_val(prec, &coeff / k_plus_1.pow(s));
        if let Some(first) = numerator.first_mut() {
            *first += &series_term;
        }
        for (degree, num_coeff) in numerator.iter_mut().enumerate().skip(1) {
            let j_f = Float::with_val(prec, degree);
            series_term = Float::with_val(prec, &series_term * &neg_ln_k);
            series_term = Float::with_val(prec, &series_term / &j_f);
            *num_coeff += &series_term;
        }
    }

    for a in &mut numerator {
        let neg_a = Float::with_val(prec, -&*a);
        *a = Float::with_val(prec, neg_a / &d_n);
    }
    numerator
}

/// Computes the Taylor coefficients of $\zeta(s)$ via the Borwein algorithm
/// combined with the Dirichlet eta relation: $\zeta(s) = \eta(s) / (1 - 2^{1-s})$
/// (DLMF §25.2.3).  The numerator is computed by `eta_taylor_rug`, the denominator
/// is expanded as a power series in $s$ around the evaluation point.
///
/// **Reference**: Borwein et al. (2000), *ibid.*; DLMF §25.2.3.
fn borwein_taylor_rug(order: usize, s: &Float, prec: u32) -> Vec<Float> {
    let numerator = eta_taylor_rug(order, s, prec);

    let one = Float::with_val(prec, 1);
    let two = Float::with_val(prec, 2);
    let ln2 = two.clone().ln();
    let neg_ln2 = Float::with_val(prec, -&ln2);
    let base = two.pow(Float::with_val(prec, &one - s));

    let mut denom = alloc::vec![Float::with_val(prec, 0); order + 1];
    if let Some(first) = denom.first_mut() {
        *first = Float::with_val(prec, &one - &base);
    }

    let mut exp_term = None;
    for (degree, denom_coeff) in denom.iter_mut().enumerate().skip(1) {
        let j_f = Float::with_val(prec, degree);
        let current_exp = exp_term.as_ref().map_or_else(
            || Float::with_val(prec, &base * &neg_ln2),
            |e| Float::with_val(prec, e * &neg_ln2),
        );
        let new_exp = Float::with_val(prec, &current_exp / &j_f);
        *denom_coeff = Float::with_val(prec, -&new_exp);
        exp_term = Some(new_exp);
    }

    let reciprocal = reciprocal_series_rug(&denom, order, prec);
    mul_series_rug(&numerator, &reciprocal, order, prec)
}

/// Computes the Taylor coefficients of $\zeta(s)$ for $s > 1$ using the
/// Euler-Maclaurin summation formula (DLMF §25.2.9):
/// $$ \zeta(s) = \sum_{k=1}^{N-1} \frac{1}{k^s} + \frac{N^{1-s}}{s-1}
///    + \frac12 N^{-s} + \sum_{r=1}^R \frac{B_{2r}}{(2r)!} \frac{(s)_{2r-1}}{N^{s+2r-1}} + \epsilon. $$
///
/// The summation terms are expanded as power series in $(s - s_0)$ through the
/// `exp_linear_series`, `reciprocal_linear_series`, and `rising_factorial_series`
/// helper functions.  The series terminates when the correction magnitude falls
/// below $2^{-prec}$ or when the asymptotic terms begin to diverge (which occurs
/// at $R \approx \pi N$).
///
/// **Magic constants**:
/// - `n_terms = prec / 2 + 50`: The direct sum runs over $N$ terms, where $N$ is
///   chosen large enough that the Bernoulli tail converges at $O(N^{-s-2R+1})$.
///   Setting $N \approx prec/2$ ensures that the remaining Euler-Maclaurin series
///   requires only $O(prec)$ Bernoulli terms, balancing the cost of the direct sum
///   and the Bernoulli evaluation.
/// - `max_r = prec / 2 + 50`: maximum Bernoulli index.  Using the asymptotics
///   $|B_{2r}|/(2r)! \approx 2/(2\pi)^{2r}$, the tail is negligible beyond
///   $r \approx \pi N \approx (\pi/2) prec$.
/// - Break when `max_corr > prev_max_corr && r > 10`: asymptotic series divergence
///   detection.  Once the corrections start growing, further terms degrade accuracy.
///   The `r > 10` condition prevents false early termination from noise in the
///   first few terms.
///
/// **Reference**: DLMF §25.2.9 (Euler-Maclaurin for $\zeta(s)$); Borwein et al.
/// (2000), *ibid.*, §2.
fn euler_maclaurin_taylor_rug(order: usize, s: &Float, prec: u32) -> Vec<Float> {
    let one = Float::with_val(prec, 1);
    let half = Float::with_val(prec, &one / Float::with_val(prec, 2));

    let prec_usize = usize::try_from(prec).expect("u32 mathematically fits in usize");
    let n_terms = prec_usize.div_euclid(2) + 50;

    let mut coeffs = alloc::vec![Float::with_val(prec, 0); order + 1];

    for k in 1..n_terms {
        let k_f = Float::with_val(prec, k);
        let ln_k = k_f.clone().ln();
        let neg_ln_k = Float::with_val(prec, -&ln_k);
        let mut term = Float::with_val(prec, &one / k_f.pow(s));
        if let Some(first) = coeffs.first_mut() {
            *first += &term;
        }
        for (j, coeff) in coeffs.iter_mut().enumerate().skip(1) {
            let j_f = Float::with_val(prec, j);
            term = Float::with_val(prec, &term * &neg_ln_k);
            term = Float::with_val(prec, &term / &j_f);
            *coeff += &term;
        }
    }

    let n_val = Float::with_val(prec, n_terms);
    let ln_n = n_val.clone().ln();
    let neg_ln_n = Float::with_val(prec, -&ln_n);

    let s_minus_1 = Float::with_val(prec, s - &one);
    let n_pow = n_val.clone().pow(Float::with_val(prec, &one - s));
    let integral_exp = exp_linear_series_rug(&n_pow, &neg_ln_n, order, prec);
    let integral_recip = reciprocal_linear_series_rug(&s_minus_1, order, prec);
    let tail = mul_series_rug(&integral_exp, &integral_recip, order, prec);
    for (i, c) in tail.iter().enumerate() {
        if let Some(coeff) = coeffs.get_mut(i) {
            *coeff += c;
        }
    }

    let n_neg_s = Float::with_val(prec, &one / n_val.clone().pow(s));
    let boundary_base = Float::with_val(prec, &half * &n_neg_s);
    let boundary = exp_linear_series_rug(&boundary_base, &neg_ln_n, order, prec);
    for (i, c) in boundary.iter().enumerate() {
        if let Some(coeff) = coeffs.get_mut(i) {
            *coeff += c;
        }
    }

    let max_r = prec_usize.div_euclid(2) + 50;
    let bernoullis = bernoulli_even_up_to(max_r);

    let mut bernoulli_idx = 1;
    let mut prev_max_corr = Float::with_val(prec, 1);
    loop {
        let r = bernoulli_idx;
        let deriv_order = 2 * r - 1;
        if r > bernoullis.len() {
            break;
        }

        let bern_rat = bernoullis
            .get(r - 1)
            .expect("bernoullis contains enough values");
        let bn = Float::with_val(prec, bern_rat.numer());
        let bd = Float::with_val(prec, bern_rat.denom());
        let fact_2r = factorial_rug(prec, 2 * r);
        let scale = Float::with_val(prec, (bn / bd) / fact_2r);

        let rising = rising_factorial_series_rug(s, deriv_order, order, prec);
        let pow_base = Float::with_val(
            prec,
            &one / n_val.clone().pow(Float::with_val(prec, s + deriv_order)),
        );
        let pow = exp_linear_series_rug(&pow_base, &neg_ln_n, order, prec);

        let correction = mul_series_rug(&rising, &pow, order, prec);

        let mut max_corr = Float::with_val(prec, 0);
        for (i, c) in correction.iter().enumerate() {
            let term = Float::with_val(prec, &scale * c);
            if term.clone().abs() > max_corr {
                max_corr = term.clone().abs();
            }
            if let Some(coeff) = coeffs.get_mut(i) {
                *coeff += term;
            }
        }

        let tol = Float::with_val(prec, 1) >> prec;
        if max_corr < tol {
            break;
        }

        if max_corr > prev_max_corr && r > 10 {
            break;
        }
        prev_max_corr = max_corr;

        if r > max_r.div_euclid(2) {
            break;
        }

        bernoulli_idx += 1;
    }

    coeffs
}

/// Internal computation of the $n$-th derivative of $\zeta(s)$.
///
/// **Strategy**: Near $s = 1$ (within $|\delta| < 1/10$) we use the Laurent
/// expansion around the pole (DLMF §25.2.4):
/// $$ \zeta(s) = \frac{1}{s-1} + \sum_{k=0}^\infty \frac{(-1)^k}{k!} \gamma_k (s-1)^k, $$
/// where $\gamma_k$ are the Stieltjes constants.  The algorithm implements a
/// high-order Taylor expansion derived from the Borwein eta series to avoid
/// explicitly computing Stieltjes constants.
///
/// For $s > 1$: use the Euler-Maclaurin expansion (via `euler_maclaurin_taylor_rug`).
/// For $s < 1$ but $|s-1| \ge 1/10$: use the Borwein accelerated series (via `borwein_taylor_rug`).
///
/// The threshold $1/10$ was chosen so that the Borwein series converges in
/// $O(prec)$ terms even at the pole-adjacent regime $s = 0.9$, keeping the
/// series length tractable.  For $s < 0.9$, the functional equation (reflection)
/// is used instead in the caller.
///
/// **Magic constants**:
/// - `extra_terms = prec / 4`: When near the pole, the Taylor convergence is slower;
///   we allocate $prec/4$ extra series terms beyond the requested derivative order $n$.
/// - Termination: `abs_term < series_sum.abs() * 0.1 && abs_term < precision_threshold_with(prec)`.
///   The factor 0.1 ensures the term is small relative to the accumulated sum, not
///   just in absolute terms (which could be dominated by the pole term).
///
/// **Reference**: DLMF §25.2 (Zeta function); Borwein et al. (2000), *ibid.*.
fn zeta_deriv_internal(n: usize, s: &Float, prec: u32) -> Float {
    let one = Float::with_val(prec, 1);
    let delta = Float::with_val(prec, s - &one);
    let tenth = Float::with_val(prec, Float::with_val(prec, 1) / 10);

    if delta.clone().abs() < tenth {
        let extra_terms = usize::try_from(prec >> 2).unwrap_or(30);
        let order = n + extra_terms;

        let order_eta = order + 1;
        let e_coeffs = eta_taylor_rug(order_eta, &one, prec);

        let ln2 = Float::with_val(prec, 2).ln();

        let mut c_coeffs = alloc::vec![Float::with_val(prec, 0); order_eta + 1];
        let mut f_coeffs = alloc::vec![Float::with_val(prec, 0); order_eta + 2];

        let mut current_ln2_pow = ln2.clone(); // (ln 2)^1
        let mut current_fact = Float::with_val(prec, 1); // 1!

        for k in 0..=order_eta {
            let sign = if k % 2 == 0 {
                one.clone()
            } else {
                Float::with_val(prec, -1)
            };
            let num = Float::with_val(prec, &sign * &current_ln2_pow);
            *c_coeffs.get_mut(k).expect("c_coeffs k") = Float::with_val(prec, &num / &current_fact);

            f_coeffs
                .get_mut(k + 1)
                .expect("f_coeffs k+1")
                .clone_from(c_coeffs.get(k).expect("c_coeffs k"));

            current_ln2_pow = Float::with_val(prec, &current_ln2_pow * &ln2);
            let k_plus_2 = Float::with_val(prec, k + 2);
            current_fact = Float::with_val(prec, &current_fact * &k_plus_2);
        }

        let mut h_coeffs = alloc::vec![Float::with_val(prec, 0); order_eta + 1];
        for k in 0..=order_eta {
            *h_coeffs.get_mut(k).expect("h_coeffs k") = Float::with_val(
                prec,
                e_coeffs.get(k).expect("e_coeffs k") - c_coeffs.get(k).expect("c_coeffs k"),
            );
        }

        let mut g_coeffs = alloc::vec![Float::with_val(prec, 0); order + 1];
        let f1 = f_coeffs.get(1).expect("f_coeffs 1").clone();

        for m in 0..=order {
            let mut sum_fj_g = Float::with_val(prec, 0);
            for j in 2..=(m + 1) {
                sum_fj_g += Float::with_val(
                    prec,
                    f_coeffs.get(j).expect("f_coeffs j")
                        * g_coeffs.get(m + 1 - j).expect("g_coeffs m+1-j"),
                );
            }
            let diff =
                Float::with_val(prec, h_coeffs.get(m + 1).expect("h_coeffs m+1") - &sum_fj_g);
            *g_coeffs.get_mut(m).expect("g_coeffs m") = Float::with_val(prec, &diff / &f1);
        }

        let n_fact = factorial_rug(prec, n);
        let pole_sign = if n.is_multiple_of(2) {
            one
        } else {
            Float::with_val(prec, -1)
        };
        let n_plus_1 = Float::with_val(prec, n + 1);
        let pole_term = Float::with_val(
            prec,
            Float::with_val(prec, &pole_sign * &n_fact) / delta.clone().pow(&n_plus_1),
        );

        let mut series_sum = Float::with_val(prec, 0);
        let mut running_binom = Float::with_val(prec, factorial_rug(prec, n));

        for j in 0..(order - n) {
            let g_val = g_coeffs.get(n + j).expect("g_coeffs n+j");
            let term_coeff = Float::with_val(prec, g_val * &running_binom);
            let delta_j =
                Float::with_val(prec, rug::ops::Pow::pow(delta.clone(), &Integer::from(j)));

            let term = Float::with_val(prec, &term_coeff * &delta_j);
            let abs_term = term.clone().abs();
            series_sum += term;
            if abs_term < Float::with_val(prec, &series_sum.clone().abs() * &tenth)
                && abs_term < precision_threshold_with(prec)
            {
                break;
            }

            // running_binom *= (n + j + 1) / (j + 1)
            let num = Float::with_val(prec, n + j + 1);
            let den = Float::with_val(prec, j + 1);
            running_binom *= Float::with_val(prec, num / den);
        }
        return pole_term + series_sum;
    }

    if s.clone() > one {
        let coeffs = euler_maclaurin_taylor_rug(n, s, prec);
        coeffs.get(n).map_or_else(
            || Float::with_val(prec, 0),
            |coeff| Float::with_val(prec, coeff * factorial_rug(prec, n)),
        )
    } else {
        let coeffs = borwein_taylor_rug(n, s, prec);
        coeffs.get(n).map_or_else(
            || Float::with_val(prec, 0),
            |coeff| Float::with_val(prec, coeff * factorial_rug(prec, n)),
        )
    }
}

/// Multiplies two derivative arrays $(u^{(0..n)}, v^{(0..n)})$ via the Leibniz rule:
/// $$ (uv)^{(i)} = \sum_{k=0}^i \binom{i}{k} u^{(k)} v^{(i-k)}. $$
fn mul_derivs(u: &[Float], v: &[Float], n: usize, prec: u32) -> Vec<Float> {
    let mut f = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let mut sum = Float::with_val(prec, 0);
        let mut binom = Integer::from(1);
        for k in 0..=i {
            if k > 0 {
                binom *= Integer::from(i + 1 - k);
                binom /= Integer::from(k);
            }
            sum += Float::with_val(
                prec,
                u.get(k).expect("u has enough elements")
                    * v.get(i - k).expect("v has enough elements"),
            ) * &binom;
        }
        f.push(sum);
    }
    f
}
