#![allow(
    clippy::doc_markdown,
    reason = "LaTeX math notation in $...$ / $$...$$ is not recognized by clippy"
)]
//! Orthogonal polynomials (Hermite, Associated Legendre, Spherical Harmonics).
use super::super::{BackingFloat, get_precision, nan};
use crate::number::logic::int_math::IntType;
use rug::ops::Pow;
use rug::{Float, Integer};

/// Computes the Hermite polynomial $H_n(x)$ via the forward recurrence (DLMF §18.9.1, Table 18.9.1):
/// $$ H_{k+1}(x) = 2x H_k(x) - 2k H_{k-1}(x), \quad H_0=1, H_1=2x. $$
/// The recurrence is numerically stable for all real $x$ at the target precision
/// because it is a finite linear recurrence with no cancellation between large
/// terms (the coefficients are all non-negative when $H_k(x)$ has the dominant
/// sign, which holds for $x \ge 0$; for $x < 0$ the alternating sign pattern
/// likewise preserves accuracy).
///
/// **Reference**: DLMF §18.9.1, Table 18.9.1 (Hermite recurrence); DLMF Table 18.3 (values).
pub fn hermite(n: &IntType, v: &BackingFloat) -> BackingFloat {
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
#[allow(clippy::too_many_lines, reason = "complex algorithmic logic")]
pub fn assoc_legendre(l: &IntType, m: &IntType, v: &BackingFloat) -> BackingFloat {
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
pub fn spherical_harmonic(
    l: &IntType,
    m: &IntType,
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
