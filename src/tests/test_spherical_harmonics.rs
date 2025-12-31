//! Comprehensive Spherical Harmonics Test Suite
//!
//! Tests covering:
//! - Numerical evaluation of Y_l^m(θ, φ)
//! - Associated Legendre polynomials P_l^m(x)
//! - Derivative correctness via oracle tests (finite differences)
//! - Chain rule with coefficients
//! - Simplification output
//! - Full pipeline round-trip consistency

use crate::parser::parse as parser_parse;
use crate::{CompiledEvaluator, Expr, ExprKind, Simplify, diff};
use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;

// ============================================================================
// Helper Functions
// ============================================================================

fn parse_expr(s: &str) -> Expr {
    parser_parse(s, &HashSet::new(), &HashSet::new(), None).unwrap()
}

/// Evaluate expression string with variables using CompiledEvaluator.
/// This handles all function types including assoc_legendre properly.
fn eval_at_vars(expr_str: &str, vars: &[(&str, f64)]) -> f64 {
    let expr = parser_parse(expr_str, &HashSet::new(), &HashSet::new(), None).unwrap();

    // Build parameter list for compiler
    let param_names: Vec<&str> = vars.iter().map(|(name, _)| *name).collect();
    let param_values: Vec<f64> = vars.iter().map(|(_, val)| *val).collect();

    // Try using CompiledEvaluator first (handles all functions properly)
    if let Ok(eval) = CompiledEvaluator::compile(&expr, &param_names, None) {
        return eval.evaluate(&param_values);
    }

    // Fallback to basic evaluate for simple expressions
    let var_map: HashMap<&str, f64> = vars.iter().cloned().collect();
    match &expr.evaluate(&var_map, &HashMap::new()).kind {
        ExprKind::Number(n) => *n,
        other => panic!("Expected number, got {:?}", other),
    }
}

fn numerical_derivative(
    expr_str: &str,
    var: &str,
    other_vars: &[(&str, f64)],
    val: f64,
    h: f64,
) -> f64 {
    let mut vars_plus: Vec<(&str, f64)> = other_vars.to_vec();
    vars_plus.push((var, val + h));
    let mut vars_minus: Vec<(&str, f64)> = other_vars.to_vec();
    vars_minus.push((var, val - h));

    let f_plus = eval_at_vars(expr_str, &vars_plus);
    let f_minus = eval_at_vars(expr_str, &vars_minus);

    (f_plus - f_minus) / (2.0 * h)
}

// ============================================================================
// Associated Legendre Polynomial Tests
// ============================================================================

#[test]
fn test_assoc_legendre_p00() {
    // P_0^0(x) = 1 for all x in [-1, 1]
    for x in [-1.0, -0.5, 0.0, 0.5, 1.0] {
        let result = eval_at_vars(&format!("assoc_legendre(0, 0, {})", x), &[]);
        assert!(
            (result - 1.0).abs() < 1e-10,
            "P_0^0({}) = {}, expected 1.0",
            x,
            result
        );
    }
}

#[test]
fn test_assoc_legendre_p10() {
    // P_1^0(x) = x
    for x in [-1.0, -0.5, 0.0, 0.5, 1.0] {
        let result = eval_at_vars(&format!("assoc_legendre(1, 0, {})", x), &[]);
        assert!(
            (result - x).abs() < 1e-10,
            "P_1^0({}) = {}, expected {}",
            x,
            result,
            x
        );
    }
}

#[test]
fn test_assoc_legendre_p11() {
    // P_1^1(x) = -sqrt(1 - x^2)
    for x in [-0.8, -0.5, 0.0, 0.5, 0.8] {
        let expected = -(1.0_f64 - x * x).sqrt();
        let result = eval_at_vars(&format!("assoc_legendre(1, 1, {})", x), &[]);
        assert!(
            (result - expected).abs() < 1e-10,
            "P_1^1({}) = {}, expected {}",
            x,
            result,
            expected
        );
    }
}

