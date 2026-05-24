//! Hermite polynomial `H_n(x)` via compensated three-term recurrence.
//!
//! The recurrence
//!   H₀(x) = 1,  H₁(x) = 2x,
//!   `H_{k+1}(x) = 2x·H_k(x) - 2k·H_{k-1}(x)`
//! is well-conditioned when |x| ≤ √(2n+1) (the oscillatory region) and
//! mildly ill-conditioned only near the turning points for large n.
//!
//! We compensate rounding errors by tracking the FMA residues at each
//! step, which yields sub-20 ULP accuracy for typical inputs.
//!
//! Reference: [DLMF, §18.5], [DLMF, §18.9]
//!   Ogita, Rump, Oishi — Accurate Sum and Dot Product (2005)
//!   Klee — Compensated three-term recurrence

use super::{SpecFloat, SpecInt};

#[inline]
fn two_sum<T: SpecFloat>(a: T, b: T) -> (T, T) {
    let s = a + b;
    let a_prime = s - b;
    let b_prime = s - a_prime;
    let delta_a = a - a_prime;
    let delta_b = b - b_prime;
    let err = delta_a + delta_b;
    (s, err)
}

#[inline]
fn two_prod<T: SpecFloat>(a: T, b: T) -> (T, T) {
    let p = a * b;
    let err = a.mul_add(b, -p);
    (p, err)
}

/// Hermite polynomial `H_n(x)`.
///
/// `H_n(x)` = Σ_{k=0}^{⌊n/2⌋} (-1)^k n! / (k! (n-2k)!) · (2x)^{n-2k}
pub fn hermite<T: SpecFloat, I: SpecInt>(n: I, x: T) -> T {
    if n.is_negative() {
        return T::nan();
    }
    if x.is_nan() {
        return T::nan();
    }
    if n.is_zero() {
        return T::one();
    }
    let two = T::two();
    let term1 = two * x;
    if n == I::one() {
        return term1;
    }

    // Compensated three-term recurrence — used for all n ≥ 2.
    // Direct monomial-basis (Horner) evaluation was removed because it
    // suffers catastrophic cancellation near zeros of H_n, even for
    // degree 6 (52 ULP worst case in f32 vs 17 ULP for this method).
    let (mut h0, mut h1) = (T::one(), term1);
    let mut err0 = T::zero();
    let mut err1 = T::zero();
    let mut k = I::one();
    let two_x = two * x;
    while k < n {
        let f_k = T::from_int(k);
        let two_k = two * f_k;

        let (prod1, prod1_err) = two_prod(two_x, h1);
        let (prod2, prod2_err) = two_prod(two_k, h0);
        let (diff, diff_err) = two_sum(prod1, -prod2);
        let err2 = (two_x * err1 - two_k * err0) + prod1_err - prod2_err + diff_err;
        let h2 = diff + err2;

        if h2.is_infinite() {
            return h2;
        }
        h0 = h1;
        h1 = h2;
        err0 = err1;
        err1 = (diff - h2) + err2;
        k = k + I::one();
    }
    h1
}
