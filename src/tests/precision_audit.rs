use crate::parser::parse;
use crate::{ExprKind, diff};
use std::collections::{HashMap, HashSet};

const REL_TOL: f64 = 1e-12; // Relative tolerance for standard functions (trig, exp, log)
const REL_TOL_BESSEL: f64 = 1e-7; // Bessel functions: ~7-8 decimal places
const REL_TOL_GAMMA: f64 = 1e-8; // Gamma functions: ~8-9 decimal places
const ABS_TOL: f64 = 1e-14; // Absolute tolerance near zero

/// Helper to evaluate expression string
fn eval(expr_str: &str) -> f64 {
    let expr = parse(expr_str, &HashSet::new(), &HashSet::new(), None).unwrap();
    let vars: HashMap<&str, f64> = HashMap::new();
    match &expr.evaluate(&vars, &HashMap::new()).kind {
        ExprKind::Number(n) => *n,
        other => panic!("Expected number, got {:?}", other),
    }
}

/// Helper to evaluate with variable
fn eval_with_var(expr_str: &str, var: &str, val: f64) -> f64 {
    let expr = parse(expr_str, &HashSet::new(), &HashSet::new(), None).unwrap();
    let vars: HashMap<&str, f64> = [(var, val)].into_iter().collect();
    match &expr.evaluate(&vars, &HashMap::new()).kind {
        ExprKind::Number(n) => *n,
        other => panic!("Expected number, got {:?}", other),
    }
}

/// Check relative error: |a - b| / |b|
fn check_rel_error(computed: f64, expected: f64, tol: f64, context: &str) {
    if expected.abs() < ABS_TOL {
        // Near zero, use absolute error
        let abs_err = (computed - expected).abs();
        assert!(
            abs_err < ABS_TOL,
            "{}: abs error {} > {} (computed={}, expected={})",
            context,
            abs_err,
            ABS_TOL,
            computed,
            expected
        );
    } else {
        let rel_err = ((computed - expected) / expected).abs();
        assert!(
            rel_err < tol,
            "{}: rel error {} > {} (computed={}, expected={})",
            context,
            rel_err,
            tol,
            computed,
            expected
        );
    }
}

// ============================================================================
// TRIGONOMETRIC FUNCTIONS
// Reference: DLMF §4.21-4.28
// ============================================================================

#[test]
fn precision_sin() {
    // Known values from DLMF Table 4.3.1
    check_rel_error(eval("sin(0)"), 0.0, REL_TOL, "sin(0)");
    check_rel_error(eval("sin(1.5707963267948966)"), 1.0, REL_TOL, "sin(π/2)");
    check_rel_error(eval("sin(3.141_592_653_589_793)"), 0.0, REL_TOL, "sin(π)");
    check_rel_error(
        eval("sin(0.523_598_775_598_298_8)"),
        0.5,
        REL_TOL,
        "sin(π/6)",
    );

    // sin(π/4) = √2/2
    check_rel_error(
        eval("sin(0.7853981633974483)"),
        std::f64::consts::FRAC_1_SQRT_2,
        REL_TOL,
        "sin(π/4)",
    );
}

#[test]
fn precision_cos() {
    check_rel_error(eval("cos(0)"), 1.0, REL_TOL, "cos(0)");
    check_rel_error(eval("cos(1.5707963267948966)"), 0.0, REL_TOL, "cos(π/2)");
    check_rel_error(eval("cos(3.141592653589793)"), -1.0, REL_TOL, "cos(π)");
    check_rel_error(eval("cos(1.0471975511965976)"), 0.5, REL_TOL, "cos(π/3)");
}

#[test]
fn precision_tan() {
    check_rel_error(eval("tan(0)"), 0.0, REL_TOL, "tan(0)");
    check_rel_error(eval("tan(0.7853981633974483)"), 1.0, REL_TOL, "tan(π/4)");
    check_rel_error(
        eval("tan(0.4636476090008061)"),
        0.5,
        REL_TOL,
        "tan(atan(0.5))",
    );
}

