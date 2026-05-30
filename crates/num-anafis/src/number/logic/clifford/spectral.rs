use core::cmp::Ordering;

use super::super::scalar::Scalar;
use super::super::traits::Number;
use super::CliffordNumber;
use super::matrix::{self, Cmplx, Mat2C};
use super::types::{scalar_nan, scalar_zero};

use super::large_mat;

// ============================================================================
// Helper: apply a Cmplx → Cmplx function via spectral decomposition
// ============================================================================

fn apply_via_spectral<F>(mv: &CliffordNumber, f: &F) -> CliffordNumber
where
    F: Fn(&Cmplx) -> Cmplx,
{
    let gens = mv.generator_set();

    // Path 1: ≤3 generators via Mat(2,ℂ) spectral decomposition
    if let Some(mat) = matrix::to_matrix(mv) {
        if let Some((u, u_inv, l1, l2)) = matrix::eigendecompose(&mat) {
            let fd = Mat2C::new(f(&l1), Cmplx::zero(), Cmplx::zero(), f(&l2));
            let result_mat = u.mul(&fd).mul(&u_inv);
            return matrix::from_matrix_with_gens(&result_mat, gens);
        }
        return nan_clifford(gens);
    }

    // Path 2: ≥4 generators via custom eigendecomposition
    if gens.len() >= 4 {
        return large_mat::apply_via_spectral(mv, f);
    }

    nan_clifford(gens)
}

fn extract_scalar(mv: &CliffordNumber) -> Scalar {
    let blade0 = mv.coeff(0);
    if blade0.is_zero() {
        scalar_zero()
    } else {
        blade0.clone()
    }
}

/// Returns `true` when the multivector has only a scalar (grade-0) coefficient.
fn is_pure_scalar(mv: &CliffordNumber) -> bool {
    for blade in 1..mv.blade_count() {
        if !mv.coeff(blade).is_zero() {
            return false;
        }
    }
    true
}

/// Apply a real-scalar function `f` to a `CliffordNumber`.
///
/// - Pure scalar → delegates to `Scalar::f`.
/// - Cl(3,0,0) or subset → uses spectral decomposition with `apply_real_only`
///   (applies only to the real part of each eigenvalue).
/// - Other algebras → returns `NaN`.
fn apply_real_scalar_fn<F>(mv: &CliffordNumber, f: &F) -> CliffordNumber
where
    F: Fn(&Scalar) -> Scalar,
{
    if is_pure_scalar(mv) {
        let s = f(mv.coeff(0));
        let mut out = CliffordNumber::zero_unchecked(mv.gens.clone());
        out.coeffs_mut_slice()[0] = s;
        out
    } else if matrix::can_embed(&mv.gens) {
        apply_via_spectral(mv, &|c: &Cmplx| c.apply_real_only(f))
    } else {
        nan_clifford(&mv.gens)
    }
}

/// Apply a real-scalar binary function `f(order, x)` to a `CliffordNumber`.
///
/// Same guard logic as `apply_real_scalar_fn` but with an extracted scalar order.
fn apply_real_binary_fn<F>(mv: &CliffordNumber, order: &CliffordNumber, f: &F) -> CliffordNumber
where
    F: Fn(&Scalar, &Scalar) -> Scalar,
{
    if !is_pure_scalar(order) {
        return nan_clifford(&mv.gens);
    }
    let n = extract_scalar(order);
    if is_pure_scalar(mv) {
        let s = f(mv.coeff(0), &n);
        let mut out = CliffordNumber::zero_unchecked(mv.gens.clone());
        out.coeffs_mut_slice()[0] = s;
        out
    } else if matrix::can_embed(&mv.gens) {
        apply_via_spectral(mv, &|c: &Cmplx| c.apply_real_only(&|s| f(s, &n)))
    } else {
        nan_clifford(&mv.gens)
    }
}

/// Create a `CliffordNumber` whose scalar coefficient is NaN, for IEEE 754 propagation.
pub fn nan_clifford(gens: &super::types::GeneratorSet) -> CliffordNumber {
    let mut mv = CliffordNumber::zero_unchecked(gens.clone());
    mv.coeffs_mut_slice()[0] = scalar_nan();
    mv
}

// ============================================================================
// Geometric inverse via spectral decomposition
// ============================================================================

