//! Oracle Tests: Derivative correctness verified against SymPy
//!
//! Each test case has a pre-computed expected result from SymPy.
//! We verify both symbolic output and numerical evaluation at test points.

use crate::parser::parse as parser_parse;
use crate::{ExprKind, Simplify, diff};
use std::collections::{HashMap, HashSet};

/// Helper to evaluate expression at x=val
fn eval_at(expr_str: &str, val: f64) -> f64 {
    let expr = parser_parse(expr_str, &HashSet::new(), &HashSet::new(), None).unwrap();
    let vars: HashMap<&str, f64> = [("x", val)].into_iter().collect();
    match &expr.evaluate(&vars, &HashMap::new()).kind {
        ExprKind::Number(n) => *n,
        other => panic!("Expected number, got {:?}", other),
    }
}

/// Test derivative and verify both symbolic form and numerical value
fn test_derivative(expr: &str, var: &str, expected_symbolic: &str, test_points: &[(f64, f64)]) {
    let result = diff(expr, var, &[], None).unwrap();

    // Verify numerical accuracy at test points
    for (x_val, expected_val) in test_points {
        let actual = eval_at(&result, *x_val);
        let tolerance = 1e-8 * expected_val.abs().max(1.0);
        assert!(
            (actual - expected_val).abs() < tolerance,
            "d/d{}[{}] at {}={}: got {}, expected {}. Symbolic result: {}",
            var,
            expr,
            var,
            x_val,
            actual,
            expected_val,
            result
        );
    }

    // If expected symbolic form is provided, check it
    if !expected_symbolic.is_empty() {
        // Parse and simplify both to compare
        let result_expr = parser_parse(&result, &HashSet::new(), &HashSet::new(), None).unwrap();
        let expected_expr =
            parser_parse(expected_symbolic, &HashSet::new(), &HashSet::new(), None).unwrap();

        let result_simplified = Simplify::new().simplify(&result_expr).unwrap();
        let expected_simplified = Simplify::new().simplify(&expected_expr).unwrap();

        // Numerical comparison at multiple points as fallback
        for x in [0.5, 1.0, 2.0, 3.0] {
            let r_val = eval_at(&format!("{}", result_simplified), x);
            let e_val = eval_at(&format!("{}", expected_simplified), x);
            let tol = 1e-8 * e_val.abs().max(1.0);
            assert!(
                (r_val - e_val).abs() < tol,
                "Symbolic mismatch for d/d{}[{}]: got {}, expected {}",
                var,
                expr,
                result_simplified,
                expected_simplified
            );
        }
    }
}

// ============================================================================
// POLYNOMIAL DERIVATIVES (SymPy-verified)
// ============================================================================

#[test]
fn test_oracle_constant() {
    // d/dx[5] = 0
    test_derivative("5", "x", "0", &[(1.0, 0.0), (2.0, 0.0)]);
}

#[test]
fn test_oracle_linear() {
    // d/dx[x] = 1
    test_derivative("x", "x", "1", &[(1.0, 1.0), (2.0, 1.0)]);
}

#[test]
fn test_oracle_quadratic() {
    // d/dx[x^2] = 2*x
    test_derivative("x^2", "x", "2*x", &[(1.0, 2.0), (2.0, 4.0), (3.0, 6.0)]);
}

#[test]
fn test_oracle_cubic() {
    // d/dx[x^3] = 3*x^2
    test_derivative("x^3", "x", "3*x^2", &[(1.0, 3.0), (2.0, 12.0), (3.0, 27.0)]);
}

#[test]
fn test_oracle_polynomial() {
    // d/dx[x^4 + 3*x^2 - 2*x + 7] = 4*x^3 + 6*x - 2
    test_derivative(
        "x^4 + 3*x^2 - 2*x + 7",
        "x",
        "4*x^3 + 6*x - 2",
        &[(1.0, 8.0), (2.0, 42.0)],
    );
}

// ============================================================================
// TRIGONOMETRIC DERIVATIVES (SymPy-verified)
// ============================================================================

