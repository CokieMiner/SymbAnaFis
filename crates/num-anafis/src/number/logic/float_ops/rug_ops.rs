// This module exclusively uses LaTeX math notation ($...$, $$...$$).  clippy's
// doc_markdown lint does not recognize math environments and incorrectly flags
// $I_0(x)$, $\log_2$, $B_{2k}$ etc.  Adding backticks inside $...$ would break
// rendering, so we allow the lint at module scope.
#![allow(
    clippy::doc_markdown,
    reason = "LaTeX math notation in $...$ / $$...$$ is not recognized by clippy"
)]

use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::sync::atomic::{AtomicU32, Ordering as MemOrder};

use rug::float::Special;
use rug::ops::Pow;
use rug::{Float, Integer};

use crate::number::logic::int_math::IntRepr;
use crate::number::logic::rational_math::RationalRepr;

pub(super) type BackingFloat = Float;

static PRECISION: AtomicU32 = AtomicU32::new(256);

/// Set the working precision (bits) for the rug backend.
/// Returns `true` on success, `false` if `bits` is less than 2.
pub(super) fn set_precision(bits: u32) -> bool {
    if bits < 2 {
        return false;
    }
    PRECISION.store(bits, MemOrder::Relaxed);
    true
}

/// Return the current working precision in bits.
pub(super) fn get_precision() -> u32 {
    PRECISION.load(MemOrder::Relaxed)
}

#[inline]
fn with_val<T>(v: T) -> BackingFloat
where
    Float: rug::ops::AssignRound<T, Round = rug::float::Round, Ordering = Ordering>,
{
    Float::with_val(get_precision(), v)
}

pub(super) fn nan() -> BackingFloat {
    Float::with_val(get_precision(), rug::float::Special::Nan)
}

// ============================================================================
// Construction & conversion
// ============================================================================

#[inline]
pub(super) fn from_int(value: &IntRepr) -> BackingFloat {
    with_val(value)
}
#[inline]
pub(super) fn clone(value: &BackingFloat) -> BackingFloat {
    value.clone()
}
#[inline]
pub(super) fn from_f32(v: f32) -> BackingFloat {
    with_val(v)
}
#[inline]
pub(super) fn from_f64(v: f64) -> BackingFloat {
    with_val(v)
}
#[inline]
pub(super) fn from_i64(v: i64) -> BackingFloat {
    with_val(v)
}
#[inline]
pub(super) fn to_int(value: &BackingFloat) -> Option<IntRepr> {
    if value.is_integer() {
        value.to_integer()
    } else {
        None
    }
}

pub(super) fn to_rational(value: &BackingFloat) -> Option<RationalRepr> {
    if !value.is_finite() {
        return None;
    }
    // rug::Float to rug::Rational is exact
    value.to_rational()
}

pub(super) fn to_string(value: &BackingFloat) -> String {
    let prec = value.prec();
    // Convert bits to decimal digits:  prec * log₁₀(2) + 2 guard digits.
    // 30_103/100_000 ≈ log₁₀(2) = 0.30102999…, the +2 accounts for the
    // leading digit and a safety margin (Goldberg 1991, "What Every Computer
    // Scientist Should Know About Floating-Point Arithmetic", §Binary - Decimal
    // Conversion).
    let digits = usize::try_from((prec * 30_103).div_ceil(100_000) + 2).unwrap_or(17);
    alloc::format!("{value:.digits$e}")
}

#[cfg(feature = "serde")]
pub(super) fn from_str(value: &str) -> Option<BackingFloat> {
    let parsed = Float::parse(value).ok()?;
    Some(Float::with_val(get_precision(), parsed))
}
// ============================================================================
// Arithmetic — binary ops need with_val for precision control
// ============================================================================

#[inline]
pub(super) fn add(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    with_val(lhs + rhs)
}
#[inline]
pub(super) fn sub(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    with_val(lhs - rhs)
}
#[inline]
pub(super) fn mul(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    with_val(lhs * rhs)
}
#[inline]
pub(super) fn div(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    with_val(lhs / rhs)
}
#[inline]
pub(super) fn neg(value: &BackingFloat) -> BackingFloat {
    with_val(-value)
}

// ============================================================================
// Comparison & properties
// ============================================================================

#[inline]
pub(super) fn cmp(lhs: &BackingFloat, rhs: &BackingFloat) -> Option<Ordering> {
    lhs.partial_cmp(rhs)
}
#[inline]
pub(super) const fn is_zero(value: &BackingFloat) -> bool {
    value.is_zero()
}
#[inline]
pub(super) fn is_one(value: &BackingFloat) -> bool {
    *value == 1.0
}
#[inline]
pub(super) fn is_neg_one(value: &BackingFloat) -> bool {
    *value == -1.0
}
#[inline]
pub(super) fn is_integer(value: &BackingFloat) -> bool {
    value.is_integer()
}
#[inline]
pub(super) const fn is_finite(value: &BackingFloat) -> bool {
    value.is_finite()
}
#[inline]
pub(super) const fn is_negative(value: &BackingFloat) -> bool {
    value.is_sign_negative() && !value.is_zero()
}
#[inline]
pub(super) const fn is_positive(value: &BackingFloat) -> bool {
    value.is_sign_positive() && !value.is_zero()
}
#[inline]
pub(super) const fn is_nan(value: &BackingFloat) -> bool {
    value.is_nan()
}

// ============================================================================
// Unary math — result already at correct precision, no wrapper needed
// ============================================================================

#[inline]
pub(super) fn abs(value: &BackingFloat) -> BackingFloat {
    value.clone().abs()
}
#[inline]
pub(super) fn signum(value: &BackingFloat) -> BackingFloat {
    value.clone().signum()
}
#[inline]
pub(super) fn floor(value: &BackingFloat) -> BackingFloat {
    value.clone().floor()
}
#[inline]
pub(super) fn ceil(value: &BackingFloat) -> BackingFloat {
    value.clone().ceil()
}
#[inline]
pub(super) fn round(value: &BackingFloat) -> BackingFloat {
    value.clone().round()
}
#[inline]
pub(super) fn fract(value: &BackingFloat) -> BackingFloat {
    value.clone().fract()
}
#[inline]
pub(super) fn sqrt(value: &BackingFloat) -> BackingFloat {
    value.clone().sqrt()
}
#[inline]
pub(super) fn cbrt(value: &BackingFloat) -> BackingFloat {
    value.clone().cbrt()
}
#[inline]
pub(super) fn sin(value: &BackingFloat) -> BackingFloat {
    value.clone().sin()
}
#[inline]
pub(super) fn cos(value: &BackingFloat) -> BackingFloat {
    value.clone().cos()
}
#[inline]
pub(super) fn tan(value: &BackingFloat) -> BackingFloat {
    value.clone().tan()
}
#[inline]
pub(super) fn asin(value: &BackingFloat) -> BackingFloat {
    value.clone().asin()
}
#[inline]
pub(super) fn acos(value: &BackingFloat) -> BackingFloat {
    value.clone().acos()
}
#[inline]
pub(super) fn atan(value: &BackingFloat) -> BackingFloat {
    value.clone().atan()
}
#[inline]
pub(super) fn sinh(value: &BackingFloat) -> BackingFloat {
    value.clone().sinh()
}
#[inline]
pub(super) fn cosh(value: &BackingFloat) -> BackingFloat {
    value.clone().cosh()
}
#[inline]
pub(super) fn tanh(value: &BackingFloat) -> BackingFloat {
    value.clone().tanh()
}
#[inline]
pub(super) fn asinh(value: &BackingFloat) -> BackingFloat {
    value.clone().asinh()
}
#[inline]
pub(super) fn acosh(value: &BackingFloat) -> BackingFloat {
    value.clone().acosh()
}
#[inline]
pub(super) fn atanh(value: &BackingFloat) -> BackingFloat {
    value.clone().atanh()
}
#[inline]
pub(super) fn exp(value: &BackingFloat) -> BackingFloat {
    value.clone().exp()
}
#[inline]
pub(super) fn expm1(value: &BackingFloat) -> BackingFloat {
    value.clone().exp_m1()
}
#[inline]
pub(super) fn ln(value: &BackingFloat) -> BackingFloat {
    value.clone().ln()
}
#[inline]
pub(super) fn log1p(value: &BackingFloat) -> BackingFloat {
    value.clone().ln_1p()
}
#[inline]
pub(super) fn atan2(y: &BackingFloat, x: &BackingFloat) -> BackingFloat {
    with_val(y.clone().atan2(x))
}
#[inline]
pub(super) fn pow(lhs: &BackingFloat, rhs: &BackingFloat) -> BackingFloat {
    with_val(rug::ops::Pow::pow(lhs.clone(), rhs))
}

