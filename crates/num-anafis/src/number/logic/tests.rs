use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

use super::clifford_number::CliffordNumber;
use super::float_ops::float_from_f64;
#[cfg(any(feature = "backend_big_astro", feature = "backend_big_rug"))]
use super::float_ops::{float_add, float_clone};
use super::scalar::Number;

fn hash_of(value: &Number) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn nan_values_are_equal_and_totally_ordered() {
    let lhs = Number::float(f64::NAN);
    let rhs = Number::float(f64::NAN);

    assert_eq!(lhs, rhs);
    assert_eq!(lhs.cmp(&rhs), Ordering::Equal);
    assert_eq!(lhs.partial_cmp(&rhs), Some(Ordering::Equal));
}

#[test]
fn nan_orders_after_finite_values() {
    let finite = Number::float(42.0);
    let nan = Number::float(f64::NAN);

    assert_eq!(finite.cmp(&nan), Ordering::Less);
    assert_eq!(nan.cmp(&finite), Ordering::Greater);
}

#[test]
fn rational_and_float_remain_distinct_values() {
    let rational = Number::rational(1, 2);
    let float = Number::float(0.5);

    assert_ne!(rational, float);
    assert_eq!(rational.cmp(&float), Ordering::Less);
    assert_eq!(float.cmp(&rational), Ordering::Greater);
}

#[test]
fn hash_is_consistent_for_nan_and_signed_zero() {
    let nan_lhs = Number::float(f64::NAN);
    let nan_rhs = Number::float(f64::NAN);
    assert_eq!(hash_of(&nan_lhs), hash_of(&nan_rhs));

    let pos_zero = Number::from_backend_float_unchecked(float_from_f64(0.0));
    let neg_zero = Number::from_backend_float_unchecked(float_from_f64(-0.0));
    assert_eq!(pos_zero, neg_zero);
    assert_eq!(hash_of(&pos_zero), hash_of(&neg_zero));
}

#[test]
fn division_by_zero_has_explicit_and_operator_behaviors() {
    let one = Number::from(1_i64);
    let zero = Number::from(0_i64);

    assert_eq!(Number::div(&one, &zero), None);

    let via_operator = one / zero;
    assert_eq!(via_operator, Number::float(f64::NAN));
}

#[cfg(any(feature = "backend_big_astro", feature = "backend_big_rug"))]
#[test]
fn high_precision_float_to_i64_exact_handles_large_exact_integers() {
    let one = float_from_f64(1.0);
    let mut value = float_clone(&one);

    for _ in 0..53 {
        value = float_add(&value, &value);
    }
    value = float_add(&value, &one);

    let number = Number::from_backend_float_unchecked(value);
    assert_eq!(number.to_i64_exact(), Some(9_007_199_254_740_993));
}

#[cfg(any(feature = "backend_big_astro", feature = "backend_big_rug"))]
#[test]
fn bigint_perfect_roots_work_for_very_large_values() {
    let root = Number::from(2_i64)
        .pow_i64(200)
        .expect("2^200 should be representable");
    let square = &root * &root;
    let cube = &square * &root;

    assert_eq!(square.perfect_square_root(), Some(root.clone()));
    assert_eq!(cube.perfect_cube_root(), Some(root.clone()));
    assert_eq!((-&cube).perfect_cube_root(), Some(-&root));
}

#[cfg(any(feature = "backend_big_astro", feature = "backend_big_rug"))]
#[test]
#[allow(
    clippy::print_stdout,
    reason = "Intentional diagnostics in test mode to track Number stack footprint for heavy backends"
)]
fn print_number_size_for_heavy_backends() {
    println!("Size of Number: {}", std::mem::size_of::<Number>());
}

#[test]
fn clifford_number_complex_i_squared_is_minus_one() {
    let i = CliffordNumber::i();
    let result = &i * &i;

    assert_eq!(*result.coeff(0), Number::from(-1_i64));
    assert!(result.coeff(1).is_zero());
}

#[test]
fn clifford_number_dual_eps_squared_is_zero() {
    let eps = CliffordNumber::eps();
    let result = &eps * &eps;
    assert!(result.is_zero());
}

#[test]
fn clifford_number_convenience_constructors_fill_expected_components() {
    let z = CliffordNumber::complex(Number::from(1_i64), Number::from(2_i64));
    assert_eq!(*z.coeff(0), Number::from(1_i64));
    assert_eq!(*z.coeff(1), Number::from(2_i64));

    let d = CliffordNumber::dual(Number::from(3_i64), Number::from(5_i64));
    assert_eq!(*d.coeff(0), Number::from(3_i64));
    assert_eq!(*d.coeff(1), Number::from(5_i64));
}

#[test]
fn clifford_number_basis_vectors_anticommute() {
    let e1 = CliffordNumber::e1();
    let e2 = CliffordNumber::e2();
    let lhs = &e1 * &e2;
    let rhs = &e2 * &e1;

    assert_eq!(lhs, -rhs);
}

#[test]
fn clifford_number_complex_sin_matches_known_value() {
    use crate::Evaluate;
    use crate::Signature;

    let sig = Signature::new(0, 1, 0).expect("valid complex signature");
    let mut z = CliffordNumber::zero(sig, 1).expect("valid Clifford zero constructor");
    z.set_coeff(0, Number::from(1_i64));
    z.set_coeff(1, Number::from(2_i64));

    let sin_z = z.sin();
    let expected_re = Number::float(3.165_778_513_216_168);
    let expected_im = Number::float(1.959_601_041_421_606_3);

    assert!(
        sin_z
            .coeff(0)
            .approx_eq_number(&expected_re, &Number::float(1e-12))
    );
    assert!(
        sin_z
            .coeff(1)
            .approx_eq_number(&expected_im, &Number::float(1e-12))
    );
}