#[test]
fn test_oracle_sin() {
    // d/dx[sin(x)] = cos(x)
    test_derivative(
        "sin(x)",
        "x",
        "cos(x)",
        &[
            (0.0, 1.0),                        // cos(0) = 1
            (std::f64::consts::PI / 2.0, 0.0), // cos(π/2) = 0
            (std::f64::consts::PI, -1.0),      // cos(π) = -1
        ],
    );
}

#[test]
fn test_oracle_cos() {
    // d/dx[cos(x)] = -sin(x)
    test_derivative(
        "cos(x)",
        "x",
        "-sin(x)",
        &[
            (0.0, 0.0),                         // -sin(0) = 0
            (std::f64::consts::PI / 2.0, -1.0), // -sin(π/2) = -1
        ],
    );
}

#[test]
fn test_oracle_tan() {
    // d/dx[tan(x)] = sec^2(x) = 1/cos^2(x)
    test_derivative(
        "tan(x)",
        "x",
        "",
        &[
            (0.0, 1.0), // sec^2(0) = 1
            (0.5, 1.0 / (0.5_f64.cos().powi(2))),
        ],
    );
}

#[test]
fn test_oracle_sin_squared() {
    // d/dx[sin(x)^2] = 2*sin(x)*cos(x) = sin(2x)
    test_derivative(
        "sin(x)^2",
        "x",
        "",
        &[
            (0.0, 0.0),
            (std::f64::consts::PI / 4.0, 1.0), // sin(π/2) = 1
        ],
    );
}

// ============================================================================
// EXPONENTIAL AND LOGARITHMIC DERIVATIVES (SymPy-verified)
// ============================================================================

#[test]
fn test_oracle_exp() {
    // d/dx[exp(x)] = exp(x)
    test_derivative(
        "exp(x)",
        "x",
        "exp(x)",
        &[
            (0.0, 1.0),
            (1.0, std::f64::consts::E),
            (2.0, std::f64::consts::E.powi(2)),
        ],
    );
}

#[test]
fn test_oracle_ln() {
    // d/dx[ln(x)] = 1/x
    test_derivative(
        "ln(x)",
        "x",
        "1/x",
        &[
            (1.0, 1.0),
            (2.0, 0.5),
            (std::f64::consts::E, 1.0 / std::f64::consts::E),
        ],
    );
}

#[test]
fn test_oracle_exp_composite() {
    // d/dx[exp(x^2)] = 2*x*exp(x^2)
    test_derivative(
        "exp(x^2)",
        "x",
        "2*x*exp(x^2)",
        &[(0.0, 0.0), (1.0, 2.0 * std::f64::consts::E)],
    );
}

#[test]
fn test_oracle_ln_composite() {
    // d/dx[ln(x^2)] = 2/x
    test_derivative("ln(x^2)", "x", "2/x", &[(1.0, 2.0), (2.0, 1.0)]);
}

// ============================================================================
// CHAIN RULE DERIVATIVES (SymPy-verified)
// ============================================================================

#[test]
fn test_oracle_sin_x_squared() {
    // d/dx[sin(x^2)] = 2*x*cos(x^2)
    test_derivative(
        "sin(x^2)",
        "x",
        "2*x*cos(x^2)",
        &[
            (0.0, 0.0),
            (1.0, 2.0 * 1.0_f64.cos()), // 2*cos(1) ≈ 1.0806
        ],
    );
}

#[test]
fn test_oracle_cos_exp() {
    // d/dx[cos(exp(x))] = -sin(exp(x))*exp(x)
    // At x=0: -sin(exp(0))*exp(0) = -sin(1)*1 = -sin(1)
    let x = 0.0_f64;
    let expected = -x.exp().sin() * x.exp();
    test_derivative("cos(exp(x))", "x", "-sin(exp(x))*exp(x)", &[(x, expected)]);
}

#[test]
fn test_oracle_exp_sin() {
    // d/dx[exp(sin(x))] = exp(sin(x))*cos(x)
    test_derivative(
        "exp(sin(x))",
        "x",
        "exp(sin(x))*cos(x)",
        &[
            (0.0, 1.0), // exp(sin(0))*cos(0) = exp(0)*1 = 1
        ],
    );
}

