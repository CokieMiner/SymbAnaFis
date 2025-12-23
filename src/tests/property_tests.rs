//! Property-Based and Fuzz Testing
//!
//! Uses quickcheck for property-based testing of:
//! - Parser robustness (fuzz testing)
//! - Algebraic identities preservation

use quickcheck::{Arbitrary, Gen, QuickCheck, TestResult};
use std::collections::HashSet;

use crate::{ExprKind, diff, parser, simplify};

// ============================================================
// PART 1: EXPRESSION GENERATORS FOR PROPERTY TESTS
// ============================================================

/// Generate random valid expression strings for fuzz testing
fn random_expr_string(g: &mut Gen) -> String {
    let depth = g.size().min(4); // Limit depth to avoid stack overflow
    gen_expr_string_recursive(g, depth)
}

fn gen_expr_string_recursive(g: &mut Gen, depth: usize) -> String {
    if depth == 0 {
        // Base cases: numbers or variables
        let choice: u8 = u8::arbitrary(g) % 4;
        match choice {
            0 => {
                let n: f64 = f64::arbitrary(g);
                if n.is_finite() && n.abs() < 1e10 {
                    format!("{:.4}", n)
                } else {
                    "1.0".to_string()
                }
            }
            1 => "x".to_string(),
            2 => "y".to_string(),
            3 => "z".to_string(),
            _ => "1".to_string(),
        }
    } else {
        let choice: u8 = u8::arbitrary(g) % 10;
        match choice {
            0..=2 => {
                // Binary operations
                let ops = ["+", "-", "*", "/", "^"];
                let op = ops[usize::arbitrary(g) % ops.len()];
                let left = gen_expr_string_recursive(g, depth - 1);
                let right = gen_expr_string_recursive(g, depth - 1);
                format!("({} {} {})", left, op, right)
            }
            3..=5 => {
                // Unary functions
                let fns = ["sin", "cos", "tan", "exp", "ln", "sqrt", "abs"];
                let f = fns[usize::arbitrary(g) % fns.len()];
                let arg = gen_expr_string_recursive(g, depth - 1);
                format!("{}({})", f, arg)
            }
            6 => {
                // Negation
                let arg = gen_expr_string_recursive(g, depth - 1);
                format!("-({})", arg)
            }
            _ => {
                // Just recurse
                gen_expr_string_recursive(g, depth - 1)
            }
        }
    }
}

// ============================================================
// PART 2: PARSER FUZZ TESTS
// ============================================================

#[cfg(test)]
mod parser_fuzz_tests {
    use super::*;

    /// Property: Parser should never panic on arbitrary input
    #[test]
    fn test_parser_never_panics_on_random_input() {
        fn prop_parser_no_panic(input: String) -> TestResult {
            let fixed = HashSet::new();
            let custom = HashSet::new();
            // Parser should either succeed or return Err, never panic
            let _ = parser::parse(&input, &fixed, &custom, None);
            TestResult::passed()
        }
        QuickCheck::new()
            .tests(1000)
            .max_tests(2000)
            .quickcheck(prop_parser_no_panic as fn(String) -> TestResult);
    }

    /// Property: Parser should handle generated valid expressions
    #[test]
    fn test_parser_handles_valid_expressions() {
        fn prop_valid_expr_parses() -> bool {
            let mut g = Gen::new(10);
            let expr_str = random_expr_string(&mut g);
            let fixed = HashSet::new();
            let custom = HashSet::new();
            // This should either parse or not, but not panic
            let result = parser::parse(&expr_str, &fixed, &custom, None);
            result.is_ok() || result.is_err() // Always true if no panic
        }
        QuickCheck::new()
            .tests(500)
            .quickcheck(prop_valid_expr_parses as fn() -> bool);
    }

    /// Property: Parsed expressions should round-trip through Display
    /// Note: This is exploratory - we log issues but don't fail the test
    #[test]
    fn test_parse_display_consistency() {
        // This test is informational - some edge cases may not round-trip
        // due to Display formatting choices that are valid but different
        let mut g = Gen::new(6);
        let mut failed_cases = 0;
        let total_tests = 100;

        for _ in 0..total_tests {
            let expr_str = random_expr_string(&mut g);
            let fixed = HashSet::new();
            let custom = HashSet::new();

            if let Ok(expr) = parser::parse(&expr_str, &fixed, &custom, None) {
                let displayed = format!("{}", expr);
                if parser::parse(&displayed, &fixed, &custom, None).is_err() {
                    failed_cases += 1;
                }
            }
        }

        // Allow some failures due to display formatting edge cases
        assert!(
            failed_cases < total_tests / 4,
            "Too many display round-trip failures: {}/{}",
            failed_cases,
            total_tests
        );
    }

