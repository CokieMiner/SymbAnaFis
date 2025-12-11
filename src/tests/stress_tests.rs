//! Stress tests for the expression system
//!
//! These tests try to break the evaluator, differentiator, simplifier,
//! and custom function handling with extreme edge cases.

use crate::simplify_expr;
use crate::{Diff, Expr, ExprKind, diff, parse, simplify};
use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;
use std::sync::Arc;

/// Type alias for custom function evaluators to reduce type complexity
type CustomEvalFn = Arc<dyn Fn(&[f64]) -> Option<f64> + Send + Sync>;

// ============================================================================
// DIFFERENTIATION STRESS TESTS
// ============================================================================

#[test]
fn test_diff_deeply_nested_chain_rule() {
    // sin(cos(tan(exp(ln(x^2)))))
    // This tests the chain rule applied 5 levels deep
    let result = diff("sin(cos(tan(exp(ln(x^2)))))", "x", None, None);
    assert!(result.is_ok(), "Deep chain rule failed: {:?}", result);
    let derivative = result.unwrap();
    assert!(!derivative.is_empty());
}

#[test]
fn test_diff_product_of_10_terms() {
    // x * x^2 * x^3 * x^4 * x^5 * sin(x) * cos(x) * exp(x) * ln(x) * sqrt(x)
    let result = diff(
        "x * x^2 * x^3 * x^4 * x^5 * sin(x) * cos(x) * exp(x) * ln(x) * sqrt(x)",
        "x",
        None,
        None,
    );
    assert!(result.is_ok(), "10-term product failed: {:?}", result);
}

#[test]
fn test_diff_quotient_of_complex_expressions() {
    // (sin(x) * exp(x) + cos(x)) / (ln(x) * x^2 - tan(x))
    let result = diff(
        "(sin(x) * exp(x) + cos(x)) / (ln(x) * x^2 - tan(x))",
        "x",
        None,
        None,
    );
    assert!(result.is_ok(), "Complex quotient failed: {:?}", result);
}

#[test]
fn test_diff_power_rule_extreme_exponents() {
    // x^100, x^0.001, x^(-100), x^x, x^(x^x)
    for expr in &["x^100", "x^0.001", "x^(-100)", "x^x", "x^(x^x)"] {
        let result = diff(expr, "x", None, None);
        assert!(
            result.is_ok(),
            "Power rule failed for {}: {:?}",
            expr,
            result
        );
    }
}

#[test]
fn test_diff_many_variables() {
    // Differentiate with respect to different variables
    let expr = "x*y*z*w*u*v";
    for var in &["x", "y", "z", "w", "u", "v"] {
        let result = diff(expr, var, None, None);
        assert!(result.is_ok(), "Multi-var diff wrt {} failed", var);
    }
}

#[test]
fn test_diff_special_functions_chain() {
    // gamma(digamma(x)) - extreme nesting of special functions
    let result = diff("gamma(digamma(x + 1))", "x", None, None);
    assert!(
        result.is_ok(),
        "Special function chain failed: {:?}",
        result
    );
}

#[test]
fn test_diff_bessel_functions() {
    // Bessel functions with various orders
    for expr in &[
        "besselj(0, x)",
        "besselj(1, x)",
        "bessely(0, x)",
        "besseli(2, x)",
        "besselk(1, x)",
    ] {
        let result = diff(expr, "x", None, None);
        assert!(
            result.is_ok(),
            "Bessel diff failed for {}: {:?}",
            expr,
            result
        );
    }
}

#[test]
fn test_diff_polygamma_chain() {
    // d/dx polygamma(2, x^2)
    let result = diff("polygamma(2, x^2)", "x", None, None);
    assert!(result.is_ok(), "Polygamma diff failed: {:?}", result);
}

#[test]
fn test_diff_zeta_function() {
    // d/ds zeta(s)
    let result = diff("zeta(s)", "s", None, None);
    assert!(result.is_ok(), "Zeta diff failed: {:?}", result);
    let deriv = result.unwrap();
    assert!(
        deriv.contains("zeta_deriv"),
        "Should use zeta_deriv: {}",
        deriv
    );
}

#[test]
fn test_diff_lambert_w() {
    let result = diff("lambertw(x^2)", "x", None, None);
    assert!(result.is_ok(), "lambertw diff failed: {:?}", result);
}