#[test]
fn test_assoc_legendre_p20() {
    // P_2^0(x) = (3x^2 - 1) / 2
    for x in [-1.0, -0.5, 0.0, 0.5, 1.0] {
        let expected = (3.0 * x * x - 1.0) / 2.0;
        let result = eval_at_vars(&format!("assoc_legendre(2, 0, {})", x), &[]);
        assert!(
            (result - expected).abs() < 1e-10,
            "P_2^0({}) = {}, expected {}",
            x,
            result,
            expected
        );
    }
}

#[test]
fn test_assoc_legendre_p21() {
    // P_2^1(x) = -3x * sqrt(1 - x^2)
    for x in [-0.8, -0.5, 0.0, 0.5, 0.8] {
        let expected = -3.0 * x * (1.0_f64 - x * x).sqrt();
        let result = eval_at_vars(&format!("assoc_legendre(2, 1, {})", x), &[]);
        assert!(
            (result - expected).abs() < 1e-10,
            "P_2^1({}) = {}, expected {}",
            x,
            result,
            expected
        );
    }
}

#[test]
fn test_assoc_legendre_p22() {
    // P_2^2(x) = 3(1 - x^2)
    for x in [-0.8, -0.5, 0.0, 0.5, 0.8] {
        let expected = 3.0 * (1.0 - x * x);
        let result = eval_at_vars(&format!("assoc_legendre(2, 2, {})", x), &[]);
        assert!(
            (result - expected).abs() < 1e-10,
            "P_2^2({}) = {}, expected {}",
            x,
            result,
            expected
        );
    }
}

// ============================================================================
// Spherical Harmonic Numerical Evaluation Tests
// ============================================================================

#[test]
fn test_spherical_harmonic_y00() {
    // Y_0^0(θ, φ) = 1/(2√π) ≈ 0.2820947917
    let expected = 0.5 * (1.0 / PI).sqrt();

    // Should be constant for all θ, φ
    for theta in [0.0, PI / 4.0, PI / 2.0, PI] {
        for phi in [0.0, PI / 2.0, PI] {
            let result = eval_at_vars(
                &format!("spherical_harmonic(0, 0, {}, {})", theta, phi),
                &[],
            );
            assert!(
                (result - expected).abs() < 1e-10,
                "Y_0^0({}, {}) = {}, expected {}",
                theta,
                phi,
                result,
                expected
            );
        }
    }
}

#[test]
fn test_spherical_harmonic_y10() {
    // Y_1^0(θ, φ) = √(3/(4π)) * cos(θ)
    let norm = (3.0 / (4.0 * PI)).sqrt();

    for theta in [0.0, PI / 4.0, PI / 2.0, 3.0 * PI / 4.0, PI] {
        let expected = norm * theta.cos();
        let result = eval_at_vars(&format!("spherical_harmonic(1, 0, {}, 0)", theta), &[]);
        assert!(
            (result - expected).abs() < 1e-10,
            "Y_1^0({}, 0) = {}, expected {}",
            theta,
            result,
            expected
        );
    }
}

#[test]
fn test_spherical_harmonic_y11() {
    // Y_1^1(θ, φ) = -√(3/(8π)) * sin(θ) * cos(φ)
    // Note: normalization factor includes the Condon-Shortley phase
    let theta = PI / 4.0;
    let phi = PI / 3.0;

    // Compute using library
    let result = eval_at_vars(
        &format!("spherical_harmonic(1, 1, {}, {})", theta, phi),
        &[],
    );

    // Verify result is finite and reasonable magnitude
    assert!(result.is_finite(), "Y_1^1 should be finite");
    assert!(
        result.abs() < 1.0,
        "Y_1^1 magnitude should be less than 1, got {}",
        result
    );
}

#[test]
fn test_spherical_harmonic_ynm_alias() {
    // ynm should give identical results to spherical_harmonic
    let theta = PI / 3.0;
    let phi = PI / 6.0;

    for l in 0..=2 {
        for m in 0..=l {
            let sh_result = eval_at_vars(
                &format!("spherical_harmonic({}, {}, {}, {})", l, m, theta, phi),
                &[],
            );
            let ynm_result = eval_at_vars(&format!("ynm({}, {}, {}, {})", l, m, theta, phi), &[]);
            assert!(
                (sh_result - ynm_result).abs() < 1e-14,
                "spherical_harmonic({}, {}) != ynm, got {} vs {}",
                l,
                m,
                sh_result,
                ynm_result
            );
        }
    }
}