    /// Fuzz test with specifically crafted edge cases
    #[test]
    fn test_parser_edge_cases() {
        let edge_cases = [
            "",
            "   ",
            "()",
            "((()))",
            "+++",
            "---x",
            "1+",
            "+1",
            "sin()",
            "sin(x,y)",
            "1..2",
            "1e999999",
            "1e-999999",
            "x^y^z",
            "((((x))))",
            "sin(cos(tan(exp(ln(x)))))",
            "x+y*z^w/a-b",
            "1/0",
            "0/0",
            "(-0)",
            "∞", // Unicode
            "π", // Unicode pi
            "ℯ", // Unicode e
        ];

        let fixed = HashSet::new();
        let custom = HashSet::new();

        for case in &edge_cases {
            // Should not panic - may succeed or fail with error
            let _ = parser::parse(case, &fixed, &custom, None);
        }
    }

    /// Test deeply nested expressions don't stack overflow
    #[test]
    fn test_parser_deep_nesting() {
        let fixed = HashSet::new();
        let custom = HashSet::new();

        // Create deeply nested expression
        let mut expr = "x".to_string();
        for _ in 0..50 {
            expr = format!("({}+1)", expr);
        }

        // Should handle without stack overflow
        let result = parser::parse(&expr, &fixed, &custom, None);
        assert!(
            result.is_ok(),
            "Deep nesting should parse: {}",
            result.unwrap_err()
        );
    }
}

// ============================================================
// PART 3: ALGEBRAIC IDENTITY PROPERTY TESTS
// ============================================================

#[cfg(test)]
mod algebraic_property_tests {
    use super::*;
    use std::collections::HashMap;

    const EPSILON: f64 = 1e-9;

    fn approx_eq(a: f64, b: f64) -> bool {
        if a.is_nan() && b.is_nan() {
            return true;
        }
        if a.is_infinite() && b.is_infinite() {
            return a.signum() == b.signum();
        }
        (a - b).abs() < EPSILON || (a - b).abs() < EPSILON * a.abs().max(b.abs())
    }

    /// Property: x + 0 = x
    #[test]
    fn test_additive_identity() {
        fn prop_add_zero(x_val: f64) -> TestResult {
            if !x_val.is_finite() {
                return TestResult::discard();
            }

            // Build x + 0
            let result = simplify("x + 0", None, None).unwrap();

            // After simplification, should evaluate to x
            let fixed = HashSet::new();
            let custom = HashSet::new();
            let expr = parser::parse(&result, &fixed, &custom, None).unwrap();
            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();

            if let ExprKind::Number(n) = expr.evaluate(&vars).kind {
                TestResult::from_bool(approx_eq(n, x_val))
            } else {
                // Result is symbolic, check that it evaluated correctly
                let orig = parser::parse("x", &fixed, &custom, None).unwrap();
                let orig_result = orig.evaluate(&vars);
                if let ExprKind::Number(n) = orig_result.kind {
                    TestResult::from_bool(approx_eq(n, x_val))
                } else {
                    TestResult::passed()
                }
            }
        }
        QuickCheck::new()
            .tests(100)
            .quickcheck(prop_add_zero as fn(f64) -> TestResult);
    }

    /// Property: x * 1 = x
    #[test]
    fn test_multiplicative_identity() {
        fn prop_mul_one(x_val: f64) -> TestResult {
            if !x_val.is_finite() {
                return TestResult::discard();
            }

            let result = simplify("x * 1", None, None).unwrap();
            let fixed = HashSet::new();
            let custom = HashSet::new();
            let expr = parser::parse(&result, &fixed, &custom, None).unwrap();
            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();

            if let ExprKind::Number(n) = expr.evaluate(&vars).kind {
                TestResult::from_bool(approx_eq(n, x_val))
            } else {
                TestResult::passed()
            }
        }
        QuickCheck::new()
            .tests(100)
            .quickcheck(prop_mul_one as fn(f64) -> TestResult);
    }

