use alloc::vec::Vec;
use core::ops::Neg;

use super::super::scalar::Scalar;
use super::super::traits::Number;
use super::CliffordNumber;
use super::types::{GeneratorSet, coeff_count_unchecked, scalar_nan, scalar_one, scalar_zero};

// ============================================================================
// Complex number (a + bi) using Scalar for both components
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cmplx(pub Scalar, pub Scalar);

impl Cmplx {
    pub const fn new(re: Scalar, im: Scalar) -> Self {
        Self(re, im)
    }
    pub fn zero() -> Self {
        Self(scalar_zero(), scalar_zero())
    }
    pub fn one() -> Self {
        Self(scalar_one(), scalar_zero())
    }
    pub fn i() -> Self {
        Self(scalar_zero(), scalar_one())
    }

    pub fn conj(&self) -> Self {
        Self(self.0.clone(), -&self.1)
    }
    pub fn abs_sq(&self) -> Scalar {
        &self.0 * &self.0 + &self.1 * &self.1
    }
    pub fn abs(&self) -> Scalar {
        self.abs_sq().sqrt()
    }
    pub fn scale(&self, s: &Scalar) -> Self {
        Self(&self.0 * s, &self.1 * s)
    }

    pub fn add(&self, rhs: &Self) -> Self {
        Self(&self.0 + &rhs.0, &self.1 + &rhs.1)
    }
    #[allow(dead_code, reason = "Available for downstream users of Mat2C")]
    pub fn sub(&self, rhs: &Self) -> Self {
        Self(&self.0 - &rhs.0, &self.1 - &rhs.1)
    }
    pub fn mul(&self, rhs: &Self) -> Self {
        Self(
            &self.0 * &rhs.0 - &self.1 * &rhs.1,
            &self.0 * &rhs.1 + &self.1 * &rhs.0,
        )
    }
    pub fn neg(&self) -> Self {
        Self(-&self.0, -&self.1)
    }

    pub fn sqrt(&self) -> Self {
        if self.1.is_zero() {
            if self.0.is_negative() {
                Self(scalar_zero(), (-&self.0).sqrt())
            } else {
                Self(self.0.sqrt(), scalar_zero())
            }
        } else {
            let r = self.abs();
            let two = scalar_one() + scalar_one();
            let re = ((&r + &self.0) / &two).sqrt();
            let sgn = if self.1.is_negative() {
                -scalar_one()
            } else {
                scalar_one()
            };
            let im = ((&r - &self.0) / &two).sqrt() * sgn;
            Self(re, im)
        }
    }

    pub fn exp(&self) -> Self {
        let e_a = self.0.exp();
        Self(e_a.clone() * self.1.cos(), e_a * self.1.sin())
    }

    pub fn sin(&self) -> Self {
        Self(self.0.sin() * self.1.cosh(), self.0.cos() * self.1.sinh())
    }

    pub fn cos(&self) -> Self {
        Self(
            self.0.cos() * self.1.cosh(),
            self.0.sin().neg() * self.1.sinh(),
        )
    }

    pub fn ln(&self) -> Self {
        let r = self.abs();
        let theta = self.1.atan2(&self.0); // Scalar's atan2(y, x)
        Self(r.ln(), theta)
    }

    pub fn sinh(&self) -> Self {
        Self(self.0.sinh() * self.1.cos(), self.0.cosh() * self.1.sin())
    }

    pub fn cosh(&self) -> Self {
        Self(self.0.cosh() * self.1.cos(), self.0.sinh() * self.1.sin())
    }

    /// Returns true when both real and imaginary parts are zero.
    pub fn is_zero(&self) -> bool {
        self.0.is_zero() && self.1.is_zero()
    }

