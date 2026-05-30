use alloc::vec::Vec;
use core::array::from_fn;

use crate::error::{DenseLengthError, IndexOutOfRangeError, NumAnafisError};

use super::super::scalar::Scalar;
use super::super::traits::Number;

use super::CliffordNumber;
use super::types::{
    CliffordCoeffs, GeneratorSet, INLINE_COEFF_COUNT, INLINE_GENERATOR_LIMIT, coeff_count,
    coeff_count_unchecked, mul_blades, permute_blade, scalar_one, scalar_zero,
};

// ============================================================================
// CliffordNumber — internal helpers
// ============================================================================

impl CliffordNumber {
    #[allow(clippy::missing_const_for_fn, reason = "Vec Deref is not const-stable")]
    pub(crate) fn coeffs_slice(&self) -> &[Scalar] {
        match self.coeffs {
            CliffordCoeffs::Inline(ref a) => a.as_slice(),
            CliffordCoeffs::Heap(ref v) => v,
        }
    }

    #[allow(clippy::missing_const_for_fn, reason = "Vec Deref is not const-stable")]
    pub(crate) fn coeffs_mut_slice(&mut self) -> &mut [Scalar] {
        match self.coeffs {
            CliffordCoeffs::Inline(ref mut a) => a.as_mut_slice(),
            CliffordCoeffs::Heap(ref mut v) => v.as_mut_slice(),
        }
    }

    pub(crate) fn make_zero_coeffs(n: u8) -> CliffordCoeffs {
        let z = scalar_zero();
        if n <= INLINE_GENERATOR_LIMIT {
            CliffordCoeffs::Inline(from_fn(|_| z.clone()))
        } else {
            CliffordCoeffs::Heap(alloc::vec![z; coeff_count_unchecked(n)])
        }
    }

    pub(crate) fn zero_unchecked(gens: GeneratorSet) -> Self {
        #[allow(clippy::cast_possible_truncation, reason = "Validated by callers")]
        let n = gens.len() as u8;
        Self {
            coeffs: Self::make_zero_coeffs(n),
            gens,
        }
    }

    /// Re-embed `self` into a new generator set using a precomputed bit-permutation.
    pub(crate) fn reembed(&self, new_gens: GeneratorSet, perm: &[u8]) -> Self {
        #[allow(clippy::cast_possible_truncation, reason = "See zero_unchecked")]
        let new_n = new_gens.len() as u8;
        let new_count = coeff_count_unchecked(new_n);
        let old_count = self.blade_count();
        let z = scalar_zero();

        let mut buf: Vec<Scalar> = (0..new_count).map(|_| z.clone()).collect();
        for old_blade in 0..old_count {
            let c = &self.coeffs_slice()[old_blade];
            if c.is_zero() {
                continue;
            }
            buf[permute_blade(old_blade, perm)] = c.clone();
        }

        if new_n <= INLINE_GENERATOR_LIMIT {
            let arr: [Scalar; INLINE_COEFF_COUNT] = from_fn(|i| {
                if i < new_count {
                    buf[i].clone()
                } else {
                    z.clone()
                }
            });
            Self {
                coeffs: CliffordCoeffs::Inline(arr),
                gens: new_gens,
            }
        } else {
            Self {
                coeffs: CliffordCoeffs::Heap(buf),
                gens: new_gens,
            }
        }
    }
}

// ============================================================================
// CliffordNumber — public API
// ============================================================================

impl CliffordNumber {
    // --- Inspection ---

    /// Number of basis blades = 2^n.
    #[must_use]
    pub fn blade_count(&self) -> usize {
        #[allow(clippy::cast_possible_truncation, reason = "Validated")]
        let n = self.gens.len() as u8;
        coeff_count_unchecked(n)
    }

    /// Sets to zero any coefficient whose absolute value is strictly less than `Scalar::epsilon()`.
    ///
    /// This is useful to clean up `$10^{-16}$` numerical noise after spectral decompositions.
    #[must_use]
    pub fn chop(mut self) -> Self {
        let eps = super::super::scalar::Scalar::epsilon();
        for c in self.coeffs_mut_slice() {
            if c.abs().total_cmp(&eps) == core::cmp::Ordering::Less {
                *c = scalar_zero();
            }
        }
        self
    }

    /// Number of active generators.
    #[must_use]
    pub const fn n_generators(&self) -> usize {
        self.gens.len()
    }

    /// The active [`GeneratorSet`].
    #[must_use]
    pub const fn generator_set(&self) -> &GeneratorSet {
        &self.gens
    }

    /// Coefficient at blade bitmask `blade`.
    #[must_use]
    pub fn coeff(&self, blade: usize) -> &Scalar {
        &self.coeffs_slice()[blade]
    }

