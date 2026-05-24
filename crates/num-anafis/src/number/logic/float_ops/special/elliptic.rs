//! Complete elliptic integrals K(m) and E(m) via AGM.
//!
//! Here the argument is the *parameter* $m = k^2$ (not the modulus $k$).
//!
//! Reference: [DLMF, §19.8]

use super::SpecFloat;

/// Complete elliptic integral of the first kind `K(m)`.
///
/// `K(m) = ∫₀^{π/2} dθ / √(1 − m sin²θ)`
///
/// Computed via the arithmetic–geometric mean (AGM):
/// iterate `a ← (a+b)/2`, `b ← √(ab)` until convergence,
/// then `K(m) = π / (2·a_final)`.
///
/// Reference: [DLMF, §19.8.5]
pub fn elliptic_k<T: SpecFloat>(m: T) -> T {
    let one = T::one();
    if m.is_nan() || m > one {
        return T::nan();
    }
    #[allow(clippy::float_cmp, reason = "Exact comparison for pole at m=1")]
    if m == one {
        return T::infinity();
    }

    let mut a = one;
    let mut b = (one - m).sqrt();
    let two = T::two();

    while (a - b).abs() > T::eps() * a {
        let an = (a + b) / two;
        let bn = (a * b).sqrt();
        a = an;
        b = bn;
    }
    T::pi() / (two * a)
}

/// Complete elliptic integral of the second kind `E(m)`.
///
/// `E(m) = ∫₀^{π/2} √(1 − m sin²θ) dθ`
///
/// Computed via the AGM with a correction sum that tracks
/// `c_n = (a_n − b_n)/2` at each step:
/// `E(m) = π / (2·a_final) · (1 − Σ 2^{k−1}·c_k²)`.
///
/// Reference: [DLMF, §19.8.6]
pub fn elliptic_e<T: SpecFloat>(m: T) -> T {
    let one = T::one();
    if m.is_nan() || m > one {
        return T::nan();
    }
    #[allow(clippy::float_cmp, reason = "Exact comparison for endpoint m=1")]
    if m == one {
        return one;
    }
    let mut a = one;
    let mut b = (one - m).sqrt();
    let two = T::two();
    let mut sum = (one + b * b) / two;
    let mut pow2 = T::one();

    while (a - b).abs() > T::eps() * a {
        let an = (a + b) / two;
        let bn = (a * b).sqrt();
        let cn = (a - b) / two;
        sum = sum - pow2 * cn * cn;
        a = an;
        b = bn;
        pow2 = pow2 * two;
    }
    T::pi() / (two * a) * sum
}
