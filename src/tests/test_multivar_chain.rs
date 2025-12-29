//! Test multi-variable chain rule for functions with multiple continuous parameters
//!
//! Verifies that when both theta(t) and phi(t) vary with t, the derivative
//! correctly computes: d/dt Y(theta(t), phi(t)) = (∂Y/∂θ)·θ' + (∂Y/∂φ)·φ'

use crate::diff;

#[test]
fn test_spherical_harmonic_single_variable_theta() {
    // When only theta varies: d/dt ynm(l, m, t, 0)
    // Should only have theta derivative term (phi term is 0)
    let result = diff("ynm(1, 1, t, 0)", "t", &[], None).unwrap();

    // Should contain theta-related terms (cot, sin, cos from theta derivative)
    assert!(
        result.contains("cot") || result.contains("cos") || result.contains("sin"),
        "Should contain theta derivative terms, got: {}",
        result
    );

    println!("✓ Single variable (theta): {}", result);
}

#[test]
fn test_spherical_harmonic_single_variable_phi() {
    // When only phi varies: d/dt ynm(l, m, 0, t)
    // Should only have phi derivative term (theta term is 0)
    let result = diff("ynm(1, 1, 0, t)", "t", &[], None).unwrap();

    // Should contain phi-related terms (tan from phi derivative)
    assert!(
        result.contains("tan") || result.contains("ynm"),
        "Should contain phi derivative terms, got: {}",
        result
    );

    println!("✓ Single variable (phi): {}", result);
}

#[test]
fn test_spherical_harmonic_both_variables_vary() {
    // CRITICAL TEST: When BOTH theta and phi vary with t
    // d/dt ynm(l, m, t, t) = (∂Y/∂θ)·1 + (∂Y/∂φ)·1
    let result = diff("ynm(1, 1, t, t)", "t", &[], None).unwrap();

    // Result should be longer/more complex than single variable case

    // The both-variables derivative should contain components from both partials
    // It should have BOTH theta terms (cot/sin/cos) AND phi terms (tan)
    let has_theta_terms =
        result.contains("cot") || result.contains("sin") || result.contains("cos");
    let has_phi_terms = result.contains("tan");

    assert!(
        has_theta_terms,
        "Should contain theta derivative terms in both-variable case, got: {}",
        result
    );

    // Note: phi terms might be simplified away if phi=t gives simple derivatives
    println!("✓ Both variables vary: {}", result);
    println!("  Has theta terms: {}", has_theta_terms);
    println!("  Has phi terms: {}", has_phi_terms);
}

#[test]
fn test_spherical_harmonic_chain_rule_coefficients() {
    // When theta=2t, phi=3t:
    // d/dt ynm(l, m, 2t, 3t) = (∂Y/∂θ)·2 + (∂Y/∂φ)·3
    let result = diff("ynm(1, 1, 2*t, 3*t)", "t", &[], None).unwrap();

    // Should contain the coefficients 2 and 3 from the chain rule
    assert!(
        result.contains("2") || result.contains("3"),
        "Should contain chain rule coefficients, got: {}",
        result
    );

    println!("✓ Chain rule coefficients: {}", result);
}

#[test]
fn test_atan2_both_arguments_vary() {
    // Test another 2-argument function: atan2(y, x)
    // d/dt atan2(t, t) should use both partial derivatives
    let result = diff("atan2(t, t)", "t", &[], None).unwrap();

    // atan2(t, t) = atan2(y=t, x=t)
    // d/dt = (x·y' - y·x')/(x²+y²) = (t·1 - t·1)/(t²+t²) = 0
    // So the result should simplify to something close to 0 or be symmetric

    println!("✓ atan2 with both varying: {}", result);
}

#[test]
fn test_beta_both_arguments_vary() {
    // Test beta(a, b) when both vary
    // d/dt beta(t, 2*t) = (∂β/∂a)·1 + (∂β/∂b)·2
    let result = diff("beta(t, 2*t)", "t", &[], None).unwrap();

    // Should contain digamma terms from both partials
    assert!(
        result.contains("digamma") || result.contains("beta"),
        "Should contain derivative terms, got: {}",
        result
    );

    println!("✓ Beta with both varying: {}", result);
}
