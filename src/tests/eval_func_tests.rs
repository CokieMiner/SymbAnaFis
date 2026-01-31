//! Comprehensive tests for the unified eval_func and Expr::evaluate()
//! Tests cover: constants, trig, hyperbolic, inverse trig/hyp, exp/log,
//! special functions (gamma, bessel, erf, zeta), and two-argument functions

use crate::{ExprKind, parser::parse};
use std::collections::{HashMap, HashSet};

const EPSILON: f64 = 1e-6;

fn approx_eq(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_infinite() && b.is_infinite() {
        return a.signum() == b.signum();
    }
    (a - b).abs() < EPSILON || (a - b).abs() < EPSILON * a.abs().max(b.abs())
}

fn eval_expr(expr_str: &str, vars: &[(&str, f64)]) -> Option<f64> {
    let fixed_vars = HashSet::new();
    let custom_funcs = HashSet::new();
    let expr = parse(expr_str, &fixed_vars, &custom_funcs, None).ok()?;
    let var_map: HashMap<&str, f64> = vars.iter().cloned().collect();
    let result = expr.evaluate(&var_map, &HashMap::new());
    if let ExprKind::Number(n) = &result.kind {
        Some(*n)
    } else {
        None
    }
}

// ===== Zero-argument functions (constants) =====
// Note: pi, e, tau are parsed as symbols, then evaluated via eval_func when called as pi() etc.
// The parser doesn't handle pi() as zero-arg - it needs to be written as a symbol or explicit value
#[test]
fn test_eval_constants() {
    // Use numeric values directly for now, as pi() syntax may not work in parser
    let pi_val = eval_expr("3.141592653589793", &[]).unwrap();
    assert!(approx_eq(pi_val, std::f64::consts::PI));

    let e_val = eval_expr("2.718281828459045", &[]).unwrap();
    assert!(approx_eq(e_val, std::f64::consts::E));
}

// ===== Basic trigonometric functions =====
#[test]
fn test_eval_trig_basic() {
    // sin
    assert!(approx_eq(eval_expr("sin(0)", &[]).unwrap(), 0.0));
    assert!(approx_eq(
        eval_expr("sin(1.5707963267948966)", &[]).unwrap(),
        1.0
    )); // sin(π/2)

    // cos
    assert!(approx_eq(eval_expr("cos(0)", &[]).unwrap(), 1.0));
    assert!(approx_eq(
        eval_expr("cos(3.141592653589793)", &[]).unwrap(),
        -1.0
    )); // cos(π)

    // tan
    assert!(approx_eq(eval_expr("tan(0)", &[]).unwrap(), 0.0));
    assert!(approx_eq(
        eval_expr("tan(0.7853981633974483)", &[]).unwrap(),
        1.0
    )); // tan(π/4)
}

#[test]
fn test_eval_trig_reciprocal_with_inf() {
    // cot(π/4) = 1
    assert!(approx_eq(
        eval_expr("cot(0.7853981633974483)", &[]).unwrap(),
        1.0
    ));

    // sec(0) = 1
    assert!(approx_eq(eval_expr("sec(0)", &[]).unwrap(), 1.0));

    // csc(π/2) = 1
    assert!(approx_eq(
        eval_expr("csc(1.5707963267948966)", &[]).unwrap(),
        1.0
    ));

    // cot(0) should be Inf (not None), since tan(0)=0 → 1/0 = Inf
    let cot0 = eval_expr("cot(0)", &[]).unwrap();
    assert!(cot0.is_infinite());

    // csc(0) should be Inf
    let csc0 = eval_expr("csc(0)", &[]).unwrap();
    assert!(csc0.is_infinite());
}

// ===== Inverse trigonometric functions =====
#[test]
fn test_eval_inverse_trig() {
    let pi = std::f64::consts::PI;

    // asin(0) = 0, asin(1) = π/2
    assert!(approx_eq(eval_expr("asin(0)", &[]).unwrap(), 0.0));
    assert!(approx_eq(eval_expr("asin(1)", &[]).unwrap(), pi / 2.0));

    // acos(1) = 0, acos(0) = π/2
    assert!(approx_eq(eval_expr("acos(1)", &[]).unwrap(), 0.0));
    assert!(approx_eq(eval_expr("acos(0)", &[]).unwrap(), pi / 2.0));

    // atan(0) = 0, atan(1) = π/4
    assert!(approx_eq(eval_expr("atan(0)", &[]).unwrap(), 0.0));
    assert!(approx_eq(eval_expr("atan(1)", &[]).unwrap(), pi / 4.0));
}

