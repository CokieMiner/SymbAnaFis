use super::CliffordNumber;
use super::gen_id;
use super::types::{GeneratorSet, scalar_one};
use crate::r;

// ============================================================================
// CGA Cl(4,1) full generator set
// ============================================================================

/// The full CGA generator set: `(e1, e2, e3, e+, e−)` with signature `(4,1)`.
///
/// This is the canonical generator set for Conformal Geometric Algebra.
/// All derived constructions (quaternions, conformal points, etc.) live in
/// this algebra.
#[must_use]
pub fn cga_gens() -> GeneratorSet {
    GeneratorSet::from_sorted(alloc::vec![
        (gen_id::E1, 1_i8),
        (gen_id::E2, 1_i8),
        (gen_id::E3, 1_i8),
        (gen_id::E_PLUS, 1_i8),
        (gen_id::E_MINUS, -1_i8),
    ])
}

// ============================================================================
// Base generators — each is a grade-1 element of the full CGA algebra
// ============================================================================

/// Helper: build a CGA multivector with a single generator blade set to 1.
fn cga_unit(bit: usize) -> CliffordNumber {
    let gens = cga_gens();
    let mut mv = CliffordNumber::zero_unchecked(gens);
    mv.coeffs_mut_slice()[1_usize << bit] = scalar_one();
    mv
}

/// Euclidean basis vector `e1` with `e1² = +1`.
#[must_use]
pub fn e1() -> CliffordNumber {
    cga_unit(0) // bit 0 → blade mask 0b00001
}

/// Euclidean basis vector `e2` with `e2² = +1`.
#[must_use]
pub fn e2() -> CliffordNumber {
    cga_unit(1) // bit 1 → blade mask 0b00010
}

/// Euclidean basis vector `e3` with `e3² = +1`.
#[must_use]
pub fn e3() -> CliffordNumber {
    cga_unit(2) // bit 2 → blade mask 0b00100
}

/// Extra positive dimension `e+` with `e+² = +1`.
#[must_use]
pub fn e_plus() -> CliffordNumber {
    cga_unit(3) // bit 3 → blade mask 0b01000
}

/// Extra negative dimension `e−` with `e−² = −1`.
#[must_use]
pub fn e_minus() -> CliffordNumber {
    cga_unit(4) // bit 4 → blade mask 0b10000
}

// ============================================================================
// Derived constructions — built from base generators, NOT new generators
// ============================================================================

/// Conformal origin point: `eₒ = ½(e− − e+)`.
///
/// Properties: `eₒ² = 0`, `⟨eₒ e∞⟩₀ = −1` (inner product), so `eₒ·e∞ + e∞·eₒ = −2`.
#[must_use]
pub fn orig() -> CliffordNumber {
    let gens = cga_gens();
    let half = r(1, 2);
    let mut mv = CliffordNumber::zero_unchecked(gens);
    // e− is bit 4 → blade mask 0b10000 = 16
    mv.coeffs_mut_slice()[16] = half.clone();
    // e+ is bit 3 → blade mask 0b01000 = 8, with negative sign
    mv.coeffs_mut_slice()[8] = -half;
    mv
}

/// Conformal infinity: `e∞ = e− + e+`.
///
/// Properties: `e∞² = 0` (nilpotent, like `ε` in dual numbers),
/// `eₒ · e∞ + e∞ · eₒ = −1`.
#[must_use]
pub fn inf() -> CliffordNumber {
    let gens = cga_gens();
    let mut mv = CliffordNumber::zero_unchecked(gens);
    // e− is bit 4 → blade mask 0b10000 = 16
    mv.coeffs_mut_slice()[16] = scalar_one();
    // e+ is bit 3 → blade mask 0b01000 = 8
    mv.coeffs_mut_slice()[8] = scalar_one();
    mv
}

/// Quaternion imaginary unit `i = −e2·e3` (grade-2 bivector).
///
/// Satisfies `i² = j² = k² = i·j·k = −1`.
#[must_use]
pub fn qi() -> CliffordNumber {
    let gens = cga_gens();
    let mut mv = CliffordNumber::zero_unchecked(gens);
    // e2∧e3 → bits 1,2 → blade mask 0b00110 = 6
    mv.coeffs_mut_slice()[6] = -scalar_one();
    mv
}

/// Quaternion imaginary unit `j = −e3·e1` (grade-2 bivector).
///
/// Satisfies `i² = j² = k² = i·j·k = −1`.
#[must_use]
pub fn qj() -> CliffordNumber {
    let gens = cga_gens();
    let mut mv = CliffordNumber::zero_unchecked(gens);
    // e3∧e1 → bits 0,2 → blade mask 0b00101 = 5
    mv.coeffs_mut_slice()[5] = scalar_one();
    mv
}

/// Quaternion imaginary unit `k = −e1·e2` (grade-2 bivector).
///
/// Satisfies `i² = j² = k² = i·j·k = −1`.
#[must_use]
pub fn qk() -> CliffordNumber {
    let gens = cga_gens();
    let mut mv = CliffordNumber::zero_unchecked(gens);
    // e1∧e2 → bits 0,1 → blade mask 0b00011 = 3
    mv.coeffs_mut_slice()[3] = -scalar_one();
    mv
}