#[test]
fn precision_inverse_trig() {
    // asin
    check_rel_error(eval("asin(0)"), 0.0, REL_TOL, "asin(0)");
    check_rel_error(
        eval("asin(0.5)"),
        0.523_598_775_598_298_8,
        REL_TOL,
        "asin(0.5) = π/6",
    );
    check_rel_error(
        eval("asin(1)"),
        std::f64::consts::FRAC_PI_2,
        REL_TOL,
        "asin(1)",
    );

    // acos
    check_rel_error(eval("acos(1)"), 0.0, REL_TOL, "acos(1)");
    check_rel_error(
        eval("acos(0)"),
        std::f64::consts::FRAC_PI_2,
        REL_TOL,
        "acos(0)",
    );

    // atan
    check_rel_error(eval("atan(0)"), 0.0, REL_TOL, "atan(0)");
    check_rel_error(
        eval("atan(1)"),
        std::f64::consts::FRAC_PI_4,
        REL_TOL,
        "atan(1)",
    );
}

// ============================================================================
// HYPERBOLIC FUNCTIONS
// Reference: DLMF §4.35-4.37
// ============================================================================

#[test]
fn precision_sinh() {
    check_rel_error(eval("sinh(0)"), 0.0, REL_TOL, "sinh(0)");
    check_rel_error(eval("sinh(1)"), 1.175_201_193_643_801_4, REL_TOL, "sinh(1)");
    check_rel_error(
        eval("sinh(-1)"),
        -1.175_201_193_643_801_4,
        REL_TOL,
        "sinh(-1)",
    );
}

#[test]
fn precision_cosh() {
    check_rel_error(eval("cosh(0)"), 1.0, REL_TOL, "cosh(0)");
    check_rel_error(eval("cosh(1)"), 1.543_080_634_815_243_7, REL_TOL, "cosh(1)");
    // cosh is even
    check_rel_error(
        eval("cosh(-1)"),
        1.543_080_634_815_243_7,
        REL_TOL,
        "cosh(-1)",
    );
}

#[test]
fn precision_tanh() {
    check_rel_error(eval("tanh(0)"), 0.0, REL_TOL, "tanh(0)");
    check_rel_error(eval("tanh(1)"), 0.761_594_155_955_764_9, REL_TOL, "tanh(1)");
    // Verify identity: tanh² + sech² = 1
    let tanh1 = eval("tanh(1)");
    let sech1 = eval("sech(1)");
    check_rel_error(tanh1.powi(2) + sech1.powi(2), 1.0, REL_TOL, "tanh²+sech²=1");
}

#[test]
fn precision_inverse_hyperbolic() {
    // asinh(x) = ln(x + √(x²+1))
    check_rel_error(eval("asinh(0)"), 0.0, REL_TOL, "asinh(0)");
    check_rel_error(eval("asinh(1)"), 0.881_373_587_019_543, REL_TOL, "asinh(1)");

    // acosh(x) = ln(x + √(x²-1))
    check_rel_error(eval("acosh(1)"), 0.0, REL_TOL, "acosh(1)");
    check_rel_error(
        eval("acosh(2)"),
        1.316_957_896_924_816_6,
        REL_TOL,
        "acosh(2)",
    );

    // atanh
    check_rel_error(eval("atanh(0)"), 0.0, REL_TOL, "atanh(0)");
    check_rel_error(
        eval("atanh(0.5)"),
        0.549_306_144_334_054_9,
        REL_TOL,
        "atanh(0.5)",
    );
}

// ============================================================================
// EXPONENTIAL AND LOGARITHMIC
// Reference: DLMF §4.2
// ============================================================================

#[test]
fn precision_exp() {
    check_rel_error(eval("exp(0)"), 1.0, REL_TOL, "exp(0)");
    check_rel_error(eval("exp(1)"), std::f64::consts::E, REL_TOL, "exp(1)");
    check_rel_error(
        eval("exp(2)"),
        std::f64::consts::E.powi(2),
        REL_TOL,
        "exp(2)",
    );
    check_rel_error(
        eval("exp(-1)"),
        1.0 / std::f64::consts::E,
        REL_TOL,
        "exp(-1)",
    );
}

#[test]
fn precision_ln() {
    check_rel_error(eval("ln(1)"), 0.0, REL_TOL, "ln(1)");
    check_rel_error(eval("ln(2.718_281_828_459_045)"), 1.0, REL_TOL, "ln(e)");
    check_rel_error(eval("ln(2)"), std::f64::consts::LN_2, REL_TOL, "ln(2)");
    check_rel_error(eval("ln(10)"), std::f64::consts::LN_10, REL_TOL, "ln(10)");
}