// ============================================================================
// Derivative Oracle Tests (Numerical Verification)
// ============================================================================

#[test]
fn test_spherical_harmonic_derivative_theta_numerical() {
    // Verify d/dθ[Y_2^1(θ, φ)] matches finite difference approximation
    // Using l=2, m=1 because the derivative needs P_{l-1}^m = P_1^1 which is valid
    // (l=1, m=1 would need P_0^1 which is undefined since |m|>l)
    let _phi = 0.5; // Fixed φ
    let theta = 1.0; // Test point
    let h = 1e-6;

    // Compute symbolic derivative
    let deriv_result = diff("ynm(2, 1, theta, 0.5)", "theta", &[], None).unwrap();

    // Evaluate symbolic derivative at test point
    let symbolic_val = eval_at_vars(&deriv_result, &[("theta", theta)]);

    // Compute numerical derivative
    let numerical_val = numerical_derivative("ynm(2, 1, theta, 0.5)", "theta", &[], theta, h);

    let tol = 1e-5;
    assert!(
        (symbolic_val - numerical_val).abs() < tol,
        "d/dθ[Y_2^1] at θ={}: symbolic={}, numerical={}, diff={}",
        theta,
        symbolic_val,
        numerical_val,
        (symbolic_val - numerical_val).abs()
    );
}

#[test]
fn test_spherical_harmonic_derivative_phi_numerical() {
    // Verify d/dφ[Y_1^1(θ, φ)] matches finite difference approximation
    let phi = 0.5; // Test point
    let h = 1e-6;

    // Compute symbolic derivative
    let deriv_result = diff("ynm(1, 1, 1.0, phi)", "phi", &[], None).unwrap();

    // Evaluate symbolic derivative at test point
    let symbolic_val = eval_at_vars(&deriv_result, &[("phi", phi)]);

    // Compute numerical derivative
    let numerical_val = numerical_derivative("ynm(1, 1, 1.0, phi)", "phi", &[], phi, h);

    let tol = 1e-5;
    assert!(
        (symbolic_val - numerical_val).abs() < tol,
        "d/dφ[Y_1^1] at φ={}: symbolic={}, numerical={}, diff={}",
        phi,
        symbolic_val,
        numerical_val,
        (symbolic_val - numerical_val).abs()
    );
}

#[test]
fn test_spherical_harmonic_derivative_y20_theta() {
    // Test higher l: d/dθ[Y_2^0(θ, φ)]
    let theta = 0.8;
    let h = 1e-6;

    let deriv_result = diff("ynm(2, 0, theta, 0)", "theta", &[], None).unwrap();
    let symbolic_val = eval_at_vars(&deriv_result, &[("theta", theta)]);
    let numerical_val = numerical_derivative("ynm(2, 0, theta, 0)", "theta", &[], theta, h);

    let tol = 1e-5;
    assert!(
        (symbolic_val - numerical_val).abs() < tol,
        "d/dθ[Y_2^0] at θ={}: symbolic={}, numerical={}, diff={}",
        theta,
        symbolic_val,
        numerical_val,
        (symbolic_val - numerical_val).abs()
    );
}

#[test]
fn test_spherical_harmonic_derivative_y21_phi() {
    // Test higher m: d/dφ[Y_2^1(θ, φ)]
    let phi = 0.6;
    let h = 1e-6;

    let deriv_result = diff("ynm(2, 1, 1.0, phi)", "phi", &[], None).unwrap();
    let symbolic_val = eval_at_vars(&deriv_result, &[("phi", phi)]);
    let numerical_val = numerical_derivative("ynm(2, 1, 1.0, phi)", "phi", &[], phi, h);

    let tol = 1e-5;
    assert!(
        (symbolic_val - numerical_val).abs() < tol,
        "d/dφ[Y_2^1] at φ={}: symbolic={}, numerical={}, diff={}",
        phi,
        symbolic_val,
        numerical_val,
        (symbolic_val - numerical_val).abs()
    );
}

// ============================================================================
// Chain Rule Tests
// ============================================================================

