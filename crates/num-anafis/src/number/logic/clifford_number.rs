#![allow(clippy::use_self, reason = "Explicit types are more readable here")]
#![allow(clippy::manual_let_else, reason = "More explicit control flow")]
#![allow(
    clippy::panic,
    reason = "Metric logic is sealed and only returns 1, 0, or -1"
)]
#![allow(
    clippy::same_name_method,
    reason = "Trait and inherent methods deliberately share the same name for ergonomics"
)]
use core::array::from_fn;
use core::ops::{Add, Mul, Neg, Sub};

use crate::NumAnafisError;

use super::scalar::Number;

const INLINE_GENERATOR_LIMIT: u8 = 4;
const INLINE_COEFF_COUNT: usize = 16;

/// Signature for a geometric algebra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Signature {
    /// Number of generators that square to +1.
    pub p: u8,
    /// Number of generators that square to -1.
    pub q: u8,
    /// Number of generators that square to 0.
    pub r: u8,
}

impl Signature {
    /// Create a signature if `p + q + r` does not overflow `u8`.
    #[must_use]
    pub const fn new(p: u8, q: u8, r: u8) -> Option<Self> {
        let Some(pq) = p.checked_add(q) else {
            return None;
        };
        let Some(_sum) = pq.checked_add(r) else {
            return None;
        };
        Some(Self { p, q, r })
    }

    /// Number of generators in this signature.
    #[must_use]
    pub const fn generators(self) -> u8 {
        self.p + self.q + self.r
    }

    /// Get the metric factor for a given basis vector index.
    #[must_use]
    pub const fn metric(self, index: u8) -> i8 {
        if index < self.p {
            1
        } else if index < self.p + self.q {
            -1
        } else {
            0
        }
    }
}

/// Coefficient storage for a Clifford number.
#[allow(
    clippy::large_enum_variant,
    reason = "Inline coefficients intentionally keep the fast path stack-resident"
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CliffordCoeffs {
    /// Inline coefficients for the fast path (`n <= 4`).
    Inline([Number; INLINE_COEFF_COUNT]),
    /// Heap coefficients for larger algebras (`n > 4`).
    Heap(Vec<Number>),
}

/// Dense Clifford number representation with inline fast path and heap fallback.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CliffordNumber {
    /// Coefficients in canonical basis order.
    coeffs: CliffordCoeffs,
    /// Metric signature (p, q, r).
    pub(crate) sig: Signature,
    /// Number of active generators.
    pub(crate) n: u8,
}

impl CliffordNumber {
    fn try_coeff_count(n: u8) -> Result<usize, NumAnafisError> {
        let shift = u32::from(n);
        1_usize
            .checked_shl(shift)
            .ok_or(NumAnafisError::ActiveGeneratorsTooLargeForPlatform { active: n })
    }

    fn coeff_count_unchecked(n: u8) -> usize {
        Self::try_coeff_count(n)
            .unwrap_or_else(|_| panic!("active generator count already validated"))
    }

    fn validate_active_generators(sig: Signature, n: u8) -> Result<(), NumAnafisError> {
        let available = sig.generators();
        if n > available {
            return Err(NumAnafisError::ActiveGeneratorsExceedSignature {
                active: n,
                available,
            });
        }
        Self::try_coeff_count(n).map(|_| ())
    }

    fn coeffs_slice(&self) -> &[Number] {
        match &self.coeffs {
            CliffordCoeffs::Inline(values) => values,
            CliffordCoeffs::Heap(values) => values,
        }
    }

    fn coeffs_mut_slice(&mut self) -> &mut [Number] {
        match &mut self.coeffs {
            CliffordCoeffs::Inline(values) => values,
            CliffordCoeffs::Heap(values) => values,
        }
    }

    fn coeffs_kind_for_n(n: u8) -> CliffordCoeffs {
        let zero = Number::from(0_i64);
        if n <= INLINE_GENERATOR_LIMIT {
            CliffordCoeffs::Inline(from_fn(|_| zero.clone()))
        } else {
            CliffordCoeffs::Heap(vec![zero; Self::coeff_count_unchecked(n)])
        }
    }

    pub(crate) fn zero_unchecked(sig: Signature, n: u8) -> Self {
        Self {
            coeffs: Self::coeffs_kind_for_n(n),
            sig,
            n,
        }
    }