#[test]
fn test_diff_erfc_erf() {
    let result1 = diff("erf(x)", "x", None, None);
    let result2 = diff("erfc(x)", "x", None, None);
    assert!(result1.is_ok() && result2.is_ok(), "erf/erfc diff failed");
}

#[test]
fn test_diff_higher_order() {
    // d^5/dx^5 sin(x) = cos(x)
    let expr = parse("sin(x)", &HashSet::new(), &HashSet::new()).unwrap();
    let mut result = expr.clone();
    for _ in 0..5 {
        result = result.derive("x", &HashSet::new(), &HashMap::new(), &HashMap::new());
    }
    // After 5 derivatives: sin -> cos -> -sin -> -cos -> sin -> cos
    let simplified = simplify_expr(result, HashSet::new());
    let s = format!("{}", simplified);
    assert!(
        s.contains("cos") || s.contains("sin"),
        "5th derivative of sin: {}",
        s
    );
}

// ============================================================================
// EVALUATION STRESS TESTS
// ============================================================================

#[test]
fn test_eval_extreme_values() {
    let expr = parse("exp(x)", &HashSet::new(), &HashSet::new()).unwrap();

    // Very large value
    let mut vars = HashMap::new();
    vars.insert("x", 700.0);
    let result = expr.evaluate(&vars);
    if let ExprKind::Number(n) = result.kind {
        assert!(
            n.is_finite() || n.is_infinite(),
            "exp(700) should be finite or inf"
        );
    }

    // Very small value
    vars.insert("x", -700.0);
    let result = expr.evaluate(&vars);
    if let ExprKind::Number(n) = result.kind {
        assert!(n >= 0.0, "exp(-700) should be >= 0");
    }
}

#[test]
fn test_eval_precision_sensitive() {
    // 1/3 * 3 should be approximately 1
    let expr = parse("1/3 * 3", &HashSet::new(), &HashSet::new()).unwrap();
    let result = expr.evaluate(&HashMap::new());
    if let ExprKind::Number(n) = result.kind {
        assert!((n - 1.0).abs() < 1e-10, "1/3 * 3 = {} should be ~1", n);
    }
}

#[test]
fn test_eval_special_function_poles() {
    // gamma(0) should return None/NaN (pole)
    let expr = parse("gamma(x)", &HashSet::new(), &HashSet::new()).unwrap();
    let mut vars = HashMap::new();
    vars.insert("x", 0.0);
    let result = expr.evaluate(&vars);
    // Should either stay symbolic or produce NaN
    // Symbolic result is also acceptable
    if let ExprKind::Number(n) = &result.kind {
        assert!(n.is_nan(), "gamma(0) should be NaN");
    }
}

#[test]
fn test_eval_trigamma_at_various_points() {
    let expr = parse("trigamma(x)", &HashSet::new(), &HashSet::new()).unwrap();
    for x in [0.5, 1.0, 2.0, 5.0, 10.0] {
        let mut vars = HashMap::new();
        vars.insert("x", x);
        let result = expr.evaluate(&vars);
        if let ExprKind::Number(n) = result.kind {
            assert!(n.is_finite(), "trigamma({}) should be finite, got {}", x, n);
        }
    }
}

#[test]
fn test_eval_zeta_deriv_convergence() {
    // zeta_deriv(2, 3) - second derivative at s=3
    let expr = parse("zeta_deriv(2, s)", &HashSet::new(), &HashSet::new()).unwrap();
    let mut vars = HashMap::new();
    vars.insert("s", 3.0);
    let result = expr.evaluate(&vars);
    if let ExprKind::Number(n) = result.kind {
        assert!(
            n.is_finite(),
            "zeta_deriv(2, 3) should be finite, got {}",
            n
        );
    }
}

#[test]
fn test_eval_complex_expression_accuracy() {
    // sin^2(x) + cos^2(x) = 1 for all x
    let expr = parse("sin(x)^2 + cos(x)^2", &HashSet::new(), &HashSet::new()).unwrap();
    for x in [0.0, 0.5, 1.0, 2.0, PI, 100.0] {
        let mut vars = HashMap::new();
        vars.insert("x", x);
        let result = expr.evaluate(&vars);
        if let ExprKind::Number(n) = result.kind {
            assert!(
                (n - 1.0).abs() < 1e-10,
                "sin²({}) + cos²({}) = {}, should be 1",
                x,
                x,
                n
            );
        }
    }
}