impl CliffordNumber {
    /// Geometric inverse via spectral decomposition.
    /// For embeddable algebras (≤3 generators, or ≥4 with custom matrices),
    /// computes A⁻¹ = U · diag(1/λᵢ) · U⁻¹.
    /// For other algebras (nilpotent, too many gens), returns
    /// a NaN-valued Clifford number for IEEE 754 error propagation.
    #[must_use]
    pub fn geometric_inverse(&self) -> Self {
        let gens = self.generator_set();

        // Path 1: ≤3 generators via Mat(2,ℂ)
        if let Some(mat) = matrix::to_matrix(self) {
            if let Some((u, u_inv, l1, l2)) = matrix::eigendecompose(&mat) {
                let one = Cmplx::one();
                let inv_l1 = one.div(&l1);
                let inv_l2 = one.div(&l2);
                let inv_d = Mat2C::new(inv_l1, Cmplx::zero(), Cmplx::zero(), inv_l2);
                let result_mat = u.mul(&inv_d).mul(&u_inv);
                return matrix::from_matrix_with_gens(&result_mat, gens);
            }
            return nan_clifford(gens);
        }

        // Path 2: ≥4 generators via custom implementation
        if gens.len() >= 4 {
            return large_mat::apply_via_spectral(self, &|c: &Cmplx| {
                let one = Cmplx::one();
                one.div(c)
            });
        }

        nan_clifford(gens)
    }
}

// ============================================================================
// Number trait implementation for CliffordNumber
// ============================================================================

impl Number for CliffordNumber {
    // =====================================================================
    // Trigonometric
    // =====================================================================