    /// Complex division: `self / rhs`.
    ///
    /// When `rhs` is zero, follows IEEE 754 semantics: `0/0 → NaN`, `x/0 → NaN`
    /// (complex infinity is not representable, so NaN is the safe fallback).
    pub fn div(&self, rhs: &Self) -> Self {
        let denom = rhs.abs_sq();
        if denom.is_zero() {
            return Self(scalar_nan(), scalar_nan());
        }
        // a / b = a * conj(b) / |b|²
        Self(
            (&self.0 * &rhs.0 + &self.1 * &rhs.1) / &denom,
            (&self.1 * &rhs.0 - &self.0 * &rhs.1) / &denom,
        )
    }

    /// Apply a real scalar function to the real part only, returning NaN if the
    /// imaginary part is non-zero (not a real number). Used for functions
    /// (erf, gamma, floor, etc.) that lack a standard complex extension.
    pub fn apply_real_only(&self, f: &impl Fn(&Scalar) -> Scalar) -> Self {
        if self.1.is_zero() {
            Self(f(&self.0), scalar_zero())
        } else {
            Self(scalar_nan(), scalar_nan())
        }
    }
}

// ============================================================================
// 2×2 complex matrix [[a, b], [c, d]]
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mat2C {
    pub a: Cmplx,
    pub b: Cmplx,
    pub c: Cmplx,
    pub d: Cmplx,
}

impl Mat2C {
    pub const fn new(a: Cmplx, b: Cmplx, c: Cmplx, d: Cmplx) -> Self {
        Self { a, b, c, d }
    }

    pub fn zero() -> Self {
        let z = Cmplx::zero();
        Self {
            a: z.clone(),
            b: z.clone(),
            c: z.clone(),
            d: z,
        }
    }

    pub fn identity() -> Self {
        Self {
            a: Cmplx::one(),
            b: Cmplx::zero(),
            c: Cmplx::zero(),
            d: Cmplx::one(),
        }
    }

    pub fn det(&self) -> Cmplx {
        self.a.mul(&self.d).sub(&self.b.mul(&self.c))
    }
    pub fn trace(&self) -> Cmplx {
        self.a.add(&self.d)
    }

    pub fn add(&self, rhs: &Self) -> Self {
        Self {
            a: self.a.add(&rhs.a),
            b: self.b.add(&rhs.b),
            c: self.c.add(&rhs.c),
            d: self.d.add(&rhs.d),
        }
    }

    #[allow(dead_code, reason = "Available for downstream users of Mat2C")]
    pub fn sub(&self, rhs: &Self) -> Self {
        Self {
            a: self.a.sub(&rhs.a),
            b: self.b.sub(&rhs.b),
            c: self.c.sub(&rhs.c),
            d: self.d.sub(&rhs.d),
        }
    }

    pub fn mul(&self, rhs: &Self) -> Self {
        Self {
            a: self.a.mul(&rhs.a).add(&self.b.mul(&rhs.c)),
            b: self.a.mul(&rhs.b).add(&self.b.mul(&rhs.d)),
            c: self.c.mul(&rhs.a).add(&self.d.mul(&rhs.c)),
            d: self.c.mul(&rhs.b).add(&self.d.mul(&rhs.d)),
        }
    }

    pub fn scale(&self, s: &Cmplx) -> Self {
        Self {
            a: self.a.mul(s),
            b: self.b.mul(s),
            c: self.c.mul(s),
            d: self.d.mul(s),
        }
    }

    pub fn dagger(&self) -> Self {
        Self {
            a: self.a.conj(),
            b: self.c.conj(),
            c: self.b.conj(),
            d: self.d.conj(),
        }
    }

    /// Eigenvalues `(λ₁, λ₂)` via `(tr ± √(tr² − 4·det)) / 2`.
    pub fn eigenvalues(&self) -> (Cmplx, Cmplx) {
        let tr = self.trace();
        let det = self.det();
        let four = Cmplx::new(
            scalar_one() + scalar_one() + scalar_one() + scalar_one(),
            scalar_zero(),
        );
        let disc = tr.mul(&tr).sub(&det.mul(&four));
        let sqrt_disc = disc.sqrt();
        let half = Cmplx::new(scalar_one() / (scalar_one() + scalar_one()), scalar_zero());
        (tr.add(&sqrt_disc).mul(&half), tr.sub(&sqrt_disc).mul(&half))
    }
}