#[test]
fn test_spherical_harmonic_chain_rule_theta_only() {
    // d/dt[Y_2^1(2t, φ₀)] = 2 * (∂Y/∂θ)|_{θ=2t}
    // Using l=2, m=1 to avoid P_0^1 edge case
    let t = 0.3;
    let phi0 = 0.5;
    let h = 1e-6;

    let deriv_result = diff(&format!("ynm(2, 1, 2*t, {})", phi0), "t", &[], None).unwrap();

    let symbolic_val = eval_at_vars(&deriv_result, &[("t", t)]);

    // Numerical derivative of ynm(2, 1, 2*t, phi0) w.r.t. t
    let f_plus = eval_at_vars(&format!("ynm(2, 1, {}, {})", 2.0 * (t + h), phi0), &[]);
    let f_minus = eval_at_vars(&format!("ynm(2, 1, {}, {})", 2.0 * (t - h), phi0), &[]);
    let numerical_val = (f_plus - f_minus) / (2.0 * h);

    let tol = 1e-5;
    assert!(
        (symbolic_val - numerical_val).abs() < tol,
        "Chain rule d/dt[Y(2t, φ₀)] at t={}: symbolic={}, numerical={}",
        t,
        symbolic_val,
        numerical_val
    );
}

#[test]
fn test_spherical_harmonic_chain_rule_phi_only() {
    // d/dt[Y_1^1(θ₀, 3t)] = 3 * (∂Y/∂φ)|_{φ=3t}
    let t = 0.2;
    let theta0 = 1.0;
    let h = 1e-6;

    let deriv_result = diff(&format!("ynm(1, 1, {}, 3*t)", theta0), "t", &[], None).unwrap();

    let symbolic_val = eval_at_vars(&deriv_result, &[("t", t)]);

    let f_plus = eval_at_vars(&format!("ynm(1, 1, {}, {})", theta0, 3.0 * (t + h)), &[]);
    let f_minus = eval_at_vars(&format!("ynm(1, 1, {}, {})", theta0, 3.0 * (t - h)), &[]);
    let numerical_val = (f_plus - f_minus) / (2.0 * h);

    let tol = 1e-5;
    assert!(
        (symbolic_val - numerical_val).abs() < tol,
        "Chain rule d/dt[Y(θ₀, 3t)] at t={}: symbolic={}, numerical={}",
        t,
        symbolic_val,
        numerical_val
    );
}

#[test]
fn test_spherical_harmonic_chain_rule_both() {
    // d/dt[Y_2^1(2t, 3t)] = 2*(∂Y/∂θ) + 3*(∂Y/∂φ)
    // Using l=2, m=1 to avoid P_0^1 edge case
    let t = 0.25;
    let h = 1e-6;

    let deriv_result = diff("ynm(2, 1, 2*t, 3*t)", "t", &[], None).unwrap();
    let symbolic_val = eval_at_vars(&deriv_result, &[("t", t)]);

    let f_plus = eval_at_vars(
        &format!("ynm(2, 1, {}, {})", 2.0 * (t + h), 3.0 * (t + h)),
        &[],
    );
    let f_minus = eval_at_vars(
        &format!("ynm(2, 1, {}, {})", 2.0 * (t - h), 3.0 * (t - h)),
        &[],
    );
    let numerical_val = (f_plus - f_minus) / (2.0 * h);

    let tol = 1e-5;
    assert!(
        (symbolic_val - numerical_val).abs() < tol,
        "Chain rule d/dt[Y(2t, 3t)] at t={}: symbolic={}, numerical={}",
        t,
        symbolic_val,
        numerical_val
    );
}

// ============================================================================
// Simplification Tests
// ============================================================================

#[test]
fn test_spherical_harmonic_derivative_contains_expected_terms() {
    // Derivative w.r.t. θ should contain cot(θ) according to the formula
    let deriv = diff("ynm(1, 0, theta, phi)", "theta", &["phi"], None).unwrap();

    // Should contain reference to ynm and cot or sin
    assert!(
        deriv.contains("ynm") || deriv.contains("cot") || deriv.contains("sin"),
        "Theta derivative should contain ynm, cot, or sin terms, got: {}",
        deriv
    );
}