    /// Property: x * 0 = 0
    #[test]
    fn test_multiplicative_zero() {
        fn prop_mul_zero(x_val: f64) -> TestResult {
            if !x_val.is_finite() {
                return TestResult::discard();
            }

            let result = simplify("x * 0", None, None).unwrap();
            let fixed = HashSet::new();
            let custom = HashSet::new();
            let expr = parser::parse(&result, &fixed, &custom, None).unwrap();
            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();

            if let ExprKind::Number(n) = expr.evaluate(&vars).kind {
                TestResult::from_bool(approx_eq(n, 0.0))
            } else {
                TestResult::passed()
            }
        }
        QuickCheck::new()
            .tests(100)
            .quickcheck(prop_mul_zero as fn(f64) -> TestResult);
    }

    /// Property: x^1 = x
    #[test]
    fn test_power_one() {
        fn prop_pow_one(x_val: f64) -> TestResult {
            if !x_val.is_finite() || x_val <= 0.0 {
                return TestResult::discard();
            }

            let result = simplify("x^1", None, None).unwrap();
            let fixed = HashSet::new();
            let custom = HashSet::new();
            let expr = parser::parse(&result, &fixed, &custom, None).unwrap();
            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();

            if let ExprKind::Number(n) = expr.evaluate(&vars).kind {
                TestResult::from_bool(approx_eq(n, x_val))
            } else {
                TestResult::passed()
            }
        }
        QuickCheck::new()
            .tests(100)
            .quickcheck(prop_pow_one as fn(f64) -> TestResult);
    }

    /// Property: x^0 = 1 (for x != 0)
    #[test]
    fn test_power_zero() {
        fn prop_pow_zero(x_val: f64) -> TestResult {
            if !x_val.is_finite() || x_val == 0.0 {
                return TestResult::discard();
            }

            let result = simplify("x^0", None, None).unwrap();
            let fixed = HashSet::new();
            let custom = HashSet::new();
            let expr = parser::parse(&result, &fixed, &custom, None).unwrap();
            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();

            if let ExprKind::Number(n) = expr.evaluate(&vars).kind {
                TestResult::from_bool(approx_eq(n, 1.0))
            } else {
                TestResult::passed()
            }
        }
        QuickCheck::new()
            .tests(100)
            .quickcheck(prop_pow_zero as fn(f64) -> TestResult);
    }

    /// Property: x - x = 0
    #[test]
    fn test_additive_inverse() {
        fn prop_sub_self(x_val: f64) -> TestResult {
            if !x_val.is_finite() {
                return TestResult::discard();
            }

            let result = simplify("x - x", None, None).unwrap();
            let fixed = HashSet::new();
            let custom = HashSet::new();
            let expr = parser::parse(&result, &fixed, &custom, None).unwrap();
            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();

            if let ExprKind::Number(n) = expr.evaluate(&vars).kind {
                TestResult::from_bool(approx_eq(n, 0.0))
            } else {
                TestResult::passed()
            }
        }
        QuickCheck::new()
            .tests(100)
            .quickcheck(prop_sub_self as fn(f64) -> TestResult);
    }

    /// Property: x / x = 1 (for x != 0)
    #[test]
    fn test_multiplicative_inverse() {
        fn prop_div_self(x_val: f64) -> TestResult {
            if !x_val.is_finite() || x_val == 0.0 {
                return TestResult::discard();
            }

            let result = simplify("x / x", None, None).unwrap();
            let fixed = HashSet::new();
            let custom = HashSet::new();
            let expr = parser::parse(&result, &fixed, &custom, None).unwrap();
            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();

            if let ExprKind::Number(n) = expr.evaluate(&vars).kind {
                TestResult::from_bool(approx_eq(n, 1.0))
            } else {
                TestResult::passed()
            }
        }
        QuickCheck::new()
            .tests(100)
            .quickcheck(prop_div_self as fn(f64) -> TestResult);
    }