// ============================================================================
// Generic CliffordNumber ↔ Mat2C embedding (≤ 3 generators)
// ============================================================================

/// Maximum generators that can be embedded in Mat(2,ℂ).
const MAX_EMBED_GENS: usize = 3;

/// Check whether `gens` can be embedded in Mat(2,ℂ) (at most 3 generators).
pub const fn can_embed(gens: &GeneratorSet) -> bool {
    gens.len() <= MAX_EMBED_GENS
}

/// Return the 2×2 Pauli matrix for a given generator position.
/// Position 0 -> `sigma_x`, 1 -> `sigma_y`, 2 -> `sigma_z`.
fn pauli_matrix(pos: usize) -> Mat2C {
    let z = Cmplx::zero();
    let one = Cmplx::one();
    let i = Cmplx::i();
    if pos == 0 {
        Mat2C::new(z.clone(), one.clone(), one, z)
    } else if pos == 1 {
        Mat2C::new(z.clone(), i.neg(), i, z)
    } else {
        Mat2C::new(one.clone(), z.clone(), z, one.neg())
    }
}

/// Return the 2×2 matrix for a nilpotent (metric = 0) generator.
fn nilpotent_matrix() -> Mat2C {
    let z = Cmplx::zero();
    Mat2C::new(z.clone(), Cmplx::one(), z.clone(), z)
}

/// Compute ALL blade matrices for a given generator set.
/// Returns `None` if the generator set is too large.
pub fn compute_blade_basis(gens: &GeneratorSet) -> Option<Vec<Mat2C>> {
    if !can_embed(gens) {
        return None;
    }
    let n = gens.len();
    let mut gen_mats: Vec<Mat2C> = Vec::with_capacity(n);
    for pos in 0..n {
        let metric = gens.metric_at(pos);
        let base = pauli_matrix(pos);
        if metric == 1 {
            gen_mats.push(base);
        } else if metric == -1 {
            gen_mats.push(base.scale(&Cmplx::i()));
        } else {
            gen_mats.push(nilpotent_matrix());
        }
    }

    let count = 1_usize << n;
    let mut basis = Vec::with_capacity(count);
    for blade in 0..count {
        let mut m = Mat2C::identity();
        for (i, gm) in gen_mats.iter().enumerate() {
            if (blade >> i) & 1 == 1 {
                m = m.mul(gm);
            }
        }
        basis.push(m);
    }
    Some(basis)
}

/// Convert a [`CliffordNumber`] to a 2×2 complex matrix.
/// Returns `None` if the generator set cannot be embedded in Mat(2,ℂ).
pub fn to_matrix(mv: &CliffordNumber) -> Option<Mat2C> {
    let gens = mv.generator_set();
    let blade_mats = compute_blade_basis(gens)?;
    let mut result = Mat2C::zero();
    for (blade, bm) in blade_mats.iter().enumerate() {
        let coeff = mv.coeff(blade);
        if coeff.is_zero() {
            continue;
        }
        let scaled = bm.scale(&Cmplx::new(coeff.clone(), scalar_zero()));
        result = result.add(&scaled);
    }
    Some(result)
}

/// Project a Mat(2,ℂ) matrix back onto the blade basis of `gens`, returning
/// the blade coefficients.
fn project_onto_basis(mat: &Mat2C, blade_mats: &[Mat2C]) -> Vec<Scalar> {
    blade_mats
        .iter()
        .map(|bm| {
            let prod = mat.mul(&bm.dagger());
            let numer = prod.trace().0;
            let norm_bm = bm.mul(&bm.dagger());
            let denom = norm_bm.trace().0;
            if denom.is_zero() {
                scalar_zero()
            } else {
                numer / denom
            }
        })
        .collect()
}

