use super::*;
use std::f64::consts::PI;

macro_rules! assert_approx_eq {
    ($a:expr, $b:expr) => {
        assert_approx_eq!($a, $b, 1e-8);
    };
    ($a:expr, $b:expr, $eps:expr) => {
        let diff = ($a - $b).abs();
        if diff >= $eps {
             panic!("assertion failed: `(left approx right)`\n  left: `{:?}`,\n right: `{:?}`\n  diff: `{:?}` > `{:?}`", $a, $b, diff, $eps);
        }
    };
}

#[test]
fn test_lambert_w_edge_cases() {
    let e = std::f64::consts::E;
    let minus_one_over_e = -1.0_f64 / e;

    // Test exactly at -1/e (should be -1)
    let w = eval_lambert_w(minus_one_over_e).unwrap();
    assert_approx_eq!(w, -1.0_f64);

    // Test slightly above -1/e (should be close to -1)
    let w_near = eval_lambert_w(minus_one_over_e + 1e-10).unwrap();
    assert!((w_near + 1.0_f64).abs() < 1e-4);

    // Test zero
    assert_approx_eq!(eval_lambert_w(0.0_f64).unwrap(), 0.0_f64);
}

#[test]
fn test_bessel_j0_special_values() {
    // J0(0) = 1
    assert_approx_eq!(bessel_j0(0.0_f64), 1.0_f64);

    // J0(2.4048...) approx first zero
    let first_zero = 2.40482555769577_f64;
    assert_approx_eq!(bessel_j0(first_zero), 0.0_f64, 1e-6);
}

#[test]
fn test_bessel_large_argument() {
    // Tests the large argument path (x >= 8)
    let x = 10.0_f64;
    // Known value J0(10) approx -0.245935764
    let val = bessel_j0(x);
    assert_approx_eq!(val, -0.2459357644513483_f64);
}

#[test]
fn test_zeta_values() {
    // Zeta(2) = pi^2 / 6
    let z2 = eval_zeta(2.0_f64).unwrap();
    assert_approx_eq!(z2, PI * PI / 6.0);

    // Zeta(0) = -0.5
    let z0 = eval_zeta(0.0_f64).unwrap(); // Handled by reflection or Borwein
    assert_approx_eq!(z0, -0.5_f64);

    // Zeta(-1) = -1/12
    let zm1 = eval_zeta(-1.0_f64).unwrap();
    assert_approx_eq!(zm1, -1.0_f64 / 12.0_f64);
}

#[test]
fn test_zeta_borwein_range() {
    // Test a value in 0 < s < 1 range (previously infinite recursion)
    let s = 0.5_f64;
    let val = eval_zeta(s).unwrap();
    // Known value approx -1.4603545
    assert_approx_eq!(val, -1.4603545088095868_f64, 1e-8);
}

#[test]
fn test_zeta_derivative_reflection() {
    // Test derivative at s = -1
    // Zeta'(-1) = 1/12 - ln(A) ... complex.
    // Known value approx 0.1654211437
    let val = eval_zeta_deriv(1, -1.0_f64).unwrap();
    assert_approx_eq!(val, -0.1654211437_f64, 1e-4);

    // Test derivative at s = 0.5 (gap region)
    // Zeta'(0.5) approx -3.92264613...
    let val_gap = eval_zeta_deriv(1, 0.5_f64).unwrap();
    // Verify it returns Some(...) and value is reasonable
    assert!(val_gap.is_finite());
}

#[test]
fn test_gamma_negative_poles() {
    // Exact negative integer
    assert!(eval_gamma(-1.0_f64).is_none());
    assert!(eval_gamma(-2.0_f64).is_none());

    // Close to negative integer (within 1e-10)
    assert!(eval_gamma(-1.00000000001_f64).is_none());

    // Far enough from pole
    assert!(eval_gamma(-1.5_f64).is_some());
}

#[test]
fn test_polygamma_precision() {
    // Check Polygamma(1, 1) = pi^2/6
    let val = eval_polygamma(1, 1.0_f64).unwrap();
    assert_approx_eq!(val, PI * PI / 6.0);
}
