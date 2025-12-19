//! Regression tests for expression equality via simplification.
//!
//! These tests verify that mathematically equivalent expressions simplify
//! to the same canonical form, enabling equality checking via string comparison.

use crate::simplify;

/// Helper to check if two expressions simplify to the same form.
fn check_equality(expr1: &str, expr2: &str) -> bool {
    let simp1 = simplify(expr1, None, None).unwrap();
    let simp2 = simplify(expr2, None, None).unwrap();
    simp1 == simp2
}

// =============================================================================
// Difference of Squares Regression Tests
// =============================================================================

/// REGRESSION: x^2 - y^2 and (x-y)*(x+y) should simplify to the same form.
/// Previously, these would cycle between factored and expanded forms.
#[test]
fn test_difference_of_squares_equality_symbols() {
    // Both should converge to the same canonical form
    let simp1 = simplify("x^2 - y^2", None, None).unwrap();
    let simp2 = simplify("(x-y)*(x+y)", None, None).unwrap();

    assert_eq!(
        simp1, simp2,
        "Difference of squares should have consistent canonical form.\n\
         x^2 - y^2 -> {}\n\
         (x-y)*(x+y) -> {}",
        simp1, simp2
    );
}

/// REGRESSION: Same pattern with numbers.
#[test]
fn test_difference_of_squares_equality_with_number() {
    let simp1 = simplify("x^2 - 4", None, None).unwrap();
    let simp2 = simplify("(x-2)*(x+2)", None, None).unwrap();

    assert_eq!(
        simp1, simp2,
        "x^2 - 4 and (x-2)*(x+2) should simplify to the same form.\n\
         x^2 - 4 -> {}\n\
         (x-2)*(x+2) -> {}",
        simp1, simp2
    );
}

/// REGRESSION: Number minus symbol squared.
#[test]
fn test_difference_of_squares_number_minus_symbol() {
    let simp1 = simplify("9 - x^2", None, None).unwrap();
    let simp2 = simplify("(3-x)*(3+x)", None, None).unwrap();

    assert_eq!(
        simp1, simp2,
        "9 - x^2 and (3-x)*(3+x) should simplify to the same form.\n\
         9 - x^2 -> {}\n\
         (3-x)*(3+x) -> {}",
        simp1, simp2
    );
}

// =============================================================================
// Power Expansion Regression Tests
// =============================================================================

/// REGRESSION: (2*x)^2 should expand to 4*x^2.
#[test]
fn test_power_expansion_2x_squared() {
    let result = simplify("(2*x)^2", None, None).unwrap();
    // Should be 4*x^2 in some canonical form
    assert!(
        result.contains("4") && result.contains("x"),
        "(2*x)^2 should expand to contain 4 and x^2, got: {}",
        result
    );
    // Should NOT contain (2x)^2 or similar unexpanded form
    assert!(
        !result.contains("(2"),
        "(2*x)^2 should be fully expanded, but got: {}",
        result
    );
}

/// REGRESSION: (3*y)^2 should expand to 9*y^2.
#[test]
fn test_power_expansion_3y_squared() {
    let result = simplify("(3*y)^2", None, None).unwrap();
    // Should be 9*y^2 in some canonical form
    assert!(
        result.contains("9") && result.contains("y"),
        "(3*y)^2 should expand to contain 9 and y^2, got: {}",
        result
    );
    // Should NOT contain (3y)^2 or similar unexpanded form
    assert!(
        !result.contains("(3"),
        "(3*y)^2 should be fully expanded, but got: {}",
        result
    );
}

/// REGRESSION: Negated power expansion should work.
/// -(3*y)^2 should expand to -9*y^2, not stay as -(3y)^2.
#[test]
fn test_negated_power_expansion() {
    let result = simplify("-(3*y)^2", None, None).unwrap();
    // Should contain 9 (expanded coefficient), not (3y)^2
    assert!(
        result.contains("9"),
        "-(3*y)^2 should expand to -9*y^2, got: {}",
        result
    );
}

/// REGRESSION: Complex difference of squares with coefficients.
/// (2*x)^2 - (3*y)^2 should expand to 4*x^2 - 9*y^2.
#[test]
fn test_difference_of_squares_complex() {
    let result = simplify("(2*x)^2 - (3*y)^2", None, None).unwrap();

    // Should contain 4 and 9 (expanded coefficients)
    assert!(
        result.contains("4") && result.contains("9"),
        "(2*x)^2 - (3*y)^2 should expand coefficients to 4 and 9, got: {}",
        result
    );

    // Or alternatively, should be factored consistently
    // Either way, it should NOT have mixed forms like "4x^2 - (3y)^2"
    let has_unexpanded_3y = result.contains("(3") && result.contains(")^2");
    assert!(
        !has_unexpanded_3y,
        "(2*x)^2 - (3*y)^2 should not have mixed expansion, got: {}",
        result
    );
}

// =============================================================================
// Known Working Equality Tests (Sanity Checks)
// =============================================================================

#[test]
fn test_pythagorean_identity() {
    assert!(
        check_equality("sin(x)^2 + cos(x)^2", "1"),
        "Pythagorean identity should simplify to 1"
    );
}

#[test]
fn test_perfect_square_equality() {
    assert!(
        check_equality("(x+1)^2", "x^2 + 2*x + 1"),
        "Perfect square should match expanded form"
    );
}

#[test]
fn test_exponential_addition() {
    assert!(
        check_equality("exp(a + b)", "exp(a) * exp(b)"),
        "Exponential addition identity"
    );
}

#[test]
fn test_fraction_addition() {
    assert!(
        check_equality("1/x + 1/y", "(x+y)/(x*y)"),
        "Fraction addition should match"
    );
}

#[test]
fn test_nested_abs() {
    assert!(
        check_equality("abs(abs(x))", "abs(x)"),
        "Nested abs should simplify"
    );
}

#[test]
fn test_double_angle_sin() {
    assert!(
        check_equality("sin(2*x)", "2*sin(x)*cos(x)"),
        "Double angle formula should match"
    );
}