    pub(crate) fn scalar_unchecked(sig: Signature, n: u8, value: Number) -> Self {
        let mut mv = Self::zero_unchecked(sig, n);
        mv.coeffs_mut_slice()[0] = value;
        mv
    }

    /// Build a zero Clifford number for a given signature and active generator count.
    ///
    /// Returns an error when `n` exceeds signature generators or platform index width.
    ///
    /// # Errors
    ///
    /// Returns [`NumAnafisError`] when `n` exceeds `sig.generators()` or `2^n` cannot be indexed
    /// on the current platform.
    pub fn zero(sig: Signature, n: u8) -> Result<Self, NumAnafisError> {
        Self::validate_active_generators(sig, n)?;
        Ok(Self::zero_unchecked(sig, n))
    }

    /// Build a scalar Clifford number.
    ///
    /// # Errors
    ///
    /// Returns [`NumAnafisError`] under the same conditions as [`Self::zero`].
    pub fn scalar(sig: Signature, n: u8, value: Number) -> Result<Self, NumAnafisError> {
        Self::zero(sig, n).map(|mut mv| {
            mv.coeffs_mut_slice()[0] = value;
            mv
        })
    }

    /// Build a basis generator `e_index`.
    ///
    /// # Errors
    ///
    /// Returns [`NumAnafisError`] if `index >= n` or when `n` is invalid for `sig`/platform.
    pub fn generator(sig: Signature, n: u8, index: u8) -> Result<Self, NumAnafisError> {
        if index >= n {
            return Err(NumAnafisError::GeneratorIndexOutOfRange { active: n, index });
        }
        let mut mv = Self::zero(sig, n)?;
        mv.coeffs_mut_slice()[1_usize << usize::from(index)] = Number::from(1_i64);
        Ok(mv)
    }

    /// Build directly from 16 coefficients.
    ///
    /// # Errors
    ///
    /// Returns [`NumAnafisError`] when `n > 4`, when `n` exceeds signature generators,
    /// or when `2^n` cannot be indexed on the current platform.
    pub fn from_coeffs(
        sig: Signature,
        n: u8,
        coeffs: [Number; 16],
    ) -> Result<Self, NumAnafisError> {
        if n > INLINE_GENERATOR_LIMIT {
            return Err(
                NumAnafisError::InlineCoefficientsRequireAtMostFourGenerators { active: n },
            );
        }
        Self::validate_active_generators(sig, n)?;
        Ok(Self {
            coeffs: CliffordCoeffs::Inline(coeffs),
            sig,
            n,
        })
    }

    /// Build directly from dense coefficients with dynamic size.
    ///
    /// # Errors
    ///
    /// Returns [`NumAnafisError`] when `n` exceeds signature/platform limits or when
    /// `coeffs.len() != 2^n`.
    pub fn from_dense_coeffs(
        sig: Signature,
        n: u8,
        coeffs: Vec<Number>,
    ) -> Result<Self, NumAnafisError> {
        Self::validate_active_generators(sig, n)?;
        let expected = Self::try_coeff_count(n)?;
        if coeffs.len() != expected {
            return Err(NumAnafisError::DenseCoefficientLengthMismatch {
                expected,
                found: coeffs.len(),
            });
        }
        let coeffs = if n <= INLINE_GENERATOR_LIMIT {
            let mut inline = from_fn(|_| Number::from(0_i64));
            inline[..expected].clone_from_slice(&coeffs);
            CliffordCoeffs::Inline(inline)
        } else {
            CliffordCoeffs::Heap(coeffs)
        };

        Ok(Self { coeffs, sig, n })
    }

    /// Build a complex-like value `a + b i`.
    ///
    #[must_use]
    pub fn complex(real: Number, imag: Number) -> Self {
        let sig = Signature { p: 0, q: 1, r: 0 };
        let mut mv = Self::zero_unchecked(sig, 1);
        mv.coeffs_mut_slice()[0] = real;
        mv.coeffs_mut_slice()[1] = imag;
        mv
    }

    /// Build a dual-like value `a + b eps`.
    ///
    #[must_use]
    pub fn dual(real: Number, eps: Number) -> Self {
        let sig = Signature { p: 0, q: 0, r: 1 };
        let mut mv = Self::zero_unchecked(sig, 1);
        mv.coeffs_mut_slice()[0] = real;
        mv.coeffs_mut_slice()[1] = eps;
        mv
    }