// ============================================================================
// PRODUCT RULE DERIVATIVES (SymPy-verified)
// ============================================================================

#[test]
fn test_oracle_x_sin_x() {
    // d/dx[x*sin(x)] = sin(x) + x*cos(x)
    test_derivative(
        "x*sin(x)",
        "x",
        "sin(x) + x*cos(x)",
        &[
            (0.0, 0.0),
            (std::f64::consts::PI / 2.0, 1.0), // sin(π/2) + π/2*cos(π/2) = 1
        ],
    );
}

#[test]
fn test_oracle_x_exp() {
    // d/dx[x*exp(x)] = exp(x) + x*exp(x) = (1+x)*exp(x)
    test_derivative(
        "x*exp(x)",
        "x",
        "(1+x)*exp(x)",
        &[(0.0, 1.0), (1.0, 2.0 * std::f64::consts::E)],
    );
}

#[test]
fn test_oracle_x2_ln() {
    // d/dx[x^2*ln(x)] = 2*x*ln(x) + x
    test_derivative(
        "x^2*ln(x)",
        "x",
        "2*x*ln(x) + x",
        &[
            (1.0, 1.0), // 2*1*0 + 1 = 1
            (
                std::f64::consts::E,
                2.0 * std::f64::consts::E + std::f64::consts::E,
            ),
        ],
    );
}

// ============================================================================
// QUOTIENT RULE DERIVATIVES (SymPy-verified)
// ============================================================================

#[test]
fn test_oracle_sin_over_x() {
    // d/dx[sin(x)/x] = (x*cos(x) - sin(x))/x^2
    let x = std::f64::consts::PI / 2.0;
    let expected = (x * x.cos() - x.sin()) / (x * x);
    test_derivative("sin(x)/x", "x", "(x*cos(x) - sin(x))/x^2", &[(x, expected)]);
}

#[test]
fn test_oracle_1_over_x() {
    // d/dx[1/x] = -1/x^2
    test_derivative("1/x", "x", "-1/x^2", &[(1.0, -1.0), (2.0, -0.25)]);
}

#[test]
fn test_oracle_x_over_sin() {
    // d/dx[x/sin(x)] = (sin(x) - x*cos(x))/sin(x)^2
    let x = 1.0_f64;
    let expected = (x.sin() - x * x.cos()) / x.sin().powi(2);
    test_derivative("x/sin(x)", "x", "", &[(1.0, expected)]);
}

// ============================================================================
// POWER RULE WITH EXPRESSIONS (SymPy-verified)
// ============================================================================

#[test]
fn test_oracle_sqrt_x() {
    // d/dx[sqrt(x)] = 1/(2*sqrt(x))
    test_derivative("sqrt(x)", "x", "1/(2*sqrt(x))", &[(1.0, 0.5), (4.0, 0.25)]);
}

#[test]
fn test_oracle_x_to_half() {
    // d/dx[x^0.5] = 0.5*x^(-0.5)
    test_derivative("x^0.5", "x", "", &[(1.0, 0.5), (4.0, 0.25)]);
}

#[test]
fn test_oracle_x_to_x() {
    // d/dx[x^x] = x^x * (ln(x) + 1)
    test_derivative(
        "x^x",
        "x",
        "x^x * (ln(x) + 1)",
        &[
            (1.0, 1.0), // 1^1 * (0 + 1) = 1
            (2.0, 4.0 * (2.0_f64.ln() + 1.0)),
        ],
    );
}

// ============================================================================
// HYPERBOLIC FUNCTIONS (SymPy-verified)
// ============================================================================

#[test]
fn test_oracle_sinh() {
    // d/dx[sinh(x)] = cosh(x)
    test_derivative(
        "sinh(x)",
        "x",
        "cosh(x)",
        &[(0.0, 1.0), (1.0, 1.0_f64.cosh())],
    );
}

