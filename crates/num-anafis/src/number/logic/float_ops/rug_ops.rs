#![allow(
    clippy::cast_possible_truncation,
    clippy::manual_is_multiple_of,
    reason = "Bessel order must be i32 for MPFR's jn/yn"
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

fn precision_threshold() -> BackingFloat {
    let prec = get_precision();
    Float::with_val(prec, 1) >> (prec.saturating_sub(10))
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
    value.to_integer()
}

pub(super) fn to_rational(value: &BackingFloat) -> Option<RationalRepr> {
    if !value.is_finite() {
        return None;
    }
    // rug::Float to rug::Rational is exact
    value.to_rational()
}

#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    reason = "prec is strictly positive"
)]
pub(super) fn to_string(value: &BackingFloat) -> String {
    let digits = (f64::from(value.prec()) * core::f64::consts::LOG10_2) as usize;
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
pub(super) fn lgamma(value: &BackingFloat) -> BackingFloat {
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

const ARG_SHIFT: i32 = 100;

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

#[allow(clippy::many_single_char_names, reason = "Standard math notation")]
pub(super) fn polygamma(n: &IntRepr, value: &BackingFloat) -> BackingFloat {
    let ni = n;
    if value.is_sign_negative() || value.is_zero() {
        return nan();
    }
    if ni.is_zero() {
        return digamma(value);
    }

    let neg_np1 = -Integer::from(ni + 1);

    let mut x = value.clone();
    let mut s = with_val(0);
    let shift = with_val(ARG_SHIFT);
    let one = with_val(1);
    while x < shift {
        s += with_val(rug::ops::Pow::pow(x.clone(), &neg_np1));
        x += &one;
    }

    // Factorial: (n-1)! computed as n! / n
    let mut factorial_n = with_val(1);
    let mut k_fact = Integer::from(2);
    while k_fact <= *ni {
        factorial_n *= with_val(&k_fact);
        k_fact += 1;
    }
    let factorial_nm1 = with_val(&factorial_n / with_val(ni));

    // DLMF 5.15.2:  ψ⁽ⁿ⁾(z) ~ (-1)^(n-1) * [ (n-1)! / zⁿ + n!/(2·zⁿ⁺¹) + sum ]
    let n_int = Integer::from(ni);
    let n1 = Integer::from(ni + 1);

    // Leading: (n-1)! / xⁿ
    let mut sum = with_val(factorial_nm1 / with_val(rug::ops::Pow::pow(x.clone(), &n_int)));

    // Second term: n! / (2 · xⁿ⁺¹)
    let half_fact = with_val(&factorial_n / 2);
    sum += with_val(half_fact / with_val(rug::ops::Pow::pow(x.clone(), &n1)));

    let tol = precision_threshold();
    let max_k = 40;
    let b_evens = bernoulli_even_up_to(max_k);

    let mut prod_n = Integer::from(ni);
    prod_n *= Integer::from(ni + 1);
    let mut fact_2k = Integer::from(2);

    for k in 1..=max_k {
        let b2k = with_val(b_evens.get(k - 1).expect("b_evens has enough elements"));
        let pow_val = with_val(rug::ops::Pow::pow(
            x.clone(),
            Integer::from(ni) + Integer::from(2 * k),
        ));
        let term =
            with_val(with_val(b2k * with_val(&prod_n)) / with_val(with_val(&fact_2k) * pow_val));

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

    let sign = with_val(if ni.is_odd() { 1 } else { -1 });
    with_val(sign * (with_val(factorial_n.clone() * s) + sum))
}

// ============================================================================
// Lambert W — Halley iteration (Corless et al., 1996)
// ============================================================================

pub(super) fn lambert_w(value: &BackingFloat) -> BackingFloat {
    // Domain: [-1/e, ∞).   -1/e ≈ -0.367879...
    let e = Float::with_val(get_precision(), 1).exp();
    let neg_inv_e = with_val(-1) / &e;
    if *value < neg_inv_e {
        return nan();
    }

    let one = with_val(1);
    let two = with_val(2);

    // Piecewise initial estimate
    let w = if *value > 3 {
        let ln_v = value.clone().ln();
        with_val(ln_v.clone() - ln_v.ln())
    } else if with_val(value * 2) > 1 {
        let log1p_v = with_val(value + &one).ln();
        with_val(&log1p_v / 2)
    } else {
        value.clone()
    };
    let mut w = w;
    let thr = precision_threshold();
    loop {
        let ew = w.clone().exp();
        let wew = with_val(&w * &ew);
        let num = with_val(&wew - value);
        let wp1 = with_val(&w + &one);
        // Halley: w -= (w*e^w - z) / (e^w*(w+1) - (w+2)*(w*e^w - z)/(2*(w+1)))
        let denom =
            with_val(&ew * &wp1 - with_val(with_val(&wp1 + &one) * &num) / with_val(&two * &wp1));
        let correction = with_val(&num / &denom);
        let new_w = with_val(&w - &correction);
        if with_val(&new_w - &w).abs() < thr {
            break;
        }
        w = new_w;
    }
    w
}

/// W₋₁(x) — lower real branch, defined for x ∈ [-1/e, 0). Returns W ≤ -1.
pub(super) fn lambert_wm1(value: &BackingFloat) -> BackingFloat {
    let e = Float::with_val(get_precision(), 1).exp();
    let neg_inv_e = with_val(-1) / &e;
    if *value < neg_inv_e || *value >= 0 {
        return nan();
    }
    let tol = precision_threshold();
    let delta = with_val(value + &neg_inv_e);
    if delta.abs() < tol {
        return with_val(-1);
    }

    let one = with_val(1);
    let two = with_val(2);
    // Initial guess: log-log near 0, branch-point expansion near -1/e
    let w = if *value > with_val(with_val(-1) / 100) {
        let neg_v = with_val(-value);
        let l1 = neg_v.ln();
        let l2 = with_val(-l1.clone()).ln();
        with_val(l1.clone() - l2.clone() + l2 / l1)
    } else {
        let e_const = Float::with_val(get_precision(), 1).exp();
        let ev = with_val(&e_const * value);
        let p = with_val(two.clone() * with_val(&ev + &one)).sqrt();
        with_val(-one.clone() - p)
    };

    let mut w = w;
    let thr = precision_threshold();
    loop {
        let ew = w.clone().exp();
        let wew = with_val(&w * &ew);
        let num = with_val(&wew - value);
        let wp1 = with_val(&w + &one);
        let denom =
            with_val(&ew * &wp1 - with_val(with_val(&wp1 + &one) * &num) / with_val(&two * &wp1));
        let correction = with_val(&num / &denom);
        let new_w = with_val(&w - &correction);
        if with_val(&new_w - &w).abs() < thr {
            break;
        }
        w = new_w;
    }
    w
}

// ============================================================================
// Beta — exact via lgamma
// ============================================================================

pub(super) fn beta(a: &BackingFloat, b: &BackingFloat) -> BackingFloat {
    let lga = a.clone().ln_gamma();
    let lgb = b.clone().ln_gamma();
    let lgapb = with_val(a + b).ln_gamma();
    with_val(with_val(lga + lgb - lgapb).exp())
}

// ============================================================================
// Special functions implemented directly on rug::Float
//
// References:
// - DLMF (NIST): §§5.10, 5.11, 5.15, 4.13, 10.6, 10.8, 10.29, 14, 18.5, 19.8, 25.2
// - Corless et al. (1996). "On the Lambert W function." Adv. Comput. Math. 5, 329–359.
// - Borwein, Bradley & Crandall (2000). "Computational Strategies for the Riemann
//   Zeta Function." J. Comput. Appl. Math. 121, 247–285.
// ============================================================================

/// Complete elliptic integral K(m) via AGM (parameter m = k^2).
pub(super) fn elliptic_k(v: &BackingFloat) -> BackingFloat {
    let m = v.clone();
    let one = with_val(1);
    if m > one {
        return nan();
    }
    if m == one {
        return Float::with_val(get_precision(), Special::Infinity);
    }
    let mut a = one.clone();
    let mut b = with_val(&one - &m).sqrt();
    let two = with_val(2);
    let tol = precision_threshold();

    for _ in 0..100 {
        let an = with_val(with_val(&a + &b) / &two);
        let bn = with_val(&a * &b).sqrt();
        a = an;
        b = bn;
        if with_val(&a - &b).abs() < tol {
            break;
        }
    }
    let pi = Float::with_val(get_precision(), rug::float::Constant::Pi);
    with_val(pi / (two * a))
}

/// Complete elliptic integral E(m) via AGM with corrections (parameter m = k^2).
pub(super) fn elliptic_e(v: &BackingFloat) -> BackingFloat {
    let m = v.clone();
    let one = with_val(1);
    if m > one {
        return nan();
    }
    if m == one {
        return one;
    }
    let mut a = one.clone();
    let mut b = with_val(&one - &m).sqrt();
    let two = with_val(2);
    let mut sum = with_val(with_val(&one + with_val(&b * &b)) / &two);
    let mut pow2 = with_val(1);
    let tol = precision_threshold();

    for _ in 0..100 {
        let an = with_val(with_val(&a + &b) / &two);
        let bn = with_val(&a * &b).sqrt();
        let cn = with_val(with_val(&a - &b) / &two);
        sum = with_val(&sum - with_val(&pow2 * with_val(&cn * &cn)));
        a = an;
        b = bn;
        pow2 = with_val(&pow2 * &two);
        if cn.abs() < tol {
            break;
        }
    }
    let pi = Float::with_val(get_precision(), rug::float::Constant::Pi);
    with_val(pi / (two * a) * sum)
}

/// Hermite polynomial `H_n(x)` via recurrence.
pub(super) fn hermite(n: &IntRepr, v: &BackingFloat) -> BackingFloat {
    let ni = n;
    if ni.is_zero() {
        return with_val(1);
    }
    let x = v.clone();
    let two = with_val(2);
    let term1 = with_val(&two * &x);
    if *ni == 1 {
        return term1;
    }
    let one = with_val(1);
    let (mut h0, mut h1) = (one, term1);
    let mut k = Integer::from(1);
    while k < *ni {
        let f_k = with_val(&k);
        let h2 = with_val(with_val(&two * &x) * &h1 - with_val(with_val(&two * &f_k) * &h0));
        h0 = h1;
        h1 = h2;
        k += 1;
    }
    h1
}

/// Associated Legendre polynomial `P_l^m(x)` via recurrence.
#[allow(clippy::many_single_char_names, reason = "Standard math notation")]
#[allow(
    clippy::too_many_lines,
    clippy::integer_division,
    clippy::cast_precision_loss,
    reason = "Recurrence kept together; integer seed estimate; exact power-of-two cast"
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
    let x = v.clone();
    let x_abs = x.clone().abs();
    let one = with_val(1);
    if x_abs > one {
        return nan();
    }
    let two = with_val(2);

    let result = 'blk: {
        // Forward recurrence for all |x| <= 1 with m > 0
        if !m_abs.is_zero() {
            let sqx = (with_val(&one - &x) * with_val(&one + &x)).sqrt();
            let mut pmm = one.clone();
            let mut fact = with_val(1);
            let mut cnt = Integer::from(0);
            while cnt < m_abs {
                pmm = with_val(&pmm * with_val(-&fact) * &sqx);
                fact = with_val(&fact + &two);
                cnt += 1;
            }
            if *li == m_abs {
                break 'blk pmm;
            }

            let two_m_plus_1 = with_val(Integer::from(&m_abs + &m_abs) + 1);
            let pmmp1 = with_val(&x * two_m_plus_1 * &pmm);

            if *li == Integer::from(&m_abs + 1) {
                break 'blk pmmp1;
            }

            let (mut pmm_prev, mut pmm_curr) = (pmm, pmmp1);
            let mut pll = with_val(0);
            let mut ll = Integer::from(&m_abs + 2);

            while ll <= *li {
                let f_ll = with_val(&ll);
                let f_m_abs = with_val(&m_abs);
                let term1_fact = with_val(Integer::from(&ll + &ll) - 1);
                let term2_fact = with_val(Integer::from(&ll + &m_abs) - 1);
                let denom = with_val(&f_ll - &f_m_abs);

                pll = with_val(
                    (with_val(&x * term1_fact * &pmm_curr) - with_val(term2_fact * &pmm_prev))
                        / denom,
                );
                pmm_prev.clone_from(&pmm_curr);
                pmm_curr.clone_from(&pll);
                ll += 1;
            }
            break 'blk pll;
        }

        if li.is_zero() {
            break 'blk one.clone();
        }
        if *li == 1 {
            break 'blk x;
        }

        let mut p0 = with_val(1);
        let mut p1 = x.clone();
        let mut ll = Integer::from(2);
        while ll <= *li {
            let f_ll = with_val(&ll);
            let two_ll = with_val(&f_ll + &f_ll);
            let two_ll_minus_1 = with_val(&two_ll - &one);
            let ll_minus_1 = with_val(&f_ll - &one);
            let term1 = with_val(&two_ll_minus_1 * &x);
            let term2 = with_val(&term1 * &p1);
            let term3 = with_val(&ll_minus_1 * &p0);
            let div1 = with_val(&term2 / &f_ll);
            let div2 = with_val(&term3 / &f_ll);
            let p2 = with_val(&div1 - &div2);
            p0 = p1;
            p1 = p2;
            ll += 1;
        }
        p1
    };

    if mi.is_negative() && !m_abs.is_zero() && result.is_finite() {
        let sign = if m_abs.is_even() {
            one.clone()
        } else {
            with_val(-1)
        };
        let diff = Integer::from(li - &m_abs);
        let sum = Integer::from(li + &m_abs);
        let mut ratio = one;
        let mut j = Integer::from(&diff + 1);
        while j <= sum {
            ratio = with_val(&ratio / with_val(&j));
            j += 1;
        }
        with_val(result * sign * ratio)
    } else {
        result
    }
}

/// Spherical harmonic `Y_l^m(theta, phi)`.
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
    let cos_theta = theta.clone().cos();
    if cos_theta.is_nan() {
        return nan();
    }
    let plm = assoc_legendre(l, m, &cos_theta);

    let diff = Integer::from(li - mi);
    let sum_fact = Integer::from(li + mi);

    let mut ratio = with_val(1);
    if diff > sum_fact {
        // (l-m)! / (l+m)! where m < 0
        let mut i = Integer::from(&sum_fact + 1);
        while i <= diff {
            ratio = with_val(&ratio * with_val(&i));
            i += 1;
        }
    } else if diff < sum_fact {
        // (l-m)! / (l+m)! where m > 0
        let mut i = Integer::from(&diff + 1);
        while i <= sum_fact {
            ratio = with_val(&ratio / with_val(&i));
            i += 1;
        }
    }

    let four = with_val(4);
    let two_l_plus_1 = with_val(Integer::from(li + li) + 1);
    let pi = Float::with_val(get_precision(), rug::float::Constant::Pi);

    let norm_sq = with_val(with_val(two_l_plus_1 / (four * pi)) * ratio);
    let norm = norm_sq.sqrt();

    let m_phi = with_val(mi) * phi;
    with_val(norm * plm * m_phi.cos())
}

