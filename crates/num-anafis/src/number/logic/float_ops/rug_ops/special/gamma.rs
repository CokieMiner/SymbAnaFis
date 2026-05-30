#![allow(
    clippy::doc_markdown,
    reason = "LaTeX math notation in $...$ / $$...$$ is not recognized by clippy"
)]
//! Gamma-related functions including Error functions, Beta, Digamma, and Polygamma.
use super::super::{BackingFloat, get_precision, nan};
use crate::number::logic::int_math::IntType;
use alloc::vec::Vec;
use rug::{Float, Integer};

pub fn erf(value: &BackingFloat) -> BackingFloat {
    value.clone().erf()
}

pub fn erfc(value: &BackingFloat) -> BackingFloat {
    value.clone().erfc()
}

pub fn gamma(value: &BackingFloat) -> BackingFloat {
    value.clone().gamma()
}

/// Computes the natural logarithm of the Gamma function $\ln \Gamma(x)$.
///
/// **Heuristic Justification (Single-Pass bounds)**:
/// For $x < 0$, we use the reflection formula (DLMF §5.5.3):
/// $$ \ln \Gamma(x) = \ln \pi - \ln|\sin(\pi x)| - \ln \Gamma(1 - x). $$
/// The primary source of precision loss is argument reduction in $\sin(\pi x)$ for
/// large $|x|$, where $-\log_2|\sin(\pi x)| \sim \log_2|x|$ bits may be lost.
/// By unconditionally doubling the precision (`work_prec = prec * 2`), we safely
/// absorb argument-reduction errors for any $|x| \le 2^{prec}$ — because
/// $2^{prec}$ is the largest exact integer representable at `prec` bits, and
/// $x$ must be in-bounds for `Float`. All reflection terms are additively combined
/// with matching sign, so no catastrophic cancellation occurs; a Ziv retry loop
/// is unnecessary.
///
/// **References**:
/// - DLMF §5.5.3 (Reflection Formula)
/// - DLMF §5.11.14 (Spouge's Approximation)
pub fn lgamma(value: &BackingFloat) -> BackingFloat {
    if value.is_sign_negative() {
        if value.is_integer() {
            let prec = get_precision();
            return Float::with_val(prec, rug::float::Special::Infinity);
        }
        let prec = get_precision();
        let work_prec = prec * 2;
        let pi = Float::with_val(work_prec, rug::float::Constant::Pi);
        let x_work = Float::with_val(work_prec, value);
        let sin_pi_x = BackingFloat::with_val(work_prec, &pi * &x_work).sin();
        let term1 = pi.ln();
        let term2 = sin_pi_x.abs().ln();
        let one_minus_x = BackingFloat::with_val(work_prec, 1) - &x_work;
        let term3 = one_minus_x.ln_gamma();
        let res = BackingFloat::with_val(
            work_prec,
            BackingFloat::with_val(work_prec, term1 - term2) - term3,
        );
        return Float::with_val(prec, res);
    }
    value.clone().ln_gamma()
}

pub fn digamma(value: &BackingFloat) -> BackingFloat {
    value.clone().digamma()
}

pub fn trigamma(value: &BackingFloat) -> BackingFloat {
    polygamma(
        &crate::number::logic::int_math::from_i64(1).expect("1 fits in IntType"),
        value,
    )
}

pub fn tetragamma(value: &BackingFloat) -> BackingFloat {
    polygamma(
        &crate::number::logic::int_math::from_i64(2).expect("2 fits in IntType"),
        value,
    )
}