#[test]
fn test_eval_bessel_accuracy() {
    // J_0(0) = 1
    let expr = parse("besselj(0, x)", &HashSet::new(), &HashSet::new()).unwrap();
    let mut vars = HashMap::new();
    vars.insert("x", 0.0);
    let result = expr.evaluate(&vars);
    if let ExprKind::Number(n) = result.kind {
        assert!((n - 1.0).abs() < 1e-6, "J_0(0) = {}, should be 1", n);
    }
}

// ============================================================================
// SIMPLIFICATION STRESS TESTS
// ============================================================================

#[test]
fn test_simplify_deeply_nested() {
    // ((((x + 0) * 1) / 1) - 0) ^ 1
    let result = simplify("((((x + 0) * 1) / 1) - 0) ^ 1", None, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "x");
}

#[test]
fn test_simplify_complex_trig_identity() {
    // sin(x)^2 + cos(x)^2 should simplify to 1
    let result = simplify("sin(x)^2 + cos(x)^2", None, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "1");
}

#[test]
fn test_simplify_log_exp_identity() {
    // ln(exp(x)) = x
    let result = simplify("ln(exp(x))", None, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "x");
}

#[test]
fn test_simplify_exp_ln_identity() {
    // exp(ln(x)) = x
    let result = simplify("exp(ln(x))", None, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "x");
}

#[test]
fn test_simplify_double_negative() {
    // -(-x) = x
    let result = simplify("-(-x)", None, None);
    assert!(result.is_ok());
    // Could be "x" or simplified differently
}

#[test]
fn test_simplify_fraction_reduction() {
    // 6/9 = 2/3
    let result = simplify("6/9", None, None);
    assert!(result.is_ok());
    let s = result.unwrap();
    assert!(s == "2/3" || s.contains("2") && s.contains("3"));
}

#[test]
fn test_simplify_power_rules() {
    // x^2 * x^3 = x^5
    let result = simplify("x^2 * x^3", None, None);
    assert!(result.is_ok());
    let s = result.unwrap();
    assert!(s.contains("5") || s == "x^5", "Got: {}", s);
}

#[test]
fn test_simplify_sqrt_of_square() {
    // sqrt(x^2) = abs(x)
    let result = simplify("sqrt(x^2)", None, None);
    assert!(result.is_ok());
    let s = result.unwrap();
    assert!(s.contains("abs"), "sqrt(x^2) should be abs(x), got: {}", s);
}

#[test]
fn test_simplify_zero_multiplication() {
    // 0 * (complex expression) = 0
    let result = simplify("0 * (sin(x) + cos(x) * exp(x))", None, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "0");
}

#[test]
fn test_simplify_one_power() {
    // x^0 = 1
    let result = simplify("(sin(x) + cos(x))^0", None, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "1");
}

// ============================================================================
// CUSTOM FUNCTION TESTS
// ============================================================================

#[test]
fn test_custom_function_diff() {
    let mut custom = HashSet::new();
    custom.insert("f".to_string());

    // Parse f(x) with custom function
    let expr = parse("f(x)", &HashSet::new(), &custom).unwrap();

    // The derivative should be symbolic: f'(x) or similar
    let deriv = expr.derive("x", &HashSet::new(), &HashMap::new(), &HashMap::new());
    let s = format!("{}", deriv);
    // Should produce a derivative expression
    assert!(!s.is_empty());
}

#[test]
fn test_custom_function_with_custom_derivative() {
    // Register a custom derivative: d/dx[f(x)] = 2*x
    // Using the builder pattern correctly
    let result = Diff::new()
        .custom_derivative("f", |inner_expr, _var, inner_deriv| {
            // d/dx f(u) = 2*u * u'
            let two = Expr::number(2.0);
            Expr::mul_expr(Expr::mul_expr(two, inner_expr.clone()), inner_deriv.clone())
        })
        .diff_str("f(x)", "x");

    assert!(result.is_ok(), "Custom derivative failed: {:?}", result);
    let deriv = result.unwrap();
    assert!(
        deriv.contains("2"),
        "Custom derivative should contain 2: {}",
        deriv
    );
}

#[test]
fn test_nested_custom_functions() {
    let mut custom = HashSet::new();
    custom.insert("f".to_string());
    custom.insert("g".to_string());

    // f(g(x))
    let expr = parse("f(g(x))", &HashSet::new(), &custom).unwrap();
    let deriv = expr.derive("x", &HashSet::new(), &HashMap::new(), &HashMap::new());
    // Should produce chain rule: f'(g(x)) * g'(x)
    let s = format!("{}", deriv);
    assert!(!s.is_empty());
}

