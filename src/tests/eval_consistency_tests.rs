//! Evaluation Consistency Tests
//!
//! Verify that symbolic derivatives match numerical derivatives (finite differences).
//! This catches errors where symbolic form is mathematically correct but evaluation fails.

use crate::parser::parse as parser_parse;
use crate::{ExprKind, Simplify, diff};
use std::collections::{HashMap, HashSet};

/// Compute numerical derivative using central differences
fn numerical_derivative(expr_str: &str, var: &str, point: f64, h: f64) -> f64 {
    let expr = parser_parse(expr_str, &HashSet::new(), &HashSet::new(), None).unwrap();

    let vars_plus: HashMap<&str, f64> = [(var, point + h)].into_iter().collect();
    let f_plus = match &expr.evaluate(&vars_plus, &HashMap::new()).kind {
        ExprKind::Number(n) => *n,
        _ => panic!("Expected number"),
    };

    let vars_minus: HashMap<&str, f64> = [(var, point - h)].into_iter().collect();
    let f_minus = match &expr.evaluate(&vars_minus, &HashMap::new()).kind {
        ExprKind::Number(n) => *n,
        _ => panic!("Expected number"),
    };

    (f_plus - f_minus) / (2.0 * h)
}

/// Evaluate symbolic derivative at a point
fn symbolic_derivative_at(expr_str: &str, var: &str, point: f64) -> f64 {
    let derivative = diff(expr_str, var, &[], None).unwrap();
    let expr = parser_parse(&derivative, &HashSet::new(), &HashSet::new(), None).unwrap();
    let simplified = Simplify::new().simplify(&expr).unwrap();

    let vars: HashMap<&str, f64> = [(var, point)].into_iter().collect();
    match &simplified.evaluate(&vars, &HashMap::new()).kind {
        ExprKind::Number(n) => *n,
        _ => panic!("Expected number"),
    }
}

/// Test that symbolic and numerical derivatives agree
fn verify_consistency(expr_str: &str, var: &str, test_points: &[f64], tolerance: f64) {
    let h = 1e-7;

    for &point in test_points {
        let numerical = numerical_derivative(expr_str, var, point, h);
        let symbolic = symbolic_derivative_at(expr_str, var, point);

        let diff_val = (numerical - symbolic).abs();
        let relative_tol = tolerance * numerical.abs().max(symbolic.abs()).max(1.0);

        assert!(
            diff_val < relative_tol,
            "Consistency check failed for d/d{}[{}] at {}={}: \
             numerical={}, symbolic={}, diff={}",
            var,
            expr_str,
            var,
            point,
            numerical,
            symbolic,
            diff_val
        );
    }
}

// ============================================================================
// BASIC FUNCTION CONSISTENCY TESTS
// ============================================================================

#[test]
fn test_consistency_polynomials() {
    verify_consistency("x^2", "x", &[0.5, 1.0, 2.0, 5.0], 1e-5);
    verify_consistency("x^3 - 2*x + 1", "x", &[0.5, 1.0, 2.0], 1e-5);
    verify_consistency("x^5 + x^3 - x", "x", &[0.5, 1.0, 1.5], 1e-5);
}

#[test]
fn test_consistency_trig() {
    let points = &[0.5, 1.0, 1.5, 2.0];
    verify_consistency("sin(x)", "x", points, 1e-5);
    verify_consistency("cos(x)", "x", points, 1e-5);
    verify_consistency("tan(x)", "x", &[0.5, 1.0], 1e-5); // Avoid singularities
}

#[test]
fn test_consistency_exp_log() {
    verify_consistency("exp(x)", "x", &[0.0, 0.5, 1.0, 2.0], 1e-5);
    verify_consistency("ln(x)", "x", &[0.5, 1.0, 2.0, 5.0], 1e-5);
}

#[test]
fn test_consistency_sqrt() {
    verify_consistency("sqrt(x)", "x", &[0.5, 1.0, 2.0, 4.0], 1e-5);
}

// ============================================================================
// COMPOSITE FUNCTION CONSISTENCY TESTS
// ============================================================================

#[test]
fn test_consistency_chain_rule() {
    verify_consistency("sin(x^2)", "x", &[0.5, 1.0, 1.5], 1e-5);
    verify_consistency("exp(sin(x))", "x", &[0.5, 1.0, 1.5], 1e-5);
    verify_consistency("ln(cos(x))", "x", &[0.1, 0.2, 0.5], 1e-5); // cos(x) > 0
}

#[test]
fn test_consistency_product_rule() {
    verify_consistency("x*sin(x)", "x", &[0.5, 1.0, 2.0], 1e-5);
    verify_consistency("x^2*exp(x)", "x", &[0.5, 1.0, 1.5], 1e-5);
    verify_consistency("sin(x)*cos(x)", "x", &[0.5, 1.0, 1.5], 1e-5);
}

