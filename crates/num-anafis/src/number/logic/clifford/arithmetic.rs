use core::ops::{Add, Div, Mul, Neg, Sub};

use super::super::float_ops::FloatType;
use super::super::int_math::IntType;
use super::super::scalar::{Scalar, ScalarRepr};
use super::CliffordNumber;

// ============================================================================
// Shared helpers
// ============================================================================

fn scale(mv: &CliffordNumber, s: &Scalar) -> CliffordNumber {
    let mut out = CliffordNumber::zero_unchecked(mv.generator_set().clone());
    let limit = mv.blade_count();
    for i in 0..limit {
        out.coeffs_mut_slice()[i] = &mv.coeffs_slice()[i] * s;
    }
    out
}

fn scalar_plus_mv(s: &Scalar, mut mv: CliffordNumber) -> CliffordNumber {
    let new_scalar = &mv.coeffs_slice()[0] + s;
    mv.coeffs_mut_slice()[0] = new_scalar;
    mv
}

// ============================================================================
// Neg
// ============================================================================

impl Neg for CliffordNumber {
    type Output = Self;
    fn neg(self) -> Self {
        let mut out = Self::zero_unchecked(self.generator_set().clone());
        let limit = self.blade_count();
        for i in 0..limit {
            out.coeffs_mut_slice()[i] = -&self.coeffs_slice()[i];
        }
        out
    }
}

impl Neg for &CliffordNumber {
    type Output = CliffordNumber;
    fn neg(self) -> CliffordNumber {
        -(self.clone())
    }
}

// ============================================================================
// Add
// ============================================================================

impl Add for CliffordNumber {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        if self.generator_set() == rhs.generator_set() {
            let mut out = Self::zero_unchecked(self.generator_set().clone());
            let limit = self.blade_count();
            for i in 0..limit {
                out.coeffs_mut_slice()[i] = &self.coeffs_slice()[i] + &rhs.coeffs_slice()[i];
            }
            out
        } else {
            let (u, pa, pb) = self.generator_set().union_with(rhs.generator_set());
            self.reembed(u.clone(), &pa) + rhs.reembed(u, &pb)
        }
    }
}

impl Add<&Self> for CliffordNumber {
    type Output = Self;
    fn add(self, rhs: &Self) -> Self {
        self + rhs.clone()
    }
}

impl Add<CliffordNumber> for &CliffordNumber {
    type Output = CliffordNumber;
    fn add(self, rhs: CliffordNumber) -> CliffordNumber {
        self.clone() + rhs
    }
}

impl Add<&CliffordNumber> for &CliffordNumber {
    type Output = CliffordNumber;
    fn add(self, rhs: &CliffordNumber) -> CliffordNumber {
        self.clone() + rhs.clone()
    }
}

// ============================================================================
// Sub
// ============================================================================

impl Sub for CliffordNumber {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self + (-rhs)
    }
}

impl Sub<&Self> for CliffordNumber {
    type Output = Self;
    fn sub(self, rhs: &Self) -> Self {
        self - rhs.clone()
    }
}

impl Sub<CliffordNumber> for &CliffordNumber {
    type Output = CliffordNumber;
    fn sub(self, rhs: CliffordNumber) -> CliffordNumber {
        self.clone() - rhs
    }
}

impl Sub<&CliffordNumber> for &CliffordNumber {
    type Output = CliffordNumber;
    fn sub(self, rhs: &CliffordNumber) -> CliffordNumber {
        self.clone() - rhs.clone()
    }
}

// ============================================================================
// Mul (geometric product)
// ============================================================================

impl Mul for CliffordNumber {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        self.geometric_mul(&rhs)
    }
}

impl Mul<&Self> for CliffordNumber {
    type Output = Self;
    fn mul(self, rhs: &Self) -> Self {
        self.geometric_mul(rhs)
    }
}

impl Mul<CliffordNumber> for &CliffordNumber {
    type Output = CliffordNumber;
    fn mul(self, rhs: CliffordNumber) -> CliffordNumber {
        self.geometric_mul(&rhs)
    }
}

impl Mul<&CliffordNumber> for &CliffordNumber {
    type Output = CliffordNumber;
    fn mul(self, rhs: &CliffordNumber) -> CliffordNumber {
        self.geometric_mul(rhs)
    }
}

// ============================================================================
// Div (geometric product by inverse)
// ============================================================================

impl Div for CliffordNumber {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        self.geometric_mul(&rhs.geometric_inverse())
    }
}

impl Div<&Self> for CliffordNumber {
    type Output = Self;
    fn div(self, rhs: &Self) -> Self {
        self.geometric_mul(&rhs.geometric_inverse())
    }
}

impl Div<CliffordNumber> for &CliffordNumber {
    type Output = CliffordNumber;
    fn div(self, rhs: CliffordNumber) -> CliffordNumber {
        self.geometric_mul(&rhs.geometric_inverse())
    }
}