/// Convert a 2×2 complex matrix back to a [`CliffordNumber`] for a given
/// generator set. The generator set must have been used to produce the matrix
/// (same blade basis).
pub fn from_matrix_with_gens(mat: &Mat2C, gens: &GeneratorSet) -> CliffordNumber {
    let Some(blade_mats) = compute_blade_basis(gens) else {
        return CliffordNumber::zero_unchecked(gens.clone());
    };
    let coeffs = project_onto_basis(mat, &blade_mats);

    #[allow(clippy::cast_possible_truncation, reason = "can_embed ensures n <= 3")]
    let count = coeff_count_unchecked(gens.len() as u8);
    let mut mv = CliffordNumber::zero_unchecked(gens.clone());
    for (blade, c) in coeffs.iter().enumerate().take(count) {
        if !c.is_zero() {
            mv.coeffs_mut_slice()[blade] = c.clone();
        }
    }
    mv
}

// ============================================================================
// Eigendecomposition
// ============================================================================

/// Compute eigenvectors and return `(U, U⁻¹)` for a 2×2 matrix, or `None` if
/// the matrix is not diagonalizable (defective).
pub fn eigendecompose(mat: &Mat2C) -> Option<(Mat2C, Mat2C, Cmplx, Cmplx)> {
    let (lambda1, lambda2) = mat.eigenvalues();

    // Check if already diagonal
    if mat.b.is_zero() && mat.c.is_zero() {
        return Some((Mat2C::identity(), Mat2C::identity(), lambda1, lambda2));
    }

    // Check for defective case: λ₁ ≈ λ₂ but A ≠ λI
    let is_defective = lambda1.sub(&lambda2).abs_sq().is_zero();
    if is_defective {
        return None; // not diagonalizable
    }

    // Eigenvector for λ₁: solve (M - λ₁I)v = 0
    // Using the row with larger pivot for numerical stability
    let (v1_a, v1_b) = {
        let pivot_row1_b = mat.b.clone(); // from first row
        let pivot_row2_a = lambda1.sub(&mat.d); // from second row alternative
        if pivot_row1_b.abs_sq() > pivot_row2_a.abs_sq() && !pivot_row1_b.abs_sq().is_zero() {
            // v = (b, λ - a)
            (pivot_row1_b, lambda1.sub(&mat.a))
        } else if !pivot_row2_a.abs_sq().is_zero() {
            // v = (λ - d, c)
            (pivot_row2_a, mat.c.clone())
        } else {
            // Degenerate — shouldn't happen for non-defective, non-diagonal
            return None;
        }
    };

    // Eigenvector for λ₂: same approach
    let (v2_a, v2_b) = {
        let pivot_row1_b = mat.b.clone();
        let pivot_row2_a = lambda2.sub(&mat.d);
        if pivot_row1_b.abs_sq() > pivot_row2_a.abs_sq() && !pivot_row1_b.abs_sq().is_zero() {
            (pivot_row1_b, lambda2.sub(&mat.a))
        } else if !pivot_row2_a.abs_sq().is_zero() {
            (pivot_row2_a, mat.c.clone())
        } else {
            return None;
        }
    };

    // U = [v1 v2] (columns are eigenvectors)
    let u = Mat2C::new(v1_a, v2_a, v1_b, v2_b);

    // U⁻¹ = (1/det) · [[d, -b], [-c, a]]
    let det_u = u.det();
    // Actually: 1/det_u. We need division of complex numbers.
    // inv = (1/det) * [[d, -b], [-c, a]]
    // Since det is a complex number, 1/det = conj(det) / |det|² if det ≠ 0
    let det_abs_sq = det_u.abs_sq();
    // We need to check if det_abs_sq is zero (singular U, shouldn't happen for
    // non-defective matrices).
    if det_abs_sq.is_zero() {
        return None;
    }
    let inv_det = Cmplx::new(det_u.0.clone(), det_u.1.neg()).scale(&(scalar_one() / det_abs_sq));

    let u_inv = Mat2C::new(
        u.d.mul(&inv_det),
        u.b.neg().mul(&inv_det),
        u.c.neg().mul(&inv_det),
        u.a.mul(&inv_det),
    );

    Some((u, u_inv, lambda1, lambda2))
}