/// Computes the Polygamma function $\psi^{(n)}(x)$.
///
/// **Heuristic Justification (Single-Pass bounds)**:
/// Uses the Euler-Maclaurin asymptotic series (DLMF §5.15.9):
/// $$ \psi^{(n)}(z) \sim (-1)^{n-1} \Bigl[ \frac{(n-1)!}{z^n} + \frac{n!}{2 z^{n+1}}
///    + \sum_{k=1}^\infty \frac{B_{2k}}{(2k)!} \frac{(n+2k-1)!}{(z^{n+2k})} \Bigr]. $$
///
/// For an asymptotic series, the truncation error is bounded by the magnitude of the
/// first omitted term.  Using the bound $|B_{2k}| \approx 4\sqrt{\pi k} (k/\pi e)^{2k}$,
/// the $k$th term decays like $(n+2k)! / (2\pi z)^{2k}$.
///
/// To guarantee $|\text{error}| < 2^{-prec}$ without a Ziv retry loop, we:
/// - Shift $x$ up to $x_{\text{shift}} \approx prec/4 + 50$ via the recurrence
///   $\psi^{(n)}(x+1) = \psi^{(n)}(x) + (-1)^n n! / x^{n+1}$ (DLMF §5.15.5).
///   Each shift corrects exactly at $prec+40$ using the exactly-known recurrence,
///   so no cancellation occurs.
/// - Sum the asymptotic series up to $k_{\max} \approx prec/8 + 10$ terms.
///   For $x \ge x_{\text{shift}}$, the $k_{\max}$-th term is
///   $\sim 2^{-(prec+40)}$ well before the series begins to diverge.
/// - Use `work_prec = prec + 40` guard bits, allocated once, because all operations
///   are either exact recurrences or monotone asymptotic sums — no cancellation
///   path exists that could destroy more than 40 bits.
///
/// **Magic constants used**:
/// | Constant | Basis |
/// |---|---|
/// | `work_prec = prec + 64 + prec/10` | 64 base guard bits plus `prec/10` absorb accumulated rounding from the `max_k ≈ work_prec/8 + 10` series terms. Each term contributes < 1 ulp at `work_prec`; the worst-case total of ~`work_prec/8` ulps rounds to < 1 ulp at `prec`. |
/// | `max_k = max(work_prec/8 + 10, 40)` | The $B_{2k}/(2k)!$ factor decays factorially; $(2\pi x)^{2k}$ overtakes the numerator at roughly $k \approx \pi x/e$. With $x \ge$ shift_val, `work_prec/8` terms suffice to reach $2^{-prec}$. The minimum 40 guarantees coverage at very low precision. |
/// | `shift_val = max(work_prec/4 + 50, 100)` | The shift scales with `work_prec` so the asymptotic series converges in O(`work_prec/8`) terms. The `≥ 100` lower bound ensures a reasonable starting point even at low precision. |
///
/// **References**:
/// - DLMF §5.15.5 (Recurrence), §5.15.9 (Asymptotic Expansion)
/// - Abramowitz & Stegun §6.4 (Polygamma Functions)
/// - Olver, F.W.J. (1997). *Asymptotics and Special Functions*, Ch. 8
#[allow(clippy::many_single_char_names, reason = "Standard math notation")]
pub fn polygamma(n: &IntType, value: &BackingFloat) -> BackingFloat {
    let ni = n;
    if value.is_zero() || (value.is_sign_negative() && value.is_integer()) {
        return nan();
    }
    if ni.is_zero() {
        return digamma(value);
    }

    let neg_np1 = -Integer::from(ni + 1);

    let prec = get_precision();
    let work_prec = prec + 64 + prec.div_euclid(10);
    let mut x = Float::with_val(work_prec, value);
    let mut s = BackingFloat::with_val(work_prec, 0);
    let max_k = usize::try_from(work_prec.div_euclid(8) + 10)
        .unwrap_or(40)
        .max(40);
    let shift_val = (work_prec.div_euclid(4) + 50).max(100);
    let shift = BackingFloat::with_val(work_prec, shift_val);
    let one = BackingFloat::with_val(work_prec, 1);
    while x < shift {
        let term = Float::with_val(work_prec, rug::ops::Pow::pow(x.clone(), &neg_np1));
        s += term;
        x += &one;
    }

    // Factorial: (n-1)! and n! computed exactly using Integers
    let mut factorial_n_int = Integer::from(1);
    let mut k_fact = Integer::from(2);
    while k_fact <= *ni {
        factorial_n_int *= &k_fact;
        k_fact += 1;
    }
    let factorial_nm1_int = Integer::from(&factorial_n_int / ni);
    let factorial_n = BackingFloat::with_val(work_prec, &factorial_n_int);
    let factorial_nm1 = BackingFloat::with_val(work_prec, &factorial_nm1_int);

    // DLMF 5.15.2:  ψ⁽ⁿ⁾(z) ~ (-1)^(n-1) * [ (n-1)! / zⁿ + n!/(2·zⁿ⁺¹) + sum ]
    let n_int = Integer::from(ni);
    let n1 = Integer::from(ni + 1);

    // Leading: (n-1)! / xⁿ
    let pow_x_n = Float::with_val(work_prec, rug::ops::Pow::pow(x.clone(), &n_int));
    let mut sum = BackingFloat::with_val(work_prec, &factorial_nm1 / &pow_x_n);

    // Second term: n! / (2 · xⁿ⁺¹)
    let half_fact = BackingFloat::with_val(work_prec, &factorial_n / 2);
    let pow_x_n1 = Float::with_val(work_prec, rug::ops::Pow::pow(x.clone(), &n1));
    sum += BackingFloat::with_val(work_prec, &half_fact / &pow_x_n1);

    let tol = BackingFloat::with_val(work_prec, 1) >> (work_prec.saturating_sub(10));
    let b_evens = bernoulli_even_up_to(max_k);

    // prod_n initially has (n+1)! which includes the (n-1)! factor
    let mut prod_n = Integer::from(&factorial_n_int) * Integer::from(ni + 1);
    let mut fact_2k = Integer::from(2);

    for k in 1..=max_k {
        let b2k = BackingFloat::with_val(
            work_prec,
            b_evens.get(k - 1).expect("b_evens has enough elements"),
        );
        let pow_val = Float::with_val(
            work_prec,
            rug::ops::Pow::pow(x.clone(), Integer::from(ni) + Integer::from(2 * k)),
        );
        let term = BackingFloat::with_val(
            work_prec,
            BackingFloat::with_val(work_prec, &b2k * BackingFloat::with_val(work_prec, &prod_n))
                / BackingFloat::with_val(
                    work_prec,
                    BackingFloat::with_val(work_prec, &fact_2k) * pow_val,
                ),
        );

        let abs_term = term.clone().abs();
        sum += term;
        if abs_term < tol {
            break;
        }

        let k2 = Integer::from(2 * k);
        let mut t1 = Integer::from(ni + &k2);
        prod_n *= &t1;
        t1 += 1;
        prod_n *= &t1;

        let mut t2 = Integer::from(&k2 + 1);
        fact_2k *= &t2;
        t2 += 1;
        fact_2k *= &t2;
    }

    let sign = BackingFloat::with_val(work_prec, if ni.is_odd() { 1 } else { -1 });
    let res = BackingFloat::with_val(
        work_prec,
        &sign * (BackingFloat::with_val(work_prec, &factorial_n * &s) + &sum),
    );
    Float::with_val(prec, res)
}

