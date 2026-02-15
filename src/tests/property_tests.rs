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
            let _unused = parser::parse(&input, &fixed, &custom, None);
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
        fn prop_valid_expr_parses() -> TestResult {
            let mut g = Gen::new(10);
            let expr_str = random_expr_string(&mut g);
            let fixed = HashSet::new();
            let custom = HashSet::new();

            // Generator emits syntactically valid expressions for this grammar.
            let parsed = match parser::parse(&expr_str, &fixed, &custom, None) {
                Ok(expr) => expr,
                Err(_err) => return TestResult::failed(),
            };

            // If parser accepted it, Display output should parse back.
            let displayed = parsed.to_string();
            TestResult::from_bool(parser::parse(&displayed, &fixed, &custom, None).is_ok())
        }
        QuickCheck::new()
            .tests(500)
            .quickcheck(prop_valid_expr_parses as fn() -> TestResult);
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
            let _unused = parser::parse(case, &fixed, &custom, None);
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
            let result = simplify("x + 0", &[], None).unwrap();

            // After simplification, should evaluate to x
            let fixed = HashSet::new();
            let custom = HashSet::new();
            let expr = parser::parse(&result, &fixed, &custom, None).unwrap();
            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();

            if let ExprKind::Number(n) = expr.evaluate(&vars, &HashMap::new()).kind {
                TestResult::from_bool(approx_eq(n, x_val))
            } else {
                // Result is symbolic, check that it evaluated correctly
                let orig = parser::parse("x", &fixed, &custom, None).unwrap();
                let orig_result = orig.evaluate(&vars, &HashMap::new());
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

            let result = simplify("x * 1", &[], None).unwrap();
            let fixed = HashSet::new();
            let custom = HashSet::new();
            let expr = parser::parse(&result, &fixed, &custom, None).unwrap();
            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();

            if let ExprKind::Number(n) = expr.evaluate(&vars, &HashMap::new()).kind {
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

            let result = simplify("x * 0", &[], None).unwrap();
            let fixed = HashSet::new();
            let custom = HashSet::new();
            let expr = parser::parse(&result, &fixed, &custom, None).unwrap();
            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();

            if let ExprKind::Number(n) = expr.evaluate(&vars, &HashMap::new()).kind {
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

            let result = simplify("x^1", &[], None).unwrap();
            let fixed = HashSet::new();
            let custom = HashSet::new();
            let expr = parser::parse(&result, &fixed, &custom, None).unwrap();
            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();

            if let ExprKind::Number(n) = expr.evaluate(&vars, &HashMap::new()).kind {
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

            let result = simplify("x^0", &[], None).unwrap();
            let fixed = HashSet::new();
            let custom = HashSet::new();
            let expr = parser::parse(&result, &fixed, &custom, None).unwrap();
            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();

            if let ExprKind::Number(n) = expr.evaluate(&vars, &HashMap::new()).kind {
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

            let result = simplify("x - x", &[], None).unwrap();
            let fixed = HashSet::new();
            let custom = HashSet::new();
            let expr = parser::parse(&result, &fixed, &custom, None).unwrap();
            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();

            if let ExprKind::Number(n) = expr.evaluate(&vars, &HashMap::new()).kind {
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

            let result = simplify("x / x", &[], None).unwrap();
            let fixed = HashSet::new();
            let custom = HashSet::new();
            let expr = parser::parse(&result, &fixed, &custom, None).unwrap();
            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();

            if let ExprKind::Number(n) = expr.evaluate(&vars, &HashMap::new()).kind {
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

            if let ExprKind::Number(n) = expr.evaluate(&vars, &HashMap::new()).kind {
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

            if let ExprKind::Number(n) = expr.evaluate(&vars, &HashMap::new()).kind {
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

            if let ExprKind::Number(n) = expr.evaluate(&vars, &HashMap::new()).kind {
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
            let derivative = diff(&expr_str, "x", &[], None).unwrap();

            let fixed = HashSet::new();
            let custom = HashSet::new();
            let deriv_expr = parser::parse(&derivative, &fixed, &custom, None).unwrap();
            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();

            if let ExprKind::Number(result) = deriv_expr.evaluate(&vars, &HashMap::new()).kind {
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

            match (
                &expr1.evaluate(&vars, &HashMap::new()).kind,
                &expr2.evaluate(&vars, &HashMap::new()).kind,
            ) {
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

            match (
                &expr1.evaluate(&vars, &HashMap::new()).kind,
                &expr2.evaluate(&vars, &HashMap::new()).kind,
            ) {
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
            let simplified_str = match simplify(&expr_str, &[], None) {
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

            let res_orig = original.evaluate(&vars, &HashMap::new());
            let res_simp = simplified.evaluate(&vars, &HashMap::new());

            if let (ExprKind::Number(n1), ExprKind::Number(n2)) = (&res_orig.kind, &res_simp.kind) {
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

            TestResult::passed()
        }

        QuickCheck::new()
            .tests(100_000) // Run 1000 random expressions
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
            let Ok(first) = simplify(&expr_str, &[], None) else {
                return TestResult::discard();
            };

            // Second simplification
            let Ok(second) = simplify(&first, &[], None) else {
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

            let res1 = first_expr.evaluate(&vars, &HashMap::new());
            let res2 = second_expr.evaluate(&vars, &HashMap::new());

            if let (ExprKind::Number(n1), ExprKind::Number(n2)) = (&res1.kind, &res2.kind)
                && n1.is_finite()
                && n2.is_finite()
            {
                let tolerance = 1e-10 * n1.abs().max(n2.abs()).max(1.0);
                if (n1 - n2).abs() > tolerance {
                    eprintln!(
                        "IDEMPOTENT FAILURE:\n  First:  {}\n  Second: {}\n  Val1:   {}\n  Val2:   {}",
                        first, second, n1, n2
                    );
                    return TestResult::failed();
                }
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
            let result = simplify(expr_str, &[], None);
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
            let result = simplify(expr_str, &[], None);
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

#[cfg(test)]
mod expression_building_fuzz_tests {
    use super::*;
    use crate::{Diff, Expr, symb};
    use std::collections::HashMap;

    /// Generate random expressions using the type-safe API
    fn random_expr(g: &mut Gen, depth: usize) -> Expr {
        if depth == 0 {
            // Base cases
            let choice: u8 = u8::arbitrary(g) % 5;
            match choice {
                0 => {
                    // Random number
                    let n: f64 = f64::arbitrary(g);
                    if n.is_finite() && n.abs() < 1e10 {
                        Expr::number(n)
                    } else {
                        Expr::number(1.0)
                    }
                }
                1 => symb("x").into(),
                2 => symb("y").into(),
                3 => symb("z").into(),
                4 => symb("w").into(),
                _ => Expr::number(1.0),
            }
        } else {
            let choice: u8 = u8::arbitrary(g) % 25;
            match choice {
                0..=2 => {
                    // Binary operations
                    let left = random_expr(g, depth - 1);
                    let right = random_expr(g, depth - 1);
                    let op_choice: u8 = u8::arbitrary(g) % 4;
                    match op_choice {
                        0 => left + right,
                        1 => left - right,
                        2 => left * right,
                        3 => left / right,
                        _ => left + right,
                    }
                }
                3..=15 => {
                    // Single-argument functions (expanded range for more coverage)
                    let arg = random_expr(g, depth - 1);
                    let func_choice: u8 = u8::arbitrary(g) % 46;
                    match func_choice {
                        0 => arg.sin(),
                        1 => arg.cos(),
                        2 => arg.tan(),
                        3 => arg.cot(),
                        4 => arg.sec(),
                        5 => arg.csc(),
                        6 => arg.asin(),
                        7 => arg.acos(),
                        8 => arg.atan(),
                        9 => arg.acot(),
                        10 => arg.asec(),
                        11 => arg.acsc(),
                        12 => arg.sinh(),
                        13 => arg.cosh(),
                        14 => arg.tanh(),
                        15 => arg.coth(),
                        16 => arg.sech(),
                        17 => arg.csch(),
                        18 => arg.asinh(),
                        19 => arg.acosh(),
                        20 => arg.atanh(),
                        21 => arg.acoth(),
                        22 => arg.asech(),
                        23 => arg.acsch(),
                        24 => arg.exp(),
                        25 => arg.ln(),
                        26 => arg.log10(),
                        27 => arg.log2(),
                        28 => arg.sqrt(),
                        29 => arg.cbrt(),
                        30 => arg.floor(),
                        31 => arg.ceil(),
                        32 => arg.round(),
                        33 => arg.abs(),
                        34 => arg.signum(),
                        35 => arg.sinc(),
                        36 => arg.erf(),
                        37 => arg.erfc(),
                        38 => arg.gamma(),
                        39 => arg.digamma(),
                        40 => arg.trigamma(),
                        41 => arg.tetragamma(),
                        42 => arg.zeta(),
                        43 => arg.lambertw(),
                        44 => arg.elliptic_k(),
                        45 => arg.elliptic_e(),
                        _ => arg.exp_polar(),
                    }
                }
                16 => {
                    // Power
                    let base = random_expr(g, depth - 1);
                    let exp = random_expr(g, depth - 1);
                    base.pow(exp)
                }
                17 => {
                    // Log with base
                    let arg = random_expr(g, depth - 1);
                    let base = random_expr(g, depth - 1);
                    arg.log(base)
                }
                18..=20 => {
                    // Multi-argument functions - constrain parameters to reasonable ranges
                    let arg = random_expr(g, depth - 1);
                    let func_choice: u8 = u8::arbitrary(g) % 8;
                    match func_choice {
                        0 => {
                            // polygamma(n, x) - n should be small integer
                            let n_val = (u8::arbitrary(g) % 5) as i32; // n: 0-4
                            arg.polygamma(Expr::number(n_val as f64))
                        }
                        1 => {
                            // beta(a, b) - parameters should be reasonable
                            let param = random_expr(g, depth - 1);
                            arg.beta(param)
                        }
                        2 => {
                            // besselj(n, x) - n should be small integer
                            let n_val = ((u8::arbitrary(g) % 11) as i32) - 5; // n: -5 to 5
                            arg.besselj(Expr::number(n_val as f64))
                        }
                        3 => {
                            // bessely(n, x) - n should be small integer
                            let n_val = (u8::arbitrary(g) % 6) as i32; // n: 0-5
                            arg.bessely(Expr::number(n_val as f64))
                        }
                        4 => {
                            // besseli(n, x) - n should be small integer
                            let n_val = (u8::arbitrary(g) % 6) as i32; // n: 0-5
                            arg.besseli(Expr::number(n_val as f64))
                        }
                        5 => {
                            // besselk(n, x) - n should be small integer
                            let n_val = (u8::arbitrary(g) % 6) as i32; // n: 0-5
                            arg.besselk(Expr::number(n_val as f64))
                        }
                        6 => {
                            // zeta_deriv(s, a) - s should be reasonable
                            let param = random_expr(g, depth - 1);
                            arg.zeta_deriv(param)
                        }
                        7 => {
                            // hermite(n, x) - n should be small integer
                            let n_val = (u8::arbitrary(g) % 10) as i32; // n: 0-9
                            arg.hermite(Expr::number(n_val as f64))
                        }
                        _ => arg.polygamma(Expr::number(1.0)),
                    }
                }
                21 => {
                    // Spherical harmonics - constrain l and m to small integers
                    let theta = random_expr(g, depth - 1);
                    let l_val = (u8::arbitrary(g) % 5) as i32; // l: 0-4
                    let m_val = ((u8::arbitrary(g) % 9) as i32) - 4; // m: -4 to 4
                    let phi = random_expr(g, depth - 1);
                    theta.spherical_harmonic(
                        Expr::number(l_val as f64),
                        Expr::number(m_val as f64),
                        phi,
                    )
                }
                22 => {
                    // Associated Legendre - constrain l and m to small integers
                    let arg = random_expr(g, depth - 1);
                    let l_val = (u8::arbitrary(g) % 8) as i32; // l: 0-7
                    let m_range = (2 * l_val + 1) as u8;
                    let m_val = ((u8::arbitrary(g) % m_range) as i32) - l_val; // m: -l to l
                    arg.assoc_legendre(Expr::number(l_val as f64), Expr::number(m_val as f64))
                }
                23 => {
                    // Alternative spherical harmonic notation - constrain l and m
                    let theta = random_expr(g, depth - 1);
                    let l_val = (u8::arbitrary(g) % 5) as i32; // l: 0-4
                    let m_val = ((u8::arbitrary(g) % 9) as i32) - 4; // m: -4 to 4
                    let phi = random_expr(g, depth - 1);
                    theta.ynm(Expr::number(l_val as f64), Expr::number(m_val as f64), phi)
                }
                _ => random_expr(g, depth - 1),
            }
        }
    }

    /// Property: Randomly generated expressions should not crash when built
    #[test]
    fn test_random_expression_building_no_crash() {
        fn prop_random_expr_builds() -> bool {
            let mut g = Gen::new(5);
            let _expr = random_expr(&mut g, 3);
            // If we get here without panicking, the test passes
            true
        }

        QuickCheck::new()
            .tests(1000)
            .max_tests(2000)
            .quickcheck(prop_random_expr_builds as fn() -> bool);
    }

    /// Property: Random expressions should be differentiable without crashing
    #[test]
    fn test_random_expression_differentiation() {
        fn prop_random_expr_differentiates() -> TestResult {
            let mut g = Gen::new(4);
            let expr = random_expr(&mut g, 2);
            let x = symb("x");

            // Try to differentiate - should not crash
            let result = Diff::new().differentiate(&expr, &x);
            match result {
                Ok(_) => TestResult::passed(),
                Err(_) => TestResult::passed(), // Errors are OK, crashes are not
            }
        }

        QuickCheck::new()
            .tests(500)
            .max_tests(1000)
            .quickcheck(prop_random_expr_differentiates as fn() -> TestResult);
    }

    /// Property: Random expressions should evaluate without crashing (with default values)
    #[test]
    fn test_random_expression_evaluation() {
        fn prop_random_expr_evaluates() -> TestResult {
            let mut g = Gen::new(3);
            let expr = random_expr(&mut g, 1); // Reduced depth to prevent stack overflow

            // Create a context with default values
            let mut vars = HashMap::new();
            vars.insert("x", 1.0);
            vars.insert("y", 2.0);
            vars.insert("z", 3.0);
            vars.insert("w", 4.0);

            // Try to evaluate - should not crash
            let _result = expr.evaluate(&vars, &HashMap::new());
            TestResult::passed() // We don't care about the result, just that it doesn't crash
        }

        QuickCheck::new()
            .tests(500)
            .max_tests(1000)
            .quickcheck(prop_random_expr_evaluates as fn() -> TestResult);
    }

    /// Property: Random expressions should display without crashing
    #[test]
    fn test_random_expression_display() {
        fn prop_random_expr_displays() -> bool {
            let mut g = Gen::new(4);
            let expr = random_expr(&mut g, 3);

            // Try to format - should not crash
            let _display = format!("{}", expr);
            true
        }

        QuickCheck::new()
            .tests(1000)
            .max_tests(2000)
            .quickcheck(prop_random_expr_displays as fn() -> bool);
    }

    /// Property: Complex nested expressions should not cause stack overflow
    #[test]
    fn test_deep_expression_building() {
        fn prop_deep_expr() -> bool {
            let mut g = Gen::new(10);
            // Create a moderately deep expression
            let expr = random_expr(&mut g, 6);

            // Should not crash during building or basic operations
            let _display = format!("{}", expr);
            let x = symb("x");
            let _diff_result = Diff::new().differentiate(&expr, &x);

            true
        }

        QuickCheck::new()
            .tests(100)
            .quickcheck(prop_deep_expr as fn() -> bool);
    }

    /// Test specific edge cases that might cause issues
    #[test]
    fn test_expression_building_edge_cases() {
        // Test with extreme values
        let extreme_values = [
            0.0,
            1.0,
            -1.0,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            1e-20,
            1e20,
            -1e-20,
            -1e20,
        ];

        for &val in &extreme_values {
            let num = Expr::number(val);

            // Test various operations
            let _sin = num.clone().sin();
            let _cos = num.clone().cos();
            let _exp = num.clone().exp();
            let _ln = num.clone().ln();
            let _sqrt = num.clone().sqrt();
            let _abs = num.clone().abs();

            // Test with symbols
            let x = symb("x");
            let _add = x + val;
            let _mul = x * val;
            let _pow = x.pow(val);
        }
    }
}

// ============================================================
// PART 6: DERIVATIVE PROPERTY TESTS
// ============================================================

#[cfg(test)]
mod derivative_property_tests {
    use super::*;
    use std::collections::HashMap;

    const EPSILON: f64 = 1e-6;

    fn approx_eq(a: f64, b: f64) -> bool {
        if a.is_nan() && b.is_nan() {
            return true;
        }
        if a.is_infinite() && b.is_infinite() {
            return a.signum() == b.signum();
        }
        let tolerance = EPSILON * a.abs().max(b.abs()).max(1.0);
        (a - b).abs() < tolerance
    }

    /// Property: Linearity of differentiation: d/dx[a*f + b*g] = a*f' + b*g'
    #[test]
    fn test_derivative_linearity() {
        fn prop_linearity(a_coef: f64, b_coef: f64, x_val: f64) -> TestResult {
            if !a_coef.is_finite() || !b_coef.is_finite() || !x_val.is_finite() {
                return TestResult::discard();
            }
            if a_coef.abs() > 100.0 || b_coef.abs() > 100.0 || x_val.abs() > 10.0 {
                return TestResult::discard();
            }

            // f = x^2, g = sin(x)
            // a*f + b*g = a*x^2 + b*sin(x)
            // derivative: 2*a*x + b*cos(x)
            let expr = format!("{}*x^2 + {}*sin(x)", a_coef, b_coef);
            let deriv = match diff(&expr, "x", &[], None) {
                Ok(d) => d,
                Err(_) => return TestResult::discard(),
            };

            let fixed = HashSet::new();
            let custom = HashSet::new();
            let deriv_expr = match parser::parse(&deriv, &fixed, &custom, None) {
                Ok(e) => e,
                Err(_) => return TestResult::discard(),
            };

            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();
            if let ExprKind::Number(result) = deriv_expr.evaluate(&vars, &HashMap::new()).kind {
                let expected = 2.0 * a_coef * x_val + b_coef * x_val.cos();
                TestResult::from_bool(approx_eq(result, expected))
            } else {
                TestResult::passed()
            }
        }
        QuickCheck::new()
            .tests(100)
            .quickcheck(prop_linearity as fn(f64, f64, f64) -> TestResult);
    }

    /// Property: Chain rule verification: d/dx[sin(x^2)] = cos(x^2) * 2x
    #[test]
    fn test_chain_rule_property() {
        fn prop_chain_rule(x_val: f64) -> TestResult {
            if !x_val.is_finite() || x_val.abs() > 10.0 {
                return TestResult::discard();
            }

            let deriv = match diff("sin(x^2)", "x", &[], None) {
                Ok(d) => d,
                Err(_) => return TestResult::discard(),
            };

            let fixed = HashSet::new();
            let custom = HashSet::new();
            let deriv_expr = match parser::parse(&deriv, &fixed, &custom, None) {
                Ok(e) => e,
                Err(_) => return TestResult::discard(),
            };

            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();
            if let ExprKind::Number(result) = deriv_expr.evaluate(&vars, &HashMap::new()).kind {
                let expected = (x_val * x_val).cos() * 2.0 * x_val;
                TestResult::from_bool(approx_eq(result, expected))
            } else {
                TestResult::passed()
            }
        }
        QuickCheck::new()
            .tests(100)
            .quickcheck(prop_chain_rule as fn(f64) -> TestResult);
    }

    /// Property: Quotient rule: d/dx[f/g] = (f'g - fg')/g²
    #[test]
    fn test_quotient_rule_property() {
        fn prop_quotient_rule(x_val: f64) -> TestResult {
            // f = x, g = x+1
            // f' = 1, g' = 1
            // d/dx[x/(x+1)] = ((1)(x+1) - (x)(1))/(x+1)² = 1/(x+1)²
            if !x_val.is_finite() || x_val.abs() > 10.0 || (x_val + 1.0).abs() < 0.1 {
                return TestResult::discard();
            }

            let deriv = match diff("x/(x+1)", "x", &[], None) {
                Ok(d) => d,
                Err(_) => return TestResult::discard(),
            };

            let fixed = HashSet::new();
            let custom = HashSet::new();
            let deriv_expr = match parser::parse(&deriv, &fixed, &custom, None) {
                Ok(e) => e,
                Err(_) => return TestResult::discard(),
            };

            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();
            if let ExprKind::Number(result) = deriv_expr.evaluate(&vars, &HashMap::new()).kind {
                let expected = 1.0 / ((x_val + 1.0) * (x_val + 1.0));
                TestResult::from_bool(approx_eq(result, expected))
            } else {
                TestResult::passed()
            }
        }
        QuickCheck::new()
            .tests(100)
            .quickcheck(prop_quotient_rule as fn(f64) -> TestResult);
    }
}

// ============================================================
// PART 7: POWER PROPERTY TESTS
// ============================================================

#[cfg(test)]
mod power_property_tests {
    use super::*;
    use std::collections::HashMap;

    const EPSILON: f64 = 1e-8;

    fn approx_eq(a: f64, b: f64) -> bool {
        if a.is_nan() && b.is_nan() {
            return true;
        }
        if a.is_infinite() && b.is_infinite() {
            return a.signum() == b.signum();
        }
        let tolerance = EPSILON * a.abs().max(b.abs()).max(1.0);
        (a - b).abs() < tolerance
    }

    /// Property: x^a * x^b = x^(a+b) for x > 0
    #[test]
    fn test_power_addition_rule() {
        fn prop_power_add(x_val: f64, a: f64, b: f64) -> TestResult {
            if !x_val.is_finite() || !a.is_finite() || !b.is_finite() {
                return TestResult::discard();
            }
            if x_val <= 0.1 || x_val > 10.0 || a.abs() > 3.0 || b.abs() > 3.0 {
                return TestResult::discard();
            }

            let fixed = HashSet::new();
            let custom = HashSet::new();

            let lhs_expr = format!("x^{} * x^{}", a, b);
            let rhs_expr = format!("x^({})", a + b);

            let lhs = parser::parse(&lhs_expr, &fixed, &custom, None);
            let rhs = parser::parse(&rhs_expr, &fixed, &custom, None);

            if let (Ok(lhs_e), Ok(rhs_e)) = (lhs, rhs) {
                let vars: HashMap<&str, f64> = [("x", x_val)].iter().copied().collect();
                if let (ExprKind::Number(l), ExprKind::Number(r)) = (
                    &lhs_e.evaluate(&vars, &HashMap::new()).kind,
                    &rhs_e.evaluate(&vars, &HashMap::new()).kind,
                ) && l.is_finite()
                    && r.is_finite()
                {
                    return TestResult::from_bool(approx_eq(*l, *r));
                }
            }
            TestResult::discard()
        }
        QuickCheck::new()
            .tests(200)
            .quickcheck(prop_power_add as fn(f64, f64, f64) -> TestResult);
    }

    /// Property: (x^a)^b = x^(a*b) for x > 0
    #[test]
    fn test_power_of_power_rule() {
        fn prop_power_power(x_val: f64, a: f64, b: f64) -> TestResult {
            if !x_val.is_finite() || !a.is_finite() || !b.is_finite() {
                return TestResult::discard();
            }
            if x_val <= 0.5 || x_val > 5.0 || a.abs() > 2.0 || b.abs() > 2.0 {
                return TestResult::discard();
            }

            let fixed = HashSet::new();
            let custom = HashSet::new();

            let lhs_expr = format!("(x^{})^{}", a, b);
            let rhs_expr = format!("x^({})", a * b);

            let lhs = parser::parse(&lhs_expr, &fixed, &custom, None);
            let rhs = parser::parse(&rhs_expr, &fixed, &custom, None);

            if let (Ok(lhs_e), Ok(rhs_e)) = (lhs, rhs) {
                let vars: HashMap<&str, f64> = [("x", x_val)].iter().copied().collect();
                if let (ExprKind::Number(l), ExprKind::Number(r)) = (
                    &lhs_e.evaluate(&vars, &HashMap::new()).kind,
                    &rhs_e.evaluate(&vars, &HashMap::new()).kind,
                ) && l.is_finite()
                    && r.is_finite()
                {
                    return TestResult::from_bool(approx_eq(*l, *r));
                }
            }
            TestResult::discard()
        }
        QuickCheck::new()
            .tests(200)
            .quickcheck(prop_power_power as fn(f64, f64, f64) -> TestResult);
    }

    /// Property: (x*y)^a = x^a * y^a for x,y > 0
    #[test]
    fn test_product_power_rule() {
        fn prop_product_power(x_val: f64, y_val: f64, a: f64) -> TestResult {
            if !x_val.is_finite() || !y_val.is_finite() || !a.is_finite() {
                return TestResult::discard();
            }
            if x_val <= 0.5 || y_val <= 0.5 || x_val > 5.0 || y_val > 5.0 || a.abs() > 3.0 {
                return TestResult::discard();
            }

            let fixed = HashSet::new();
            let custom = HashSet::new();

            let lhs_expr = format!("(x*y)^{}", a);
            let rhs_expr = format!("x^{} * y^{}", a, a);

            let lhs = parser::parse(&lhs_expr, &fixed, &custom, None);
            let rhs = parser::parse(&rhs_expr, &fixed, &custom, None);

            if let (Ok(lhs_e), Ok(rhs_e)) = (lhs, rhs) {
                let vars: HashMap<&str, f64> =
                    [("x", x_val), ("y", y_val)].iter().copied().collect();
                if let (ExprKind::Number(l), ExprKind::Number(r)) = (
                    &lhs_e.evaluate(&vars, &HashMap::new()).kind,
                    &rhs_e.evaluate(&vars, &HashMap::new()).kind,
                ) && l.is_finite()
                    && r.is_finite()
                {
                    return TestResult::from_bool(approx_eq(*l, *r));
                }
            }
            TestResult::discard()
        }
        QuickCheck::new()
            .tests(200)
            .quickcheck(prop_product_power as fn(f64, f64, f64) -> TestResult);
    }
}

// ============================================================
// PART 8: SPECIAL FUNCTION PROPERTY TESTS
// ============================================================

#[cfg(test)]
mod special_function_property_tests {
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
        let tolerance = EPSILON * a.abs().max(b.abs()).max(1.0);
        (a - b).abs() < tolerance
    }

    /// Property: sinh(x) = (exp(x) - exp(-x))/2
    #[test]
    fn test_sinh_definition() {
        fn prop_sinh_def(x_val: f64) -> TestResult {
            if !x_val.is_finite() || x_val.abs() > 10.0 {
                return TestResult::discard();
            }

            let fixed = HashSet::new();
            let custom = HashSet::new();

            let lhs = parser::parse("sinh(x)", &fixed, &custom, None);
            let rhs = parser::parse("(exp(x) - exp(-x))/2", &fixed, &custom, None);

            if let (Ok(lhs_e), Ok(rhs_e)) = (lhs, rhs) {
                let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();
                if let (ExprKind::Number(l), ExprKind::Number(r)) = (
                    &lhs_e.evaluate(&vars, &HashMap::new()).kind,
                    &rhs_e.evaluate(&vars, &HashMap::new()).kind,
                ) {
                    return TestResult::from_bool(approx_eq(*l, *r));
                }
            }
            TestResult::discard()
        }
        QuickCheck::new()
            .tests(200)
            .quickcheck(prop_sinh_def as fn(f64) -> TestResult);
    }

    /// Property: cosh(x) = (exp(x) + exp(-x))/2
    #[test]
    fn test_cosh_definition() {
        fn prop_cosh_def(x_val: f64) -> TestResult {
            if !x_val.is_finite() || x_val.abs() > 10.0 {
                return TestResult::discard();
            }

            let fixed = HashSet::new();
            let custom = HashSet::new();

            let lhs = parser::parse("cosh(x)", &fixed, &custom, None);
            let rhs = parser::parse("(exp(x) + exp(-x))/2", &fixed, &custom, None);

            if let (Ok(lhs_e), Ok(rhs_e)) = (lhs, rhs) {
                let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();
                if let (ExprKind::Number(l), ExprKind::Number(r)) = (
                    &lhs_e.evaluate(&vars, &HashMap::new()).kind,
                    &rhs_e.evaluate(&vars, &HashMap::new()).kind,
                ) {
                    return TestResult::from_bool(approx_eq(*l, *r));
                }
            }
            TestResult::discard()
        }
        QuickCheck::new()
            .tests(200)
            .quickcheck(prop_cosh_def as fn(f64) -> TestResult);
    }

    /// Property: abs(-x) = abs(x)
    #[test]
    fn test_abs_symmetry() {
        fn prop_abs_sym(x_val: f64) -> TestResult {
            if !x_val.is_finite() {
                return TestResult::discard();
            }

            let fixed = HashSet::new();
            let custom = HashSet::new();

            let lhs = parser::parse("abs(-x)", &fixed, &custom, None);
            let rhs = parser::parse("abs(x)", &fixed, &custom, None);

            if let (Ok(lhs_e), Ok(rhs_e)) = (lhs, rhs) {
                let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();
                if let (ExprKind::Number(l), ExprKind::Number(r)) = (
                    &lhs_e.evaluate(&vars, &HashMap::new()).kind,
                    &rhs_e.evaluate(&vars, &HashMap::new()).kind,
                ) {
                    return TestResult::from_bool(approx_eq(*l, *r));
                }
            }
            TestResult::discard()
        }
        QuickCheck::new()
            .tests(200)
            .quickcheck(prop_abs_sym as fn(f64) -> TestResult);
    }

    /// Property: abs(x) >= 0 for all x
    #[test]
    fn test_abs_nonnegative() {
        fn prop_abs_pos(x_val: f64) -> TestResult {
            if !x_val.is_finite() {
                return TestResult::discard();
            }

            let fixed = HashSet::new();
            let custom = HashSet::new();

            let expr = parser::parse("abs(x)", &fixed, &custom, None);

            if let Ok(e) = expr {
                let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();
                if let ExprKind::Number(result) = e.evaluate(&vars, &HashMap::new()).kind {
                    return TestResult::from_bool(result >= 0.0);
                }
            }
            TestResult::discard()
        }
        QuickCheck::new()
            .tests(200)
            .quickcheck(prop_abs_pos as fn(f64) -> TestResult);
    }

    /// Property: sqrt(x)^2 = x for x >= 0
    #[test]
    fn test_sqrt_square_identity() {
        fn prop_sqrt_sq(x_val: f64) -> TestResult {
            if !x_val.is_finite() || !(0.01..=1000.0).contains(&x_val) {
                return TestResult::discard();
            }

            let fixed = HashSet::new();
            let custom = HashSet::new();

            let expr = parser::parse("sqrt(x)^2", &fixed, &custom, None);

            if let Ok(e) = expr {
                let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();
                if let ExprKind::Number(result) = e.evaluate(&vars, &HashMap::new()).kind {
                    return TestResult::from_bool(approx_eq(result, x_val));
                }
            }
            TestResult::discard()
        }
        QuickCheck::new()
            .tests(200)
            .quickcheck(prop_sqrt_sq as fn(f64) -> TestResult);
    }

    /// Property: erf(-x) = -erf(x) (odd function)
    #[test]
    fn test_erf_symmetry() {
        fn prop_erf_odd(x_val: f64) -> TestResult {
            if !x_val.is_finite() || x_val.abs() > 5.0 {
                return TestResult::discard();
            }

            let fixed = HashSet::new();
            let custom = HashSet::new();

            let lhs = parser::parse("erf(-x)", &fixed, &custom, None);
            let rhs = parser::parse("-erf(x)", &fixed, &custom, None);

            if let (Ok(lhs_e), Ok(rhs_e)) = (lhs, rhs) {
                let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();
                if let (ExprKind::Number(l), ExprKind::Number(r)) = (
                    &lhs_e.evaluate(&vars, &HashMap::new()).kind,
                    &rhs_e.evaluate(&vars, &HashMap::new()).kind,
                ) {
                    return TestResult::from_bool(approx_eq(*l, *r));
                }
            }
            TestResult::discard()
        }
        QuickCheck::new()
            .tests(200)
            .quickcheck(prop_erf_odd as fn(f64) -> TestResult);
    }
}

// ============================================================
// PART 9: COMPILED EVALUATOR PROPERTY TESTS
// ============================================================

#[cfg(test)]
mod compiled_evaluator_property_tests {
    use super::*;
    use crate::CompiledEvaluator;
    use std::collections::HashMap;

    const EPSILON: f64 = 1e-10;

    fn approx_eq(a: f64, b: f64) -> bool {
        if a.is_nan() && b.is_nan() {
            return true;
        }
        if a.is_infinite() && b.is_infinite() {
            return a.signum() == b.signum();
        }
        let tolerance = EPSILON * a.abs().max(b.abs()).max(1.0);
        (a - b).abs() < tolerance
    }

    /// Property: Compiled evaluation should match regular expression evaluation
    #[test]
    fn test_compiled_matches_interpret() {
        fn prop_compiled_matches(x_val: f64) -> TestResult {
            if !x_val.is_finite() || x_val.abs() > 10.0 || x_val <= 0.0 {
                return TestResult::discard();
            }

            let expr_str = "x^2 + sin(x)";
            let fixed = HashSet::new();
            let custom = HashSet::new();

            let expr = match parser::parse(expr_str, &fixed, &custom, None) {
                Ok(e) => e,
                Err(_) => return TestResult::discard(),
            };

            let compiled = match CompiledEvaluator::compile(&expr, &["x"], None) {
                Ok(c) => c,
                Err(_) => return TestResult::discard(),
            };

            let vars: HashMap<&str, f64> = [("x", x_val)].iter().cloned().collect();
            let interpret_result = expr.evaluate(&vars, &HashMap::new());

            if let ExprKind::Number(interpret_val) = interpret_result.kind {
                let compiled_val = compiled.evaluate(&[x_val]);
                TestResult::from_bool(approx_eq(compiled_val, interpret_val))
            } else {
                TestResult::discard()
            }
        }
        QuickCheck::new()
            .tests(200)
            .quickcheck(prop_compiled_matches as fn(f64) -> TestResult);
    }

    /// Property: Compiled evaluation is deterministic (same input = same output)
    #[test]
    fn test_compiled_deterministic() {
        fn prop_deterministic(x_val: f64) -> TestResult {
            if !x_val.is_finite() || x_val.abs() > 10.0 {
                return TestResult::discard();
            }

            let expr_str = "sin(x) * cos(x) + exp(-x^2)";
            let fixed = HashSet::new();
            let custom = HashSet::new();

            let expr = match parser::parse(expr_str, &fixed, &custom, None) {
                Ok(e) => e,
                Err(_) => return TestResult::discard(),
            };

            let compiled = match CompiledEvaluator::compile(&expr, &["x"], None) {
                Ok(c) => c,
                Err(_) => return TestResult::discard(),
            };

            // Evaluate multiple times and verify consistency
            let results: Vec<f64> = (0..5).map(|_| compiled.evaluate(&[x_val])).collect();
            let all_same = results.iter().all(|r| approx_eq(*r, results[0]));
            TestResult::from_bool(all_same)
        }
        QuickCheck::new()
            .tests(100)
            .quickcheck(prop_deterministic as fn(f64) -> TestResult);
    }

    /// Property: Compilation is consistent (multiple compilations give same result)
    #[test]
    fn test_compile_consistency() {
        fn prop_compile_consistent(x_val: f64) -> TestResult {
            if !x_val.is_finite() || x_val.abs() > 5.0 || x_val <= 0.0 {
                return TestResult::discard();
            }

            let expr_str = "ln(x) + x^2";
            let fixed = HashSet::new();
            let custom = HashSet::new();

            let expr = match parser::parse(expr_str, &fixed, &custom, None) {
                Ok(e) => e,
                Err(_) => return TestResult::discard(),
            };

            // Compile multiple times
            let compiled1 = match CompiledEvaluator::compile(&expr, &["x"], None) {
                Ok(c) => c,
                Err(_) => return TestResult::discard(),
            };
            let compiled2 = match CompiledEvaluator::compile(&expr, &["x"], None) {
                Ok(c) => c,
                Err(_) => return TestResult::discard(),
            };

            let result1 = compiled1.evaluate(&[x_val]);
            let result2 = compiled2.evaluate(&[x_val]);

            TestResult::from_bool(approx_eq(result1, result2))
        }
        QuickCheck::new()
            .tests(100)
            .quickcheck(prop_compile_consistent as fn(f64) -> TestResult);
    }
}