    /// Build a split-complex-like value `a + b j`.
    ///
    #[must_use]
    pub fn split(real: Number, j: Number) -> Self {
        let sig = Signature { p: 1, q: 0, r: 0 };
        let mut mv = Self::zero_unchecked(sig, 1);
        mv.coeffs_mut_slice()[0] = real;
        mv.coeffs_mut_slice()[1] = j;
        mv
    }

    /// Active signature.
    #[must_use]
    pub const fn signature(&self) -> Signature {
        self.sig
    }

    /// Number of active generators.
    #[must_use]
    pub const fn active_generators(&self) -> u8 {
        self.n
    }

    /// Get coefficient by blade bitmask (0..2^n).
    #[must_use]
    pub fn coeff(&self, blade: usize) -> &Number {
        &self.coeffs_slice()[blade]
    }

    /// Number of basis blades represented by this value (2^n).
    #[must_use]
    pub fn blade_count(&self) -> usize {
        Self::coeff_count_unchecked(self.n)
    }

    /// Return true when this value uses heap storage for coefficients.
    #[must_use]
    pub const fn is_heap_allocated(&self) -> bool {
        matches!(&self.coeffs, CliffordCoeffs::Heap(_))
    }

    /// Iterate all non-zero blade coefficients as `(blade_index, coefficient)`.
    pub fn nonzero_blades(&self) -> impl Iterator<Item = (usize, &Number)> {
        self.coeffs_slice()
            .iter()
            .take(self.blade_count())
            .enumerate()
            .filter(|(_, coeff)| !coeff.is_zero())
    }

    /// Set coefficient by blade bitmask (0..2^n).
    pub fn set_coeff(&mut self, blade: usize, value: Number) {
        self.coeffs_mut_slice()[blade] = value;
    }

    /// Return true when all coefficients are zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        let limit = Self::coeff_count_unchecked(self.n);
        self.coeffs_slice()[..limit].iter().all(Number::is_zero)
    }

    /// Geometric product.
    #[must_use]
    pub fn geometric_mul(&self, other: &Self) -> Self {
        self.assert_compatible(other);

        let mut out = Self::zero_unchecked(self.sig, self.n);
        let limit = Self::coeff_count_unchecked(self.n);

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

                let (mask, factor) = if let Some(res) = Self::mul_blades(self.sig, self.n, a, b) {
                    res
                } else {
                    continue;
                };

                let mut term = lhs * rhs;
                if factor < 0 {
                    term = -term;
                }

                let current = out.coeffs_slice()[mask].clone();
                out.coeffs_mut_slice()[mask] = &current + &term;
            }
        }

        out
    }

    fn assert_compatible(&self, other: &Self) {
        assert_eq!(
            self.sig, other.sig,
            "signature mismatch in Clifford number operation"
        );
        assert_eq!(
            self.n, other.n,
            "active generator mismatch in Clifford number operation"
        );
    }

    fn mul_blades(sig: Signature, n: u8, a: usize, b: usize) -> Option<(usize, i8)> {
        let mut swaps = 0_u32;
        for i in 0..n {
            let shift = usize::from(i);
            if ((b >> shift) & 1_usize) == 1_usize {
                swaps += (a >> (shift + 1)).count_ones();
            }
        }

        let mut factor = if (swaps & 1) == 0 { 1_i8 } else { -1_i8 };
        let overlap = a & b;

        for i in 0..n {
            if ((overlap >> usize::from(i)) & 1_usize) == 1_usize {
                match sig.metric(i) {
                    1 => {}
                    -1 => factor = -factor,
                    0 => return None,
                    _ => panic!("invalid metric factor"),
                }
            }
        }

        Some((a ^ b, factor))
    }

    /// Scalar unit in real algebra.
    ///
    #[must_use]
    pub fn r() -> Self {
        let sig = Signature { p: 0, q: 0, r: 0 };
        Self::scalar_unchecked(sig, 0, Number::from(1_i64))
    }

    /// Complex unit `i` with `i^2 = -1`.
    ///
    #[must_use]
    pub fn i() -> Self {
        let sig = Signature { p: 0, q: 1, r: 0 };
        let mut mv = Self::zero_unchecked(sig, 1);
        mv.coeffs_mut_slice()[1] = Number::from(1_i64);
        mv
    }

    /// Dual unit `eps` with `eps^2 = 0`.
    ///
    #[must_use]
    pub fn eps() -> Self {
        let sig = Signature { p: 0, q: 0, r: 1 };
        let mut mv = Self::zero_unchecked(sig, 1);
        mv.coeffs_mut_slice()[1] = Number::from(1_i64);
        mv
    }

    /// Split-complex unit `j` with `j^2 = +1`.
    ///
    #[must_use]
    pub fn j() -> Self {
        let sig = Signature { p: 1, q: 0, r: 0 };
        let mut mv = Self::zero_unchecked(sig, 1);
        mv.coeffs_mut_slice()[1] = Number::from(1_i64);
        mv
    }

    /// Euclidean basis vector e1 in R3.
    ///
    #[must_use]
    pub fn e1() -> Self {
        let sig = Signature { p: 3, q: 0, r: 0 };
        let mut mv = Self::zero_unchecked(sig, 3);
        mv.coeffs_mut_slice()[1] = Number::from(1_i64);
        mv
    }

    /// Euclidean basis vector e2 in R3.
    ///
    #[must_use]
    pub fn e2() -> Self {
        let sig = Signature { p: 3, q: 0, r: 0 };
        let mut mv = Self::zero_unchecked(sig, 3);
        mv.coeffs_mut_slice()[2] = Number::from(1_i64);
        mv
    }

    /// Euclidean basis vector e3 in R3.
    ///
    #[must_use]
    pub fn e3() -> Self {
        let sig = Signature { p: 3, q: 0, r: 0 };
        let mut mv = Self::zero_unchecked(sig, 3);
        mv.coeffs_mut_slice()[4] = Number::from(1_i64);
        mv
    }
}

