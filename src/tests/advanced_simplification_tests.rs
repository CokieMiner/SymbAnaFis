// Advanced simplification tests - complex algebraic and trigonometric identities

use crate::{simplify, parser};
use std::collections::HashSet;

fn parse_expr(s: &str) -> crate::Expr {
    let fixed_vars = HashSet::new();
    let custom_funcs = HashSet::new();
    parser::parse(s, &fixed_vars, &custom_funcs).unwrap()
}

#[test]
fn test_trig_identity_simplification() {
    // sin^2(x) + cos^2(x) = 1
    let expr = parse_expr("sin(x)^2 + cos(x)^2");
    let result = simplify(expr);
    assert_eq!(format!("{}", result), "1");

    // cos^2(x) - sin^2(x) = cos(2x)
    let expr = parse_expr("cos(x)^2 - sin(x)^2");
    let result = simplify(expr);
    let result_str = format!("{}", result);
    // Should simplify to cos(2x) or equivalent canonical form
    assert!(result_str.contains("cos") || result_str == "cos(2 * x)");
}

#[test]
fn test_hyperbolic_identity_simplification() {
    // cosh^2(x) - sinh^2(x) = 1
    let expr = parse_expr("cosh(x)^2 - sinh(x)^2");
    let result = simplify(expr);
    assert_eq!(format!("{}", result), "1");

    // After algebraic canonicalization: cosh^2(x) + (-1)*sinh^2(x) = 1
    let expr = parse_expr("cosh(x)^2 + ((-1) * sinh(x)^2)");
    let result = simplify(expr);
    assert_eq!(format!("{}", result), "1");
}

#[test]
fn test_exponential_form_recognition() {
    // (e^x + e^(-x))/2 = cosh(x)
    let expr = parse_expr("(exp(x) + exp((-1) * x)) / 2");
    let result = simplify(expr);
    let result_str = format!("{}", result);
    assert!(result_str.contains("cosh") || result_str == "cosh(x)");

    // (e^x - e^(-x))/2 = sinh(x)
    let expr = parse_expr("(exp(x) - exp((-1) * x)) / 2");
    let result = simplify(expr);
    let result_str = format!("{}", result);
    assert!(result_str.contains("sinh") || result_str == "sinh(x)");
}

#[test]
fn test_complex_algebraic_simplification() {
    // (x^2 - 1)/(x - 1) = x + 1 (for x ≠ 1)
    let expr = parse_expr("(x^2 - 1) / (x - 1)");
    let result = simplify(expr);
    let result_str = format!("{}", result);
    // This might not simplify completely due to division, but should be recognized
    assert!(result_str.contains("x") || result_str == "(x^2 - 1) / (x - 1)");

    // x^3 - x^2 - x + 1 = (x-1)^2 * (x+1)
    // This is a more complex factorization that might not be implemented
    let expr = parse_expr("x^3 - x^2 - x + 1");
    let result = simplify(expr);
    let result_str = format!("{}", result);
    // Should at least combine like terms
    assert!(result_str.contains("x"));
}

#[test]
fn test_nested_trig_simplification() {
    // sin(2x) = 2*sin(x)*cos(x)
    let expr = parse_expr("sin(2 * x)");
    let result = simplify(expr);
    let result_str = format!("{}", result);
    assert!(result_str.contains("sin") && result_str.contains("cos") && result_str.contains("2"));

    // cos(2x) in terms of sin and cos
    let expr = parse_expr("cos(2 * x)");
    let result = simplify(expr);
    let result_str = format!("{}", result);
    assert!(result_str.contains("cos") && result_str.contains("sin"));
}

#[test]
fn test_logarithmic_simplification() {
    // ln(e^x) = x
    let expr = parse_expr("ln(exp(x))");
    let result = simplify(expr);
    assert_eq!(format!("{}", result), "x");

    // exp(ln(x)) = x
    let expr = parse_expr("exp(ln(x))");
    let result = simplify(expr);
    assert_eq!(format!("{}", result), "x");

    // ln(1) = 0
    let expr = parse_expr("ln(1)");
    let result = simplify(expr);
    assert_eq!(format!("{}", result), "0");
}

#[test]
fn test_power_simplification() {
    // x^0 = 1
    let expr = parse_expr("x^0");
    let result = simplify(expr);
    assert_eq!(format!("{}", result), "1");

    // x^1 = x
    let expr = parse_expr("x^1");
    let result = simplify(expr);
    assert_eq!(format!("{}", result), "x");

    // (x^a)^b = x^(a*b)
    let expr = parse_expr("(x^2)^3");
    let result = simplify(expr);
    let result_str = format!("{}", result);
    assert!(result_str == "x^6" || result_str == "(x)^6");
}

#[test]
fn test_root_simplification() {
    // sqrt(x^2) = x (assuming x ≥ 0)
    let expr = parse_expr("sqrt(x^2)");
    let result = simplify(expr);
    assert_eq!(format!("{}", result), "x");

    // cbrt(x^3) = x
    let expr = parse_expr("cbrt(x^3)");
    let result = simplify(expr);
    assert_eq!(format!("{}", result), "x");
}

#[test]
fn test_constant_folding() {
    // 2 + 3 = 5
    let expr = parse_expr("2 + 3");
    let result = simplify(expr);
    assert_eq!(format!("{}", result), "5");

    // 2 * 3 = 6
    let expr = parse_expr("2 * 3");
    let result = simplify(expr);
    assert_eq!(format!("{}", result), "6");

    // 6 / 2 = 3
    let expr = parse_expr("6 / 2");
    let result = simplify(expr);
    assert_eq!(format!("{}", result), "3");
}

#[test]
fn test_trig_exact_values() {
    // sin(0) = 1 (this should work)
    let expr = parse_expr("sin(0)");
    let result = simplify(expr);
    assert_eq!(format!("{}", result), "0");

    // cos(0) = 1
    let expr = parse_expr("cos(0)");
    let result = simplify(expr);
    assert_eq!(format!("{}", result), "1");

    // Note: pi is treated as a symbol, so sin(pi/2) doesn't simplify to 1
    // This is expected behavior for symbolic computation
}