// ============================================================================
// Bessel I/K — power series for base functions, forward recurrence for order
// ============================================================================

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

fn precision_threshold_with(prec: u32) -> BackingFloat {
    Float::with_val(prec, 1) >> (prec.saturating_sub(10))
}

/// Power series for the modified Bessel function of the first kind, order 0.
fn bessel_i0_with_prec(x: &Float, prec: u32) -> Float {
    if let Some(res) = bessel_i0_asymp(x, prec) {
        return res;
    }
    let one = with_prec_val(prec, 1);
    let x_half = with_prec_val(prec, x / 2);
    let t = with_prec_val(prec, &x_half * &x_half);
    let mut sum = one.clone();
    let mut term = one;
    let mut k = Integer::from(1);
    let tol = precision_threshold_with(prec);

    loop {
        let k_val = with_prec_val(prec, &k);
        term = with_prec_val(prec, &term * &t) / with_prec_val(prec, &k_val * &k_val);
        let prev_sum = sum.clone();
        sum = with_prec_val(prec, &sum + &term);
        if with_prec_val(prec, &sum - &prev_sum).abs() < tol {
            break;
        }
        k += 1;
    }
    sum
}

/// Power series for the modified Bessel function of the first kind, order 1.
fn bessel_i1_with_prec(x: &Float, prec: u32) -> Float {
    if let Some(res) = bessel_i1_asymp(x, prec) {
        return res;
    }
    let one = with_prec_val(prec, 1);
    let x_half = with_prec_val(prec, x / 2);
    let t = with_prec_val(prec, &x_half * &x_half);
    let mut sum = one.clone();
    let mut term = one;
    let mut k = Integer::from(1);
    let tol = precision_threshold_with(prec);

    loop {
        let k_val = with_prec_val(prec, &k);
        let kp1 = with_prec_val(prec, Integer::from(&k + 1));
        term = with_prec_val(prec, &term * &t) / with_prec_val(prec, &k_val * &kp1);
        let prev_sum = sum.clone();
        sum = with_prec_val(prec, &sum + &term);
        if with_prec_val(prec, &sum - &prev_sum).abs() < tol {
            break;
        }
        k += 1;
    }
    with_prec_val(prec, x_half * sum)
}