#[test]
fn precision_log_bases() {
    // log10
    check_rel_error(eval("log10(1)"), 0.0, REL_TOL, "log10(1)");
    check_rel_error(eval("log10(10)"), 1.0, REL_TOL, "log10(10)");
    check_rel_error(eval("log10(100)"), 2.0, REL_TOL, "log10(100)");

    // log2
    check_rel_error(eval("log2(1)"), 0.0, REL_TOL, "log2(1)");
    check_rel_error(eval("log2(2)"), 1.0, REL_TOL, "log2(2)");
    check_rel_error(eval("log2(8)"), 3.0, REL_TOL, "log2(8)");
}

#[test]
fn precision_sqrt_cbrt() {
    // sqrt
    check_rel_error(eval("sqrt(0)"), 0.0, REL_TOL, "sqrt(0)");
    check_rel_error(eval("sqrt(4)"), 2.0, REL_TOL, "sqrt(4)");
    check_rel_error(
        eval("sqrt(2)"),
        std::f64::consts::SQRT_2,
        REL_TOL,
        "sqrt(2)",
    );

    // cbrt
    check_rel_error(eval("cbrt(0)"), 0.0, REL_TOL, "cbrt(0)");
    check_rel_error(eval("cbrt(8)"), 2.0, REL_TOL, "cbrt(8)");
    check_rel_error(eval("cbrt(27)"), 3.0, REL_TOL, "cbrt(27)");
    check_rel_error(eval("cbrt(-8)"), -2.0, REL_TOL, "cbrt(-8)");
}

// ============================================================================
// ERROR FUNCTIONS
// Reference: DLMF §7.2
// ============================================================================

#[test]
fn precision_erf() {
    // Reference values from DLMF Table 7.3.1
    check_rel_error(eval("erf(0)"), 0.0, REL_TOL_GAMMA, "erf(0)");
    check_rel_error(
        eval("erf(0.5)"),
        0.520_499_877_813_046_5,
        REL_TOL_GAMMA,
        "erf(0.5)",
    );
    check_rel_error(
        eval("erf(1)"),
        0.842_700_792_949_714_9,
        REL_TOL_GAMMA,
        "erf(1)",
    );
    check_rel_error(
        eval("erf(2)"),
        0.995_322_265_018_952_7,
        REL_TOL_GAMMA,
        "erf(2)",
    );

    // erf is odd
    check_rel_error(
        eval("erf(-1)"),
        -0.842_700_792_949_714_9,
        REL_TOL_GAMMA,
        "erf(-1)",
    );
}

#[test]
fn precision_erfc() {
    // erfc(x) = 1 - erf(x)
    check_rel_error(eval("erfc(0)"), 1.0, REL_TOL_GAMMA, "erfc(0)");

    // Verify identity: erf + erfc = 1
    for x in [0.0, 0.5, 1.0, 2.0] {
        let erf_val = eval_with_var("erf(x)", "x", x);
        let erfc_res = eval_with_var("erfc(x)", "x", x);
        check_rel_error(
            erf_val + erfc_res,
            1.0,
            REL_TOL_GAMMA,
            &format!("erf({x}) + erfc({x}) = 1"),
        );
    }
}

// ============================================================================
// GAMMA FUNCTIONS
// Reference: DLMF §5.2, §5.11, §5.15
// ============================================================================

#[test]
fn precision_gamma() {
    // Γ(n) = (n-1)! for positive integers
    check_rel_error(eval("gamma(1)"), 1.0, REL_TOL_GAMMA, "Γ(1) = 1");
    check_rel_error(eval("gamma(2)"), 1.0, REL_TOL_GAMMA, "Γ(2) = 1!");
    check_rel_error(eval("gamma(3)"), 2.0, REL_TOL_GAMMA, "Γ(3) = 2!");
    check_rel_error(eval("gamma(4)"), 6.0, REL_TOL_GAMMA, "Γ(4) = 3!");
    check_rel_error(eval("gamma(5)"), 24.0, REL_TOL_GAMMA, "Γ(5) = 4!");
    check_rel_error(eval("gamma(6)"), 120.0, REL_TOL_GAMMA, "Γ(6) = 5!");

    // Γ(1/2) = √π
    check_rel_error(
        eval("gamma(0.5)"),
        std::f64::consts::PI.sqrt(),
        REL_TOL_GAMMA,
        "Γ(1/2) = √π",
    );

    // Γ(3/2) = √π/2
    check_rel_error(
        eval("gamma(1.5)"),
        std::f64::consts::PI.sqrt() / 2.0,
        REL_TOL_GAMMA,
        "Γ(3/2)",
    );
}