impl Add for CliffordNumber {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.assert_compatible(&rhs);

        let mut out = Self::zero_unchecked(self.sig, self.n);
        let limit = Self::coeff_count_unchecked(self.n);
        for i in 0..limit {
            out.coeffs_mut_slice()[i] = &self.coeffs_slice()[i] + &rhs.coeffs_slice()[i];
        }
        out
    }
}

impl Add<&CliffordNumber> for CliffordNumber {
    type Output = Self;

    fn add(self, rhs: &CliffordNumber) -> Self::Output {
        self + rhs.clone()
    }
}

impl Add<CliffordNumber> for &CliffordNumber {
    type Output = CliffordNumber;

    fn add(self, rhs: CliffordNumber) -> Self::Output {
        self.clone() + rhs
    }
}

impl Add<&CliffordNumber> for &CliffordNumber {
    type Output = CliffordNumber;

    fn add(self, rhs: &CliffordNumber) -> Self::Output {
        self.clone() + rhs.clone()
    }
}

impl Add<CliffordNumber> for Number {
    type Output = CliffordNumber;

    fn add(self, rhs: CliffordNumber) -> Self::Output {
        CliffordNumber::scalar_unchecked(rhs.sig, rhs.n, self) + rhs
    }
}

impl Add<&CliffordNumber> for Number {
    type Output = CliffordNumber;

    fn add(self, rhs: &CliffordNumber) -> Self::Output {
        CliffordNumber::scalar_unchecked(rhs.sig, rhs.n, self) + rhs
    }
}

impl Add<CliffordNumber> for &Number {
    type Output = CliffordNumber;

    fn add(self, rhs: CliffordNumber) -> Self::Output {
        self.clone() + rhs
    }
}

impl Add<&CliffordNumber> for &Number {
    type Output = CliffordNumber;

    fn add(self, rhs: &CliffordNumber) -> Self::Output {
        self.clone() + rhs
    }
}

impl Sub for CliffordNumber {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self + (-rhs)
    }
}

impl Sub<&CliffordNumber> for CliffordNumber {
    type Output = Self;

    fn sub(self, rhs: &CliffordNumber) -> Self::Output {
        self - rhs.clone()
    }
}

impl Sub<CliffordNumber> for &CliffordNumber {
    type Output = CliffordNumber;

    fn sub(self, rhs: CliffordNumber) -> Self::Output {
        self.clone() - rhs
    }
}

impl Sub<&CliffordNumber> for &CliffordNumber {
    type Output = CliffordNumber;

    fn sub(self, rhs: &CliffordNumber) -> Self::Output {
        self.clone() - rhs.clone()
    }
}

impl Sub<CliffordNumber> for Number {
    type Output = CliffordNumber;

    fn sub(self, rhs: CliffordNumber) -> Self::Output {
        CliffordNumber::scalar_unchecked(rhs.sig, rhs.n, self) - rhs
    }
}