pub(super) fn bessel_i(n: &IntRepr, v: &BackingFloat) -> BackingFloat {
    let prec = v.prec();
    let n_abs = Integer::from(n.clone().abs_ref());
    if n_abs.is_zero() {
        return bessel_i0_with_prec(v, prec);
    }
    if n_abs == 1 {
        let i1 = bessel_i1_with_prec(v, prec);
        return if n.is_negative() { -i1 } else { i1 };
    }

    let tol = precision_threshold_with(prec);
    if v.clone().abs() < tol {
        return with_prec_val(prec, 0);
    }

    let two = with_prec_val(prec, 2);
    let n_start = compute_i_start(&n_abs, v, prec);

    let mut i_next = with_prec_val(prec, 0);
    let mut i_curr = precision_threshold_with(prec);
    let mut result = with_prec_val(prec, 0);
    let mut sum = with_prec_val(prec, 0);

    let mut k = n_start;
    let mut k_t = with_prec_val(prec, &k);

    loop {
        let i_prev = with_prec_val(prec, with_prec_val(prec, &two * &k_t) / v * &i_curr) + &i_next;

        if k == n_abs {
            result.clone_from(&i_curr);
        }

        if k.is_zero() {
            sum = with_prec_val(prec, &sum + &i_curr);
        } else if Integer::from(&k % 2).is_zero() {
            sum = with_prec_val(prec, &sum + with_prec_val(prec, &two * &i_curr));
        }

        i_next = i_curr;
        i_curr = i_prev;
        if k.is_zero() {
            break;
        }
        k -= 1;
        k_t = with_prec_val(prec, &k_t - 1);
    }

    let i0_actual = bessel_i0_with_prec(v, prec);
    let scale = with_prec_val(prec, &i0_actual / &sum);
    let ans = with_prec_val(prec, result * scale);

    if n.is_negative() && (Integer::from(&n_abs % 2) == 1) {
        -ans
    } else {
        ans
    }
}