#[test]
fn precision_digamma() {
    // ψ(1) = -γ (Euler-Mascheroni constant)
    const EULER_GAMMA: f64 = 0.577_215_664_901_532_9;
    check_rel_error(eval("digamma(1)"), -EULER_GAMMA, REL_TOL_GAMMA, "ψ(1) = -γ");

    // ψ(2) = 1 - γ
    check_rel_error(eval("digamma(2)"), 1.0 - EULER_GAMMA, REL_TOL_GAMMA, "ψ(2)");
}

#[test]
fn precision_trigamma() {
    // ψ₁(1) = π²/6
    let expected = std::f64::consts::PI.powi(2) / 6.0;
    // Trigamma uses asymptotic series, slightly lower precision expected (~9 decimal places)
    check_rel_error(eval("trigamma(1)"), expected, REL_TOL_GAMMA, "ψ₁(1) = π²/6");

    // ψ₁(2) = π²/6 - 1
    check_rel_error(eval("trigamma(2)"), expected - 1.0, REL_TOL_GAMMA, "ψ₁(2)");
}

#[test]
fn precision_beta() {
    // B(a,b) = Γ(a)Γ(b)/Γ(a+b)
    check_rel_error(eval("beta(1, 1)"), 1.0, REL_TOL_GAMMA, "B(1,1) = 1");
    check_rel_error(eval("beta(2, 2)"), 1.0 / 6.0, REL_TOL_GAMMA, "B(2,2) = 1/6");

    // Symmetry: B(a,b) = B(b,a)
    let beta23 = eval("beta(2, 3)");
    let beta32 = eval("beta(3, 2)");
    check_rel_error(beta23, beta32, REL_TOL_GAMMA, "B(2,3) = B(3,2)");
}

// ============================================================================
// BESSEL FUNCTIONS
// Reference: DLMF §10, A&S §9.1
// ============================================================================

#[test]
fn precision_bessel_j0() {
    // J₀(0) = 1
    check_rel_error(eval("besselj(0, 0)"), 1.0, REL_TOL_BESSEL, "J₀(0) = 1");

    // Reference values from A&S Table 9.1
    check_rel_error(
        eval("besselj(0, 1)"),
        0.765_197_686_557_966_6,
        REL_TOL_BESSEL,
        "J₀(1)",
    );
    check_rel_error(
        eval("besselj(0, 2)"),
        0.223_890_779_141_235_7,
        REL_TOL_BESSEL,
        "J₀(2)",
    );
}

#[test]
fn precision_bessel_j1() {
    // J₁(0) = 0
    check_rel_error(eval("besselj(1, 0)"), 0.0, REL_TOL_BESSEL, "J₁(0) = 0");

    // Reference values from A&S Table 9.1
    check_rel_error(
        eval("besselj(1, 1)"),
        0.440_050_585_744_933_5,
        REL_TOL_BESSEL,
        "J₁(1)",
    );
    check_rel_error(
        eval("besselj(1, 2)"),
        0.576_724_807_756_873_4,
        REL_TOL_BESSEL,
        "J₁(2)",
    );
}

#[test]
fn precision_bessel_y0() {
    // Y₀(1) ≈ 0.0883 (A&S Table 9.1)
    check_rel_error(
        eval("bessely(0, 1)"),
        0.088_256_964_215_676_96,
        REL_TOL_BESSEL,
        "Y₀(1)",
    );
    check_rel_error(
        eval("bessely(0, 2)"),
        0.510_375_672_649_745_1,
        REL_TOL_BESSEL,
        "Y₀(2)",
    );
}

#[test]
fn precision_bessel_i0() {
    // I₀(0) = 1
    check_rel_error(eval("besseli(0, 0)"), 1.0, REL_TOL_BESSEL, "I₀(0) = 1");

    // Reference values from A&S Table 9.8
    check_rel_error(
        eval("besseli(0, 1)"),
        1.266_065_877_752_008_4,
        REL_TOL_BESSEL,
        "I₀(1)",
    );
}