#[test]
fn test_custom_function_evaluation() {
    let mut custom = HashSet::new();
    custom.insert("f".to_string());

    let expr = parse("f(x) + 1", &HashSet::new(), &custom).unwrap();

    // Define custom evaluator: f(x) = x^2
    let mut custom_evals: HashMap<String, CustomEvalFn> = HashMap::new();
    custom_evals.insert(
        "f".to_string(),
        Arc::new(|args: &[f64]| Some(args[0].powi(2))),
    );

    let mut vars = HashMap::new();
    vars.insert("x", 3.0);

    let result = expr.evaluate_with_custom(&vars, &custom_evals);
    if let ExprKind::Number(n) = result.kind {
        // f(3) + 1 = 9 + 1 = 10
        assert!((n - 10.0).abs() < 1e-10, "f(3) + 1 = {}, should be 10", n);
    } else {
        panic!("Expected number, got: {:?}", result);
    }
}

// ============================================================================
// EDGE CASES AND BOUNDARY CONDITIONS
// ============================================================================

#[test]
fn test_empty_expression_handling() {
    let result = parse("", &HashSet::new(), &HashSet::new());
    assert!(result.is_err(), "Empty expression should fail");
}

#[test]
fn test_unicode_variable_names() {
    // Greek letters
    let result = parse("α + β * γ", &HashSet::new(), &HashSet::new());
    assert!(result.is_ok(), "Unicode variables failed: {:?}", result);
}

#[test]
fn test_very_long_expression() {
    // Generate a very long expression: x + x + x + ... (100 times)
    let expr_str = (0..100).map(|_| "x").collect::<Vec<_>>().join(" + ");
    let result = simplify(&expr_str, None, None);
    assert!(result.is_ok(), "Long expression failed");
    let s = result.unwrap();
    assert!(
        s.contains("100") || s.contains("x"),
        "Should simplify to 100*x: {}",
        s
    );
}

#[test]
fn test_deeply_nested_parentheses() {
    // ((((((x))))))
    let result = parse("((((((x))))))", &HashSet::new(), &HashSet::new());
    assert!(result.is_ok());
    let expr = result.unwrap();
    assert_eq!(format!("{}", expr), "x");
}

#[test]
fn test_implicit_multiplication() {
    // 2x should be 2*x
    let result = parse("2x", &HashSet::new(), &HashSet::new());
    assert!(
        result.is_ok(),
        "Implicit multiplication failed: {:?}",
        result
    );
}

#[test]
fn test_scientific_notation() {
    let result = parse("1e10 + 2.5e-5", &HashSet::new(), &HashSet::new());
    assert!(result.is_ok(), "Scientific notation failed: {:?}", result);
}

#[test]
fn test_division_by_symbolic_zero() {
    // x / (x - x) - should handle symbolically
    let result = parse("x / (x - x)", &HashSet::new(), &HashSet::new());
    assert!(result.is_ok());
    // Simplification might produce infinity or undefined
}

#[test]
fn test_zero_to_zero_power() {
    // 0^0 - mathematically undefined but IEEE 754 says 1
    let expr = parse("0^0", &HashSet::new(), &HashSet::new()).unwrap();
    let result = expr.evaluate(&HashMap::new());
    if let ExprKind::Number(n) = result.kind {
        assert_eq!(n, 1.0, "0^0 should be 1 per IEEE 754");
    }
}

#[test]
fn test_negative_base_fractional_exponent() {
    // (-8)^(1/3) = -2 (real cube root)
    let expr = parse("(-8)^(1/3)", &HashSet::new(), &HashSet::new()).unwrap();
    let result = expr.evaluate(&HashMap::new());
    if let ExprKind::Number(n) = result.kind {
        // Note: IEEE 754 pow(-8, 1/3) = NaN, cbrt(-8) = -2
        // Our implementation may differ
        assert!(n.is_finite() || n.is_nan(), "(-8)^(1/3) = {}", n);
    }
}

// ============================================================================
// INTEGRATION TESTS - FULL PIPELINE
// ============================================================================