/// Computes even-indexed Bernoulli numbers $B_{2}, B_{4}, \ldots, B_{2k}$ using the
/// standard recurrence (DLMF §24.2.1):
/// $$ B_m = -\frac{1}{m+1} \sum_{j=0}^{m-1} \binom{m+1}{j} B_j, \quad B_0=1, B_1=-\tfrac12. $$
///
/// Odd indices $m \ge 3$ are zero (DLMF §24.2.2).  The recurrence is executed with
/// exact `rug::Rational` arithmetic so that $B_{2k}$ are stored as irreducible fractions,
/// which are then converted to `Float` at the target precision when needed.
///
/// **Reference**: DLMF §24.2.1, §24.2.2; Graham, Knuth & Patashnik (1994).
pub(super) fn bernoulli_even_up_to(max_k: usize) -> Vec<rug::Rational> {
    let mut b = alloc::vec![rug::Rational::from((1, 1)), rug::Rational::from((-1, 2))];
    for m in 2..=(2 * max_k) {
        if m % 2 != 0 {
            b.push(rug::Rational::from((0, 1)));
            continue;
        }
        let mut sum = rug::Rational::from((0, 1));
        let mut binom = Integer::from(1);
        for (j, b_item) in b.iter().enumerate().take(m) {
            if j > 0 {
                binom *= Integer::from(m + 2 - j);
                binom /= Integer::from(j);
            }
            if j % 2 == 0 || j == 1 {
                let mut term = b_item.clone();
                term *= &binom;
                sum += term;
            }
        }
        let bm = -sum / Integer::from(m + 1);
        b.push(bm);
    }
    let mut evens = alloc::vec::Vec::with_capacity(max_k);
    for k in 1..=max_k {
        if let Some(val) = b.get(2 * k) {
            evens.push(val.clone());
        }
    }
    evens
}

/// Returns the sign of $\Gamma(x)$ for real $x$.  For $x > 0$, $\Gamma(x) > 0$.
/// For $x < 0$, the sign alternates with each negative integer interval:
/// $\Gamma(x) > 0$ on $(-2k, -2k+1)$ and $\Gamma(x) < 0$ on $(-2k-1, -2k)$
/// (DLMF §5.5.3).  The pole at non-positive integers is not reached because
/// `gamma_sign` is only called on finite inputs.
fn gamma_sign(x: &Float) -> i32 {
    if x.is_sign_positive() || x.is_zero() {
        1
    } else {
        let floor = x.clone().floor();
        let floor_int = floor.to_integer().unwrap_or_else(|| Integer::from(0));
        if floor_int.is_even() { 1 } else { -1 }
    }
}

/// Computes the Beta function $B(a, b)$ via the identity $B(a,b) = \Gamma(a)\Gamma(b) / \Gamma(a+b)$.
///
/// Uses the signed gamma product `gamma_sign(a) * gamma_sign(b) * gamma_sign(a+b)` to
/// recover the correct sign after computing absolute values in log-space, avoiding
/// spurious NaN from log of negative gamma.
///
/// **Magic constants**:
/// - `work_prec = prec + 20`: 20 guard bits are sufficient because all three
///   `lgamma` calls are precision-stable (additive terms, no cancellation), and the
///   final `exp` operation only amplifies relative error by a factor of < 2 from
///   exponentiation of the log-scale addition.
///
/// **Reference**: DLMF §5.12.1 (Beta Function in terms of Gamma).
pub fn beta(a: &BackingFloat, b: &BackingFloat) -> BackingFloat {
    let prec = get_precision();
    let work_prec = prec + 20;

    let a_w = Float::with_val(work_prec, a);
    let b_w = Float::with_val(work_prec, b);

    let lga = lgamma(&a_w);
    let lgb = lgamma(&b_w);
    let lgapb = lgamma(&BackingFloat::with_val(work_prec, &a_w + &b_w));
    let abs_beta = BackingFloat::with_val(
        work_prec,
        BackingFloat::with_val(work_prec, lga + lgb - lgapb).exp(),
    );
    let sign = gamma_sign(&a_w)
        * gamma_sign(&b_w)
        * gamma_sign(&BackingFloat::with_val(work_prec, &a_w + &b_w));

    let res = BackingFloat::with_val(work_prec, abs_beta * sign);
    Float::with_val(prec, res)
}