impl Sub<&CliffordNumber> for Number {
    type Output = CliffordNumber;

    fn sub(self, rhs: &CliffordNumber) -> Self::Output {
        CliffordNumber::scalar_unchecked(rhs.sig, rhs.n, self) - rhs
    }
}

impl Sub<CliffordNumber> for &Number {
    type Output = CliffordNumber;

    fn sub(self, rhs: CliffordNumber) -> Self::Output {
        self.clone() - rhs
    }
}

impl Sub<&CliffordNumber> for &Number {
    type Output = CliffordNumber;

    fn sub(self, rhs: &CliffordNumber) -> Self::Output {
        self.clone() - rhs
    }
}

impl Neg for CliffordNumber {
    type Output = Self;

    fn neg(self) -> Self::Output {
        let mut out = Self::zero_unchecked(self.sig, self.n);
        let limit = Self::coeff_count_unchecked(self.n);
        for i in 0..limit {
            out.coeffs_mut_slice()[i] = -self.coeffs_slice()[i].clone();
        }
        out
    }
}

impl Neg for &CliffordNumber {
    type Output = CliffordNumber;

    fn neg(self) -> Self::Output {
        -self.clone()
    }
}

impl Mul for CliffordNumber {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.geometric_mul(&rhs)
    }
}

impl Mul<&CliffordNumber> for CliffordNumber {
    type Output = Self;

    fn mul(self, rhs: &CliffordNumber) -> Self::Output {
        self.geometric_mul(rhs)
    }
}

impl Mul<CliffordNumber> for &CliffordNumber {
    type Output = CliffordNumber;

    fn mul(self, rhs: CliffordNumber) -> Self::Output {
        self.geometric_mul(&rhs)
    }
}

impl Mul<&CliffordNumber> for &CliffordNumber {
    type Output = CliffordNumber;

    fn mul(self, rhs: &CliffordNumber) -> Self::Output {
        self.geometric_mul(rhs)
    }
}

impl Mul<Number> for CliffordNumber {
    type Output = Self;

    fn mul(self, rhs: Number) -> Self::Output {
        let mut out = Self::zero_unchecked(self.sig, self.n);
        let limit = Self::coeff_count_unchecked(self.n);
        for i in 0..limit {
            out.coeffs_mut_slice()[i] = &self.coeffs_slice()[i] * &rhs;
        }
        out
    }
}

impl Mul<Number> for &CliffordNumber {
    type Output = CliffordNumber;

    fn mul(self, rhs: Number) -> Self::Output {
        self.clone() * rhs
    }
}

impl Mul<i64> for CliffordNumber {
    type Output = Self;

    fn mul(self, rhs: i64) -> Self::Output {
        self * Number::from(rhs)
    }
}

impl Mul<i64> for &CliffordNumber {
    type Output = CliffordNumber;

    fn mul(self, rhs: i64) -> Self::Output {
        self.clone() * rhs
    }
}

impl Mul<i32> for CliffordNumber {
    type Output = Self;

    fn mul(self, rhs: i32) -> Self::Output {
        self * Number::from(rhs)
    }
}

impl Mul<i32> for &CliffordNumber {
    type Output = CliffordNumber;

    fn mul(self, rhs: i32) -> Self::Output {
        self.clone() * rhs
    }
}

impl Mul<f64> for CliffordNumber {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        self * Number::from(rhs)
    }
}

impl Mul<f64> for &CliffordNumber {
    type Output = CliffordNumber;

    fn mul(self, rhs: f64) -> Self::Output {
        self.clone() * rhs
    }
}

impl Mul<CliffordNumber> for Number {
    type Output = CliffordNumber;

    fn mul(self, rhs: CliffordNumber) -> Self::Output {
        rhs * self
    }
}

impl Mul<&CliffordNumber> for Number {
    type Output = CliffordNumber;

    fn mul(self, rhs: &CliffordNumber) -> Self::Output {
        rhs * self
    }
}

impl Mul<CliffordNumber> for &Number {
    type Output = CliffordNumber;

    fn mul(self, rhs: CliffordNumber) -> Self::Output {
        rhs * self.clone()
    }
}

impl Mul<&CliffordNumber> for &Number {
    type Output = CliffordNumber;

    fn mul(self, rhs: &CliffordNumber) -> Self::Output {
        rhs * self.clone()
    }
}