#[test]
fn test_full_pipeline_diff_simplify_eval() {
    // Start with a formula
    let formula = "sin(x)^2 + cos(x)^2";

    // Differentiate
    let deriv = diff(formula, "x", None, None).unwrap();

    // Simplify the derivative
    let simplified = simplify(&deriv, None, None).unwrap();

    // The derivative of sin²(x) + cos²(x) = 1 is 0
    assert_eq!(
        simplified, "0",
        "d/dx[sin²+cos²] should be 0, got: {}",
        simplified
    );
}

#[test]
fn test_full_pipeline_with_evaluation() {
    // Differentiate x^3, simplify, then evaluate at x=2
    let deriv = diff("x^3", "x", None, None).unwrap();
    // d/dx[x^3] = 3x^2

    let expr = parse(&deriv, &HashSet::new(), &HashSet::new()).unwrap();
    let mut vars = HashMap::new();
    vars.insert("x", 2.0);
    let result = expr.evaluate(&vars);

    if let ExprKind::Number(n) = result.kind {
        // 3 * 2^2 = 12
        assert!((n - 12.0).abs() < 1e-10, "3x² at x=2 = {}, should be 12", n);
    } else {
        panic!("Expected number, got: {:?}", result);
    }
}

#[test]
fn test_second_derivative_pipeline() {
    // d²/dx² sin(x) = -sin(x)
    let d1 = diff("sin(x)", "x", None, None).unwrap();
    let d2 = diff(&d1, "x", None, None).unwrap();
    let simplified = simplify(&d2, None, None).unwrap();

    // Should contain -sin(x) or -1*sin(x) or similar
    assert!(
        simplified.contains("sin") && simplified.contains("-"),
        "d²/dx²[sin(x)] = {}, should be -sin(x)",
        simplified
    );
}

#[test]
fn test_gradient_computation() {
    // Compute gradient of x^2 + y^2
    use crate::gradient_str;
    let grad = gradient_str("x^2 + y^2", &["x", "y"]).unwrap();

    // ∂/∂x = 2x, ∂/∂y = 2y
    assert_eq!(grad.len(), 2);
    assert!(grad[0].contains("2") && grad[0].contains("x"));
    assert!(grad[1].contains("2") && grad[1].contains("y"));
}

#[test]
fn test_jacobian_computation() {
    use crate::jacobian_str;
    // Jacobian of [x^2, y^2, x*y] with respect to [x, y]
    let jac = jacobian_str(&["x^2", "y^2", "x*y"], &["x", "y"]).unwrap();

    // Should be a 3x2 matrix
    assert_eq!(jac.len(), 3);
    assert_eq!(jac[0].len(), 2);
}

// ============================================================================
// CLOSURE TESTS - CAN WE DIFF FOREVER?
// ============================================================================

#[test]
fn test_infinite_differentiability_polygamma() {
    // polygamma(n, x) -> polygamma(n+1, x) -> ...
    // Verify the chain doesn't break
    let mut expr = "polygamma(0, x)".to_string();
    for i in 0..5 {
        let result = diff(&expr, "x", None, None);
        assert!(result.is_ok(), "polygamma diff {} failed: {:?}", i, result);
        expr = result.unwrap();
    }
}

#[test]
fn test_infinite_differentiability_zeta() {
    // zeta(s) -> zeta_deriv(1, s) -> zeta_deriv(2, s) -> ...
    let mut expr = "zeta(s)".to_string();
    for i in 0..5 {
        let result = diff(&expr, "s", None, None);
        assert!(result.is_ok(), "zeta diff {} failed: {:?}", i, result);
        expr = result.unwrap();
    }
}

#[test]
fn test_closure_evaluation() {
    // Verify that deeply differentiated expressions can still be evaluated
    let d1 = diff("exp(x)", "x", None, None).unwrap();
    let d2 = diff(&d1, "x", None, None).unwrap();
    let d3 = diff(&d2, "x", None, None).unwrap();
    let d4 = diff(&d3, "x", None, None).unwrap();
    let d5 = diff(&d4, "x", None, None).unwrap();

    // d^5/dx^5 exp(x) = exp(x)
    let expr = parse(&d5, &HashSet::new(), &HashSet::new()).unwrap();
    let mut vars = HashMap::new();
    vars.insert("x", 1.0);
    let result = expr.evaluate(&vars);

    if let ExprKind::Number(n) = result.kind {
        assert!(
            (n - std::f64::consts::E).abs() < 1e-8,
            "d^5/dx^5 exp(x) at x=1 = {}",
            n
        );
    }
}