    fn sin(&self) -> Self {
        apply_via_spectral(self, &Cmplx::sin)
    }
    fn cos(&self) -> Self {
        apply_via_spectral(self, &Cmplx::cos)
    }
    fn tan(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.sin().div(&c.cos()))
    }
    fn cot(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.cos().div(&c.sin()))
    }
    fn sec(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| Cmplx::one().div(&c.cos()))
    }
    fn csc(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| Cmplx::one().div(&c.sin()))
    }

    fn asin(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.asin()))
    }
    fn acos(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.acos()))
    }
    fn atan(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.atan()))
    }
    fn acot(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.acot()))
    }
    fn asec(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.asec()))
    }
    fn acsc(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.acsc()))
    }

    // =====================================================================
    // Hyperbolic
    // =====================================================================

    fn sinh(&self) -> Self {
        apply_via_spectral(self, &Cmplx::sinh)
    }
    fn cosh(&self) -> Self {
        apply_via_spectral(self, &Cmplx::cosh)
    }
    fn tanh(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.sinh().div(&c.cosh()))
    }
    fn coth(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.cosh().div(&c.sinh()))
    }
    fn sech(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| Cmplx::one().div(&c.cosh()))
    }
    fn csch(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| Cmplx::one().div(&c.sinh()))
    }

    fn asinh(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.asinh()))
    }
    fn acosh(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.acosh()))
    }
    fn atanh(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.atanh()))
    }
    fn acoth(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.acoth()))
    }
    fn asech(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.asech()))
    }
    fn acsch(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.acsch()))
    }

    // =====================================================================
    // Exponential & Logarithmic
    // =====================================================================

    fn exp(&self) -> Self {
        apply_via_spectral(self, &Cmplx::exp)
    }
    fn expm1(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.exp().sub(&Cmplx::one()))
    }
    fn exp_neg(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.neg().exp())
    }
    fn ln(&self) -> Self {
        apply_via_spectral(self, &Cmplx::ln)
    }
    fn log1p(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.add(&Cmplx::one()).ln())
    }

    // =====================================================================
    // Powers & Roots
    // =====================================================================

    fn sqrt(&self) -> Self {
        apply_via_spectral(self, &Cmplx::sqrt)
    }
    fn cbrt(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.cbrt()))
    }

    // =====================================================================
    // Basic Math
    // =====================================================================

    fn abs(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| Cmplx::new(c.abs(), scalar_zero()))
    }

    fn signum(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| {
            let a = c.abs();
            if a.is_zero() {
                Cmplx::zero()
            } else {
                Cmplx::new(c.0.clone() / &a, c.1.clone() / &a)
            }
        })
    }

    fn floor(&self) -> Self {
        apply_real_scalar_fn(self, &|s: &Scalar| s.floor())
    }
    fn ceil(&self) -> Self {
        apply_real_scalar_fn(self, &|s: &Scalar| s.ceil())
    }
    fn round(&self) -> Self {
        apply_real_scalar_fn(self, &|s: &Scalar| s.round())
    }
    fn fract(&self) -> Self {
        apply_real_scalar_fn(self, &|s: &Scalar| s.fract())
    }
    fn negate(&self) -> Self {
        -self.clone()
    }

    // =====================================================================
    // Special Functions — real-only on eigenvalues
    // =====================================================================

    fn erf(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.erf()))
    }
    fn erfc(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.erfc()))
    }
    fn gamma(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.gamma()))
    }
    fn lgamma(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.lgamma()))
    }
    fn digamma(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.digamma()))
    }
    fn trigamma(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.trigamma()))
    }
    fn tetragamma(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.tetragamma()))
    }

    fn sinc(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| {
            if c.0.is_zero() && c.1.is_zero() {
                Cmplx::one()
            } else {
                c.sin().div(c)
            }
        })
    }

    fn elliptic_k(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.elliptic_k()))
    }
    fn elliptic_e(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.elliptic_e()))
    }
    fn zeta(&self) -> Self {
        apply_via_spectral(self, &|c: &Cmplx| c.apply_real_only(&|s| s.zeta()))
    }
    fn exp_polar(&self) -> Self {
        self.exp()
    }

    // =====================================================================
    // Binary / multi-arg
    // =====================================================================

    fn atan2(&self, x: &Self) -> Self {
        if is_pure_scalar(self) && is_pure_scalar(x) {
            let s = self.coeff(0).atan2(x.coeff(0));
            let mut mv = Self::zero_unchecked(self.gens.clone());
            mv.coeffs_mut_slice()[0] = s;
            mv
        } else {
            nan_clifford(&self.gens)
        }
    }

    fn log_base(&self, base: &Self) -> Self {
        if is_pure_scalar(self) && is_pure_scalar(base) {
            let s = self.coeff(0).log_base(base.coeff(0));
            let mut mv = Self::zero_unchecked(self.gens.clone());
            mv.coeffs_mut_slice()[0] = s;
            mv
        } else {
            nan_clifford(&self.gens)
        }
    }

    fn pow(&self, exp: &Self) -> Self {
        if is_pure_scalar(self) && is_pure_scalar(exp) {
            let s = self.coeff(0).pow(exp.coeff(0));
            let mut mv = Self::zero_unchecked(self.gens.clone());
            mv.coeffs_mut_slice()[0] = s;
            mv
        } else if is_pure_scalar(exp) && matrix::can_embed(&self.gens) {
            let exp_s = extract_scalar(exp);
            apply_via_spectral(self, &|c: &Cmplx| {
                let log_c = c.ln();
                log_c.scale(&exp_s).exp()
            })
        } else {
            nan_clifford(&self.gens)
        }
    }

    fn besselj(&self, order: &Self) -> Self {
        apply_real_binary_fn(self, order, &|s: &Scalar, n: &Scalar| s.besselj(n))
    }

    fn bessely(&self, order: &Self) -> Self {
        apply_real_binary_fn(self, order, &|s: &Scalar, n: &Scalar| s.bessely(n))
    }

    fn besseli(&self, order: &Self) -> Self {
        apply_real_binary_fn(self, order, &|s: &Scalar, n: &Scalar| s.besseli(n))
    }

    fn besselk(&self, order: &Self) -> Self {
        apply_real_binary_fn(self, order, &|s: &Scalar, n: &Scalar| s.besselk(n))
    }

    fn polygamma(&self, order: &Self) -> Self {
        apply_real_binary_fn(self, order, &|s: &Scalar, n: &Scalar| s.polygamma(n))
    }

    fn beta(&self, other: &Self) -> Self {
        if is_pure_scalar(self) && is_pure_scalar(other) {
            let s = self.coeff(0).beta(other.coeff(0));
            let mut mv = Self::zero_unchecked(self.gens.clone());
            mv.coeffs_mut_slice()[0] = s;
            mv
        } else {
            nan_clifford(&self.gens)
        }
    }

    fn zeta_deriv(&self, order: &Self) -> Self {
        apply_real_binary_fn(self, order, &|s: &Scalar, n: &Scalar| s.zeta_deriv(n))
    }

    fn lambertw(&self, n: &Self) -> Self {
        apply_real_binary_fn(self, n, &|s: &Scalar, k: &Scalar| s.lambertw(k))
    }

    fn hermite(&self, n: &Self) -> Self {
        apply_real_binary_fn(self, n, &|s: &Scalar, nn: &Scalar| s.hermite(nn))
    }

    fn assoc_legendre(&self, l: &Self, m: &Self) -> Self {
        if is_pure_scalar(self) && is_pure_scalar(l) && is_pure_scalar(m) {
            let s = self.coeff(0).assoc_legendre(l.coeff(0), m.coeff(0));
            let mut mv = Self::zero_unchecked(self.gens.clone());
            mv.coeffs_mut_slice()[0] = s;
            mv
        } else {
            nan_clifford(&self.gens)
        }
    }

    fn spherical_harmonic(&self, l: &Self, m: &Self, phi: &Self) -> Self {
        if is_pure_scalar(self) && is_pure_scalar(l) && is_pure_scalar(m) && is_pure_scalar(phi) {
            let s = self
                .coeff(0)
                .spherical_harmonic(l.coeff(0), m.coeff(0), phi.coeff(0));
            let mut mv = Self::zero_unchecked(self.gens.clone());
            mv.coeffs_mut_slice()[0] = s;
            mv
        } else {
            nan_clifford(&self.gens)
        }
    }

    // =====================================================================
    // Core properties
    // =====================================================================

    fn is_zero(&self) -> bool {
        self.coeffs_slice()[..self.blade_count()]
            .iter()
            .all(Number::is_zero)
    }

    fn is_one(&self) -> bool {
        self.coeff(0).is_one()
            && self.coeffs_slice()[1..self.blade_count()]
                .iter()
                .all(Number::is_zero)
    }

    fn is_neg_one(&self) -> bool {
        self.coeff(0).is_neg_one()
            && self.coeffs_slice()[1..self.blade_count()]
                .iter()
                .all(Number::is_zero)
    }

    fn is_integer(&self) -> bool {
        is_pure_scalar(self) && self.coeff(0).is_integer()
    }

    fn is_negative(&self) -> bool {
        is_pure_scalar(self) && self.coeff(0).is_negative()
    }

    fn is_positive(&self) -> bool {
        is_pure_scalar(self) && self.coeff(0).is_positive()
    }

    fn is_finite(&self) -> bool {
        self.coeffs_slice()[..self.blade_count()]
            .iter()
            .all(Number::is_finite)
    }

    fn to_float(&self) -> Self {
        let mut mv = Self::zero_unchecked(self.gens.clone());
        for blade in 0..self.blade_count() {
            mv.coeffs_mut_slice()[blade] = self.coeff(blade).to_float();
        }
        mv
    }

    fn approx_eq_number(&self, other: &Self, tolerance: &Self) -> bool {
        if self.gens != other.gens {
            return false;
        }
        let tol = extract_scalar(tolerance);
        let limit = self.blade_count().max(other.blade_count());
        for blade in 0..limit {
            let a = if blade < self.blade_count() {
                self.coeff(blade)
            } else {
                &scalar_zero()
            };
            let b = if blade < other.blade_count() {
                other.coeff(blade)
            } else {
                &scalar_zero()
            };
            if !a.approx_eq_number(b, &tol) {
                return false;
            }
        }
        true
    }

    fn total_cmp(&self, other: &Self) -> Ordering {
        if self.gens != other.gens {
            return self.blade_count().cmp(&other.blade_count());
        }
        let limit = self.blade_count().max(other.blade_count());
        for blade in 0..limit {
            let a = if blade < self.blade_count() {
                self.coeff(blade)
            } else {
                &scalar_zero()
            };
            let b = if blade < other.blade_count() {
                other.coeff(blade)
            } else {
                &scalar_zero()
            };
            let ord = a.total_cmp(b);
            if ord != Ordering::Equal {
                return ord;
            }
        }
        Ordering::Equal
    }

    fn num_max(&self, other: &Self) -> Self {
        if !is_pure_scalar(self) || !is_pure_scalar(other) {
            return nan_clifford(&self.gens);
        }
        let s = self.coeff(0).num_max(other.coeff(0));
        let mut mv = Self::zero_unchecked(self.gens.clone());
        mv.coeffs_mut_slice()[0] = s;
        mv
    }

    fn num_min(&self, other: &Self) -> Self {
        if !is_pure_scalar(self) || !is_pure_scalar(other) {
            return nan_clifford(&self.gens);
        }
        let s = self.coeff(0).num_min(other.coeff(0));
        let mut mv = Self::zero_unchecked(self.gens.clone());
        mv.coeffs_mut_slice()[0] = s;
        mv
    }
}