#[test]
fn test_oracle_cosh() {
    // d/dx[cosh(x)] = sinh(x)
    test_derivative(
        "cosh(x)",
        "x",
        "sinh(x)",
        &[(0.0, 0.0), (1.0, 1.0_f64.sinh())],
    );
}

#[test]
fn test_oracle_tanh() {
    // d/dx[tanh(x)] = 1 - tanh(x)^2 = sech^2(x)
    test_derivative(
        "tanh(x)",
        "x",
        "",
        &[(0.0, 1.0), (1.0, 1.0 - 1.0_f64.tanh().powi(2))],
    );
}

// ============================================================================
// INVERSE TRIG FUNCTIONS (SymPy-verified)
// ============================================================================

#[test]
fn test_oracle_asin() {
    // d/dx[asin(x)] = 1/sqrt(1-x^2)
    // Skip symbolic comparison since formula is invalid for |x| >= 1
    test_derivative("asin(x)", "x", "", &[(0.5, 1.0 / (1.0 - 0.25_f64).sqrt())]);
}

#[test]
fn test_oracle_acos() {
    // d/dx[acos(x)] = -1/sqrt(1-x^2)
    // Skip symbolic comparison since formula is invalid for |x| >= 1
    test_derivative("acos(x)", "x", "", &[(0.5, -1.0 / (0.75_f64).sqrt())]);
}

#[test]
fn test_oracle_atan() {
    // d/dx[atan(x)] = 1/(1+x^2)
    test_derivative("atan(x)", "x", "1/(1+x^2)", &[(0.0, 1.0), (1.0, 0.5)]);
}

// ============================================================================
// COMPLEX COMPOSITE EXPRESSIONS (SymPy-verified)
// ============================================================================

#[test]
fn test_oracle_ln_sin_x() {
    // d/dx[ln(sin(x))] = cos(x)/sin(x) = cot(x)
    let x = 1.0_f64;
    let expected = x.cos() / x.sin();
    test_derivative("ln(sin(x))", "x", "cos(x)/sin(x)", &[(1.0, expected)]);
}

#[test]
fn test_oracle_exp_x2_sin() {
    // d/dx[exp(x^2)*sin(x)] = exp(x^2)*(2*x*sin(x) + cos(x))
    let x = 1.0_f64;
    let ex2 = x.powi(2).exp();
    let expected = ex2 * (2.0 * x * x.sin() + x.cos());
    test_derivative("exp(x^2)*sin(x)", "x", "", &[(1.0, expected)]);
}

#[test]
fn test_oracle_nested_exp() {
    // d/dx[exp(exp(x))] = exp(x) * exp(exp(x))
    let x = 0.5_f64;
    let ex = x.exp();
    let expected = ex * ex.exp();
    test_derivative("exp(exp(x))", "x", "exp(x)*exp(exp(x))", &[(0.5, expected)]);
}

// ============================================================================
// SECOND DERIVATIVES (SymPy-verified)
// ============================================================================

#[test]
fn test_oracle_second_derivative_x3() {
    // d²/dx²[x^3] = 6*x
    let d1 = diff("x^3", "x", &[], None).unwrap();
    let d2 = diff(&d1, "x", &[], None).unwrap();
    let val = eval_at(&d2, 2.0);
    assert!((val - 12.0).abs() < 1e-10);
}

#[test]
fn test_oracle_second_derivative_sin() {
    // d²/dx²[sin(x)] = -sin(x)
    let d1 = diff("sin(x)", "x", &[], None).unwrap();
    let d2 = diff(&d1, "x", &[], None).unwrap();
    let x = 1.0_f64;
    let val = eval_at(&d2, x);
    assert!((val - (-x.sin())).abs() < 1e-10);
}

#[test]
fn test_oracle_second_derivative_exp() {
    // d²/dx²[exp(x)] = exp(x)
    let d1 = diff("exp(x)", "x", &[], None).unwrap();
    let d2 = diff(&d1, "x", &[], None).unwrap();
    let x = 1.0_f64;
    let val = eval_at(&d2, x);
    assert!((val - x.exp()).abs() < 1e-10);
}
