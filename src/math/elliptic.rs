use crate::core::traits::MathScalar;

/// Complete elliptic integral of the first kind K(k)
///
/// K(k) = ∫₀^(π/2) dθ / √(1 - k² sin²θ)
/// Uses the arithmetic-geometric mean (AGM) algorithm: K(k) = π/(2 · AGM(1, √(1-k²)))
///
/// Reference: DLMF §19.8 <https://dlmf.nist.gov/19.8>
#[allow(
    clippy::unnecessary_wraps,
    reason = "API consistency requires Option return type"
)]
pub fn eval_elliptic_k<T: MathScalar>(k: T) -> Option<T> {
    let one = T::one();
    if k.abs() >= one {
        return Some(T::infinity());
    }
    let mut a = one;
    let mut b = (one - k * k).sqrt();

    let two = T::from(2.0).expect("Failed to convert mathematical constant");
    let tolerance = T::from(1e-15).expect("Failed to convert mathematical constant");

    for _ in 0..25 {
        let an = (a + b) / two;
        let bn = (a * b).sqrt();
        a = an;
        b = bn;
        if (a - b).abs() < tolerance {
            break;
        }
    }
    let pi = T::PI();
    Some(pi / (two * a))
}

/// Complete elliptic integral of the second kind E(k)
///
/// E(k) = ∫₀^(π/2) √(1 - k² sin²θ) dθ
/// Uses the AGM algorithm with correction terms.
///
/// Reference: DLMF §19.8 <https://dlmf.nist.gov/19.8>
#[allow(
    clippy::unnecessary_wraps,
    reason = "API consistency requires Option return type"
)]
pub fn eval_elliptic_e<T: MathScalar>(k: T) -> Option<T> {
    let one = T::one();
    if k.abs() > one {
        return Some(T::nan());
    }
    let mut a = one;
    let mut b = (one - k * k).sqrt();

    let k2 = k * k;
    let mut sum = one - k2 / T::from(2.0).expect("Failed to convert mathematical constant");
    let mut pow2 = T::from(0.5).expect("Failed to convert mathematical constant");
    let two = T::from(2.0).expect("Failed to convert mathematical constant");
    let tolerance = T::from(1e-15).expect("Failed to convert mathematical constant");

    for _ in 0..25 {
        let an = (a + b) / two;
        let bn = (a * b).sqrt();
        let cn = (a - b) / two;
        sum -= pow2 * cn * cn;
        a = an;
        b = bn;
        pow2 *= two;
        if cn.abs() < tolerance {
            break;
        }
    }
    let pi = T::PI();
    Some(pi / (two * a) * sum)
}
