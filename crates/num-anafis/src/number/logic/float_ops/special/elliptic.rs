//! Complete elliptic integrals K(m) and E(m) via AGM.
//!
//! Here the argument is the *parameter* $m = k^2$ (not the modulus $k$).
//!
//! Reference: DLMF §19.8

use super::SpecFloat;

/// K(m) = ∫₀^(π/2) dθ / √(1 - m sin²θ) — AGM algorithm.
// AGM for K(m): DLMF §19.8.5
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

/// E(m) = ∫₀^(π/2) √(1 - m sin²θ) dθ — AGM with corrections.
// AGM with corrections for E(m): DLMF §19.8.6
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
