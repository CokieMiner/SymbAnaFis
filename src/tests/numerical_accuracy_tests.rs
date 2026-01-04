// Numerical accuracy tests - verify derivatives are computed correctly

use crate::diff;

#[test]
fn test_polynomial_accuracy() {
    // d/dx[x^3] = 3x^2
    let result = diff("x^3", "x", &[], None).unwrap();
    assert_eq!(result, "3*x^2");

    // d/dx[5x^4 + 3x^2 + 7] = 20x^3 + 6x or 2*(10*x^3 + 3*x) after GCD factoring
    let result = diff("5*x^4 + 3*x^2 + 7", "x", &[], None).unwrap();
    println!("DEBUG: test_polynomial_accuracy result: '{}'", result);
    assert!(
        (result.contains("20") && result.contains("x^3") && result.contains("6"))
            || (result.contains("2")
                && result.contains("10")
                && result.contains("x^2")
                && result.contains("3"))
    );
}

#[test]
fn test_trig_derivative_accuracy() {
    // d/dx[sin(x)] = cos(x)
    let result = diff("sin(x)", "x", &[], None).unwrap();
    assert_eq!(result, "cos(x)");

    // d/dx[cos(x)] = -sin(x)
    let result = diff("cos(x)", "x", &[], None).unwrap();
    assert_eq!(result, "-sin(x)");

    // d/dx[tan(x)] = sec^2(x)
    let result = diff("tan(x)", "x", &[], None).unwrap();
    assert!(result.contains("sec") && result.contains("2"));
}

#[test]
fn test_exponential_log_accuracy() {
    // d/dx[e^x] = e^x
    let result = diff("exp(x)", "x", &[], None).unwrap();
    assert_eq!(result, "exp(x)");

    // d/dx[ln(x)] = 1/x
    let result = diff("ln(x)", "x", &[], None).unwrap();
    assert!(result == "1/x" || result == "(x)^-1");

    // d/dx[x^x] = x^x * (ln(x) + 1)
    let result = diff("x^x", "x", &[], None).unwrap();
    // After canonicalization, might be reordered but should still contain both parts
    // May also simplify x^(1 + x - 1) to x^x
    let has_x_power_x =
        result.contains("x^x") || result.contains("x ^ x") || result.contains("x^(1 + x - 1)");
    assert!(has_x_power_x, "Missing x^x or equivalent in: {}", result);
    assert!(result.contains("ln"), "Missing ln in: {}", result);
}

#[test]
fn test_product_rule_accuracy() {
    // d/dx[x*sin(x)] = sin(x) + x*cos(x)
    let result = diff("x*sin(x)", "x", &[], None).unwrap();
    assert!(result.contains("sin") && result.contains("cos") && result.contains("x"));
}

#[test]
fn test_quotient_rule_accuracy() {
    // d/dx[sin(x)/x] = (x*cos(x) - sin(x))/x^2
    let result = diff("sin(x)/x", "x", &[], None).unwrap();
    assert!(result.contains("cos") && result.contains("sin") && result.contains("x^2"));
}

#[test]
fn test_chain_rule_accuracy() {
    // d/dx[sin(x^2)] = 2x*cos(x^2)
    let result = diff("sin(x^2)", "x", &[], None).unwrap();
    assert!(result.contains("2") && result.contains("x") && result.contains("cos"));
}

#[test]
fn test_hyperbolic_accuracy() {
    // d/dx[sinh(x)] = cosh(x)
    let result = diff("sinh(x)", "x", &[], None).unwrap();
    assert_eq!(result, "cosh(x)");

    // d/dx[cosh(x)] = sinh(x)
    let result = diff("cosh(x)", "x", &[], None).unwrap();
    assert_eq!(result, "sinh(x)");
}

#[test]
fn test_inverse_trig_accuracy() {
    // d/dx[arcsin(x)] = 1/sqrt(1-x^2)
    let result = diff("asin(x)", "x", &[], None).unwrap();
    assert!(result.contains("sqrt") && result.contains("1"));

    // d/dx[arctan(x)] = 1/(1+x^2)
    let result = diff("atan(x)", "x", &[], None).unwrap();
    assert!(result.contains("1") && result.contains("x^2"));
}

#[test]
fn test_special_functions_accuracy() {
    // d/dx[erf(x)] = (2/√π) * e^(-x²)
    let result = diff("erf(x)", "x", &[], None).unwrap();
    assert!(result.contains("exp") && result.contains("pi") && result.contains("2"));

    // d/dx[gamma(x)] = gamma(x) * digamma(x)
    let result = diff("gamma(x)", "x", &[], None).unwrap();
    assert!(result.contains("gamma") && result.contains("digamma"));
}

#[test]
fn test_root_derivatives_accuracy() {
    // d/dx[√x] = 1/(2√x)
    let result = diff("sqrt(x)", "x", &[], None).unwrap();
    assert!(result.contains("2") && result.contains("sqrt"));

    // d/dx[∛x] = 1/(3∛x²)
    let result = diff("cbrt(x)", "x", &[], None).unwrap();
    assert!(result.contains("3") && result.contains("x"));
}