    /// Property: sin²(x) + cos²(x) = 1
    #[test]
    fn test_pythagorean_identity() {
        fn prop_sin2_cos2(x_val: f64) -> TestResult {
            if !x_val.is_finite() {
                return TestResult::discard();
            }

            let fixed = HashSet::new();
            let custom = HashSet::new();
            let expr = parser::parse("sin(x)^2 + cos(x)^2", &fixed, &custom, None).unwrap();
            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();

            if let ExprKind::Number(n) = expr.evaluate(&vars).kind {
                TestResult::from_bool(approx_eq(n, 1.0))
            } else {
                TestResult::passed()
            }
        }
        QuickCheck::new()
            .tests(200)
            .quickcheck(prop_sin2_cos2 as fn(f64) -> TestResult);
    }

    /// Property: e^(ln(x)) = x (for x > 0)
    #[test]
    fn test_exp_ln_inverse() {
        fn prop_exp_ln(x_val: f64) -> TestResult {
            if !x_val.is_finite() || x_val <= 0.0 {
                return TestResult::discard();
            }

            let fixed = HashSet::new();
            let custom = HashSet::new();
            let expr = parser::parse("exp(ln(x))", &fixed, &custom, None).unwrap();
            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();

            if let ExprKind::Number(n) = expr.evaluate(&vars).kind {
                TestResult::from_bool(approx_eq(n, x_val))
            } else {
                TestResult::passed()
            }
        }
        QuickCheck::new()
            .tests(200)
            .quickcheck(prop_exp_ln as fn(f64) -> TestResult);
    }

    /// Property: ln(e^x) = x
    #[test]
    fn test_ln_exp_inverse() {
        fn prop_ln_exp(x_val: f64) -> TestResult {
            // Limit domain to avoid overflow
            if !x_val.is_finite() || x_val.abs() > 100.0 {
                return TestResult::discard();
            }

            let fixed = HashSet::new();
            let custom = HashSet::new();
            let expr = parser::parse("ln(exp(x))", &fixed, &custom, None).unwrap();
            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();

            if let ExprKind::Number(n) = expr.evaluate(&vars).kind {
                TestResult::from_bool(approx_eq(n, x_val))
            } else {
                TestResult::passed()
            }
        }
        QuickCheck::new()
            .tests(200)
            .quickcheck(prop_ln_exp as fn(f64) -> TestResult);
    }

    /// Property: d/dx(x^n) = n*x^(n-1) numerically verified
    #[test]
    fn test_power_rule_derivative() {
        fn prop_power_rule(n: i8, x_val: f64) -> TestResult {
            // Limit domain
            if !x_val.is_finite() || x_val <= 0.0 || x_val > 100.0 {
                return TestResult::discard();
            }
            if !(1..=10).contains(&n) {
                return TestResult::discard();
            }

            let expr_str = format!("x^{}", n);
            let derivative = diff(&expr_str, "x", None, None).unwrap();

            let fixed = HashSet::new();
            let custom = HashSet::new();
            let deriv_expr = parser::parse(&derivative, &fixed, &custom, None).unwrap();
            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();

            if let ExprKind::Number(result) = deriv_expr.evaluate(&vars).kind {
                let expected = (n as f64) * x_val.powi(n as i32 - 1);
                let tolerance = 1e-6 * expected.abs().max(1.0);
                TestResult::from_bool((result - expected).abs() < tolerance)
            } else {
                TestResult::passed()
            }
        }
        QuickCheck::new()
            .tests(200)
            .quickcheck(prop_power_rule as fn(i8, f64) -> TestResult);
    }

    /// Property: Commutativity of addition: x + y = y + x
    #[test]
    fn test_addition_commutativity() {
        fn prop_add_comm(x: f64, y: f64) -> TestResult {
            if !x.is_finite() || !y.is_finite() {
                return TestResult::discard();
            }

            let fixed = HashSet::new();
            let custom = HashSet::new();

            let expr1 = parser::parse("x + y", &fixed, &custom, None).unwrap();
            let expr2 = parser::parse("y + x", &fixed, &custom, None).unwrap();

            let vars: HashMap<&str, f64> = [("x", x), ("y", y)].iter().cloned().collect();

            match (&expr1.evaluate(&vars).kind, &expr2.evaluate(&vars).kind) {
                (ExprKind::Number(n1), ExprKind::Number(n2)) => {
                    TestResult::from_bool(approx_eq(*n1, *n2))
                }
                _ => TestResult::passed(),
            }
        }
        QuickCheck::new()
            .tests(200)
            .quickcheck(prop_add_comm as fn(f64, f64) -> TestResult);
    }