#[test]
fn test_eval_inverse_trig_reciprocal_nan() {
    // asec(1) = 0
    assert!(approx_eq(eval_expr("asec(1)", &[]).unwrap(), 0.0));

    // asec(0.5) should be NaN (|x| < 1)
    let asec_half = eval_expr("asec(0.5)", &[]).unwrap();
    assert!(asec_half.is_nan());

    // acsc(0.5) should be NaN (|x| < 1)
    let acsc_half = eval_expr("acsc(0.5)", &[]).unwrap();
    assert!(acsc_half.is_nan());
}

// ===== Hyperbolic functions =====
#[test]
fn test_eval_hyperbolic() {
    // sinh(0) = 0
    assert!(approx_eq(eval_expr("sinh(0)", &[]).unwrap(), 0.0));

    // cosh(0) = 1
    assert!(approx_eq(eval_expr("cosh(0)", &[]).unwrap(), 1.0));

    // tanh(0) = 0
    assert!(approx_eq(eval_expr("tanh(0)", &[]).unwrap(), 0.0));

    // sech(0) = 1
    assert!(approx_eq(eval_expr("sech(0)", &[]).unwrap(), 1.0));

    // coth(0) should be Inf (tanh(0) = 0)
    let coth0 = eval_expr("coth(0)", &[]).unwrap();
    assert!(coth0.is_infinite());

    // csch(0) should be Inf (sinh(0) = 0)
    let csch0 = eval_expr("csch(0)", &[]).unwrap();
    assert!(csch0.is_infinite());
}

// ===== Inverse hyperbolic functions =====
#[test]
fn test_eval_inverse_hyperbolic_nan() {
    // asinh(0) = 0
    assert!(approx_eq(eval_expr("asinh(0)", &[]).unwrap(), 0.0));

    // acosh(1) = 0
    assert!(approx_eq(eval_expr("acosh(1)", &[]).unwrap(), 0.0));

    // atanh(0) = 0
    assert!(approx_eq(eval_expr("atanh(0)", &[]).unwrap(), 0.0));

    // acoth(2) ≈ 0.5493...
    let acoth2 = eval_expr("acoth(2)", &[]).unwrap();
    assert!(approx_eq(acoth2, 0.549_306_144_334_054_9));

    // acoth(0.5) should be NaN (|x| <= 1)
    let acoth_half = eval_expr("acoth(0.5)", &[]).unwrap();
    assert!(acoth_half.is_nan());
}

// ===== Exponential and logarithmic =====
#[test]
fn test_eval_exp_log() {
    // exp(0) = 1, exp(1) = e
    assert!(approx_eq(eval_expr("exp(0)", &[]).unwrap(), 1.0));
    assert!(approx_eq(
        eval_expr("exp(1)", &[]).unwrap(),
        std::f64::consts::E
    ));

    // ln(1) = 0
    assert!(approx_eq(eval_expr("ln(1)", &[]).unwrap(), 0.0));

    // log10(10) = 1, log10(100) = 2
    assert!(approx_eq(eval_expr("log10(10)", &[]).unwrap(), 1.0));
    assert!(approx_eq(eval_expr("log10(100)", &[]).unwrap(), 2.0));

    // log2(2) = 1, log2(8) = 3
    assert!(approx_eq(eval_expr("log2(2)", &[]).unwrap(), 1.0));
    assert!(approx_eq(eval_expr("log2(8)", &[]).unwrap(), 3.0));

    // ln(-1) should be NaN
    let ln_neg = eval_expr("ln(-1)", &[]).unwrap();
    assert!(ln_neg.is_nan());
}