/// 3D pseudoscalar `I₃ = e1·e2·e3` (grade-3 trivector).
///
/// Properties: `I₃² = −1`, commutes with all even-grade elements of the
/// Euclidean sub-algebra, making it behave as the complex imaginary unit `i`
/// for the even sub-algebra.
#[must_use]
pub fn pseudo3d() -> CliffordNumber {
    let gens = cga_gens();
    let mut mv = CliffordNumber::zero_unchecked(gens);
    // e1∧e2∧e3 → bits 0,1,2 → blade mask 0b00111 = 7
    mv.coeffs_mut_slice()[7] = scalar_one();
    mv
}

/// 5D pseudoscalar `I₅ = e1·e2·e3·e+·e−` (grade-5 blade).
///
/// Properties: `I₅² = +1` (signature dependent).
#[must_use]
pub fn pseudo5d() -> CliffordNumber {
    let gens = cga_gens();
    let mut mv = CliffordNumber::zero_unchecked(gens);
    // all 5 bits → blade mask 0b11111 = 31
    mv.coeffs_mut_slice()[31] = scalar_one();
    mv
}

// ============================================================================
// Aliases — convenient names for derived CGA elements
// ============================================================================

/// Dual / nilpotent unit `ε` — alias for [`inf`] (`e∞ = e− + e+`).
///
/// `ε² = 0`, exactly like the classical dual-number epsilon.
#[must_use]
#[inline]
pub fn eps() -> CliffordNumber {
    inf()
}

/// Complex imaginary unit `i` — alias for [`pseudo3d`] (`I₃ = e1·e2·e3`).
///
/// `ci² = −1`, commutes with all even-grade elements of the Euclidean
/// sub-algebra, behaving exactly like the complex imaginary unit.
#[must_use]
#[inline]
pub fn ci() -> CliffordNumber {
    pseudo3d()
}

/// Split-complex unit `j` — alias for [`e_plus`] (`e+`).
///
/// `sj² = +1`, behaving like the split-complex unit.
#[must_use]
#[inline]
pub fn sj() -> CliffordNumber {
    e_plus()
}

#[cfg(test)]
mod tests {
    use super::super::types::scalar_one;
    use super::*;

    fn test_scalar(val: crate::number::Scalar) -> CliffordNumber {
        CliffordNumber::scalar(cga_gens(), val).expect("scalar creation is infallible")
    }

    #[test]
    fn test_cga_base_squares() {
        // e1² = e2² = e3² = e+² = +1
        assert_eq!(&e1() * &e1(), test_scalar(scalar_one()));
        assert_eq!(&e2() * &e2(), test_scalar(scalar_one()));
        assert_eq!(&e3() * &e3(), test_scalar(scalar_one()));
        assert_eq!(&e_plus() * &e_plus(), test_scalar(scalar_one()));

        // e−² = -1
        assert_eq!(&e_minus() * &e_minus(), test_scalar(-scalar_one()));
    }

    #[test]
    fn test_derived_identities() {
        // ε² = 0 (nilpotent, like dual numbers)
        assert!(
            (&eps() * &eps())
                .coeffs_slice()
                .iter()
                .all(crate::number::logic::traits::Number::is_zero)
        );

        // ci² = -1 (complex imaginary, like complex numbers)
        let ci_sq = &ci() * &ci();
        assert_eq!(ci_sq.coeffs_slice()[0], -scalar_one());

        // sj² = +1 (split-complex unit)
        let sj_sq = &sj() * &sj();
        assert_eq!(sj_sq.coeffs_slice()[0], scalar_one());

        // Quaternions: i² = j² = k² = ijk = -1
        assert_eq!((&qi() * &qi()).coeffs_slice()[0], -scalar_one());
        assert_eq!((&qj() * &qj()).coeffs_slice()[0], -scalar_one());
        assert_eq!((&qk() * &qk()).coeffs_slice()[0], -scalar_one());

        let ijk = &(&qi() * &qj()) * &qk();
        assert_eq!(ijk.coeffs_slice()[0], -scalar_one());
    }

    #[test]
    fn test_conformal_points() {
        // eₒ² = 0, e∞² = 0
        assert!(
            (&orig() * &orig())
                .coeffs_slice()
                .iter()
                .all(crate::number::logic::traits::Number::is_zero)
        );
        assert!(
            (&inf() * &inf())
                .coeffs_slice()
                .iter()
                .all(crate::number::logic::traits::Number::is_zero)
        );

        // eₒ · e∞ + e∞ · eₒ = -2 (since eₒ · e∞ = -1)
        let op = &orig() * &inf();
        let po = &inf() * &orig();
        let dot = &op + &po;
        assert_eq!(dot.coeffs_slice()[0], -(scalar_one() + scalar_one()));
    }
}