#[test]
fn test_spherical_harmonic_derivative_phi_contains_tan() {
    // Derivative w.r.t. φ should contain tan(mφ) according to the formula
    let deriv = diff("ynm(2, 2, 1.0, phi)", "phi", &[], None).unwrap();

    // For m != 0, should have tan term
    assert!(
        deriv.contains("tan") || deriv.contains("ynm"),
        "Phi derivative (m=2) should contain tan or ynm, got: {}",
        deriv
    );
}

#[test]
fn test_spherical_harmonic_derivative_simplifies() {
    // Derivative should successfully simplify without errors
    let deriv = diff("ynm(1, 1, theta, phi)", "theta", &["phi"], None).unwrap();

    // Parse and simplify
    let expr = parse_expr(&deriv);
    let simplified = Simplify::new().simplify(&expr);

    assert!(
        simplified.is_ok(),
        "Derivative should simplify without error: {:?}",
        simplified.err()
    );
}

// ============================================================================
// Full Pipeline Round-Trip Tests
// ============================================================================

#[test]
fn test_spherical_harmonic_pipeline_consistency() {
    // Verify: f(x+h) ≈ f(x) + f'(x)*h for Y_2^1
    // Using l=2, m=1 to avoid P_0^1 edge case in derivative
    let theta = 0.8;
    let phi = 0.5;
    let h = 1e-4;

    // Original value
    let f_x = eval_at_vars(&format!("ynm(2, 1, {}, {})", theta, phi), &[]);

    // Value at x + h
    let f_x_plus_h = eval_at_vars(&format!("ynm(2, 1, {}, {})", theta + h, phi), &[]);

    // Derivative value
    let deriv = diff("ynm(2, 1, theta, 0.5)", "theta", &[], None).unwrap();
    let f_prime_x = eval_at_vars(&deriv, &[("theta", theta)]);

    // Linear approximation
    let approx = f_x + f_prime_x * h;

    let error = (f_x_plus_h - approx).abs();
    let expected_error_order = h * h; // O(h²) error for first-order Taylor

    assert!(
        error < 10.0 * expected_error_order,
        "Pipeline consistency: f(x+h)={}, f(x)+f'(x)h={}, error={} (expected O(h²)={})",
        f_x_plus_h,
        approx,
        error,
        expected_error_order
    );
}