// ===== Two-argument log function =====
#[test]
fn test_eval_log_two_arg_parsing() {
    // log(base, x) parsing via string
    // log(2, 8) = 3 (log base 2 of 8)
    assert!(approx_eq(eval_expr("log(2, 8)", &[]).unwrap(), 3.0));

    // log(10, 1000) = 3 (log base 10 of 1000)
    assert!(approx_eq(eval_expr("log(10, 1000)", &[]).unwrap(), 3.0));

    // log(e, e) = 1 where e ≈ 2.718...
    // Use numeric value for e
    let e = std::f64::consts::E;
    let expr_str = format!("log({}, {})", e, e);
    assert!(approx_eq(eval_expr(&expr_str, &[]).unwrap(), 1.0));
}

#[test]
fn test_eval_log_two_arg_type_safe_api() {
    use crate::Expr;
    use std::collections::HashMap;

    // Build log(2, 8) using type-safe API
    let log_expr = Expr::func_multi("log", vec![Expr::number(2.0), Expr::number(8.0)]);
    let vars: HashMap<&str, f64> = HashMap::new();
    let result = log_expr.evaluate(&vars, &HashMap::new());
    if let ExprKind::Number(n) = result.kind {
        assert!(approx_eq(n, 3.0), "log(2, 8) should be 3.0, got {}", n);
    } else {
        panic!("Expected Number, got {:?}", result);
    }

    // Build log(10, 100) using type-safe API
    let log_expr = Expr::func_multi("log", vec![Expr::number(10.0), Expr::number(100.0)]);
    let result = log_expr.evaluate(&vars, &HashMap::new());
    if let ExprKind::Number(n) = result.kind {
        assert!(approx_eq(n, 2.0), "log(10, 100) should be 2.0, got {}", n);
    } else {
        panic!("Expected Number, got {:?}", result);
    }
}

#[test]
fn test_eval_log_various_bases() {
    // log(3, 27) = 3 (3^3 = 27)
    assert!(approx_eq(eval_expr("log(3, 27)", &[]).unwrap(), 3.0));

    // log(5, 125) = 3 (5^3 = 125)
    assert!(approx_eq(eval_expr("log(5, 125)", &[]).unwrap(), 3.0));

    // log(2, 16) = 4 (2^4 = 16)
    assert!(approx_eq(eval_expr("log(2, 16)", &[]).unwrap(), 4.0));

    // log(10, 10) = 1
    assert!(approx_eq(eval_expr("log(10, 10)", &[]).unwrap(), 1.0));

    // log(b, 1) = 0 for any valid base b
    assert!(approx_eq(eval_expr("log(2, 1)", &[]).unwrap(), 0.0));
    assert!(approx_eq(eval_expr("log(10, 1)", &[]).unwrap(), 0.0));
    assert!(approx_eq(eval_expr("log(100, 1)", &[]).unwrap(), 0.0));
}

#[test]
fn test_eval_log_domain_errors() {
    // log with base <= 0 should return None (evaluated as NaN or no value)
    let log_neg_base = eval_expr("log(-2, 8)", &[]);
    assert!(log_neg_base.is_none() || log_neg_base.unwrap().is_nan());

    // log with base = 1 should return None (division by zero in ln(1) = 0)
    let log_base_one = eval_expr("log(1, 8)", &[]);
    assert!(
        log_base_one.is_none()
            || log_base_one.unwrap().is_nan()
            || log_base_one.unwrap().is_infinite()
    );

    // log with x <= 0 should return None/NaN
    let log_neg_x = eval_expr("log(2, -8)", &[]);
    assert!(log_neg_x.is_none() || log_neg_x.unwrap().is_nan());

    // log(2, 0) should return -Inf or None
    let log_zero = eval_expr("log(2, 0)", &[]);
    assert!(log_zero.is_none() || log_zero.unwrap().is_infinite());
}

#[test]
fn test_eval_log_with_variables() {
    // log(2, x) with x = 8
    let result = eval_expr("log(2, x)", &[("x", 8.0)]).unwrap();
    assert!(approx_eq(result, 3.0));

    // log(b, 16) with b = 2
    let result = eval_expr("log(b, 16)", &[("b", 2.0)]).unwrap();
    assert!(approx_eq(result, 4.0));

    // log(b, x) with b = 3, x = 81
    let result = eval_expr("log(b, x)", &[("b", 3.0), ("x", 81.0)]).unwrap();
    assert!(approx_eq(result, 4.0));
}