#[test]
fn precision_bessel_i1() {
    // I₁(0) = 0
    check_rel_error(eval("besseli(1, 0)"), 0.0, REL_TOL_BESSEL, "I₁(0) = 0");

    // Reference values from A&S Table 9.8
    check_rel_error(
        eval("besseli(1, 1)"),
        0.565_159_103_992_485_1,
        REL_TOL_BESSEL,
        "I₁(1)",
    );
}

// ============================================================================
// LAMBERT W FUNCTION
// Reference: Corless et al. (1996)
// ============================================================================

#[test]
fn precision_lambert_w() {
    // W(0) = 0
    check_rel_error(eval("lambertw(0)"), 0.0, REL_TOL_GAMMA, "W(0) = 0");

    // W(e) = 1
    check_rel_error(
        eval("lambertw(2.718_281_828_459_045)"),
        1.0,
        REL_TOL_GAMMA,
        "W(e) = 1",
    );

    // W(1) ≈ 0.567143...
    check_rel_error(
        eval("lambertw(1)"),
        0.567_143_290_409_783_8,
        REL_TOL_GAMMA,
        "W(1)",
    );
}

// ============================================================================
// DERIVATIVE PRECISION
// ============================================================================

#[test]
fn precision_derivatives_basic() {
    // Numerical differentiation check: compare symbolic derivative with finite difference
    let h = 1e-7;

    // d/dx[sin(x)] = cos(x) at x=1
    let deriv_symbolic = eval_with_var("cos(x)", "x", 1.0);
    let f_plus = eval_with_var("sin(x)", "x", 1.0 + h);
    let f_minus = eval_with_var("sin(x)", "x", 1.0 - h);
    let deriv_numeric = (f_plus - f_minus) / (2.0 * h);
    check_rel_error(deriv_symbolic, deriv_numeric, 1e-6, "d/dx[sin(x)] at x=1");
}

#[test]
fn precision_derivatives_gamma() {
    // d/dx[gamma(x)] = gamma(x) * digamma(x)
    let x = 2.5;
    let gamma_val = eval_with_var("gamma(x)", "x", x);
    let digamma_val = eval_with_var("digamma(x)", "x", x);
    let deriv_expected = gamma_val * digamma_val;

    // Compute derivative symbolically
    let deriv_result = diff("gamma(x)", "x", &[], None).unwrap();
    let deriv_computed = eval_with_var(&deriv_result, "x", x);

    check_rel_error(
        deriv_computed,
        deriv_expected,
        REL_TOL_GAMMA,
        "d/dx[gamma(x)] = gamma(x)*digamma(x)",
    );
}

// ============================================================================
// EDGE CASES AND SPECIAL VALUES
// ============================================================================

#[test]
fn edge_case_zero_handling() {
    // Functions that should return 0 at x=0
    assert_eq!(eval("sin(0)"), 0.0);
    assert_eq!(eval("sinh(0)"), 0.0);
    assert_eq!(eval("tan(0)"), 0.0);
    assert_eq!(eval("tanh(0)"), 0.0);
    assert_eq!(eval("erf(0)"), 0.0);
}

#[test]
fn edge_case_one_handling() {
    // Functions that should return 1 at specific points (with floating point tolerance)
    check_rel_error(eval("cos(0)"), 1.0, 1e-15, "cos(0) = 1");
    check_rel_error(eval("cosh(0)"), 1.0, 1e-15, "cosh(0) = 1");
    check_rel_error(eval("exp(0)"), 1.0, 1e-15, "exp(0) = 1");
    check_rel_error(eval("gamma(1)"), 1.0, 1e-15, "gamma(1) = 1");
}

#[test]
fn edge_case_infinity_handling() {
    // Functions that return infinity at poles
    let cot0 = eval("cot(0)");
    assert!(cot0.is_infinite(), "cot(0) should be infinite");

    let csc0 = eval("csc(0)");
    assert!(csc0.is_infinite(), "csc(0) should be infinite");
}

#[test]
fn edge_case_nan_handling() {
    // Domain errors that should return NaN
    let sqrt_neg = eval("sqrt(-1)");
    assert!(sqrt_neg.is_nan(), "sqrt(-1) should be NaN");

    let ln_neg = eval("ln(-1)");
    assert!(ln_neg.is_nan(), "ln(-1) should be NaN");
}
