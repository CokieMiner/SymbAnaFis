use crate::diff;
// Integration tests - Phase 2 features

// MVP tests
#[test]
fn test_simple_constant() {
    let result = diff("5".to_string(), "x".to_string(), None, None).unwrap();
    assert_eq!(result, "0");
}

#[test]
fn test_simple_variable() {
    let result = diff("x".to_string(), "x".to_string(), None, None).unwrap();
    assert_eq!(result, "1");
}

#[test]
fn test_power_rule() {
    let result = diff("x^2".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("2"));
    assert!(result.contains("x"));
}

#[test]
fn test_with_fixed_constant() {
    let result = diff(
        "a*x".to_string(),
        "x".to_string(),
        Some(&["a".to_string()]),
        None,
    )
    .unwrap();
    assert_eq!(result, "a");
}

#[test]
fn test_sin_function() {
    let result = diff("sin(x)".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("cos"));
}

// PHASE 2 TESTS

#[test]
fn test_subtraction() {
    let result = diff("x - 5".to_string(), "x".to_string(), None, None).unwrap();
    assert_eq!(result, "1");
}

#[test]
fn test_division() {
    let result = diff("x / 2".to_string(), "x".to_string(), None, None).unwrap();
    // (1*2 - x*0) / 2^2 = 2 / 4 = 0.5
    assert!(result.contains("2") || result.contains("0.5"));
}

#[test]
fn test_x_to_x() {
    // x^x → x^x * (ln(x) + 1)
    let result = diff("x^x".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("ln"));
}

#[test]
fn test_sinh() {
    let result = diff("sinh(x)".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("cosh"));
}

#[test]
fn test_cosh() {
    let result = diff("cosh(x)".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("sinh"));
}

#[test]
fn test_tanh() {
    let result = diff("tanh(x)".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("tanh") || result.contains("sech"));
}

#[test]
fn test_quotient_rule() {
    // sin(x) / x
    let result = diff("sin(x) / x".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("cos") || result.contains("sin"));
}

#[test]
fn test_x_to_2x() {
    // x^(2*x) should use logarithmic diff
    let result = diff("x^(2*x)".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("ln"));
}

#[test]
fn test_scientific_notation() {
    let result = diff("1e10*x".to_string(), "x".to_string(), None, None).unwrap();
    assert!(result.contains("10000000000") || result.contains("1e"));
}

#[test]
fn test_empty_parens() {
    let result = diff("x*()".to_string(), "x".to_string(), None, None).unwrap();
    // x * 1 → derivative is 1
    assert_eq!(result, "1");
}

#[test]
fn test_auto_balance_parens() {
    let result = diff("(x+1".to_string(), "x".to_string(), None, None).unwrap();
    assert_eq!(result, "1");
}

#[test]
fn test_implicit_multiplication() {
    let result = diff("2x".to_string(), "x".to_string(), None, None).unwrap();
    assert_eq!(result, "2");
}

#[test]
fn test_error_var_in_fixed() {
    let result = diff(
        "x".to_string(),
        "x".to_string(),
        Some(&["x".to_string()]),
        None,
    );
    assert!(result.is_err());
}