#[test]
fn test_consistency_quotient_rule() {
    verify_consistency("sin(x)/x", "x", &[0.5, 1.0, 2.0], 1e-5);
    verify_consistency("x/exp(x)", "x", &[0.5, 1.0, 2.0], 1e-5);
    verify_consistency("exp(x)/sin(x)", "x", &[0.5, 1.0, 2.0], 1e-5);
}

// ============================================================================
// COMPLEX EXPRESSIONS
// ============================================================================

#[test]
fn test_consistency_complex_1() {
    // Physics-like expression
    verify_consistency("x^2*exp(-x^2)", "x", &[0.5, 1.0, 1.5], 1e-5);
}

#[test]
fn test_consistency_complex_2() {
    // Nested composition
    verify_consistency("sin(exp(x))^2", "x", &[0.1, 0.5, 1.0], 1e-5);
}

#[test]
fn test_consistency_complex_3() {
    // Rational with trig
    verify_consistency("sin(x)/(1 + cos(x))", "x", &[0.5, 1.0, 1.5], 1e-5);
}

// ============================================================================
// SECOND DERIVATIVES
// ============================================================================

#[test]
fn test_consistency_second_derivative() {
    // Numerical second derivative
    fn numerical_second_derivative(expr_str: &str, var: &str, point: f64, h: f64) -> f64 {
        let expr = parser_parse(expr_str, &HashSet::new(), &HashSet::new(), None).unwrap();

        let vars_plus: HashMap<&str, f64> = [(var, point + h)].into_iter().collect();
        let f_plus = match &expr.evaluate(&vars_plus, &HashMap::new()).kind {
            ExprKind::Number(n) => *n,
            _ => panic!("Expected number"),
        };

        let vars_center: HashMap<&str, f64> = [(var, point)].into_iter().collect();
        let f_center = match &expr.evaluate(&vars_center, &HashMap::new()).kind {
            ExprKind::Number(n) => *n,
            _ => panic!("Expected number"),
        };

        let vars_minus: HashMap<&str, f64> = [(var, point - h)].into_iter().collect();
        let f_minus = match &expr.evaluate(&vars_minus, &HashMap::new()).kind {
            ExprKind::Number(n) => *n,
            _ => panic!("Expected number"),
        };

        (f_plus - 2.0 * f_center + f_minus) / (h * h)
    }

    fn symbolic_second_at(expr_str: &str, var: &str, point: f64) -> f64 {
        let d1 = diff(expr_str, var, &[], None).unwrap();
        let d2 = diff(&d1, var, &[], None).unwrap();
        let expr = parser_parse(&d2, &HashSet::new(), &HashSet::new(), None).unwrap();
        let simplified = Simplify::new().simplify(&expr).unwrap();

        let vars: HashMap<&str, f64> = [(var, point)].into_iter().collect();
        match &simplified.evaluate(&vars, &HashMap::new()).kind {
            ExprKind::Number(n) => *n,
            _ => panic!("Expected number"),
        }
    }

    let h = 1e-5;
    let test_cases = [("x^3", 2.0), ("sin(x)", 1.0), ("exp(x)", 1.0)];

    for (expr, point) in test_cases {
        let numerical = numerical_second_derivative(expr, "x", point, h);
        let symbolic = symbolic_second_at(expr, "x", point);

        let diff_val = (numerical - symbolic).abs();
        let tol = 1e-4 * numerical.abs().max(symbolic.abs()).max(1.0);

        assert!(
            diff_val < tol,
            "Second derivative consistency failed for {}: numerical={}, symbolic={}",
            expr,
            numerical,
            symbolic
        );
    }
}

// ============================================================================
// COMPILED EVALUATOR VS TREE-WALKING CONSISTENCY
// ============================================================================

#[test]
fn test_compiled_vs_tree_walking() {
    use crate::core::evaluator::CompiledEvaluator;

    let expressions = [
        "x^2 + 2*x + 1",
        "sin(x)*cos(x)",
        "exp(-x^2/2)",
        "ln(1 + x^2)",
        "sqrt(x^2 + 1)",
    ];

    let test_points = [0.5, 1.0, 1.5, 2.0, 3.0];

    for expr_str in expressions {
        let expr = parser_parse(expr_str, &HashSet::new(), &HashSet::new(), None).unwrap();
        let compiled = CompiledEvaluator::compile(&expr, &["x"], None).unwrap();

        for &x in &test_points {
            let vars: HashMap<&str, f64> = [("x", x)].into_iter().collect();

            let tree_result = match &expr.evaluate(&vars, &HashMap::new()).kind {
                ExprKind::Number(n) => *n,
                _ => panic!("Expected number"),
            };
            let compiled_result = compiled.evaluate(&[x]);

            let diff_val = (tree_result - compiled_result).abs();
            assert!(
                diff_val < 1e-10,
                "Compiled vs tree-walking mismatch for {} at x={}: tree={}, compiled={}",
                expr_str,
                x,
                tree_result,
                compiled_result
            );
        }
    }
}