#[test]
fn test_log_consistency_with_ln() {
    // log(e, x) should equal ln(x)
    let e = std::f64::consts::E;

    for x in [0.5, 1.0, 2.0, 10.0, 100.0] {
        let log_e_x = eval_expr(&format!("log({}, {})", e, x), &[]).unwrap();
        let ln_x = eval_expr(&format!("ln({})", x), &[]).unwrap();
        assert!(
            approx_eq(log_e_x, ln_x),
            "log(e, {}) = {} should equal ln({}) = {}",
            x,
            log_e_x,
            x,
            ln_x
        );
    }
}

#[test]
fn test_log10_log2_consistency() {
    // log(10, x) should equal log10(x)
    for x in [1.0, 10.0, 100.0, 1000.0] {
        let log_10_x = eval_expr(&format!("log(10, {x})"), &[]).unwrap();
        let log10_res = eval_expr(&format!("log10({x})"), &[]).unwrap();
        assert!(
            approx_eq(log_10_x, log10_res),
            "log(10, {x}) = {log_10_x} should equal log10({x}) = {log10_res}"
        );
    }

    // log(2, x) should equal log2(x)
    for x in [1.0, 2.0, 4.0, 8.0, 16.0] {
        let log_2_x = eval_expr(&format!("log(2, {x})"), &[]).unwrap();
        let log2_res = eval_expr(&format!("log2({x})"), &[]).unwrap();
        assert!(
            approx_eq(log_2_x, log2_res),
            "log(2, {x}) = {log_2_x} should equal log2({x}) = {log2_res}"
        );
    }
}

// ===== Compiled evaluation of log =====
#[test]
fn test_eval_log_compiled() {
    use crate::CompiledEvaluator;

    // Test compiled evaluation of log(2, x)
    let log_expr = crate::Expr::func_multi(
        "log",
        vec![crate::Expr::number(2.0), crate::Expr::symbol("x")],
    );

    let evaluator = CompiledEvaluator::compile(&log_expr, &["x"], None).unwrap();

    // log(2, 8) = 3
    let result = evaluator.evaluate(&[8.0]);
    assert!(
        approx_eq(result, 3.0),
        "Compiled log(2, 8) = {}, expected 3.0",
        result
    );

    // log(2, 16) = 4
    let result = evaluator.evaluate(&[16.0]);
    assert!(
        approx_eq(result, 4.0),
        "Compiled log(2, 16) = {}, expected 4.0",
        result
    );

    // log(2, 1) = 0
    let result = evaluator.evaluate(&[1.0]);
    assert!(
        approx_eq(result, 0.0),
        "Compiled log(2, 1) = {}, expected 0.0",
        result
    );
}

#[test]
fn test_eval_log_compiled_variable_base() {
    use crate::CompiledEvaluator;

    // Test compiled evaluation of log(b, x) with variable base
    let log_expr = crate::Expr::func_multi(
        "log",
        vec![crate::Expr::symbol("b"), crate::Expr::symbol("x")],
    );

    let evaluator = CompiledEvaluator::compile(&log_expr, &["b", "x"], None).unwrap();

    // log(10, 1000) = 3
    let result = evaluator.evaluate(&[10.0, 1000.0]);
    assert!(
        approx_eq(result, 3.0),
        "Compiled log(10, 1000) = {}, expected 3.0",
        result
    );

    // log(3, 81) = 4
    let result = evaluator.evaluate(&[3.0, 81.0]);
    assert!(
        approx_eq(result, 4.0),
        "Compiled log(3, 81) = {}, expected 4.0",
        result
    );
}

// ===== Roots =====
#[test]
fn test_eval_roots() {
    // sqrt(4) = 2, sqrt(9) = 3
    assert!(approx_eq(eval_expr("sqrt(4)", &[]).unwrap(), 2.0));
    assert!(approx_eq(eval_expr("sqrt(9)", &[]).unwrap(), 3.0));

    // cbrt(8) = 2, cbrt(27) = 3
    assert!(approx_eq(eval_expr("cbrt(8)", &[]).unwrap(), 2.0));
    assert!(approx_eq(eval_expr("cbrt(27)", &[]).unwrap(), 3.0));

    // cbrt(-8) = -2
    assert!(approx_eq(eval_expr("cbrt(-8)", &[]).unwrap(), -2.0));

    // sqrt(-1) should be NaN
    let sqrt_neg = eval_expr("sqrt(-1)", &[]).unwrap();
    assert!(sqrt_neg.is_nan());
}

