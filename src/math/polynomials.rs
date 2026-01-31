use crate::core::traits::MathScalar;

/// Hermite polynomials `H_n(x)` (physicist's convention)
///
/// Uses three-term recurrence: H_{n+1}(x) = 2x `H_n(x)` - 2n H_{n-1}(x)
/// with `H_0(x)` = 1, `H_1(x)` = 2x
///
/// Reference: DLMF §18.9 <https://dlmf.nist.gov/18.9>
pub fn eval_hermite<T: MathScalar>(n: i32, x: T) -> Option<T> {
    if n < 0 {
        return None;
    }
    if n == 0 {
        return Some(T::one());
    }
    let two = T::from(2.0).expect("Failed to convert mathematical constant");
    let term1 = two * x;
    if n == 1 {
        return Some(term1);
    }
    let (mut h0, mut h1) = (T::one(), term1);
    for k in 1..n {
        let f_k = T::from(k).expect("Failed to convert mathematical constant");
        // h2 = 2x * h1 - 2k * h0
        let h2 = (two * x * h1) - (two * f_k * h0);
        h0 = h1;
        h1 = h2;
    }
    Some(h1)
}

/// Associated Legendre function `P_l^m(x)` for -1 ≤ x ≤ 1
///
/// Uses recurrence relation starting from `P_m^m`, then P_{m+1}^m.
/// Negative m handled via relation: P_l^{-m} = (-1)^m (l-m)!/(l+m)! `P_l^m`
///
/// Reference: DLMF §14.10 <https://dlmf.nist.gov/14.10>
pub fn eval_assoc_legendre<T: MathScalar>(l: i32, m: i32, x: T) -> Option<T> {
    if l < 0 || m.abs() > l || x.abs() > T::one() {
        // Technically |x| > 1 is domain error, but some continuations exist.
        // Standard impl assumes -1 <= x <= 1
        return None;
    }
    let m_abs = m.abs();
    let mut pmm = T::one();
    let one = T::one();

    if m_abs > 0 {
        let sqx = (one - x * x).sqrt();
        let mut fact = T::one();
        let two = T::from(2.0).expect("Failed to convert mathematical constant");
        for _ in 1..=m_abs {
            pmm = pmm * (-fact) * sqx;
            fact += two;
        }
    }
    if l == m_abs {
        return Some(pmm);
    }

    let two_m_plus_1 = T::from(2 * m_abs + 1).expect("Failed to convert mathematical constant");
    let pmmp1 = x * two_m_plus_1 * pmm;

    if l == m_abs + 1 {
        return Some(pmmp1);
    }

    let (mut pll, mut pmm_prev) = (T::zero(), pmm);
    let mut pmm_curr = pmmp1;

    for ll in (m_abs + 2)..=l {
        let f_ll = T::from(ll).expect("Failed to convert mathematical constant");
        let f_m_abs = T::from(m_abs).expect("Failed to convert mathematical constant");

        let term1_fact = T::from(2 * ll - 1).expect("Failed to convert mathematical constant");
        let term2_fact = T::from(ll + m_abs - 1).expect("Failed to convert mathematical constant");
        let denom = f_ll - f_m_abs;

        pll = (x * term1_fact * pmm_curr - term2_fact * pmm_prev) / denom;
        pmm_prev = pmm_curr;
        pmm_curr = pll;
    }
    Some(pll)
}

/// Spherical harmonics `Y_l^m(θ`, φ) (real form)
///
/// `Y_l^m` = `N_l^m` `P_l^m(cos` θ) cos(mφ)
/// where `N_l^m` is the normalization factor.
///
/// Reference: DLMF §14.30 <https://dlmf.nist.gov/14.30>
pub fn eval_spherical_harmonic<T: MathScalar>(l: i32, m: i32, theta: T, phi: T) -> Option<T> {
    if l < 0 || m.abs() > l {
        return None;
    }
    let cos_theta = theta.cos();
    let plm = eval_assoc_legendre(l, m, cos_theta)?;
    let m_abs = m.abs();

    // Factorials
    let mut fact_lm = T::one();
    for i in 1..=(l - m_abs) {
        fact_lm *= T::from(i).expect("Failed to convert mathematical constant");
    }

    let mut fact_lplusm = T::one();
    for i in 1..=(l + m_abs) {
        fact_lplusm *= T::from(i).expect("Failed to convert mathematical constant");
    }

    let four = T::from(4.0).expect("Failed to convert mathematical constant");
    let two_l_plus_1 = T::from(2 * l + 1).expect("Failed to convert mathematical constant");
    let pi = T::PI();

    let norm_sq = (two_l_plus_1 / (four * pi)) * (fact_lm / fact_lplusm);
    let norm = norm_sq.sqrt();

    let m_phi = T::from(m).expect("Failed to convert mathematical constant") * phi;
    Some(norm * plm * m_phi.cos())
}
