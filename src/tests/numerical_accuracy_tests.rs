// Numerical accuracy tests - verify derivatives are computed correctly

use crate::diff;

#[test]
fn test_polynomial_accuracy() {
    // d/dx[x^3] = 3x^2
    let result = diff("x^3".to_string(), "x".to_string(), None, None).unwrap();
    assert_eq!(result, "3 * x^2");

    // d/dx[5x^4 + 3x^2 + 7] = 20x^3 + 6x
    let result = diff("5*x^4 + 3*x^2 + 7".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("20") && result.contains("x^3") && result.contains("6") && result.contains("x"));
}

#[test]
fn test_trig_derivative_accuracy() {
    // d/dx[sin(x)] = cos(x)
    let result = diff("sin(x)".to_string(), "x".to_string(), None, None).unwrap();
    assert_eq!(result, "cos(x)");

    // d/dx[cos(x)] = -sin(x)
    let result = diff("cos(x)".to_string(), "x".to_string(), None, None).unwrap();
    assert_eq!(result, "-sin(x)");

    // d/dx[tan(x)] = sec^2(x)
    let result = diff("tan(x)".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("sec") && result.contains("2"));
}

#[test]
fn test_exponential_log_accuracy() {
    // d/dx[e^x] = e^x
    let result = diff("exp(x)".to_string(), "x".to_string(), None, None).unwrap();
    assert_eq!(result, "exp(x)");

    // d/dx[ln(x)] = 1/x
    let result = diff("ln(x)".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result == "1 / x" || result == "(x)^-1");

    // d/dx[x^x] = x^x * (ln(x) + 1)
    let result = diff("x^x".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("x^x") && result.contains("ln"));
}

#[test]
fn test_product_rule_accuracy() {
    // d/dx[x*sin(x)] = sin(x) + x*cos(x)
    let result = diff("x*sin(x)".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("sin") && result.contains("cos") && result.contains("x"));
}

#[test]
fn test_quotient_rule_accuracy() {
    // d/dx[sin(x)/x] = (x*cos(x) - sin(x))/x^2
    let result = diff("sin(x)/x".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("cos") && result.contains("sin") && result.contains("x^2"));
}

#[test]
fn test_chain_rule_accuracy() {
    // d/dx[sin(x^2)] = 2x*cos(x^2)
    let result = diff("sin(x^2)".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("2") && result.contains("x") && result.contains("cos"));
}

#[test]
fn test_hyperbolic_accuracy() {
    // d/dx[sinh(x)] = cosh(x)
    let result = diff("sinh(x)".to_string(), "x".to_string(), None, None).unwrap();
    assert_eq!(result, "cosh(x)");

    // d/dx[cosh(x)] = sinh(x)
    let result = diff("cosh(x)".to_string(), "x".to_string(), None, None).unwrap();
    assert_eq!(result, "sinh(x)");
}

#[test]
fn test_inverse_trig_accuracy() {
    // d/dx[arcsin(x)] = 1/sqrt(1-x^2)
    let result = diff("asin(x)".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("sqrt") && result.contains("1"));

    // d/dx[arctan(x)] = 1/(1+x^2)
    let result = diff("atan(x)".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("1") && result.contains("x^2"));
}

#[test]
fn test_special_functions_accuracy() {
    // d/dx[erf(x)] = (2/√π) * e^(-x²)
    let result = diff("erf(x)".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("exp") && result.contains("pi") && result.contains("2"));

    // d/dx[gamma(x)] = gamma(x) * digamma(x)
    let result = diff("gamma(x)".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("gamma") && result.contains("digamma"));
}

#[test]
fn test_root_derivatives_accuracy() {
    // d/dx[√x] = 1/(2√x)
    let result = diff("sqrt(x)".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("2") && result.contains("sqrt"));

    // d/dx[∛x] = 1/(3∛x²)
    let result = diff("cbrt(x)".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("3") && result.contains("x"));
}

#[test]
fn test_log_variants_accuracy() {
    // d/dx[log₁₀(x)] = 1/(x * ln(10))
    let result = diff("log10(x)".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("ln") && result.contains("10"));

    // d/dx[log₂(x)] = 1/(x * ln(2))
    let result = diff("log2(x)".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("ln") && result.contains("2"));
}

#[test]
fn test_constant_derivatives() {
    // d/dx[5] = 0
    let result = diff("5".to_string(), "x".to_string(), None, None).unwrap();
    assert_eq!(result, "0");

    // d/dx[π] = 0
    let result = diff("pi".to_string(), "x".to_string(), None, None).unwrap();
    assert_eq!(result, "0");
}

#[test]
fn test_linear_derivatives() {
    // d/dx[ax + b] = a
    let result = diff("3*x + 5".to_string(), "x".to_string(), None, None).unwrap();
    assert_eq!(result, "3");

    // d/dx[ax] = a
    let result = diff("7*x".to_string(), "x".to_string(), None, None).unwrap();
    assert_eq!(result, "7");
}