    /// Set the coefficient at blade bitmask `blade`.
    pub fn set_coeff(&mut self, blade: usize, value: Scalar) {
        self.coeffs_mut_slice()[blade] = value;
    }

    /// Iterator over `(blade_mask, &coefficient)` for every non-zero blade.
    pub fn nonzero_blades(&self) -> impl Iterator<Item = (usize, &Scalar)> {
        self.coeffs_slice()
            .iter()
            .take(self.blade_count())
            .enumerate()
            .filter(|&(_, c)| !c.is_zero())
    }

    /// `true` when the coefficient array lives on the heap (n > 5).
    #[must_use]
    pub const fn is_heap_allocated(&self) -> bool {
        matches!(&self.coeffs, CliffordCoeffs::Heap(_))
    }

    // --- Constructors ---

    /// All-zero multivector in `gens`.
    ///
    /// # Errors
    /// [`NumAnafisError::ActiveGeneratorsTooLargeForPlatform`] if 2ⁿ overflows `usize`
    /// or the generator count exceeds `u8::MAX`.
    pub fn zero(gens: GeneratorSet) -> Result<Self, NumAnafisError> {
        let n = gens.len();
        let n_u8 = u8::try_from(n).map_err(|_e| {
            NumAnafisError::ActiveGeneratorsTooLargeForPlatform { active: u8::MAX }
        })?;
        let _ = coeff_count(n_u8)?;
        Ok(Self::zero_unchecked(gens))
    }

    /// Grade-0 (scalar) multivector with the given value.
    ///
    /// # Errors
    /// Same as [`Self::zero`].
    pub fn scalar(gens: GeneratorSet, value: Scalar) -> Result<Self, NumAnafisError> {
        let mut mv = Self::zero(gens)?;
        mv.coeffs_mut_slice()[0] = value;
        Ok(mv)
    }

    /// Basis generator vector with only bit `index` set.
    ///
    /// # Errors
    /// [`NumAnafisError::GeneratorIndexOutOfRange`] when `index >= n`.
    /// [`NumAnafisError::ActiveGeneratorsTooLargeForPlatform`] if the generator count
    /// overflows `u8` or 2ⁿ overflows `usize`.
    pub fn generator(gens: GeneratorSet, index: u8) -> Result<Self, NumAnafisError> {
        let n = gens.len();
        let n_u8 = u8::try_from(n).map_err(|_e| {
            NumAnafisError::ActiveGeneratorsTooLargeForPlatform { active: u8::MAX }
        })?;
        if usize::from(index) >= n {
            return Err(NumAnafisError::GeneratorIndexOutOfRange(
                alloc::boxed::Box::new(IndexOutOfRangeError {
                    active: n_u8,
                    index,
                }),
            ));
        }
        let mut mv = Self::zero(gens)?;
        mv.coeffs_mut_slice()[1_usize << usize::from(index)] = scalar_one();
        Ok(mv)
    }

    /// Build from a fixed 16-element coefficient array (requires n ≤ 4).
    ///
    /// # Errors
    /// [`NumAnafisError::InlineCoefficientsRequireAtMostFourGenerators`] if n > 4.
    /// [`NumAnafisError::ActiveGeneratorsTooLargeForPlatform`] if the generator count
    /// overflows `u8` or 2ⁿ overflows `usize`.
    pub fn from_coeffs(
        gens: GeneratorSet,
        coeffs: [Scalar; INLINE_COEFF_COUNT],
    ) -> Result<Self, NumAnafisError> {
        let n = gens.len();
        let n_u8 = u8::try_from(n).map_err(|_e| {
            NumAnafisError::ActiveGeneratorsTooLargeForPlatform { active: u8::MAX }
        })?;
        if n_u8 > INLINE_GENERATOR_LIMIT {
            return Err(
                NumAnafisError::InlineCoefficientsRequireAtMostFiveGenerators { active: n_u8 },
            );
        }
        let _ = coeff_count(n_u8)?;
        Ok(Self {
            coeffs: CliffordCoeffs::Inline(coeffs),
            gens,
        })
    }