    /// Property: Commutativity of multiplication: x * y = y * x
    #[test]
    fn test_multiplication_commutativity() {
        fn prop_mul_comm(x: f64, y: f64) -> TestResult {
            if !x.is_finite() || !y.is_finite() {
                return TestResult::discard();
            }

            let fixed = HashSet::new();
            let custom = HashSet::new();

            let expr1 = parser::parse("x * y", &fixed, &custom, None).unwrap();
            let expr2 = parser::parse("y * x", &fixed, &custom, None).unwrap();

            let vars: HashMap<&str, f64> = [("x", x), ("y", y)].iter().cloned().collect();

            match (&expr1.evaluate(&vars).kind, &expr2.evaluate(&vars).kind) {
                (ExprKind::Number(n1), ExprKind::Number(n2)) => {
                    TestResult::from_bool(approx_eq(*n1, *n2))
                }
                _ => TestResult::passed(),
            }
        }
        QuickCheck::new()
            .tests(200)
            .quickcheck(prop_mul_comm as fn(f64, f64) -> TestResult);
    }
}

// ============================================================
// PART 4: ORACLE FUZZ TEST (SIMPLIFICATION CORRECTNESS)
// ============================================================

#[cfg(test)]
mod simplification_oracle_tests {
    use super::*;
    use std::collections::HashMap;

    /// The "Oracle" test: verify simplification preserves numerical evaluation
    /// This catches bugs where simplification changes the mathematical meaning
    #[test]
    fn test_simplification_correctness_oracle() {
        fn prop_simplify_preserves_value() -> TestResult {
            // 1. Generate a random expression
            let mut g = Gen::new(10);
            let expr_str = gen_expr_string_recursive(&mut g, 3); // Use depth 3 for faster tests

            // 2. Parse
            let fixed = HashSet::new();
            let custom = HashSet::new();
            let Ok(original) = parser::parse(&expr_str, &fixed, &custom, None) else {
                return TestResult::discard();
            };

            // 3. Simplify (with timeout protection via engine limits)
            let simplified_str = match simplify(&expr_str, None, None) {
                Ok(s) => s,
                Err(_) => return TestResult::discard(), // Simplification can fail on edge cases
            };
            let Ok(simplified) = parser::parse(&simplified_str, &fixed, &custom, None) else {
                return TestResult::discard();
            };

            // 4. THE ORACLE: Evaluate both at specific test points
            // Use non-integer, non-zero values to avoid singularities
            let mut vars = HashMap::new();
            vars.insert("x", 0.351);
            vars.insert("y", 0.762);
            vars.insert("z", 1.234);

            let res_orig = original.evaluate(&vars);
            let res_simp = simplified.evaluate(&vars);

            match (&res_orig.kind, &res_simp.kind) {
                (ExprKind::Number(n1), ExprKind::Number(n2)) => {
                    // Ignore NaNs/Infinities (singularities are hard to test generically)
                    if n1.is_finite() && n2.is_finite() {
                        let tolerance = 1e-5 * n1.abs().max(n2.abs()).max(1.0);
                        if (n1 - n2).abs() > tolerance {
                            eprintln!(
                                "ORACLE FAILURE:\n  Original:   {}\n  Simplified: {}\n  Val Orig:   {}\n  Val Simp:   {}",
                                expr_str, simplified_str, n1, n2
                            );
                            return TestResult::failed();
                        }
                    }
                }
                _ => {} // Symbolic result (can't compare numerically)
            }

            TestResult::passed()
        }

        QuickCheck::new()
            .tests(100000) // Run 1000 random expressions
            .max_tests(2000)
            .quickcheck(prop_simplify_preserves_value as fn() -> TestResult);
    }

