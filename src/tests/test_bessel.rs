use crate::math::bessel_j;

#[test]
fn test_bessel_j_forward_regime() {
    // Test forward recurrence regime: n <= x
    // From WolframAlpha: BesselJ[2, 5.0] ≈ 0.0465651
    let x: f64 = 5.0;
    let n = 2;
    let result = bessel_j(n, x);
    assert!((result - 0.046_565_116_277_749).abs() < 1e-7);
}

#[test]
fn test_bessel_j_backward_regime_miller() {
    // Test backward recurrence (Miller's Algo) regime: n > x
    // Forward recurrence is unstable here.
    // From WolframAlpha: BesselJ[10, 2.0] ≈ 2.515386e-7
    let x: f64 = 2.0;
    let n = 10;
    let result = bessel_j(n, x);
    // Tolerance might need to be relaxed depending on implementation precision
    assert!((result - 2.515_386_286e-7).abs() < 1e-10);
}

#[test]
fn test_bessel_j_negative_order() {
    // J_{-n}(x) = (-1)^n J_n(x)
    let x: f64 = 3.0;
    let n = 2;
    let j_n = bessel_j(n, x);
    let j_neg_n = bessel_j(-n, x);

    // (-1)^2 = 1, so J_{-2} == J_{2}
    assert!((j_n - j_neg_n).abs() < 1e-10);

    let n = 3;
    let j_n = bessel_j(n, x);
    let j_neg_n = bessel_j(-n, x);

    // (-1)^3 = -1, so J_{-3} == -J_{3}
    assert!((j_n + j_neg_n).abs() < 1e-10);
}

#[test]
fn test_bessel_j_zero() {
    assert_eq!(bessel_j(0, 0.0_f64), 1.0);
    assert_eq!(bessel_j(1, 0.0_f64), 0.0);
    assert_eq!(bessel_j(10, 0.0_f64), 0.0);
}

#[test]
fn test_bessel_j_hybrid_transition() {
    // Test near the transition boundary n ≈ |x|
    // This validates both algorithms produce consistent results near crossover
    let x: f64 = 5.0;

    // n < x: forward recurrence
    let j4 = bessel_j(4, x);
    let j4_ref = 0.391_232_351; // WolframAlpha

    // n > x: Miller's algorithm
    let j10 = bessel_j(10, x);
    let j10_ref = 0.001_467_802_647_31; // Verified against SciPy

    assert!((j4 - j4_ref).abs() < 1e-7);
    assert!((j10 - j10_ref).abs() < 1e-10);
}

#[test]
fn test_bessel_j_very_large_n() {
    // Test stability for large n
    let x: f64 = 2.0;
    let n = 50;

    // This should not overflow or underflow
    let jn = bessel_j(n, x);
    assert!(jn.is_finite());

    // Value should be very small but non-zero
    let jn_val = jn;
    assert!(jn_val.abs() > 0.0);
    assert!(jn_val.abs() < 1e-20); // Decays factorially
}
