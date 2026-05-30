#![allow(
    clippy::doc_markdown,
    reason = "LaTeX math notation in $...$ / $$...$$ is not recognized by clippy"
)]
//! Complete Elliptic Integrals via the Arithmetic-Geometric Mean (AGM).
use super::super::{BackingFloat, get_precision, nan};
use super::bessel::precision_threshold_with;
use rug::Float;
use rug::float::Special;

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
pub fn elliptic_k(v: &BackingFloat) -> BackingFloat {
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
pub fn elliptic_e(v: &BackingFloat) -> BackingFloat {
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