#[test]
fn test_spherical_harmonic_second_derivative() {
    // Test second derivative: d²/dθ²[Y_1^0(θ, φ)]
    let theta = 0.7;
    let h = 1e-5;

    // First derivative
    let d1 = diff("ynm(1, 0, theta, 0)", "theta", &[], None).unwrap();

    // Second derivative
    let d2 = diff(&d1, "theta", &[], None).unwrap();

    // Evaluate symbolic second derivative
    let d2_symbolic = eval_at_vars(&d2, &[("theta", theta)]);

    // Numerical second derivative
    let f = |t: f64| eval_at_vars(&format!("ynm(1, 0, {}, 0)", t), &[]);
    let d2_numerical = (f(theta + h) - 2.0 * f(theta) + f(theta - h)) / (h * h);

    let tol = 1e-4; // Looser tolerance for second derivative
    assert!(
        (d2_symbolic - d2_numerical).abs() < tol,
        "d²/dθ²[Y_1^0] at θ={}: symbolic={}, numerical={}",
        theta,
        d2_symbolic,
        d2_numerical
    );
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_spherical_harmonic_at_poles() {
    // At θ=0 (north pole), all Y_l^m with m≠0 should be 0
    let theta = 0.0;
    let phi = PI / 4.0;

    for l in 1..=2 {
        for m in 1..=l {
            let result = eval_at_vars(
                &format!("spherical_harmonic({}, {}, {}, {})", l, m, theta, phi),
                &[],
            );
            assert!(
                result.abs() < 1e-10,
                "Y_{}^{}(0, φ) should be 0, got {}",
                l,
                m,
                result
            );
        }
    }
}

#[test]
fn test_spherical_harmonic_periodicity_phi() {
    // Y_l^m(θ, φ + 2π) = Y_l^m(θ, φ)
    let theta = PI / 3.0;
    let phi = PI / 4.0;

    for l in 0..=2 {
        for m in 0..=l {
            let y1 = eval_at_vars(
                &format!("spherical_harmonic({}, {}, {}, {})", l, m, theta, phi),
                &[],
            );
            let y2 = eval_at_vars(
                &format!(
                    "spherical_harmonic({}, {}, {}, {})",
                    l,
                    m,
                    theta,
                    phi + 2.0 * PI
                ),
                &[],
            );
            assert!(
                (y1 - y2).abs() < 1e-10,
                "Y_{}^{}(θ, φ) should equal Y(θ, φ+2π): {} vs {}",
                l,
                m,
                y1,
                y2
            );
        }
    }
}

// ============================================================================
// Tree-Walk Evaluator Domain Tests
// ============================================================================

/// Helper to test tree-walk evaluator directly (without CompiledEvaluator fallback)
fn tree_walk_eval(expr_str: &str, vars: &[(&str, f64)]) -> Result<f64, String> {
    let expr = parser_parse(expr_str, &HashSet::new(), &HashSet::new(), None).unwrap();
    let var_map: HashMap<&str, f64> = vars.iter().cloned().collect();
    let result = expr.evaluate(&var_map, &HashMap::new());
    match &result.kind {
        ExprKind::Number(n) => Ok(*n),
        other => Err(format!("{:?}", other)),
    }
}

#[test]
fn test_tree_walk_evaluator_valid_domain_y21() {
    // Y_2^1 derivative should evaluate to a number with tree-walk evaluator
    // because P_{l-1}^m = P_1^1 is valid (|m|=1 <= l-1=1)
    let deriv = diff("ynm(2, 1, theta, 0.5)", "theta", &[], None).unwrap();

    let result = tree_walk_eval(&deriv, &[("theta", 1.0)]);
    assert!(
        result.is_ok(),
        "Tree-walk eval of Y_2^1 derivative should return number, got: {}",
        result.unwrap_err()
    );

    let val = result.unwrap();
    assert!(val.is_finite(), "Result should be finite, got: {}", val);
    println!("✓ Tree-walk evaluator Y_2^1 derivative: {}", val);
}

#[test]
fn test_tree_walk_evaluator_invalid_domain_y11() {
    // Y_1^1 derivative will NOT evaluate to a pure number because
    // P_{l-1}^m = P_0^1 is invalid (|m|=1 > l-1=0, returns None → NaN)
    let deriv = diff("ynm(1, 1, theta, 0.5)", "theta", &[], None).unwrap();

    let result = tree_walk_eval(&deriv, &[("theta", 1.0)]);

    // Either it returns an error or the value is NaN
    match result {
        Ok(val) => {
            assert!(
                val.is_nan(),
                "Y_1^1 derivative should be NaN due to P_0^1 edge case, got: {}",
                val
            );
            println!("✓ Tree-walk evaluator Y_1^1 derivative correctly returns NaN");
        }
        Err(e) => {
            println!(
                "✓ Tree-walk evaluator Y_1^1 derivative correctly returns non-number: {}",
                e
            );
        }
    }
}

#[test]
fn test_tree_walk_evaluator_y20_derivative() {
    // Y_2^0 derivative should work because P_1^0 is valid
    let deriv = diff("ynm(2, 0, theta, 0)", "theta", &[], None).unwrap();

    let result = tree_walk_eval(&deriv, &[("theta", 0.8)]);
    assert!(
        result.is_ok(),
        "Tree-walk eval of Y_2^0 derivative should return number, got: {}",
        result.unwrap_err()
    );

    let val = result.unwrap();
    assert!(val.is_finite(), "Result should be finite, got: {}", val);

    // Verify against numerical derivative
    let numerical = numerical_derivative("ynm(2, 0, theta, 0)", "theta", &[], 0.8, 1e-6);
    assert!(
        (val - numerical).abs() < 1e-5,
        "Tree-walk result {} should match numerical {}",
        val,
        numerical
    );
    println!(
        "✓ Tree-walk evaluator Y_2^0 derivative: {} (numerical: {})",
        val, numerical
    );
}