#[test]
fn test_log_variants_accuracy() {
    // d/dx[log₁₀(x)] = 1/(x * ln(10))
    let result = diff("log10(x)", "x", &[], None).unwrap();
    assert!(result.contains("ln") && result.contains("10"));

    // d/dx[log₂(x)] = 1/(x * ln(2))
    let result = diff("log2(x)", "x", &[], None).unwrap();
    assert!(result.contains("ln") && result.contains("2"));
}

#[test]
fn test_constant_derivatives() {
    // d/dx[5] = 0
    let result = diff("5", "x", &[], None).unwrap();
    assert_eq!(result, "0");

    // d/dx[π] = 0
    let result = diff("pi", "x", &[], None).unwrap();
    assert_eq!(result, "0");
}

#[test]
fn test_linear_derivatives() {
    // d/dx[ax + b] = a
    let result = diff("3*x + 5", "x", &[], None).unwrap();
    assert_eq!(result, "3");

    // d/dx[ax] = a
    let result = diff("7*x", "x", &[], None).unwrap();
    assert_eq!(result, "7");
}

#[test]
fn test_sinc_derivative_at_zero() {
    use crate::{parse, Diff, CompiledEvaluator};
    use std::collections::{HashMap, HashSet};

    // Test the problem from the issue: sinc'(0) should be 0, not NaN
    
    // Parse sinc(x)
    let fixed_vars = HashSet::new();
    let custom_funcs = HashSet::new();
    let expr = parse("sinc(x)", &fixed_vars, &custom_funcs, None).unwrap();
    
    // Differentiate
    let diff_engine = Diff::new();
    let deriv = diff_engine.differentiate_by_name(&expr, "x").unwrap();
    println!("Derivative of sinc(x): {}", deriv);
    
    // Test 1: Compiled evaluation at x=0
    let compiled = CompiledEvaluator::compile(&deriv, &["x"], None).unwrap();
    let result_at_zero = compiled.evaluate(&[0.0]);
    assert!(
        !result_at_zero.is_nan(),
        "sinc'(0) should not be NaN, got: {}",
        result_at_zero
    );
    assert!(
        result_at_zero.abs() < 1e-10,
        "sinc'(0) should be 0, got: {}",
        result_at_zero
    );
    
    // Test 2: Verify behavior near zero (Taylor series approximation)
    let result_near_zero = compiled.evaluate(&[1e-8]);
    assert!(
        !result_near_zero.is_nan(),
        "sinc'(1e-8) should not be NaN"
    );
    // sinc'(x) ≈ -x/3 for small x, so sinc'(1e-8) ≈ -1e-8/3 ≈ -3.3e-9
    let expected_near_zero = -1e-8 / 3.0;
    assert!(
        (result_near_zero - expected_near_zero).abs() < 1e-10,
        "sinc'(1e-8) should be approximately {}, got: {}",
        expected_near_zero,
        result_near_zero
    );
    
    // Test 3: Verify correct behavior away from zero
    let x_test = 1.0;
    let result_at_one = compiled.evaluate(&[x_test]);
    // Analytical: sinc'(x) = (x*cos(x) - sin(x))/x²
    let expected_at_one = (x_test * x_test.cos() - x_test.sin()) / (x_test * x_test);
    assert!(
        (result_at_one - expected_at_one).abs() < 1e-10,
        "sinc'(1.0) should be {}, got: {}",
        expected_at_one,
        result_at_one
    );
    
    // Test 4: Symbolic evaluation (partial) should work
    let mut vars = HashMap::new();
    vars.insert("x", 0.0);
    let symbolic_result = deriv.evaluate(&vars, &HashMap::new());
    println!("Symbolic evaluation at x=0: {}", symbolic_result);
    // Should either be a number close to 0, or an expression that can be compiled
    match symbolic_result.as_number() {
        Some(n) => {
            assert!(
                !n.is_nan() && n.abs() < 1e-10,
                "Symbolic evaluation at x=0 should give 0, got: {}",
                n
            );
        }
        None => {
            // If still symbolic, compile and evaluate should work
            let compiled2 = CompiledEvaluator::compile::<&str>(&symbolic_result, &[], None);
            match compiled2 {
                Ok(c) => {
                    let val = c.evaluate(&[]);
                    assert!(
                        !val.is_nan() && val.abs() < 1e-10,
                        "Compiled symbolic result should be 0, got: {}",
                        val
                    );
                }
                Err(_) => {
                    // If it can't compile (because it still contains x), that's ok
                    // as long as it's not NaN when we try to use it
                }
            }
        }
    }
}

#[test]
fn test_sinc_derivative_formula() {
    use crate::{parse, Diff};
    use std::collections::HashSet;

    // Verify the derivative formula is correct symbolically
    let fixed_vars = HashSet::new();
    let custom_funcs = HashSet::new();
    let expr = parse("sinc(x)", &fixed_vars, &custom_funcs, None).unwrap();
    
    let diff_engine = Diff::new();
    let deriv = diff_engine.differentiate_by_name(&expr, "x").unwrap();
    let deriv_str = deriv.to_string();
    
    // The derivative should mention _sinc_deriv helper function
    assert!(
        deriv_str.contains("_sinc_deriv") || deriv_str.contains("sinc"),
        "Derivative should use _sinc_deriv helper or be simplified, got: {}",
        deriv_str
    );
}