    /// Build from a dense coefficient `Vec` (must have length 2ⁿ).
    ///
    /// # Errors
    /// [`NumAnafisError::DenseCoefficientLengthMismatch`] when `coeffs.len() != 2ⁿ`.
    /// [`NumAnafisError::ActiveGeneratorsTooLargeForPlatform`] if the generator count
    /// overflows `u8` or 2ⁿ overflows `usize`.
    pub fn from_dense_coeffs(
        gens: GeneratorSet,
        coeffs: Vec<Scalar>,
    ) -> Result<Self, NumAnafisError> {
        let n = gens.len();
        let n_u8 = u8::try_from(n).map_err(|_e| {
            NumAnafisError::ActiveGeneratorsTooLargeForPlatform { active: u8::MAX }
        })?;
        let expected = coeff_count(n_u8)?;
        if coeffs.len() != expected {
            return Err(NumAnafisError::DenseCoefficientLengthMismatch(
                alloc::boxed::Box::new(DenseLengthError {
                    expected,
                    found: coeffs.len(),
                }),
            ));
        }
        let z = scalar_zero();
        let storage = if n_u8 <= INLINE_GENERATOR_LIMIT {
            let arr: [Scalar; INLINE_COEFF_COUNT] = from_fn(|i| {
                if i < expected {
                    coeffs[i].clone()
                } else {
                    z.clone()
                }
            });
            CliffordCoeffs::Inline(arr)
        } else {
            CliffordCoeffs::Heap(coeffs)
        };
        Ok(Self {
            coeffs: storage,
            gens,
        })
    }

    // =========================================================================
    // Products
    // =========================================================================

    /// Geometric product `self × other`.
    #[must_use]
    pub fn geometric_mul(&self, other: &Self) -> Self {
        if self.gens == other.gens {
            self.geo_same(other)
        } else {
            let (u, pa, pb) = self.gens.union_with(&other.gens);
            self.reembed(u.clone(), &pa)
                .geo_same(&other.reembed(u, &pb))
        }
    }

    fn geo_same(&self, other: &Self) -> Self {
        debug_assert_eq!(
            self.gens, other.gens,
            "geo_same requires identical generator sets"
        );
        let limit = self.blade_count();
        let mut out = Self::zero_unchecked(self.gens.clone());

        let use_cache = !self.gens.cayley_signs.is_empty();
        let stride = 1 << self.gens.len();

        for a in 0..limit {
            let lhs = &self.coeffs_slice()[a];
            if lhs.is_zero() {
                continue;
            }
            for b in 0..limit {
                let rhs = &other.coeffs_slice()[b];
                if rhs.is_zero() {
                    continue;
                }
                let factor = if use_cache {
                    self.gens.cayley_signs[a * stride + b]
                } else {
                    mul_blades(&self.gens, a, b).map_or(0, |(_, f)| f)
                };
                if factor == 0 {
                    continue;
                }
                let mask = a ^ b;
                let raw = lhs * rhs;
                let term = if factor < 0 { -raw } else { raw };
                let cur = out.coeffs_slice()[mask].clone();
                out.coeffs_mut_slice()[mask] = &cur + &term;
            }
        }
        out
    }

    /// Outer (wedge) product `self ∧ other`.
    #[must_use]
    pub fn outer_product(&self, other: &Self) -> Self {
        if self.gens == other.gens {
            self.outer_same(other)
        } else {
            let (u, pa, pb) = self.gens.union_with(&other.gens);
            self.reembed(u.clone(), &pa)
                .outer_same(&other.reembed(u, &pb))
        }
    }

    fn outer_same(&self, other: &Self) -> Self {
        debug_assert_eq!(
            self.gens, other.gens,
            "outer_same requires identical generator sets"
        );
        let n = self.gens.len();
        let limit = self.blade_count();
        let mut out = Self::zero_unchecked(self.gens.clone());

        let use_cache = !self.gens.cayley_signs.is_empty();
        let stride = 1 << n;

        for a in 0..limit {
            let lhs = &self.coeffs_slice()[a];
            if lhs.is_zero() {
                continue;
            }
            for b in 0..limit {
                if a & b != 0 {
                    continue;
                }
                let rhs = &other.coeffs_slice()[b];
                if rhs.is_zero() {
                    continue;
                }
                let factor = if use_cache {
                    self.gens.cayley_signs[a * stride + b]
                } else {
                    let mut swaps = 0_u32;
                    for i in 0..n {
                        if (b >> i) & 1 == 1 {
                            swaps += (a >> (i + 1)).count_ones();
                        }
                    }
                    if swaps & 1 == 1 { -1 } else { 1 }
                };
                if factor == 0 {
                    continue;
                }
                let raw = lhs * rhs;
                let term = if factor < 0 { -raw } else { raw };
                let mask = a | b; // equivalent to a ^ b since overlap is 0
                let cur = out.coeffs_slice()[mask].clone();
                out.coeffs_mut_slice()[mask] = &cur + &term;
            }
        }
        out
    }

    /// Left contraction `self ⌋ other`.
    #[must_use]
    pub fn inner_product(&self, other: &Self) -> Self {
        if self.gens == other.gens {
            self.inner_same(other)
        } else {
            let (u, pa, pb) = self.gens.union_with(&other.gens);
            self.reembed(u.clone(), &pa)
                .inner_same(&other.reembed(u, &pb))
        }
    }

