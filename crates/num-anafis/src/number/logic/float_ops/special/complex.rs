//! Complex tuple arithmetic — `(re, im)` pairs.
//!
//! Used by `zeta_deriv` for the Cauchy integral and complex functional equation.
//! All functions operate on `C<T> = (T, T)` and build only on [`SpecFloat`].

use super::SpecFloat;

/// Complex type alias for readability.
pub(super) type C<T> = (T, T);

/// Complex addition
#[inline]
pub(super) fn cadd<T: SpecFloat>(a: C<T>, b: C<T>) -> C<T> {
    (a.0 + b.0, a.1 + b.1)
}

/// Complex multiplication: (a+bi)(c+di) = (ac-bd) + (ad+bc)i
#[inline]
pub(super) fn cmul<T: SpecFloat>(a: C<T>, b: C<T>) -> C<T> {
    // Use mul_add for precision: ac - bd, ad + bc
    let re = a.0.mul_add(b.0, -(a.1 * b.1));
    let im = a.0.mul_add(b.1, a.1 * b.0);
    (re, im)
}

/// Complex division: (a+bi)/(c+di)
#[inline]
pub(super) fn cdiv<T: SpecFloat>(a: C<T>, b: C<T>) -> C<T> {
    let denom = b.0.mul_add(b.0, b.1 * b.1);
    let re = a.0.mul_add(b.0, a.1 * b.1) / denom;
    let im = a.1.mul_add(b.0, -(a.0 * b.1)) / denom;
    (re, im)
}

/// Complex absolute value |a+bi|
#[inline]
pub(super) fn cabs<T: SpecFloat>(z: C<T>) -> T {
    (z.0 * z.0 + z.1 * z.1).sqrt()
}

/// Complex exponential exp(a+bi) = e^a (cos b + i sin b)
#[inline]
pub(super) fn cexp<T: SpecFloat>(z: C<T>) -> C<T> {
    let r = z.0.exp();
    (r * z.1.cos(), r * z.1.sin())
}

/// Complex natural logarithm ln(a+bi) = ln|z| + i·arg(z)
#[inline]
pub(super) fn clog<T: SpecFloat>(z: C<T>) -> C<T> {
    let r = cabs(z);
    let arg = z.1.atan2(z.0);
    (r.ln(), arg)
}

/// Complex sin(a+bi) = sin(a)cosh(b) + i·cos(a)sinh(b)
#[inline]
pub(super) fn csin<T: SpecFloat>(z: C<T>) -> C<T> {
    let (sa, ca) = (z.0.sin(), z.0.cos());
    let (shb, chb) = sinh_cosh(z.1);
    (sa * chb, ca * shb)
}

// =========================================================================
// Complex gamma via Lanczos
// =========================================================================

/// Complex log-gamma via Lanczos approximation.
/// Returns ln(Γ(z)) for Re(z) > 0.5.
pub(super) fn clgamma<T: SpecFloat>(z: C<T>) -> C<T> {
    let half = T::half();
    let one = T::one();
    let coeffs = T::lanczos_coeffs();
    let g = T::from_usize(coeffs.len() - 2);

    // z_shifted = z - 1
    let zm1 = (z.0 - one, z.1);

    // Lanczos sum: coeffs[0] + Σ coeffs[k] / (zm1 + k)
    let c0 = coeffs.first().copied().unwrap_or_else(T::zero);
    let mut ag = (c0, T::zero());
    for (k, &ck) in coeffs.iter().enumerate().skip(1) {
        let denom = (zm1.0 + T::from_usize(k), zm1.1);
        let term = cdiv((ck, T::zero()), denom);
        ag = cadd(ag, term);
    }

    // t = zm1 + g + 0.5
    let t = (zm1.0 + g + half, zm1.1);
    let ln_t = clog(t);

    // ln(Γ(z)) = 0.5·ln(2π) + (zm1 + 0.5)·ln(t) - t + ln(ag)
    let sqrt_2pi_ln = (T::two() * T::pi()).sqrt().ln();
    let zm1_plus_half = (zm1.0 + half, zm1.1);
    let term1 = cmul(zm1_plus_half, ln_t);
    let ln_ag = clog(ag);

    (
        sqrt_2pi_ln + term1.0 - t.0 + ln_ag.0,
        term1.1 - t.1 + ln_ag.1,
    )
}

/// Complex gamma Γ(z) via exp(clgamma(z)), with reflection for Re(z) < 0.5.
pub(super) fn cgamma<T: SpecFloat>(z: C<T>) -> C<T> {
    let half = T::half();
    if z.0 >= half {
        cexp(clgamma(z))
    } else {
        // Reflection: Γ(z) = π / (sin(πz) · Γ(1-z))
        let pi = T::pi();
        let one_minus_z = (T::one() - z.0, -z.1);
        let g1mz = cexp(clgamma(one_minus_z));
        let sin_pz = csin((pi * z.0, pi * z.1));
        let denom = cmul(sin_pz, g1mz);
        cdiv((pi, T::zero()), denom)
    }
}

// =========================================================================
// Internal helpers
// =========================================================================

/// sinh(x) and cosh(x) via exp, using only `SpecFloat`.
#[inline]
fn sinh_cosh<T: SpecFloat>(x: T) -> (T, T) {
    let ep = x.exp();
    let em = (-x).exp();
    let half = T::half();
    ((ep - em) * half, (ep + em) * half)
}
