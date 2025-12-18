use crate::diff;
// Integration tests - Phase 2 features

// MVP tests
#[test]
fn test_simple_constant() {
    let result = diff("5", "x", None, None).unwrap();
    assert_eq!(result, "0");
}

#[test]
fn test_simple_variable() {
    let result = diff("x", "x", None, None).unwrap();
    assert_eq!(result, "1");
}

#[test]
fn test_power_rule() {
    let result = diff("x^2", "x", None, None).unwrap();
    assert!(result.contains("2"));
    assert!(result.contains("x"));
}

#[test]
fn test_with_fixed_constant() {
    let result = diff("a*x", "x", Some(&["a"]), None).unwrap();
    assert_eq!(result, "a");
}

#[test]
fn test_sin_function() {
    let result = diff("sin(x)", "x", None, None).unwrap();
    assert!(result.contains("cos"));
}

// PHASE 2 TESTS

#[test]
fn test_subtraction() {
    let result = diff("x - 5", "x", None, None).unwrap();
    assert_eq!(result, "1");
}

#[test]
fn test_division() {
    let result = diff("x / 2", "x", None, None).unwrap();
    // (1*2 - x*0) / 2^2 = 2 / 4 = 0.5
    assert!(result.contains("2") || result.contains("0.5"));
}

#[test]
fn test_x_to_x() {
    // x^x → x^x * (ln(x) + 1)
    let result = diff("x^x", "x", None, None).unwrap();
    assert!(result.contains("ln"));
}

#[test]
fn test_sinh() {
    let result = diff("sinh(x)", "x", None, None).unwrap();
    assert!(result.contains("cosh"));
}

#[test]
fn test_cosh() {
    let result = diff("cosh(x)", "x", None, None).unwrap();
    assert!(result.contains("sinh"));
}

#[test]
fn test_tanh() {
    let result = diff("tanh(x)", "x", None, None).unwrap();
    assert!(result.contains("tanh") || result.contains("sech"));
}

#[test]
fn test_quotient_rule() {
    // sin(x) / x
    let result = diff("sin(x) / x", "x", None, None).unwrap();
    assert!(result.contains("cos") || result.contains("sin"));
}

#[test]
fn test_x_to_2x() {
    // x^(2*x) should use logarithmic diff
    let result = diff("x^(2*x)", "x", None, None).unwrap();
    assert!(result.contains("ln"));
}

#[test]
fn test_scientific_notation() {
    let result = diff("1e10*x", "x", None, None).unwrap();
    assert!(result.contains("10000000000") || result.contains("1e"));
}

#[test]
fn test_empty_parens() {
    // () should be an error
    assert!(diff("x*()", "x", None, None).is_err());
}

#[test]
fn test_auto_balance_parens() {
    let result = diff("(x+1", "x", None, None).unwrap();
    assert_eq!(result, "1");
}

#[test]
fn test_implicit_multiplication() {
    let result = diff("2x", "x", None, None).unwrap();
    assert_eq!(result, "2");
}

#[test]
fn test_error_var_in_fixed() {
    let result = diff("x", "x", Some(&["x"]), None);
    assert!(result.is_err());
}

#[test]
fn test_derivative_simplification_from_examples() {
    // Test derivative from basic.rs example: d/dx[sin(x) * cos(x)]
    // This should simplify to cos(2*x)
    let result = diff("sin(x) * cos(x)", "x", None, None).unwrap();

    // The result should be cos(2*x)
    assert!(result.contains("cos(2x)"));
}

#[test]
fn test_chain_rule_derivative_from_examples() {
    // Test derivative from chain_rule.rs example: d/dx[sin(cos(2*x))]
    // This should be -4 * cos(x) * cos(cos(2 * x)) * sin(x)
    let result = diff("sin(cos(2*x))", "x", None, None).unwrap();

    println!("Result: {}", result);

    // The result should contain the correct simplified form
    assert!(result.contains("-4"), "Expected -4 in result: {}", result);
    assert!(
        result.contains("cos(x)"),
        "Expected cos(x) in result: {}",
        result
    );
    assert!(
        result.contains("sin(x)"),
        "Expected sin(x) in result: {}",
        result
    );
    assert!(
        result.contains("cos(cos(2x))"),
        "Expected cos(cos(2x)) in result: {}",
        result
    );
}