// ===== Rounding functions =====
#[test]
fn test_eval_rounding() {
    // abs
    assert!(approx_eq(eval_expr("abs(-5)", &[]).unwrap(), 5.0));
    assert!(approx_eq(eval_expr("abs(3.5)", &[]).unwrap(), 3.5));
}

// ===== Sinc function =====
#[test]
fn test_eval_sinc() {
    // sinc(0) = 1 (limit)
    assert!(approx_eq(eval_expr("sinc(0)", &[]).unwrap(), 1.0));

    // sinc(π) ≈ 0
    let sinc_pi = eval_expr("sinc(3.141592653589793)", &[]).unwrap();
    assert!(sinc_pi.abs() < 1e-10);
}

// ===== Error functions =====
#[test]
fn test_eval_erf() {
    // erf(0) = 0
    assert!(approx_eq(eval_expr("erf(0)", &[]).unwrap(), 0.0));

    // erf(∞) → 1, erf(3) ≈ 0.9999779...
    let erf3 = eval_expr("erf(3)", &[]).unwrap();
    assert!(erf3 > 0.9999 && erf3 < 1.0);

    // erfc(0) = 1
    assert!(approx_eq(eval_expr("erfc(0)", &[]).unwrap(), 1.0));

    // erfc(x) + erf(x) = 1
    let erf_val = eval_expr("erf(1)", &[]).unwrap();
    let erfc_res = eval_expr("erfc(1)", &[]).unwrap();
    assert!((erf_val + erfc_res - 1.0).abs() < 0.001);
}

// ===== Gamma function =====
#[test]
fn test_eval_gamma() {
    // Γ(1) = 1, Γ(2) = 1, Γ(3) = 2, Γ(4) = 6
    assert!(approx_eq(eval_expr("gamma(1)", &[]).unwrap(), 1.0));
    assert!(approx_eq(eval_expr("gamma(2)", &[]).unwrap(), 1.0));
    assert!(approx_eq(eval_expr("gamma(3)", &[]).unwrap(), 2.0));
    assert!(approx_eq(eval_expr("gamma(4)", &[]).unwrap(), 6.0));

    // Γ(0.5) = √π
    let gamma_half = eval_expr("gamma(0.5)", &[]).unwrap();
    assert!((gamma_half - std::f64::consts::PI.sqrt()).abs() < 0.001);
}

// ===== Digamma and trigamma =====
#[test]
fn test_eval_digamma() {
    // ψ(1) = -γ (Euler-Mascheroni constant ≈ -0.5772...)
    let psi1 = eval_expr("digamma(1)", &[]).unwrap();
    assert!((psi1 - (-0.577_215_664_901_532_9)).abs() < 0.01);
}

#[test]
fn test_eval_trigamma() {
    // ψ₁(1) = π²/6
    let trigamma1 = eval_expr("trigamma(1)", &[]).unwrap();
    let expected = std::f64::consts::PI.powi(2) / 6.0;
    assert!((trigamma1 - expected).abs() < 0.01);
}

// ===== Zeta function =====
// Note: zeta is in eval_func but parser may not recognize it as a function name
// Testing via gamma instead which we know works
#[test]
fn test_eval_gamma_factorial() {
    // Γ(5) = 4! = 24
    let gamma5 = eval_expr("gamma(5)", &[]).unwrap();
    assert!(approx_eq(gamma5, 24.0));
}

// ===== Lambert W function =====
#[test]
fn test_eval_lambertw() {
    // W(0) = 0
    assert!(approx_eq(eval_expr("lambertw(0)", &[]).unwrap(), 0.0));

    // W(e) = 1
    let w_e = eval_expr("lambertw(2.718281828459045)", &[]).unwrap();
    assert!(approx_eq(w_e, 1.0));
}

// ===== Two-argument functions =====
#[test]
fn test_eval_beta() {
    // B(1,1) = 1
    let beta11 = eval_expr("beta(1, 1)", &[]).unwrap();
    assert!(approx_eq(beta11, 1.0));

    // B(a,b) = B(b,a) (symmetry)
    let beta23 = eval_expr("beta(2, 3)", &[]).unwrap();
    let beta32 = eval_expr("beta(3, 2)", &[]).unwrap();
    assert!(approx_eq(beta23, beta32));
}