fn compute_i_start(n: &Integer, v: &Float, prec: u32) -> Integer {
    let n_f = Float::with_val(53, n);
    let v_abs = Float::with_val(53, v.clone().abs());
    let max_nv = if n_f > v_abs { n_f } else { v_abs };
    let sq = Float::with_val(53, prec) * &max_nv;
    let sq_root = sq
        .sqrt()
        .ceil()
        .to_integer()
        .unwrap_or_else(|| Integer::from(0));
    let v_ceil = v
        .clone()
        .abs()
        .ceil()
        .to_integer()
        .unwrap_or_else(|| Integer::from(0));
    let m = if n > &v_ceil { n.clone() } else { v_ceil };
    m + sq_root + 20
}

fn bessel_k0(x: &Float) -> Float {
    let orig_prec = x.prec();
    if let Some(res) = bessel_k0_asymp(x, orig_prec) {
        return res;
    }
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
    let two = Float::with_val(work_prec, 2);

    let i0 = bessel_i0_with_prec(&x_w, work_prec);
    let gamma = Float::with_val(work_prec, rug::float::Constant::Euler);
    let x_half = with_prec_val(work_prec, &x_w / &two);
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

/// Modified Bessel function of the second kind, order 1.
fn bessel_k1(x: &Float) -> Float {
    let orig_prec = x.prec();
    if let Some(res) = bessel_k1_asymp(x, orig_prec) {
        return res;
    }
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
    let two = Float::with_val(work_prec, 2);

    let i0 = bessel_i0_with_prec(&x_w, work_prec);
    let i1 = bessel_i1_with_prec(&x_w, work_prec);
    let gamma = Float::with_val(work_prec, rug::float::Constant::Euler);
    let x_half = with_prec_val(work_prec, &x_w / &two);
    let one = with_prec_val(work_prec, 1);

    let t = with_prec_val(work_prec, &x_half * &x_half);
    let mut sum = with_prec_val(work_prec, 0);
    let mut term = with_prec_val(work_prec, 1);
    let mut k = Integer::from(1);
    let mut h = with_prec_val(work_prec, 0);
    let tol = precision_threshold_with(work_prec);

    loop {
        let kf = with_prec_val(work_prec, &k);
        let kp1 = with_prec_val(work_prec, Integer::from(&k + 1));
        h = with_prec_val(
            work_prec,
            &h + with_prec_val(work_prec, with_prec_val(work_prec, 1) / &kf)
                + with_prec_val(work_prec, with_prec_val(work_prec, 1) / &kp1),
        );
        term = with_prec_val(work_prec, &term * &t) / with_prec_val(work_prec, &kf * &kp1);
        let delta = with_prec_val(work_prec, &h * &term);
        let prev = sum.clone();
        sum = with_prec_val(work_prec, &sum + &delta);
        if with_prec_val(work_prec, &sum - &prev).abs() < tol {
            break;
        }
        k += 1;
    }

    // k0, k1 asymptotic boundary uses half
    let half = with_prec_val(work_prec, with_prec_val(work_prec, 1) / 2);
    let imag = -(with_prec_val(work_prec, x_half.clone().ln()) + &gamma) + &half;
    let res = with_prec_val(work_prec, one / &x_w)
        + with_prec_val(
            work_prec,
            &x_half * with_prec_val(work_prec, &imag * &i0 - &i1),
        )
        + with_prec_val(work_prec, &x_half * sum);
    Float::with_val(orig_prec, res)
}

pub(super) fn bessel_k(n: &IntRepr, v: &BackingFloat) -> BackingFloat {
    if v.is_sign_negative() || v.is_zero() {
        return nan();
    }
    let n_abs = Integer::from(n.clone().abs_ref());
    let k0 = bessel_k0(v);
    if n_abs.is_zero() {
        return k0;
    }
    let k1 = bessel_k1(v);
    if n_abs == 1 {
        return k1;
    }
    let prec = v.prec();
    let two = with_prec_val(prec, 2);
    let (mut k_prev, mut k_curr) = (k0, k1);
    let mut k = Integer::from(1);
    let mut k_t = with_prec_val(prec, 1);

    while k < n_abs {
        let kn = with_prec_val(
            prec,
            &k_prev + with_prec_val(prec, with_prec_val(prec, &two * &k_t) / v * &k_curr),
        );
        k_prev = k_curr;
        k_curr = kn;
        k_t = with_prec_val(prec, &k_t + 1);
        k += 1;
    }
    k_curr
}

// ============================================================================
// Zeta derivative — Laurent + Dirichlet series
// ============================================================================

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
        if 2 * r >= bernoullis.len() {
            break;
        }

        let bern_rat = bernoullis
            .get(2 * r)
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

#[allow(clippy::indexing_slicing, reason = "arbitrary precision math helpers")]
fn zeta_deriv_internal(n: usize, s: &Float, prec: u32) -> Float {
    let one = Float::with_val(prec, 1);
    let delta = Float::with_val(prec, s - &one);
    let tenth = Float::with_val(prec, Float::with_val(prec, 1) / 10);

    if delta.clone().abs() < tenth {
        let extra_terms = usize::try_from(prec >> 2).unwrap_or(30);
        let order = n + extra_terms;

        let e_coeffs = eta_taylor_rug(order, &one, prec);

        let ln2 = Float::with_val(prec, 2).ln();
        let mut a_coeffs = alloc::vec![Float::with_val(prec, 0); order + 1];
        let mut current_ln2_pow = ln2.clone();
        let mut current_fact = Float::with_val(prec, 1);

        for (k, a_coeff) in a_coeffs.iter_mut().enumerate() {
            let sign = if k % 2 == 0 {
                one.clone()
            } else {
                Float::with_val(prec, -1)
            };
            let num = Float::with_val(prec, &sign * &current_ln2_pow);
            *a_coeff = Float::with_val(prec, &num / &current_fact);

            current_ln2_pow = Float::with_val(prec, &current_ln2_pow * &ln2);
            let k_plus_2 = Float::with_val(prec, k + 2);
            current_fact = Float::with_val(prec, &current_fact * &k_plus_2);
        }

        let mut c_coeffs = alloc::vec![Float::with_val(prec, 0); order + 1];
        let a0 = a_coeffs[0].clone();
        for m in 0..=order {
            let mut sum_cj_amj = Float::with_val(prec, 0);
            for j in 0..m {
                sum_cj_amj += Float::with_val(prec, &c_coeffs[j] * &a_coeffs[m - j]);
            }
            let diff = Float::with_val(prec, &e_coeffs[m] - &sum_cj_amj);
            c_coeffs[m] = Float::with_val(prec, &diff / &a0);
        }

        let n_fact = factorial_rug(prec, n);
        let pole_sign = if n % 2 == 0 {
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
            let c_val = &c_coeffs[n + j + 1];
            let term_coeff = Float::with_val(prec, c_val * &running_binom);
            let delta_j = Float::with_val(prec, rug::ops::Pow::pow(delta.clone(), &Integer::from(j)));
            
            let term = Float::with_val(prec, &term_coeff * &delta_j);
            let abs_term = term.clone().abs();
            series_sum += term;
            if abs_term < Float::with_val(prec, &series_sum.clone().abs() * &tenth) && abs_term < precision_threshold_with(prec) {
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

pub(super) fn zeta_deriv(n: &IntRepr, v: &BackingFloat) -> BackingFloat {
    if n.is_zero() {
        return v.clone().zeta();
    }
    let Some(n_usize) = n.to_usize() else {
        return Float::with_val(get_precision(), rug::float::Special::Nan);
    };
    let prec = get_precision();
    let s = v.clone();
    let one = Float::with_val(prec, 1);
    let tenth = Float::with_val(prec, Float::with_val(prec, 1) / 10);
    let delta = Float::with_val(prec, &s - &one);

    if s > one || delta.abs() < tenth {
        return zeta_deriv_internal(n_usize, &s, prec);
    }

    // Reflection for s <= 0.9 via Leibniz rule on functional equation:
    // \zeta(s) = 2 (2\pi)^{s-1} \sin(\pi s / 2) \Gamma(1-s) \zeta(1-s)
    let two_pi = Float::with_val(prec, 2.0 * Float::with_val(prec, rug::float::Constant::Pi));
    let ln_2pi = two_pi.clone().ln();

    // A(s) = 2 (2\pi)^{s-1}
    let mut a_derivs = Vec::with_capacity(n_usize + 1);
    let a_base = Float::with_val(prec, 2.0 * two_pi.pow(Float::with_val(prec, &s - &one)));
    let mut cur_factor = Float::with_val(prec, 1.0);
    for _ in 0..=n_usize {
        a_derivs.push(Float::with_val(prec, &a_base * &cur_factor));
        cur_factor *= &ln_2pi;
    }

    // B(s) = \sin(\pi s / 2)
    let pi_half = Float::with_val(prec, Float::with_val(prec, rug::float::Constant::Pi) / 2);
    let mut b_derivs = Vec::with_capacity(n_usize + 1);
    let mut cur_factor_b = Float::with_val(prec, 1);
    for k in 0..=n_usize {
        let angle = Float::with_val(
            prec,
            &pi_half * &s + Float::with_val(prec, &pi_half * &Float::with_val(prec, k)),
        );
        b_derivs.push(Float::with_val(prec, angle.sin() * &cur_factor_b));
        cur_factor_b *= &pi_half;
    }

    // C(s) = \Gamma(1-s)
    let one_minus_s = Float::with_val(prec, &one - &s);
    let mut c_derivs = Vec::with_capacity(n_usize + 1);
    c_derivs.push(one_minus_s.clone().gamma());
    let mut g_derivs = Vec::with_capacity(n_usize + 1);
    g_derivs.push(Float::with_val(prec, 0));
    for m in 1..=n_usize {
        let psi_val = polygamma(
            &crate::number::logic::int_math::from_i64(i64::try_from(m - 1).expect("fits"))
                .expect("m-1 fits"),
            &one_minus_s,
        );
        let sign = if m % 2 == 0 { 1 } else { -1 };
        g_derivs.push(Float::with_val(prec, sign * psi_val));
    }
    for k in 0..n_usize {
        let mut sum = Float::with_val(prec, 0);
        let mut binom = Integer::from(1);
        for j in 0..=k {
            if j > 0 {
                binom *= Integer::from(k + 1 - j);
                binom /= Integer::from(j);
            }
            sum += Float::with_val(
                prec,
                c_derivs.get(j).expect("c_derivs has j")
                    * g_derivs.get(k - j + 1).expect("g_derivs has k-j+1"),
            ) * &binom;
        }
        c_derivs.push(sum);
    }

    // D(s) = \zeta(1-s)
    let mut d_derivs = Vec::with_capacity(n_usize + 1);
    for k in 0..=n_usize {
        let z_val = zeta_deriv_internal(k, &one_minus_s, prec);
        let sign = if k.is_multiple_of(2) { 1.0 } else { -1.0 };
        d_derivs.push(Float::with_val(prec, sign * z_val));
    }

    let ab = mul_derivs(&a_derivs, &b_derivs, n_usize, prec);
    let abc = mul_derivs(&ab, &c_derivs, n_usize, prec);
    let abcd = mul_derivs(&abc, &d_derivs, n_usize, prec);

    abcd.get(n_usize).expect("abcd has n elements").clone()
}