impl Div<&CliffordNumber> for &CliffordNumber {
    type Output = CliffordNumber;
    fn div(self, rhs: &CliffordNumber) -> CliffordNumber {
        self.geometric_mul(&rhs.geometric_inverse())
    }
}

// ============================================================================
// Scalar interop — CliffordNumber × Scalar
// ============================================================================

impl Mul<Scalar> for CliffordNumber {
    type Output = Self;
    fn mul(self, rhs: Scalar) -> Self {
        scale(&self, &rhs)
    }
}

impl Mul<&Scalar> for CliffordNumber {
    type Output = Self;
    fn mul(self, rhs: &Scalar) -> Self {
        scale(&self, rhs)
    }
}

impl Mul<Scalar> for &CliffordNumber {
    type Output = CliffordNumber;
    fn mul(self, rhs: Scalar) -> CliffordNumber {
        scale(self, &rhs)
    }
}

impl Mul<&Scalar> for &CliffordNumber {
    type Output = CliffordNumber;
    fn mul(self, rhs: &Scalar) -> CliffordNumber {
        scale(self, rhs)
    }
}

// Scalar × CliffordNumber
impl Mul<CliffordNumber> for Scalar {
    type Output = CliffordNumber;
    fn mul(self, rhs: CliffordNumber) -> CliffordNumber {
        scale(&rhs, &self)
    }
}

impl Mul<&CliffordNumber> for Scalar {
    type Output = CliffordNumber;
    fn mul(self, rhs: &CliffordNumber) -> CliffordNumber {
        scale(rhs, &self)
    }
}

impl Mul<CliffordNumber> for &Scalar {
    type Output = CliffordNumber;
    fn mul(self, rhs: CliffordNumber) -> CliffordNumber {
        scale(&rhs, self)
    }
}

impl Mul<&CliffordNumber> for &Scalar {
    type Output = CliffordNumber;
    fn mul(self, rhs: &CliffordNumber) -> CliffordNumber {
        scale(rhs, self)
    }
}

// CliffordNumber × IntType
impl Mul<IntType> for CliffordNumber {
    type Output = Self;
    fn mul(self, rhs: IntType) -> Self {
        scale(&self, &Scalar(ScalarRepr::Int(rhs)))
    }
}

impl Mul<IntType> for &CliffordNumber {
    type Output = CliffordNumber;
    fn mul(self, rhs: IntType) -> CliffordNumber {
        scale(self, &Scalar(ScalarRepr::Int(rhs)))
    }
}

// CliffordNumber × FloatType
impl Mul<FloatType> for CliffordNumber {
    type Output = Self;
    fn mul(self, rhs: FloatType) -> Self {
        scale(&self, &Scalar::from_float_raw(rhs))
    }
}

impl Mul<FloatType> for &CliffordNumber {
    type Output = CliffordNumber;
    fn mul(self, rhs: FloatType) -> CliffordNumber {
        scale(self, &Scalar::from_float_raw(rhs))
    }
}

// ============================================================================
// Scalar + CliffordNumber
// ============================================================================

impl Add<CliffordNumber> for Scalar {
    type Output = CliffordNumber;
    fn add(self, rhs: CliffordNumber) -> CliffordNumber {
        scalar_plus_mv(&self, rhs)
    }
}

impl Add<&CliffordNumber> for Scalar {
    type Output = CliffordNumber;
    fn add(self, rhs: &CliffordNumber) -> CliffordNumber {
        self + rhs.clone()
    }
}

impl Add<CliffordNumber> for &Scalar {
    type Output = CliffordNumber;
    fn add(self, rhs: CliffordNumber) -> CliffordNumber {
        self.clone() + rhs
    }
}

impl Add<&CliffordNumber> for &Scalar {
    type Output = CliffordNumber;
    fn add(self, rhs: &CliffordNumber) -> CliffordNumber {
        self.clone() + rhs.clone()
    }
}

// ============================================================================
// Scalar - CliffordNumber
// ============================================================================

impl Sub<CliffordNumber> for Scalar {
    type Output = CliffordNumber;
    fn sub(self, rhs: CliffordNumber) -> CliffordNumber {
        self + (-rhs)
    }
}

impl Sub<&CliffordNumber> for Scalar {
    type Output = CliffordNumber;
    fn sub(self, rhs: &CliffordNumber) -> CliffordNumber {
        self - rhs.clone()
    }
}

impl Sub<CliffordNumber> for &Scalar {
    type Output = CliffordNumber;
    fn sub(self, rhs: CliffordNumber) -> CliffordNumber {
        self.clone() - rhs
    }
}

impl Sub<&CliffordNumber> for &Scalar {
    type Output = CliffordNumber;
    fn sub(self, rhs: &CliffordNumber) -> CliffordNumber {
        self.clone() - rhs.clone()
    }
}