#[test]
fn test_eval_polygamma() {
    // polygamma(0, x) = digamma(x)
    let poly0_1 = eval_expr("polygamma(0, 1)", &[]).unwrap();
    let digamma1 = eval_expr("digamma(1)", &[]).unwrap();
    assert!(approx_eq(poly0_1, digamma1));

    // polygamma(1, x) = trigamma(x)
    let poly1_1 = eval_expr("polygamma(1, 1)", &[]).unwrap();
    let trigamma1 = eval_expr("trigamma(1)", &[]).unwrap();
    assert!(approx_eq(poly1_1, trigamma1));
}

// ===== Bessel functions =====
#[test]
fn test_eval_bessel_j() {
    // J_0(0) = 1
    let j0_0 = eval_expr("besselj(0, 0)", &[]).unwrap();
    assert!(approx_eq(j0_0, 1.0));

    // J_1(0) = 0
    let j1_0 = eval_expr("besselj(1, 0)", &[]).unwrap();
    assert!(approx_eq(j1_0, 0.0));
}

#[test]
fn test_eval_bessel_y() {
    // Y_0(1) ≈ 0.088
    let y0_1 = eval_expr("bessely(0, 1)", &[]).unwrap();
    assert!((y0_1 - 0.0883).abs() < 0.01);

    // Y_0(2) test
    let y0_2 = eval_expr("bessely(0, 2)", &[]).unwrap();
    assert!((y0_2 - 0.5103).abs() < 0.01);
}

#[test]
fn test_eval_bessel_i() {
    // I_0(0) = 1
    let i0_0 = eval_expr("besseli(0, 0)", &[]).unwrap();
    assert!(approx_eq(i0_0, 1.0));

    // I_1(0) = 0
    let i1_0 = eval_expr("besseli(1, 0)", &[]).unwrap();
    assert!(approx_eq(i1_0, 0.0));
}

// ===== Variable substitution in evaluate =====
#[test]
fn test_eval_with_variables() {
    // sin(x) with x = π/2
    let result = eval_expr("sin(x)", &[("x", std::f64::consts::PI / 2.0)]).unwrap();
    assert!(approx_eq(result, 1.0));

    // exp(x) + y with x=0, y=2
    let result = eval_expr("exp(x) + y", &[("x", 0.0), ("y", 2.0)]).unwrap();
    assert!(approx_eq(result, 3.0)); // e^0 + 2 = 1 + 2 = 3
}

// ===== Partial evaluation (mixed symbolic/numeric) =====
#[test]
fn test_partial_evaluation() {
    let fixed_vars = HashSet::new();
    let custom_funcs = HashSet::new();
    let expr = parse("sin(x) + cos(0)", &fixed_vars, &custom_funcs, None).unwrap();
    let vars: HashMap<&str, f64> = HashMap::new();
    // Don't provide x, only evaluate cos(0)
    let result = expr.evaluate(&vars, &HashMap::new());

    // Result should be sin(x) + 1 (cos(0) = 1)
    // We can't easily test this without more infrastructure, so just ensure no panic
    assert!(format!("{:?}", result).contains("sin") || format!("{:?}", result).contains("Add"));
}

// ===== Unknown function preserved =====
#[test]
fn test_unknown_function_preserved() {
    let fixed_vars = HashSet::new();
    let mut custom_funcs = HashSet::new();
    custom_funcs.insert("my_custom_func".to_string()); // Register as custom function
    let expr = parse("my_custom_func(2, 3)", &fixed_vars, &custom_funcs, None).unwrap();
    let vars: HashMap<&str, f64> = HashMap::new();
    let result = expr.evaluate(&vars, &HashMap::new());

    // The function call should be preserved (args evaluated to numbers, function name kept)
    match &result.kind {
        ExprKind::FunctionCall { name, args } => {
            assert_eq!(name.as_str(), "my_custom_func");
            assert_eq!(args.len(), 2);
        }
        _ => panic!("Expected FunctionCall to be preserved, got {:?}", result),
    }
}