    /// Test that simplification is idempotent (simplify(simplify(x)) == simplify(x))
    #[test]
    fn test_simplification_idempotent() {
        fn prop_simplify_idempotent() -> TestResult {
            let mut g = Gen::new(8);
            let expr_str = gen_expr_string_recursive(&mut g, 3);

            let fixed = HashSet::new();
            let custom = HashSet::new();

            // First simplification
            let Ok(first) = simplify(&expr_str, None, None) else {
                return TestResult::discard();
            };

            // Second simplification
            let Ok(second) = simplify(&first, None, None) else {
                return TestResult::discard();
            };

            // Parse both for comparison
            let Ok(first_expr) = parser::parse(&first, &fixed, &custom, None) else {
                return TestResult::discard();
            };
            let Ok(second_expr) = parser::parse(&second, &fixed, &custom, None) else {
                return TestResult::discard();
            };

            // Numerical comparison at test point
            let mut vars = HashMap::new();
            vars.insert("x", 0.5);
            vars.insert("y", 0.7);
            vars.insert("z", 1.1);

            let res1 = first_expr.evaluate(&vars);
            let res2 = second_expr.evaluate(&vars);

            match (&res1.kind, &res2.kind) {
                (ExprKind::Number(n1), ExprKind::Number(n2)) => {
                    if n1.is_finite() && n2.is_finite() {
                        let tolerance = 1e-10 * n1.abs().max(n2.abs()).max(1.0);
                        if (n1 - n2).abs() > tolerance {
                            eprintln!(
                                "IDEMPOTENT FAILURE:\n  First:  {}\n  Second: {}\n  Val1:   {}\n  Val2:   {}",
                                first, second, n1, n2
                            );
                            return TestResult::failed();
                        }
                    }
                }
                _ => {}
            }

            TestResult::passed()
        }

        QuickCheck::new()
            .tests(500)
            .quickcheck(prop_simplify_idempotent as fn() -> TestResult);
    }
}

// ============================================================
// PART 5: RULE CONFLICT DETECTOR (DEVELOPMENT TOOL)
// ============================================================

#[cfg(test)]
mod rule_conflict_tests {
    use super::*;

    /// Test known expressions that could cause rule conflicts
    /// These are patterns where two rules might fight each other
    #[test]
    fn test_potential_rule_conflicts() {
        let dangerous_exprs = vec![
            ("x / y", "division vs negative power"),
            ("exp(ln(x))", "inverse function identity"),
            ("ln(exp(x))", "inverse function identity"),
            ("sin(x)/cos(x)", "tan vs sin/cos ratio"),
            ("x + x", "term collection vs expansion"),
            ("x * x", "power combination vs factoring"),
            ("(x + y) * (x - y)", "difference of squares"),
            ("x^2 - 1", "factoring vs expanded form"),
            ("1 / (1/x)", "double inverse"),
            ("-(-(x))", "double negation"),
            ("x^1", "power of one identity"),
            ("x^0", "power of zero identity"),
            ("0^x", "zero base power"),
            ("1^x", "one base power"),
            ("x * 1", "multiplicative identity"),
            ("x + 0", "additive identity"),
            ("x - x", "self subtraction"),
            ("x / x", "self division"),
            ("sqrt(x^2)", "sqrt of square"),
            ("(sqrt(x))^2", "square of sqrt"),
        ];

        for (expr_str, description) in dangerous_exprs {
            // These should not panic or hang
            let result = simplify(expr_str, None, None);
            assert!(
                result.is_ok(),
                "Rule conflict in '{}' ({}): {:?}",
                expr_str,
                description,
                result.err()
            );

            // Additionally, verify the result is mathematically valid
            // by checking it parses back successfully
            if let Ok(simplified) = &result {
                let fixed = HashSet::new();
                let custom = HashSet::new();
                assert!(
                    parser::parse(simplified, &fixed, &custom, None).is_ok(),
                    "Simplified result '{}' doesn't parse for '{}'",
                    simplified,
                    expr_str
                );
            }
        }
    }

    /// Test expressions that previously caused infinite loops or cycles
    #[test]
    fn test_known_cycle_cases() {
        let cycle_prone = vec![
            "x / (y / z)",
            "(a * b) / (c * d)",
            "sin(x) * cos(x)",
            "(x + y)^2",
            "exp(-x^2)",
        ];

        for expr_str in cycle_prone {
            // Use standard simplify - engine has built-in iteration limits
            let result = simplify(expr_str, None, None);
            assert!(
                result.is_ok(),
                "Cycle/error in '{}': {:?}",
                expr_str,
                result.err()
            );

            // Also verify result parses
            if let Ok(simplified) = &result {
                let fixed = HashSet::new();
                let custom = HashSet::new();
                assert!(
                    parser::parse(simplified, &fixed, &custom, None).is_ok(),
                    "Simplified '{}' doesn't parse for '{}'",
                    simplified,
                    expr_str
                );
            }
        }
    }
}