    fn inner_same(&self, other: &Self) -> Self {
        debug_assert_eq!(
            self.gens, other.gens,
            "inner_same requires identical generator sets"
        );
        let limit = self.blade_count();
        let mut out = Self::zero_unchecked(self.gens.clone());

        let use_cache = !self.gens.cayley_signs.is_empty();
        let stride = 1 << self.gens.len();

        for a in 0..limit {
            let lhs = &self.coeffs_slice()[a];
            if lhs.is_zero() {
                continue;
            }
            let ga = a.count_ones();
            for b in 0..limit {
                let gb = b.count_ones();
                if gb < ga || (a & b) != a {
                    continue;
                }
                let rhs = &other.coeffs_slice()[b];
                if rhs.is_zero() {
                    continue;
                }
                let factor = if use_cache {
                    self.gens.cayley_signs[a * stride + b]
                } else {
                    mul_blades(&self.gens, a, b).map_or(0, |(_, f)| f)
                };
                if factor == 0 {
                    continue;
                }
                let mask = a ^ b;
                let raw = lhs * rhs;
                let term = if factor < 0 { -raw } else { raw };
                let cur = out.coeffs_slice()[mask].clone();
                out.coeffs_mut_slice()[mask] = &cur + &term;
            }
        }
        out
    }

    /// Grade-0 part of the geometric product `⟨AB⟩₀`.
    #[must_use]
    pub fn scalar_product(&self, other: &Self) -> Scalar {
        if self.gens != other.gens {
            return self.geometric_mul(other).coeff(0).clone();
        }
        let use_cache = !self.gens.cayley_signs.is_empty();
        let stride = 1 << self.gens.len();
        let mut result = scalar_zero();
        for a in 0..self.blade_count() {
            let lhs = &self.coeffs_slice()[a];
            if lhs.is_zero() {
                continue;
            }
            let rhs = &other.coeffs_slice()[a];
            if rhs.is_zero() {
                continue;
            }
            let factor = if use_cache {
                self.gens.cayley_signs[a * stride + a]
            } else {
                mul_blades(&self.gens, a, a).map_or(0, |(_, f)| f)
            };
            if factor == 0 {
                continue;
            }
            let term = lhs * rhs;
            if factor < 0 {
                result = &result - &term;
            } else {
                result = &result + &term;
            }
        }
        result
    }

    // =========================================================================
    // Grade operations
    // =========================================================================

    /// Project onto the grade-`k` subspace.
    #[must_use]
    pub fn grade(&self, k: u32) -> Self {
        let mut out = Self::zero_unchecked(self.gens.clone());
        for blade in 0..self.blade_count() {
            if blade.count_ones() == k {
                out.coeffs_mut_slice()[blade] = self.coeffs_slice()[blade].clone();
            }
        }
        out
    }

    /// Reverse `Ã`: multiply grade-g blades by `(−1)^(g(g−1)/2)`.
    #[must_use]
    pub fn reverse(&self) -> Self {
        let mut out = self.clone();
        for blade in 0..self.blade_count() {
            if blade.count_ones() % 4 >= 2 {
                let c = -&out.coeffs_slice()[blade];
                out.coeffs_mut_slice()[blade] = c;
            }
        }
        out
    }

    /// Grade involution `Â`: multiply grade-g blades by `(−1)^g`.
    #[must_use]
    pub fn grade_involution(&self) -> Self {
        let mut out = self.clone();
        for blade in 0..self.blade_count() {
            if blade.count_ones() % 2 == 1 {
                let c = -&out.coeffs_slice()[blade];
                out.coeffs_mut_slice()[blade] = c;
            }
        }
        out
    }

    /// Clifford conjugate `A†` = reverse ∘ grade involution.
    #[must_use]
    pub fn clifford_conjugate(&self) -> Self {
        let mut out = self.clone();
        for blade in 0..self.blade_count() {
            let g = blade.count_ones() % 4;
            if g == 1 || g == 2 {
                let c = -&out.coeffs_slice()[blade];
                out.coeffs_mut_slice()[blade] = c;
            }
        }
        out
    }

    /// Squared norm `⟨A Ã⟩₀` (scalar product of `self` with its reverse).
    #[must_use]
    pub fn norm_sq(&self) -> Scalar {
        self.scalar_product(&self.reverse())
    }
}

// ============================================================================
// PartialOrd — lexicographic by blade coefficients
// ============================================================================

impl PartialOrd for CliffordNumber {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        // Different generator sets are incomparable without reembedding,
        // which would silently allocate.  Return None instead.
        if self.gens != other.gens {
            return None;
        }
        // IEEE 754: NaN is unordered → return None
        let limit = self.blade_count();
        for i in 0..limit {
            let a = &self.coeffs_slice()[i];
            let b = &other.coeffs_slice()[i];
            if a.is_nan_internal() || b.is_nan_internal() {
                return None;
            }
        }
        Some(self.total_cmp(other))
    }
}