// ============================================================================
// Special functions — MPFR-native
// ============================================================================

pub(super) fn erf(value: &BackingFloat) -> BackingFloat {
    value.clone().erf()
}
pub(super) fn erfc(value: &BackingFloat) -> BackingFloat {
    value.clone().erfc()
}
pub(super) fn gamma(value: &BackingFloat) -> BackingFloat {
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
pub(super) fn lgamma(value: &BackingFloat) -> BackingFloat {
    if value.is_sign_negative() {
        if value.is_integer() {
            let prec = get_precision();
            return Float::with_val(prec, rug::float::Special::Infinity);
        }
        let prec = get_precision();
        let work_prec = prec * 2;
        let pi = Float::with_val(work_prec, rug::float::Constant::Pi);
        let x_work = Float::with_val(work_prec, value);
        let sin_pi_x = with_prec_val(work_prec, &pi * &x_work).sin();
        let term1 = pi.ln();
        let term2 = sin_pi_x.abs().ln();
        let one_minus_x = with_prec_val(work_prec, 1) - &x_work;
        let term3 = one_minus_x.ln_gamma();
        let res = with_prec_val(work_prec, with_prec_val(work_prec, term1 - term2) - term3);
        return Float::with_val(prec, res);
    }
    value.clone().ln_gamma()
}
pub(super) fn digamma(value: &BackingFloat) -> BackingFloat {
    value.clone().digamma()
}
pub(super) fn zeta(value: &BackingFloat) -> BackingFloat {
    value.clone().zeta()
}

// --- Bessel J/Y (MPFR jn/yn requires i32 order) ---

pub(super) fn bessel_j(n: &IntRepr, value: &BackingFloat) -> BackingFloat {
    let Some(order) = n.to_i32() else {
        return nan();
    };
    value.clone().jn(order)
}
pub(super) fn bessel_y(n: &IntRepr, value: &BackingFloat) -> BackingFloat {
    let Some(order) = n.to_i32() else {
        return nan();
    };
    value.clone().yn(order)
}

// ============================================================================
// Trigamma / Tetragamma — recurse then asymptotic (Euler-Maclaurin)
// ============================================================================

/// Computes even-indexed Bernoulli numbers $B_{2}, B_{4}, \ldots, B_{2k}$ using the
/// standard recurrence (DLMF §24.2.1):
/// $$ B_m = -\frac{1}{m+1} \sum_{j=0}^{m-1} \binom{m+1}{j} B_j, \quad B_0=1, B_1=-\tfrac12. $$
///
/// Odd indices $m \ge 3$ are zero (DLMF §24.2.2).  The recurrence is executed with
/// exact `rug::Rational` arithmetic so that $B_{2k}$ are stored as irreducible fractions,
/// which are then converted to `Float` at the target precision when needed.
///
/// **Reference**: DLMF §24.2.1, §24.2.2; Graham, Knuth & Patashnik (1994).
fn bernoulli_even_up_to(max_k: usize) -> Vec<rug::Rational> {
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

pub(super) fn trigamma(value: &BackingFloat) -> BackingFloat {
    polygamma(
        &crate::number::logic::int_math::from_i64(1).expect("1 fits in IntRepr"),
        value,
    )
}

pub(super) fn tetragamma(value: &BackingFloat) -> BackingFloat {
    polygamma(
        &crate::number::logic::int_math::from_i64(2).expect("2 fits in IntRepr"),
        value,
    )
}

// ============================================================================
// Polygamma — recurse then asymptotic for arbitrary order
// ============================================================================

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
pub(super) fn polygamma(n: &IntRepr, value: &BackingFloat) -> BackingFloat {
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
    let mut s = with_prec_val(work_prec, 0);
    let max_k = usize::try_from(work_prec.div_euclid(8) + 10)
        .unwrap_or(40)
        .max(40);
    let shift_val = (work_prec.div_euclid(4) + 50).max(100);
    let shift = with_prec_val(work_prec, shift_val);
    let one = with_prec_val(work_prec, 1);
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
    let factorial_n = with_prec_val(work_prec, &factorial_n_int);
    let factorial_nm1 = with_prec_val(work_prec, &factorial_nm1_int);

    // DLMF 5.15.2:  ψ⁽ⁿ⁾(z) ~ (-1)^(n-1) * [ (n-1)! / zⁿ + n!/(2·zⁿ⁺¹) + sum ]
    let n_int = Integer::from(ni);
    let n1 = Integer::from(ni + 1);

    // Leading: (n-1)! / xⁿ
    let pow_x_n = Float::with_val(work_prec, rug::ops::Pow::pow(x.clone(), &n_int));
    let mut sum = with_prec_val(work_prec, &factorial_nm1 / &pow_x_n);

    // Second term: n! / (2 · xⁿ⁺¹)
    let half_fact = with_prec_val(work_prec, &factorial_n / 2);
    let pow_x_n1 = Float::with_val(work_prec, rug::ops::Pow::pow(x.clone(), &n1));
    sum += with_prec_val(work_prec, &half_fact / &pow_x_n1);

    let tol = with_prec_val(work_prec, 1) >> (work_prec.saturating_sub(10));
    let b_evens = bernoulli_even_up_to(max_k);

    // prod_n initially has (n+1)! which includes the (n-1)! factor
    let mut prod_n = Integer::from(&factorial_n_int) * Integer::from(ni + 1);
    let mut fact_2k = Integer::from(2);

    for k in 1..=max_k {
        let b2k = with_prec_val(
            work_prec,
            b_evens.get(k - 1).expect("b_evens has enough elements"),
        );
        let pow_val = Float::with_val(
            work_prec,
            rug::ops::Pow::pow(x.clone(), Integer::from(ni) + Integer::from(2 * k)),
        );
        let term = with_prec_val(
            work_prec,
            with_prec_val(work_prec, &b2k * with_prec_val(work_prec, &prod_n))
                / with_prec_val(work_prec, with_prec_val(work_prec, &fact_2k) * pow_val),
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

    let sign = with_prec_val(work_prec, if ni.is_odd() { 1 } else { -1 });
    let res = with_prec_val(
        work_prec,
        &sign * (with_prec_val(work_prec, &factorial_n * &s) + &sum),
    );
    Float::with_val(prec, res)
}

// ============================================================================
// Lambert W — Halley iteration (Corless et al., 1996)
// ============================================================================

/// Computes the principal branch $W_0(x)$ of the Lambert W function.
///
/// **Algorithm**: Halley's method (Corless et al., 1996, §4.3):
/// $$ w_{n+1} = w_n - \frac{w_n e^{w_n} - x}
///    { e^{w_n}(w_n+1) - \frac{(w_n+2)(w_n e^{w_n} - x)}{2(w_n+1)} } $$
///
/// **Initial estimate** (piecewise; Corless et al., 1996, §5.2):
/// - $x > 3$: $W_0(x) \approx \ln x - \ln\ln x$ (dominant asymptotic)
/// - $0.5 < x \le 3$: use the approximation $\ln(x+1)/2$
///   (minimax error < 0.15 over this interval, Fritsch et al., 1981)
/// - $-1/e \le x \le 0.5$: start with $W_0(x) \approx x$ (linear series)
///
/// **Convergence criterion**: Halley's method converges cubically, so once the
/// correction $|\Delta w| < 2^{-(prec - 10)}$, the iterate is accurate to the
/// full target precision (Corless et al., 1996, §4.5).  The 10-bit threshold
/// margin compensates for the final rounding step.
///
/// **Reference**: Corless, R. M. et al. (1996). "On the Lambert W Function."
/// *Adv. Comput. Math.* 5, 329–359. [DOI:10.1007/BF02124750]
pub(super) fn lambert_w(value: &BackingFloat) -> BackingFloat {
    let prec = get_precision();
    let work_prec = prec + 40;
    let v_w = Float::with_val(work_prec, value);
    let e = Float::with_val(work_prec, 1).exp();
    let neg_inv_e = with_prec_val(work_prec, -1) / &e;
    if v_w < neg_inv_e {
        return nan();
    }

    let one = with_prec_val(work_prec, 1);
    let two = with_prec_val(work_prec, 2);

    let mut w = if v_w > 3 {
        let ln_v = v_w.clone().ln();
        with_prec_val(work_prec, ln_v.clone() - ln_v.ln())
    } else if with_prec_val(work_prec, &v_w * 2) > 1 {
        let log1p_v = with_prec_val(work_prec, &v_w + &one).ln();
        with_prec_val(work_prec, &log1p_v / 2)
    } else {
        v_w.clone()
    };
    let thr = precision_threshold_with(work_prec);
    loop {
        let ew = w.clone().exp();
        let wew = with_prec_val(work_prec, &w * &ew);
        let num = with_prec_val(work_prec, &wew - &v_w);
        let wp1 = with_prec_val(work_prec, &w + &one);
        let denom = with_prec_val(
            work_prec,
            &ew * &wp1
                - with_prec_val(work_prec, with_prec_val(work_prec, &wp1 + &one) * &num)
                    / with_prec_val(work_prec, &two * &wp1),
        );
        let correction = with_prec_val(work_prec, &num / &denom);
        let new_w = with_prec_val(work_prec, &w - &correction);
        if with_prec_val(work_prec, &new_w - &w).abs() < thr {
            break;
        }
        w = new_w;
    }
    Float::with_val(prec, w)
}

/// W₋₁(x) — lower real branch, defined for x ∈ [-1/e, 0). Returns W ≤ -1.
///
/// **Initial estimate** (Corless et al. 1996 §5.2):
/// - Near 0 ($x > -1/100$): $W_{-1}(x) \approx \ln(-x) - \ln(-\ln(-x)) + \ln(-\ln(-x)) / \ln(-x)$
///   (the iterative log asymptotic; the threshold $-1/100$ was chosen so that
///   $|\ln(-x)| \gtrsim 4.6$, ensuring the correction term $\ln(-\ln(-x))$ is well-defined).
/// - Near $-1/e$: use the branch-point expansion
///   $W_{-1}(x) \approx -1 - \sqrt{2(ex+1)}$ (DLMF §4.13, Corless et al. eq. 5.10).
///
/// The same Halley iteration as $W_0$ yields cubic convergence.
///
/// **Reference**: Corless et al. (1996), *ibid.*, §5.2, §4.3.
pub(super) fn lambert_wm1(value: &BackingFloat) -> BackingFloat {
    let prec = get_precision();
    let work_prec = prec + 40;
    let v_w = Float::with_val(work_prec, value);
    let e = Float::with_val(work_prec, 1).exp();
    let neg_inv_e = with_prec_val(work_prec, -1) / &e;
    if v_w < neg_inv_e || v_w >= 0 {
        return nan();
    }
    let tol = precision_threshold_with(work_prec);
    let delta = with_prec_val(work_prec, &v_w + &neg_inv_e);
    if delta.abs() < tol {
        return Float::with_val(prec, -1);
    }

    let one = with_prec_val(work_prec, 1);
    let two = with_prec_val(work_prec, 2);
    let mut w = if v_w > with_prec_val(work_prec, with_prec_val(work_prec, -1) / 100) {
        let neg_v = with_prec_val(work_prec, -v_w.clone());
        let l1 = neg_v.ln();
        let l2 = with_prec_val(work_prec, -l1.clone()).ln();
        with_prec_val(work_prec, l1.clone() - l2.clone() + l2 / l1)
    } else {
        let ev = with_prec_val(work_prec, &e * &v_w);
        let p = with_prec_val(
            work_prec,
            two.clone() * with_prec_val(work_prec, &ev + &one),
        )
        .sqrt();
        with_prec_val(work_prec, -one.clone() - p)
    };

    let thr = precision_threshold_with(work_prec);
    loop {
        let ew = w.clone().exp();
        let wew = with_prec_val(work_prec, &w * &ew);
        let num = with_prec_val(work_prec, &wew - &v_w);
        let wp1 = with_prec_val(work_prec, &w + &one);
        let denom = with_prec_val(
            work_prec,
            &ew * &wp1
                - with_prec_val(work_prec, with_prec_val(work_prec, &wp1 + &one) * &num)
                    / with_prec_val(work_prec, &two * &wp1),
        );
        let correction = with_prec_val(work_prec, &num / &denom);
        let new_w = with_prec_val(work_prec, &w - &correction);
        if with_prec_val(work_prec, &new_w - &w).abs() < thr {
            break;
        }
        w = new_w;
    }
    Float::with_val(prec, w)
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
pub(super) fn beta(a: &BackingFloat, b: &BackingFloat) -> BackingFloat {
    let prec = get_precision();
    let work_prec = prec + 20;

    let a_w = Float::with_val(work_prec, a);
    let b_w = Float::with_val(work_prec, b);

    let lga = lgamma(&a_w);
    let lgb = lgamma(&b_w);
    let lgapb = lgamma(&with_prec_val(work_prec, &a_w + &b_w));
    let abs_beta = with_prec_val(work_prec, with_prec_val(work_prec, lga + lgb - lgapb).exp());
    let sign =
        gamma_sign(&a_w) * gamma_sign(&b_w) * gamma_sign(&with_prec_val(work_prec, &a_w + &b_w));

    let res = with_prec_val(work_prec, abs_beta * sign);
    Float::with_val(prec, res)
}

// ============================================================================
// Special functions implemented directly on rug::Float
//
// Primary references used throughout this module:
//
// **DLMF** = *NIST Digital Library of Mathematical Functions* (https://dlmf.nist.gov/)
//   §4.13  Lambert W function
//   §5.5   Gamma function (reflection §5.5.3, Spouge §5.11.14)
//   §5.11  Stirling approximation (§5.11.1)
//   §5.12  Beta function
//   §5.15  Polygamma functions (recurrence §5.15.5, asymptotic §5.15.9)
//   §5.19  Mathematical applications
//   §10.29 Modified Bessel recurrences (§10.29.1, §10.29.3)
//   §10.30 Modified Bessel power series
//   §10.31 Modified Bessel K series
//   §10.40 Modified Bessel asymptotic expansions
//   §10.74 Miller's algorithm for Bessel I
//   §14.3  Associated Legendre polynomials
//   §14.9  Negative-order relation
//   §14.10 Legendre recurrences
//   §14.30 Spherical harmonics
//   §18.5  Hermite polynomials
//   §19.8  Complete elliptic integrals (§19.8.5–6, AGM)
//   §24.2  Bernoulli numbers (§24.2.1–2)
//   §25.2  Zeta function (§25.2.3 eta, §25.2.4 Laurent, §25.2.9 Euler-Maclaurin)
//   §25.4  Zeta function (§25.4.2 functional equation)
//
//
// - Abramowitz, M. & Stegun, I.A. (1964). *Handbook of Mathematical Functions*.
//   National Bureau of Standards.
// - Borwein, J. M., & Borwein, P. B. (1987). *Pi and the AGM: A Study in Analytic
//   Number Theory and Computational Complexity*. Wiley.
// - Borwein, J. M., Bradley, D. M., & Crandall, R. E. (2000). "Computational
//   strategies for the Riemann zeta function." *Journal of Computational and
//   Applied Mathematics*, 121(1-2), 247-296. [DOI: 10.1016/S0377-0427(00)00336-8]
// - Carlson, B. C. (1995). "Numerical computation of real or complex elliptic
//   integrals." *Numerical Algorithms*, 10(1), 13-26. [DOI: 10.1007/BF02198293]
// - Clenshaw, C. W. (1955). "A note on the summation of Chebyshev series."
//   *Mathematical Tables and Other Aids to Computation*, 9(51), 118-120.
// - Corless, R. M., Gonnet, G. H., Hare, D. E. G., Jeffrey, D. J., & Knuth, D. E. (1996).
//   "On the Lambert W function." *Advances in Computational Mathematics*, 5(1),
//   329-359. [DOI: 10.1007/BF02124750]
// - Graham, R. L., Knuth, D. E., & Patashnik, O. (1994). *Concrete Mathematics:
//   A Foundation for Computer Science* (2nd ed.). Addison-Wesley.
// - Lanczos, C. (1964). "A precision approximation of the gamma function."
//   *Journal of the Society for Industrial and Applied Mathematics, Series B:
//   Numerical Analysis*, 1(1), 86-96.
// - Miller, J. C. P. (1952). "A method for the determination of converging factors,
//   applied to the asymptotic expansions for the parabolic cylinder functions."
//   *Mathematical Proceedings of the Cambridge Philosophical Society*, 48(2), 243-254.
// - Moshier, S. L. (1989). *Methods and Programs for Mathematical Functions*.
//   Ellis Horwood Limited. (Cephes Mathematical Library)
// - NIST Digital Library of Mathematical Functions. https://dlmf.nist.gov/,
//   Release 1.1.12 of 2023-12-15. F. W. J. Olver et al., eds.
// - Numerical Recipes, 3rd Ed. (2007). Cambridge University Press. (Section 6.8)
// - Watson, G.N. (1944). *A Treatise on the Theory of Bessel Functions.*
//   2nd Ed. Cambridge University Press.
// ============================================================================

/// Complete elliptic integral $K(m)$ via the arithmetic-geometric mean (AGM).
///
/// **Algorithm**: $K(m) = \pi / (2 M(1, \sqrt{1-m}))$, where $M$ is the AGM
/// (DLMF §19.8.5).  The AGM converges quadratically (the number of correct
/// digits doubles each iteration), so at most $\lceil \log_2(prec) \rceil \le 16$
/// iterations suffice for any realistic precision.  The hard limit of 100 iterations
/// is a safety guard never reached in practice.
///
/// **Magic constants**:
/// - `work_prec = prec + 40`: AGM preserves relative accuracy; 40 guard bits
///   absorb the final division by the converged arithmetic mean.
/// - `tol = 1 >> (prec - 2)`: accept AGM convergence when $a_n = b_n$ to
///   `prec - 2` bits (2 bits margin relative to target precision).
///
/// **Reference**: DLMF §19.8.5 (AGM Representation); Borwein & Borwein (1987).
/// *Pi and the AGM*, Ch. 1.
pub(super) fn elliptic_k(v: &BackingFloat) -> BackingFloat {
    let prec = get_precision();
    let work_prec = prec + 40;
    let m = Float::with_val(work_prec, v);
    let one = Float::with_val(work_prec, 1);
    if m > one {
        return nan();
    }
    if m == one {
        return Float::with_val(prec, Special::Infinity);
    }
    let mut a = one.clone();
    let mut b = Float::with_val(work_prec, &one - &m).sqrt();
    let two = Float::with_val(work_prec, 2);
    let tol = precision_threshold_with(work_prec);

    for _ in 0..100 {
        let an = Float::with_val(work_prec, Float::with_val(work_prec, &a + &b) / &two);
        let bn = Float::with_val(work_prec, Float::with_val(work_prec, &a * &b).sqrt());
        let diff = Float::with_val(work_prec, &an - &bn).abs();
        a = an;
        b = bn;
        if diff < tol || a == b {
            break;
        }
    }
    let pi = Float::with_val(work_prec, rug::float::Constant::Pi);
    let res = Float::with_val(work_prec, pi / (two * a));
    Float::with_val(prec, res)
}

/// Complete elliptic integral $E(m)$ via AGM with corrections (parameter $m = k^2$).
///
/// **Algorithm**: After computing the AGM $(a_n, b_n)$ of $(1, \sqrt{1-m})$, we also
/// accumulate the correction series $c_n = (a_n - b_n)/2$:
/// $$ E(m) = \frac{\pi}{2 M(1, \sqrt{1-m})} \Bigl[ 1 - \frac12 \sum_{n=0}^\infty 2^n c_n^2 \Bigr] $$
/// (DLMF §19.8.6).  The series converges quadratically alongside the AGM.
///
/// **Magic constants**: same as `elliptic_k` — 40 guard bits, `prec - 2` tolerance,
/// 100 iterations as a safety limit.
///
/// **Reference**: DLMF §19.8.6; Carlson (1995). "Numerical Computation of Real
/// or Complex Elliptic Integrals." *Numer. Algorithms* 10, 13–26.
pub(super) fn elliptic_e(v: &BackingFloat) -> BackingFloat {
    let prec = get_precision();
    let work_prec = prec + 40;
    let m = Float::with_val(work_prec, v);
    let one = Float::with_val(work_prec, 1);
    if m > one {
        return nan();
    }
    if m == one {
        return Float::with_val(prec, 1);
    }
    let mut a = one.clone();
    let mut b = Float::with_val(work_prec, &one - &m).sqrt();
    let mut c_sq_sum = Float::with_val(work_prec, &m / 2);
    let mut two_pow_n = Float::with_val(work_prec, 1);
    let two = Float::with_val(work_prec, 2);
    let tol = precision_threshold_with(work_prec);

    for _ in 0..100 {
        let an = Float::with_val(work_prec, Float::with_val(work_prec, &a + &b) / &two);
        let bn = Float::with_val(work_prec, Float::with_val(work_prec, &a * &b).sqrt());
        let cn = Float::with_val(work_prec, Float::with_val(work_prec, &a - &b) / &two);
        c_sq_sum += Float::with_val(
            work_prec,
            &two_pow_n * Float::with_val(work_prec, &cn * &cn),
        );
        a = an;
        b = bn;
        two_pow_n *= &two;
        if cn.clone().abs() < tol || cn.is_zero() {
            break;
        }
    }
    let pi = Float::with_val(work_prec, rug::float::Constant::Pi);
    let factor = Float::with_val(work_prec, 1 - &c_sq_sum);
    let res = Float::with_val(
        work_prec,
        Float::with_val(work_prec, pi / (two * a)) * factor,
    );
    Float::with_val(prec, res)
}

/// Computes the Hermite polynomial $H_n(x)$ via the forward recurrence (DLMF §18.9.1, Table 18.9.1):
/// $$ H_{k+1}(x) = 2x H_k(x) - 2k H_{k-1}(x), \quad H_0=1, H_1=2x. $$
/// The recurrence is numerically stable for all real $x$ at the target precision
/// because it is a finite linear recurrence with no cancellation between large
/// terms (the coefficients are all non-negative when $H_k(x)$ has the dominant
/// sign, which holds for $x \ge 0$; for $x < 0$ the alternating sign pattern
/// likewise preserves accuracy).
///
/// **Reference**: DLMF §18.9.1, Table 18.9.1 (Hermite recurrence); DLMF Table 18.3 (values).
pub(super) fn hermite(n: &IntRepr, v: &BackingFloat) -> BackingFloat {
    let prec = get_precision();
    if n.is_zero() {
        return Float::with_val(prec, 1);
    }
    let work_prec = prec
        .saturating_add(64)
        .saturating_add(n.to_u32().unwrap_or(0));
    let x = Float::with_val(work_prec, v);
    let two = Float::with_val(work_prec, 2);
    let term1 = Float::with_val(work_prec, &two * &x);
    if *n == 1 {
        return Float::with_val(prec, term1);
    }
    let one = Float::with_val(work_prec, 1);
    let (mut h0, mut h1) = (one, term1);
    let mut k = Integer::from(1);
    while k < *n {
        let f_k = Float::with_val(work_prec, &k);
        let h2 = Float::with_val(
            work_prec,
            Float::with_val(work_prec, &two * &x) * &h1
                - Float::with_val(work_prec, Float::with_val(work_prec, &two * &f_k) * &h0),
        );
        h0 = h1;
        h1 = h2;
        k += 1;
    }
    Float::with_val(prec, h1)
}

/// Computes the Associated Legendre Polynomial $P_l^m(x)$ (DLMF §14.3).
///
/// **Algorithm**: Forward three-term recurrence (DLMF §14.10.3):
/// $$ (l-m) P_l^m(x) = x (2l-1) P_{l-1}^m(x) - (l+m-1) P_{l-2}^m(x). $$
/// For $m > 0$ we start from $P_m^m = (-1)^m (2m-1)!! (1 - x^2)^{m/2}$
/// (DLMF §14.3.4), then recurse in $l$.
///
/// **Heuristic Justification (Single-Pass bounds)**:
/// For $|x| \le 1$, the three-term recurrence is known to be numerically stable
/// (DLMF §14.10; *Numerical Recipes* §6.8).  Since the recurrence runs for exactly
/// $l - m$ iterations, the accumulated rounding error grows at most $O(\sqrt{l})$ in
/// the random-walk model (Higham 2002, §16.2).  Evaluation at the target precision
/// without guard bits is therefore safe.
///
/// For negative $m$, the result is derived via $P_l^{-m} = (-1)^m (l-m)!/(l+m)! P_l^m$
/// (DLMF §14.9.3).  The factorial ratio is computed by incremental multiplication
/// or division to avoid overflow.
///
/// **Magic constants**: none — the recurrence is evaluated directly at the input
/// precision with no guard bits.
///
/// **References**:
/// - DLMF §14.3, §14.9.3, §14.10.3
/// - *Numerical Recipes*, 3rd Ed., §6.8 (Associated Legendre Functions)
/// - Higham, N.J. (2002). *Accuracy and Stability of Numerical Algorithms*, 2nd Ed., §16.2
#[allow(
    clippy::many_single_char_names,
    clippy::too_many_lines,
    reason = "Standard math notation; 3-term recurrence over l steps"
)]
pub(super) fn assoc_legendre(l: &IntRepr, m: &IntRepr, v: &BackingFloat) -> BackingFloat {
    let (li, mi) = (l, m);
    if li.is_negative() {
        return nan();
    }
    let m_abs = Integer::from(mi.clone().abs_ref());
    if m_abs > *li {
        return nan();
    }
    let prec = get_precision();
    let work_prec = prec
        .saturating_add(32)
        .saturating_add(li.to_u32().unwrap_or(0).div_euclid(2));
    let v_w = Float::with_val(work_prec, v);
    let x_abs = v_w.clone().abs();
    let one = Float::with_val(work_prec, 1);
    if x_abs > one {
        return nan();
    }
    let two = Float::with_val(work_prec, 2);

    let result = 'blk: {
        if !m_abs.is_zero() {
            let x_sq = Float::with_val(work_prec, &v_w * &v_w);
            let mut p_m_m = one.clone();
            let fact =
                Float::with_val(work_prec, 1 - x_sq).pow(Float::with_val(work_prec, &m_abs) / 2);
            p_m_m *= fact;
            let mut odd = Float::with_val(work_prec, 1);
            let mut cnt = Integer::from(0);
            while cnt < m_abs {
                p_m_m *= Float::with_val(work_prec, -&odd);
                odd += &two;
                cnt += 1;
            }
            if *li == m_abs {
                break 'blk p_m_m;
            }
            let x = v_w;
            let two_m_plus_1 = Float::with_val(work_prec, Integer::from(2 * &m_abs) + 1);
            let p_m_m_plus_1 = Float::with_val(
                work_prec,
                Float::with_val(work_prec, &x * &two_m_plus_1) * &p_m_m,
            );
            if *li == Integer::from(&m_abs + 1) {
                break 'blk p_m_m_plus_1;
            }
            let (mut p_m_m_prev, mut p_m_m_curr) = (p_m_m, p_m_m_plus_1);
            let mut pl = Float::with_val(work_prec, 0);
            let mut ll = Integer::from(&m_abs + 2);
            while ll <= *li {
                let f_ll = Float::with_val(work_prec, &ll);
                let f_m_abs = Float::with_val(work_prec, &m_abs);
                let term1_fact = Float::with_val(work_prec, Integer::from(&ll + &ll) - 1);
                let term2_fact = Float::with_val(work_prec, Integer::from(&ll + &m_abs) - 1);
                pl = (Float::with_val(work_prec, &x * term1_fact * &p_m_m_curr)
                    - Float::with_val(work_prec, term2_fact * &p_m_m_prev))
                    / Float::with_val(work_prec, &f_ll - &f_m_abs);
                p_m_m_prev.clone_from(&p_m_m_curr);
                p_m_m_curr.clone_from(&pl);
                ll += 1;
            }
            break 'blk pl;
        }
        if li.is_zero() {
            break 'blk one.clone();
        }
        if *li == 1 {
            break 'blk v_w;
        }
        let (mut p0, mut p1, mut ll) = (one.clone(), v_w.clone(), Integer::from(2));
        while ll <= *li {
            let f_ll = Float::with_val(work_prec, &ll);
            let term2 = Float::with_val(
                work_prec,
                Float::with_val(
                    work_prec,
                    (Float::with_val(work_prec, &f_ll + &f_ll) - &one) * &v_w,
                ) * &p1,
            );
            let term3 = Float::with_val(work_prec, Float::with_val(work_prec, &f_ll - &one) * &p0);
            let p2 = Float::with_val(
                work_prec,
                Float::with_val(work_prec, &term2 - &term3) / &f_ll,
            );
            p0 = p1;
            p1 = p2;
            ll += 1;
        }
        p1
    };

    if mi.is_negative() && !m_abs.is_zero() && result.is_finite() {
        let sign = if m_abs.is_even() {
            one
        } else {
            Float::with_val(work_prec, -1)
        };
        let diff = Integer::from(li - &m_abs);
        let sum = Integer::from(li + &m_abs);
        let mut ratio = Float::with_val(work_prec, 1);
        let mut j = Integer::from(&diff + 1);
        while j <= sum {
            ratio /= Float::with_val(work_prec, &j);
            j += 1;
        }
        Float::with_val(prec, result * sign * ratio)
    } else {
        Float::with_val(prec, result)
    }
}

/// Computes the real spherical harmonic $Y_l^m(\theta, \phi)$ using the standard
/// normalization (DLMF §14.30.1):
/// $$ Y_l^m(\theta, \phi) = \sqrt{ \frac{2l+1}{4\pi} \frac{(l-m)!}{(l+m)!} }
///    P_l^m(\cos\theta) \cos(m\phi). $$
///
/// The Condon-Shortley phase is embedded in the associated Legendre convention
/// (the $(-1)^m$ factor is part of $P_l^m$ per DLMF §14.3.4).
///
/// **Magic constants**:
/// - `work_prec = prec + 30`: 30 guard bits absorb the accumulation of the
///   factorial ratio (which loses up to $\log_2 (l+m)!$ bits) and the final
///   product with the associated Legendre polynomial.
///
/// **Reference**: DLMF §14.30.1 (Spherical Harmonics).
pub(super) fn spherical_harmonic(
    l: &IntRepr,
    m: &IntRepr,
    theta: &BackingFloat,
    phi: &BackingFloat,
) -> BackingFloat {
    let (li, mi) = (l, m);
    if li.is_negative() {
        return nan();
    }
    let m_abs = Integer::from(mi.clone().abs_ref());
    if m_abs > *li {
        return nan();
    }
    let prec = get_precision();
    let work_prec = prec + 30;

    let theta_w = Float::with_val(work_prec, theta);
    let phi_w = Float::with_val(work_prec, phi);

    let cos_theta = theta_w.cos();
    if cos_theta.is_nan() {
        return nan();
    }
    let plm = assoc_legendre(l, m, &cos_theta);

    let diff = Integer::from(li - mi);
    let sum_fact = Integer::from(li + mi);

    let mut ratio = Float::with_val(work_prec, 1);
    if diff > sum_fact {
        // (l-m)! / (l+m)! where m < 0
        let mut i = Integer::from(&sum_fact + 1);
        while i <= diff {
            ratio = Float::with_val(work_prec, &ratio * Float::with_val(work_prec, &i));
            i += 1;
        }
    } else if diff < sum_fact {
        // (l-m)! / (l+m)! where m > 0
        let mut i = Integer::from(&diff + 1);
        while i <= sum_fact {
            ratio = Float::with_val(work_prec, &ratio / Float::with_val(work_prec, &i));
            i += 1;
        }
    }

    let four = Float::with_val(work_prec, 4);
    let two_l_plus_1 = Float::with_val(work_prec, Integer::from(li + li) + 1);
    let pi = Float::with_val(work_prec, rug::float::Constant::Pi);

    let norm_sq = Float::with_val(
        work_prec,
        Float::with_val(work_prec, two_l_plus_1 / (four * pi)) * ratio,
    );
    let norm = norm_sq.sqrt();

    let m_phi = Float::with_val(work_prec, mi) * phi_w;
    let res = Float::with_val(work_prec, norm * plm * m_phi.cos());
    Float::with_val(prec, res)
}

// ============================================================================
// Bessel I/K — power series for base functions, forward recurrence for order
// ============================================================================

/// Asymptotic expansion of $I_0(x)$ for large $|x|$ (DLMF §10.40.1):
/// $$ I_0(x) \sim \frac{e^x}{\sqrt{2\pi x}} \Bigl[ 1 + \frac{1}{8x}
///    + \frac{9}{128x^2} + \cdots \Bigr]. $$
///
/// The series coefficients are $(2k-1)^2 / (8k)^k$.  Returns `None` if the series
/// starts to diverge (asymptotic series must be truncated at the optimal term).
fn bessel_i0_asymp(x: &Float, prec: u32) -> Option<Float> {
    let pi = Float::with_val(prec, rug::float::Constant::Pi);
    let two = Float::with_val(prec, 2);
    let prefactor_inner = Float::with_val(prec, &two * &pi);
    let prefactor_sqrt = Float::with_val(prec, Float::with_val(prec, &prefactor_inner * x).sqrt());
    let prefactor = Float::with_val(prec, x.clone().exp() / prefactor_sqrt);

    let mut sum = Float::with_val(prec, 1);
    let mut term = Float::with_val(prec, 1);
    let mut k: usize = 1;
    let tol = precision_threshold_with(prec);
    let mut prev_abs = Float::with_val(prec, 1);

    loop {
        let mut num = Integer::from(2 * k - 1);
        num = Integer::from(&num * &num);
        let denom = Float::with_val(prec, 8 * k) * x;
        let factor = Float::with_val(prec, num / denom);
        term = Float::with_val(prec, &term * factor);

        let term_abs = term.clone().abs();
        if term_abs < tol {
            sum = Float::with_val(prec, &sum + &term);
            break;
        }
        if term_abs >= prev_abs {
            return None;
        }
        sum = Float::with_val(prec, &sum + &term);
        prev_abs = term_abs;
        k += 1;
    }
    Some(Float::with_val(prec, sum * prefactor))
}

/// Asymptotic expansion of $I_1(x)$ for large $|x|$ (DLMF §10.40.1):
/// $$ I_1(x) \sim \frac{e^x}{\sqrt{2\pi x}} \Bigl[ 1 - \frac{3}{8x}
///    - \frac{15}{128x^2} - \cdots \Bigr]. $$
///
/// Coefficients: $\bigl((2k-1)^2 - 4\bigr) / (8k)^k$.
fn bessel_i1_asymp(x: &Float, prec: u32) -> Option<Float> {
    let pi = Float::with_val(prec, rug::float::Constant::Pi);
    let two = Float::with_val(prec, 2);
    let prefactor_inner = Float::with_val(prec, &two * &pi);
    let prefactor_sqrt = Float::with_val(prec, Float::with_val(prec, &prefactor_inner * x).sqrt());
    let prefactor = Float::with_val(prec, x.clone().exp() / prefactor_sqrt);

    let mut sum = Float::with_val(prec, 1);
    let mut term = Float::with_val(prec, 1);
    let mut k: usize = 1;
    let tol = precision_threshold_with(prec);
    let mut prev_abs = Float::with_val(prec, 1);

    loop {
        let mut num = Integer::from(2 * k - 1);
        num = Integer::from(&num * &num) - 4;
        let denom = Float::with_val(prec, 8 * k) * x;
        let factor = Float::with_val(prec, num / denom);
        term = Float::with_val(prec, &term * factor);

        let term_abs = term.clone().abs();
        if term_abs < tol {
            sum = Float::with_val(prec, &sum + &term);
            break;
        }
        if term_abs >= prev_abs {
            return None;
        }
        sum = Float::with_val(prec, &sum + &term);
        prev_abs = term_abs;
        k += 1;
    }
    Some(Float::with_val(prec, sum * prefactor))
}

/// Asymptotic expansion of $K_0(x)$ for large $|x|$ (DLMF §10.40.2):
/// $$ K_0(x) \sim \sqrt{\frac{\pi}{2x}} e^{-x} \Bigl[ 1 - \frac{1}{8x}
///    + \frac{9}{128x^2} - \cdots \Bigr]. $$
///
/// The series alternates in sign; coefficients are $(-1)^k (2k-1)^2 / (8k)^k$.
fn bessel_k0_asymp(x: &Float, prec: u32) -> Option<Float> {
    let pi = Float::with_val(prec, rug::float::Constant::Pi);
    let two = Float::with_val(prec, 2);
    let two_x = Float::with_val(prec, &two * x);
    let prefactor_inner = Float::with_val(prec, &pi / &two_x);
    let prefactor_sqrt = Float::with_val(prec, prefactor_inner.sqrt());
    let prefactor = Float::with_val(
        prec,
        prefactor_sqrt * Float::with_val(prec, -x.clone()).exp(),
    );

    let mut sum = Float::with_val(prec, 1);
    let mut term = Float::with_val(prec, 1);
    let mut k: usize = 1;
    let tol = precision_threshold_with(prec);
    let mut prev_abs = Float::with_val(prec, 1);

    loop {
        let mut num = Integer::from(2 * k - 1);
        num = Integer::from(&num * &num);
        let denom = Float::with_val(prec, 8 * k) * x;
        let factor = Float::with_val(prec, num / denom);
        let t_mul = Float::with_val(prec, &term * factor);
        term = Float::with_val(prec, -t_mul);

        let term_abs = term.clone().abs();
        if term_abs < tol {
            sum = Float::with_val(prec, &sum + &term);
            break;
        }
        if term_abs >= prev_abs {
            return None;
        }
        sum = Float::with_val(prec, &sum + &term);
        prev_abs = term_abs;
        k += 1;
    }
    Some(Float::with_val(prec, sum * prefactor))
}

/// Asymptotic expansion of $K_1(x)$ for large $|x|$ (DLMF §10.40.2):
/// $$ K_1(x) \sim \sqrt{\frac{\pi}{2x}} e^{-x} \Bigl[ 1 + \frac{3}{8x}
///    - \frac{15}{128x^2} + \cdots \Bigr]. $$
///
/// Coefficients: $\bigl(4 - (2k-1)^2\bigr) / (8k)^k$.
fn bessel_k1_asymp(x: &Float, prec: u32) -> Option<Float> {
    let pi = Float::with_val(prec, rug::float::Constant::Pi);
    let two = Float::with_val(prec, 2);
    let two_x = Float::with_val(prec, &two * x);
    let prefactor_inner = Float::with_val(prec, &pi / &two_x);
    let prefactor_sqrt = Float::with_val(prec, prefactor_inner.sqrt());
    let prefactor = Float::with_val(
        prec,
        prefactor_sqrt * Float::with_val(prec, -x.clone()).exp(),
    );

    let mut sum = Float::with_val(prec, 1);
    let mut term = Float::with_val(prec, 1);
    let mut k: usize = 1;
    let tol = precision_threshold_with(prec);
    let mut prev_abs = Float::with_val(prec, 1);

    loop {
        let mut num = Integer::from(2 * k - 1);
        num = Integer::from(4) - Integer::from(&num * &num);
        let denom = Float::with_val(prec, 8 * k) * x;
        let factor = Float::with_val(prec, num / denom);
        term = Float::with_val(prec, &term * factor);

        let term_abs = term.clone().abs();
        if term_abs < tol {
            sum = Float::with_val(prec, &sum + &term);
            break;
        }
        if term_abs >= prev_abs {
            return None;
        }
        sum = Float::with_val(prec, &sum + &term);
        prev_abs = term_abs;
        k += 1;
    }
    Some(Float::with_val(prec, sum * prefactor))
}

fn with_prec_val<T>(prec: u32, v: T) -> BackingFloat
where
    Float: rug::ops::AssignRound<T, Round = rug::float::Round, Ordering = Ordering>,
{
    Float::with_val(prec, v)
}

/// Same as `precision_threshold()` but accepts an explicit precision parameter.
fn precision_threshold_with(prec: u32) -> BackingFloat {
    Float::with_val(prec, 1) >> (prec.saturating_sub(10))
}

/// Power series for $I_0(x)$ (DLMF §10.25.2):
/// $$ I_0(x) = \sum_{k=0}^\infty \frac{(x/2)^{2k}}{(k!)^2}. $$
/// Falls back to the asymptotic expansion for large $|x|$ when the series
/// converges too slowly.
fn bessel_i0_with_prec(x: &Float, prec: u32) -> Float {
    let work_prec = prec + 40;
    let x_abs = Float::with_val(work_prec, x.clone().abs());
    if let Some(res) = bessel_i0_asymp(&x_abs, work_prec) {
        return Float::with_val(prec, res);
    }
    let mut sum = Float::with_val(work_prec, 1);
    let mut term = Float::with_val(work_prec, 1);
    let x_half_sq = Float::with_val(work_prec, &x_abs * &x_abs) / 4;
    let tol = precision_threshold_with(work_prec);
    for k in 1..=3000 {
        term *= &x_half_sq;
        term /= Float::with_val(work_prec, k * k);
        sum += &term;
        if term.clone().abs() < tol {
            break;
        }
    }
    Float::with_val(prec, sum)
}

/// Power series for $I_1(x)$ (DLMF §10.25.2):
/// $$ I_1(x) = \frac{x}{2} \sum_{k=0}^\infty \frac{(x/2)^{2k}}{k!(k+1)!}. $$
fn bessel_i1_with_prec(x: &Float, prec: u32) -> Float {
    let work_prec = prec + 40;
    let x_abs = Float::with_val(work_prec, x.clone().abs());
    if let Some(res) = bessel_i1_asymp(&x_abs, work_prec) {
        let final_res = Float::with_val(prec, res);
        return if x.is_sign_negative() {
            -final_res
        } else {
            final_res
        };
    }
    let mut sum = Float::with_val(work_prec, 1);
    let mut term = Float::with_val(work_prec, 1);
    let x_half_sq = Float::with_val(work_prec, &x_abs * &x_abs) / 4;
    let tol = precision_threshold_with(work_prec);
    for k in 1..=3000 {
        term *= &x_half_sq;
        term /= Float::with_val(work_prec, k * (k + 1));
        sum += &term;
        if term.clone().abs() < tol {
            break;
        }
    }
    let x_half = Float::with_val(work_prec, x) / 2;
    Float::with_val(prec, x_half * sum)
}

/// Computes the modified Bessel function of the first kind $I_n(x)$.
///
/// **Algorithm**:
/// - For $n=0,1$: power series or asymptotic series via `bessel_i{0,1}_with_prec`.
/// - For $n \ge 2, x \ge n$: Miller's forward recurrence (DLMF §10.29.1):
///   $$ I_{k-1}(x) = I_{k+1}(x) - \frac{2k}{x} I_k(x). $$
/// - For $n \ge 2, x < n$: Miller's backward recurrence (DLMF §10.74),
///   starting from a computed starting index $N$ where $I_N(x) \approx 0$,
///   then normalizing against a separately computed $I_0(x)$.
///
/// **Magic constants**:
/// - `work_prec = prec + n * clamp(50, 300)`: each forward recurrence step can
///   lose up to ~1 bit of relative accuracy; with $n$ up to 300, 300 guard bits
///   safely absorb all accumulated error.  The `n_abs * 1` scaling is a rough
///   model of the worst-case error growth in the forward recurrence.
///
/// **References**:
/// - DLMF §10.29.1 (forward recurrence), §10.74 (Miller's algorithm)
/// - Olver, F.W.J. (1997). *Asymptotics and Special Functions*, Ch. 10.
pub(super) fn bessel_i(n: &IntRepr, v: &BackingFloat) -> BackingFloat {
    let prec = get_precision();
    let n_abs = Integer::from(n.clone().abs_ref());
    let is_neg = v.is_sign_negative();
    let v_abs = v.clone().abs();

    if n_abs.is_zero() {
        let res = bessel_i0_with_prec(&v_abs, prec + 20);
        return Float::with_val(prec, res);
    }
    if n_abs == 1 {
        let res = bessel_i1_with_prec(&v_abs, prec + 20);
        let final_res = if is_neg { -res } else { res };
        return Float::with_val(prec, final_res);
    }

    // Guard bits scale with n: running forward recurrence in the direction of the
    // decreasing solution (from I0, I1 to In) causes catastrophic cancellation.
    // The precision loss is proportional to log2(I0(x)/In(x)), which is bounded by
    // 1.5 * n for x > n. We allocate 2 * n + 80 guard bits to absorb all lost precision.
    let work_prec = prec
        .saturating_add(n_abs.to_u32().unwrap_or(80).saturating_mul(2))
        .saturating_add(80);
    let v_w = Float::with_val(work_prec, &v_abs);
    let tol = precision_threshold_with(work_prec);
    if v_w < tol {
        return Float::with_val(prec, 0);
    }

    let two = with_prec_val(work_prec, 2);

    let res = if v_w > Float::with_val(work_prec, &n_abs) {
        let i0 = bessel_i0_with_prec(&v_w, work_prec);
        let i1 = bessel_i1_with_prec(&v_w, work_prec);
        let mut i_prev = i0;
        let mut i_curr = i1;
        let mut k = Integer::from(1);
        let mut k_t = with_prec_val(work_prec, 1);

        while k < n_abs {
            let term = with_prec_val(
                work_prec,
                with_prec_val(work_prec, &two * &k_t) / &v_w * &i_curr,
            );
            let ik_plus_1 = with_prec_val(work_prec, &i_prev - term);
            i_prev = i_curr;
            i_curr = ik_plus_1;
            k_t = with_prec_val(work_prec, &k_t + 1);
            k += 1;
        }
        i_curr
    } else {
        let n_start = compute_i_start(&n_abs, &v_w, work_prec);

        let mut i_next = with_prec_val(work_prec, 0);
        let mut i_curr = precision_threshold_with(work_prec);
        let mut result = with_prec_val(work_prec, 0);
        let mut i0_unnorm = with_prec_val(work_prec, 0);

        let mut k = n_start;
        let mut k_t = with_prec_val(work_prec, &k);

        loop {
            let i_prev = with_prec_val(
                work_prec,
                with_prec_val(work_prec, &two * &k_t) / &v_w * &i_curr,
            ) + &i_next;

            if k == n_abs {
                result.clone_from(&i_curr);
            }

            if k.is_zero() {
                i0_unnorm.clone_from(&i_curr);
            }

            i_next = i_curr;
            i_curr = i_prev;
            if k.is_zero() {
                break;
            }
            k -= 1;
            k_t = with_prec_val(work_prec, &k_t - 1);
        }

        let i0_actual = bessel_i0_with_prec(&v_w, work_prec);
        let scale = with_prec_val(work_prec, &i0_actual / &i0_unnorm);
        Float::with_val(work_prec, result * scale)
    };

    if is_neg && n_abs.is_odd() {
        Float::with_val(prec, -res)
    } else {
        Float::with_val(prec, res)
    }
}

/// Computes the starting index $N$ for Miller's backward recurrence of $I_n(x)$.
///
/// **Heuristic Justification (Single-Pass bounds)**:
/// Miller's backward recurrence (DLMF §10.74) assumes $I_N(x) = 0$ for some
/// sufficiently large $N$. The error introduced by this initialization is roughly
/// $$ I_N(x) \sim \frac{(x/2)^N}{N!}. $$
/// To guarantee `prec` bits of accuracy without a Ziv retry loop, we need $N$ such
/// that
/// $$ \log_2(N!) - N \log_2(x/2) > prec + 50. $$
/// Using Stirling's approximation $\log_2(N!) \approx N \log_2 N - N \log_2 e$
/// (Abramowitz & Stegun §6.1.34, DLMF §5.11.1), we solve this inequality with
/// native `f64` arithmetic (53-bit mantissa).  `f64` precision is mathematically
/// sufficient for the bound because:
/// - The error in Stirling's approximation is $O(1/N)$, so asymptotically negligible.
/// - We always overshoot to be safe: the `prec + 50` target and `+20` initial offset
///   guarantee a conservative overestimate.
///
/// **Magic constants**:
/// - `n_start += 20`: initial safety margin ensuring $N > n$ always.
/// - `target = prec + 50`: the 50-bit safety margin compensates for Stirling error
///   and any `f64` rounding (max ~0.5 ulp ≪ 1 bit).
/// - `chunk = max(10, prec / 20)`: adaptive step size.  At high prec, the loop
///   requires fewer passes (larger chunk); at low prec, we use a minimal step of 10.
///
/// **References**:
/// - DLMF §5.11.1 (Stirling's approximation)
/// - DLMF §10.74 (Miller's algorithm for Bessel functions)
/// - Abramowitz & Stegun §9.7.1 (Bessel function asymptotics)
/// - Olver, F.W.J. (1997). *Asymptotics and Special Functions*.
fn compute_i_start(n: &Integer, v: &Float, prec: u32) -> Integer {
    let mut n_start = n.clone().abs();
    let v_abs = v.clone().abs();
    let v_ceil = v_abs
        .clone()
        .ceil()
        .to_integer()
        .unwrap_or_else(|| Integer::from(0));

    if n_start < v_ceil {
        n_start = v_ceil;
    }
    n_start += 20;

    let x_val = v_abs.to_f64();
    let x_bits = if x_val > 0.0 {
        (x_val / 2.0).log2()
    } else {
        0.0
    };
    let log2_e = core::f64::consts::LOG2_E;
    let target = f64::from(prec + 50);

    // Fast approximation of log2(N!) - N * log2(x/2) using Stirling
    loop {
        let n_f = n_start.to_f64();
        let log2_fact = n_f * n_f.log2() - n_f * log2_e;
        if log2_fact - n_f * x_bits > target {
            break;
        }
        let chunk = 10.max(prec.div_euclid(20));
        n_start += chunk;
    }

    n_start
}

/// Evaluates the Modified Bessel Function of the Second Kind $K_0(x)$.
///
/// **Heuristic Justification (Single-Pass bounds)**:
/// Uses the standard identity (DLMF §10.31.1, Watson Ch. 3):
/// $$ K_0(x) = -\ln(x/2) I_0(x) + \sum_{k=0}^\infty \psi(k+1) \frac{(x/2)^{2k}}{(k!)^2}, $$
/// where $\psi$ is the digamma function.  Since $I_0(x) \sim e^x / \sqrt{2\pi x}$ and
/// $K_0(x) \sim e^{-x} \sqrt{\pi / 2x}$, the sum subtracts terms of magnitude $O(e^x)$
/// to produce a result of magnitude $O(e^{-x})$.  This catastrophic cancellation
/// destroys exactly
/// $$ \log_2(e^x / e^{-x}) = 2x \log_2(e) = 2x / \ln 2 $$
/// bits (Watson, Ch. 3, §3.1).  By allocating
/// $$ \text{work\_prec} = \text{prec} + x \cdot (2 / \ln 2) + 50 $$
/// bits upfront, we perfectly absorb the cancellation in a single pass — no
/// Ziv retry loop is needed.  The `+ 50` guard covers the series summation error
/// (sum of `k` terms, each contributing < 1 ulp, bounded by ~50 ulp for typical `k`).
///
/// **References**:
/// - DLMF §10.31.1 (series for $K_0$)
/// - Watson, G.N. (1944). *A Treatise on the Theory of Bessel Functions*, Ch. 3.
/// - DLMF §10.40.2 (asymptotic expansion, used for large $x$ fallback).
fn bessel_k0(x: &Float) -> Float {
    let orig_prec = x.prec();
    let work_prec = if *x > 0 {
        let mut ln2 = Float::with_val(orig_prec, 2);
        ln2 = ln2.ln();
        let factor = Float::with_val(orig_prec, Float::with_val(orig_prec, 2) / &ln2);
        let extra = Float::with_val(orig_prec, x) * factor;
        let extra_int = extra
            .ceil()
            .to_integer()
            .unwrap_or_else(|| Integer::from(0));
        let extra_u32 = extra_int.to_u32().unwrap_or(u32::MAX - orig_prec - 50);
        orig_prec.saturating_add(extra_u32).saturating_add(50)
    } else {
        orig_prec + 50
    };

    let x_w = Float::with_val(work_prec, x);

    if let Some(res) = bessel_k0_asymp(&x_w, work_prec) {
        return Float::with_val(orig_prec, res);
    }
    let two_w = Float::with_val(work_prec, 2);

    let i0 = bessel_i0_with_prec(&x_w, work_prec);
    let gamma = Float::with_val(work_prec, rug::float::Constant::Euler);
    let x_half = with_prec_val(work_prec, &x_w / &two_w);
    let ln_term = with_prec_val(
        work_prec,
        -(with_prec_val(work_prec, x_half.clone().ln()) + &gamma),
    ) * &i0;

    let t = with_prec_val(work_prec, &x_half * &x_half);
    let mut sum = with_prec_val(work_prec, 0);
    let mut term = with_prec_val(work_prec, 1);
    let mut k = Integer::from(1);
    let mut h = with_prec_val(work_prec, 0);
    let tol = precision_threshold_with(work_prec);

    loop {
        let kf = with_prec_val(work_prec, &k);
        h = with_prec_val(
            work_prec,
            &h + with_prec_val(work_prec, with_prec_val(work_prec, 1) / &kf),
        );
        term = with_prec_val(work_prec, &term * &t) / with_prec_val(work_prec, &kf * &kf);
        let delta = with_prec_val(work_prec, &h * &term);
        let prev = sum.clone();
        sum = with_prec_val(work_prec, &sum + &delta);
        if with_prec_val(work_prec, &sum - &prev).abs() < tol {
            break;
        }
        k += 1;
    }
    let res = with_prec_val(work_prec, ln_term + sum);
    Float::with_val(orig_prec, res)
}

/// Modified Bessel function of the second kind, order 1 (DLMF §10.31.1):
/// $$ K_1(x) = \ln(x/2) I_1(x) + \frac{1}{x}
///    - \sum_{k=0}^\infty \bigl(\psi(k+1) + \psi(k+2)\bigr) \frac{(x/2)^{2k}}{k!(k+1)!}. $$
///
/// Same cancellation analysis as $K_0$ applies: $2x/\ln 2$ bits destroyed.
fn bessel_k1(x: &Float) -> Float {
    let orig_prec = x.prec();
    let work_prec = if *x > 0 {
        let mut ln2 = Float::with_val(orig_prec, 2);
        ln2 = ln2.ln();
        let factor = Float::with_val(orig_prec, Float::with_val(orig_prec, 2) / &ln2);
        let extra = Float::with_val(orig_prec, x) * factor;
        let extra_int = extra
            .ceil()
            .to_integer()
            .unwrap_or_else(|| Integer::from(0));
        let extra_u32 = extra_int.to_u32().unwrap_or(u32::MAX - orig_prec - 50);
        orig_prec.saturating_add(extra_u32).saturating_add(50)
    } else {
        orig_prec + 50
    };

    let x_w = Float::with_val(work_prec, x);
    if let Some(res) = bessel_k1_asymp(&x_w, work_prec) {
        return Float::with_val(orig_prec, res);
    }
    let two_w = Float::with_val(work_prec, 2);

    let i1 = bessel_i1_with_prec(&x_w, work_prec);
    let gamma = Float::with_val(work_prec, rug::float::Constant::Euler);
    let x_half = with_prec_val(work_prec, &x_w / &two_w);
    let ln_term = with_prec_val(
        work_prec,
        with_prec_val(work_prec, x_half.clone().ln()) + &gamma,
    ) * &i1;

    let t = with_prec_val(work_prec, &x_half * &x_half);
    let mut sum = with_prec_val(work_prec, 0);
    let mut term = with_prec_val(work_prec, 1);
    let mut k = Integer::from(1);
    let mut h_k = with_prec_val(work_prec, 0);
    let tol = precision_threshold_with(work_prec);

    loop {
        let kf = with_prec_val(work_prec, &k);
        let kp1 = with_prec_val(work_prec, Integer::from(&k + 1));

        h_k = with_prec_val(work_prec, &h_k + with_prec_val(work_prec, 1) / &kf);
        let h_kp1 = with_prec_val(work_prec, &h_k + with_prec_val(work_prec, 1) / &kp1);
        let h_sum = with_prec_val(work_prec, &h_k + &h_kp1);

        term = with_prec_val(work_prec, &term * &t) / with_prec_val(work_prec, &kf * &kp1);
        let delta = with_prec_val(work_prec, &h_sum * &term);
        let prev = sum.clone();
        sum = with_prec_val(work_prec, &sum + &delta);
        if with_prec_val(work_prec, &sum - &prev).abs() < tol {
            break;
        }
        k += 1;
    }

    let x_fourth = with_prec_val(work_prec, &x_half / 2);
    let res = with_prec_val(work_prec, 1) / &x_w + &ln_term
        - &x_fourth
        - with_prec_val(work_prec, x_fourth * sum);
    Float::with_val(orig_prec, res)
}

/// Computes the modified Bessel function of the second kind $K_n(x)$.
///
/// **Algorithm**: uses the forward recurrence (DLMF §10.29.1):
/// $$ K_{k+1}(x) = K_{k-1}(x) + \frac{2k}{x} K_k(x), $$
/// starting from $K_0$ and $K_1$ computed by `bessel_k0` / `bessel_k1`.
/// The forward recurrence for $K_n$ is numerically stable (no cancellation
/// because $K_n(x) > 0$ for $x > 0$).
///
/// **Magic constants**:
/// - `work_prec = prec + 20`: 20 guard bits absorb the recurrence
///   accumulation (each of the $n$ steps contributes < 1 ulp).
///
/// **Reference**: DLMF §10.29.1 (recurrence), §10.31 (series).
pub(super) fn bessel_k(n: &IntRepr, v: &BackingFloat) -> BackingFloat {
    if v.is_sign_negative() || v.is_zero() {
        return nan();
    }
    let n_abs = Integer::from(n.clone().abs_ref());
    let prec = get_precision();
    let work_prec = prec + 20;

    let v_w = Float::with_val(work_prec, v);
    let k0 = bessel_k0(&v_w);
    if n_abs.is_zero() {
        return Float::with_val(prec, k0);
    }
    let k1 = bessel_k1(&v_w);
    if n_abs == 1 {
        return Float::with_val(prec, k1);
    }

    let two = with_prec_val(work_prec, 2);
    let (mut k_prev, mut k_curr) = (k0, k1);
    let mut k = Integer::from(1);
    let mut k_t = with_prec_val(work_prec, 1);

    while k < n_abs {
        let kn = with_prec_val(
            work_prec,
            &k_prev
                + with_prec_val(
                    work_prec,
                    with_prec_val(work_prec, &two * &k_t) / &v_w * &k_curr,
                ),
        );
        k_prev = k_curr;
        k_curr = kn;
        k_t = with_prec_val(work_prec, &k_t + 1);
        k += 1;
    }
    Float::with_val(prec, k_curr)
}

// ============================================================================
// Zeta derivative — Laurent + Dirichlet series
// ============================================================================

/// Compute $n!$ as a `Float` at the given precision via direct multiplication.
fn factorial_rug(prec: u32, n: usize) -> Float {
    let mut f = Float::with_val(prec, 1);
    for j in 2..=n {
        f = Float::with_val(prec, &f * Float::with_val(prec, j));
    }
    f
}

// =========================================================================
// Taylor Series helpers for arbitrary precision
// =========================================================================

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
pub(super) fn zeta_deriv(n: &IntRepr, v: &BackingFloat) -> BackingFloat {
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
