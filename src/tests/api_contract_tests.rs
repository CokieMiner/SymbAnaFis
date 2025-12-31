//! API Contract Tests
//!
//! Verify that API contracts and invariants hold for all public types.
//! These are not mathematical correctness tests, but API design tests.

use crate::parser::parse as parser_parse;
use crate::{Diff, Expr, ExprKind, Simplify, diff, symb};
use std::collections::{HashMap, HashSet};

// ============================================================================
// SYMBOL API CONTRACTS
// ============================================================================

#[test]
fn test_symbol_is_copy() {
    let x = symb("x_copy");
    let y = x; // Copy, not move
    let z = x; // Can copy again
    assert_eq!(x, y);
    assert_eq!(y, z);
}

#[test]
fn test_symbol_equality() {
    // Same name = same symbol
    let x1 = symb("x_eq1");
    let x2 = symb("x_eq1");
    assert_eq!(x1, x2);

    // Different name = different symbol
    let y = symb("y_eq");
    assert_ne!(x1, y);
}

#[test]
fn test_symbol_hash_consistency() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn hash<T: Hash>(t: &T) -> u64 {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }

    // Equal symbols must have equal hashes
    let x1 = symb("x_hash1");
    let x2 = symb("x_hash1");
    assert_eq!(hash(&x1), hash(&x2));
}

#[test]
fn test_symb_function_returns_same_symbol() {
    let x1 = symb("x_same");
    let x2 = symb("x_same");
    assert_eq!(x1, x2);
}

// ============================================================================
// EXPR API CONTRACTS
// ============================================================================

#[test]
fn test_expr_number_roundtrip() {
    let num = Expr::number(42.5);
    if let ExprKind::Number(n) = &num.kind {
        assert_eq!(*n, 42.5);
    } else {
        panic!("Expected Number");
    }
}

#[test]
fn test_expr_clone_equality() {
    let expr = parser_parse("x^2 + sin(x)", &HashSet::new(), &HashSet::new(), None).unwrap();
    let cloned = expr.clone();

    // Clone should evaluate to same values
    let vars: HashMap<&str, f64> = [("x", 2.0)].into_iter().collect();

    let v1 = match &expr.evaluate(&vars, &HashMap::new()).kind {
        ExprKind::Number(n) => *n,
        _ => panic!("Expected number"),
    };
    let v2 = match &cloned.evaluate(&vars, &HashMap::new()).kind {
        ExprKind::Number(n) => *n,
        _ => panic!("Expected number"),
    };
    assert_eq!(v1, v2);
}

#[test]
fn test_expr_display_parse_roundtrip() {
    let expressions = ["x + y", "x * y", "x^2", "sin(x)", "exp(x) + ln(x)"];

    for expr_str in expressions {
        let expr = parser_parse(expr_str, &HashSet::new(), &HashSet::new(), None).unwrap();
        let displayed = format!("{}", expr);
        let reparsed = parser_parse(&displayed, &HashSet::new(), &HashSet::new(), None).unwrap();

        // Should evaluate to same values
        let vars: HashMap<&str, f64> = [("x", 2.0), ("y", 3.0)].into_iter().collect();

        let v1 = match &expr.evaluate(&vars, &HashMap::new()).kind {
            ExprKind::Number(n) => *n,
            other => panic!("Expected number, got {:?}", other),
        };
        let v2 = match &reparsed.evaluate(&vars, &HashMap::new()).kind {
            ExprKind::Number(n) => *n,
            other => panic!("Expected number, got {:?}", other),
        };

        let diff_val = (v1 - v2).abs();
        assert!(
            diff_val < 1e-10,
            "Roundtrip failed for {}: {} vs {}",
            expr_str,
            v1,
            v2
        );
    }
}

// ============================================================================
// DIFF BUILDER API CONTRACTS
// ============================================================================

#[test]
fn test_diff_builder_basic() {
    let result = Diff::new().diff_str("x^3", "x", &[]).unwrap();
    assert!(result.contains("3") && result.contains("x"));
}

#[test]
fn test_diff_builder_order() {
    let x = symb("x_diff_order");
    let expr: Expr = x.pow(3.0);

    // Get second derivative by nesting
    let d1 = Diff::new().differentiate(&expr, &x).unwrap();
    let d2 = Diff::new().differentiate(&d1, &x).unwrap();

    // d²/dx²[x³] = 6x, at x=2: 12
    let vars: HashMap<&str, f64> = [("x_diff_order", 2.0)].into_iter().collect();
    let d2_val = match &d2.evaluate(&vars, &HashMap::new()).kind {
        ExprKind::Number(n) => *n,
        _ => panic!("Expected number"),
    };
    assert!((d2_val - 12.0).abs() < 1e-10);
}

#[test]
fn test_diff_function_vs_builder() {
    // String API
    let str_result = diff("x^2", "x", &[], None).unwrap();

    // Builder API
    let builder_result = Diff::new().diff_str("x^2", "x", &[]).unwrap();

    // Should evaluate to same
    let str_expr = parser_parse(&str_result, &HashSet::new(), &HashSet::new(), None).unwrap();
    let builder_expr =
        parser_parse(&builder_result, &HashSet::new(), &HashSet::new(), None).unwrap();

    let vars: HashMap<&str, f64> = [("x", 3.0)].into_iter().collect();

    let str_val = match &str_expr.evaluate(&vars, &HashMap::new()).kind {
        ExprKind::Number(n) => *n,
        _ => panic!("Expected number"),
    };
    let builder_val = match &builder_expr.evaluate(&vars, &HashMap::new()).kind {
        ExprKind::Number(n) => *n,
        _ => panic!("Expected number"),
    };

    assert!((str_val - builder_val).abs() < 1e-10);
}

// ============================================================================
// SIMPLIFY BUILDER API CONTRACTS
// ============================================================================

#[test]
fn test_simplify_builder_basic() {
    let expr = parser_parse("x + x", &HashSet::new(), &HashSet::new(), None).unwrap();
    let simplified = Simplify::new().simplify(&expr).unwrap();

    let vars: HashMap<&str, f64> = [("x", 5.0)].into_iter().collect();

    // Should evaluate to 10
    let val = match &simplified.evaluate(&vars, &HashMap::new()).kind {
        ExprKind::Number(n) => *n,
        _ => panic!("Expected number"),
    };
    assert!((val - 10.0).abs() < 1e-10);
}

#[test]
fn test_simplify_idempotence() {
    let expressions = ["x + x", "x * x", "sin(x)^2 + cos(x)^2"];

    for expr_str in expressions {
        let expr = parser_parse(expr_str, &HashSet::new(), &HashSet::new(), None).unwrap();
        let s1 = Simplify::new().simplify(&expr).unwrap();
        let s2 = Simplify::new().simplify(&s1).unwrap();

        // Simplifying twice should give same result
        let vars: HashMap<&str, f64> = [("x", 1.5)].into_iter().collect();

        let v1 = match &s1.evaluate(&vars, &HashMap::new()).kind {
            ExprKind::Number(n) => *n,
            _ => panic!("Expected number"),
        };
        let v2 = match &s2.evaluate(&vars, &HashMap::new()).kind {
            ExprKind::Number(n) => *n,
            _ => panic!("Expected number"),
        };

        assert!(
            (v1 - v2).abs() < 1e-10,
            "Idempotence failed for {}",
            expr_str
        );
    }
}

// ============================================================================
// PARSE API CONTRACTS
// ============================================================================

#[test]
fn test_parse_empty_rejects() {
    let result = parser_parse("", &HashSet::new(), &HashSet::new(), None);
    assert!(result.is_err());
}

#[test]
fn test_parse_invalid_rejects() {
    let invalid = ["(", ")", "++", "x +"];

    for expr in invalid {
        let result = parser_parse(expr, &HashSet::new(), &HashSet::new(), None);
        assert!(result.is_err(), "Should reject: {}", expr);
    }
}

#[test]
fn test_parse_accepts_valid() {
    let valid = ["0", "1", "x", "x + y", "sin(x)", "x^2", "-x", "(x + y) * z"];

    for expr in valid {
        let result = parser_parse(expr, &HashSet::new(), &HashSet::new(), None);
        assert!(result.is_ok(), "Should accept: {}", expr);
    }
}

// ============================================================================
// EVALUATION API CONTRACTS
// ============================================================================

#[test]
fn test_evaluate_with_all_variables() {
    let expr = parser_parse("x + y", &HashSet::new(), &HashSet::new(), None).unwrap();

    let vars: HashMap<&str, f64> = [("x", 1.0), ("y", 2.0)].into_iter().collect();
    let result = expr.evaluate(&vars, &HashMap::new());

    if let ExprKind::Number(n) = &result.kind {
        assert!((*n - 3.0).abs() < 1e-10);
    } else {
        panic!("Expected number");
    }
}

#[test]
fn test_evaluate_extra_variables_ok() {
    // Providing extra variables should be fine
    let expr = parser_parse("x", &HashSet::new(), &HashSet::new(), None).unwrap();

    let vars: HashMap<&str, f64> = [("x", 5.0), ("y", 999.0)].into_iter().collect();
    let result = expr.evaluate(&vars, &HashMap::new());

    if let ExprKind::Number(n) = &result.kind {
        assert!((*n - 5.0).abs() < 1e-10);
    } else {
        panic!("Expected number");
    }
}

// ============================================================================
// MATHEMATICAL OPERATOR CONTRACTS (Symbol methods)
// ============================================================================

#[test]
fn test_symbol_arithmetic_ops() {
    let x = symb("x_arith");
    let y = symb("y_arith");

    // Test all basic operations work
    let _sum = x + y;
    let _diff = x - y;
    let _prod = x * y;
    let _quot = x / y;
    let _pow = x.pow(2.0);
    let _neg = -x;
    // Should all be valid expressions
}

#[test]
fn test_symbol_math_functions() {
    let x = symb("x_mathfn");

    // Test all math functions work
    let _sin = x.sin();
    let _cos = x.cos();
    let _tan = x.tan();
    let _exp = x.exp();
    let _ln = x.ln();
    let _sqrt = x.sqrt();
    let _abs = x.abs();

    // Hyperbolic
    let _sinh = x.sinh();
    let _cosh = x.cosh();
    let _tanh = x.tanh();

    // Inverse trig
    let _asin = x.asin();
    let _acos = x.acos();
    let _atan = x.atan();
}

#[test]
fn test_parse_rejects_empty_function_call() {
    // sin() should be rejected - requires at least 1 argument
    let result = parser_parse("sin()", &HashSet::new(), &HashSet::new(), None);
    assert!(result.is_err(), "sin() should be rejected");

    // The error should be InvalidFunctionCall
    if let Err(e) = result {
        let msg = format!("{}", e);
        assert!(
            msg.contains("sin") && msg.contains("1"),
            "Error should mention sin and 1 arg: {}",
            msg
        );
    }
}

// ============================================================================
// ERROR MESSAGE QUALITY
// ============================================================================

#[test]
fn test_parse_error_messages_are_informative() {
    // Parse error should mention the issue
    let result = parser_parse("sin(", &HashSet::new(), &HashSet::new(), None);
    if let Err(e) = result {
        let msg = format!("{}", e);
        // Should have some indication of what went wrong
        assert!(!msg.is_empty());
    }
}
