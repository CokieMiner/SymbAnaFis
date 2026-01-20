//! Unit tests for the evaluator module.
//!
//! These tests verify correctness of:
//! - Compilation and evaluation of various expression types
//! - SIMD batch evaluation
//! - CSE (Common Subexpression Elimination)
//! - User function expansion
//! - Error handling

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::let_underscore_must_use,
    clippy::no_effect_underscore_binding
)]

use super::*;
use crate::parser;
use std::collections::HashSet;

fn parse_expr(s: &str) -> Expr {
    parser::parse(s, &HashSet::new(), &HashSet::new(), None).expect("Should parse")
}

// =============================================================================
// Basic Evaluation Tests
// =============================================================================

#[test]
fn test_simple_arithmetic() {
    let expr = parse_expr("x + 2");
    let eval = CompiledEvaluator::compile(&expr, &["x"], None).expect("Should compile");
    assert!((eval.evaluate(&[3.0]) - 5.0).abs() < 1e-10);
}

#[test]
fn test_polynomial() {
    let expr = parse_expr("x^2 + 2*x + 1");
    let eval = CompiledEvaluator::compile(&expr, &["x"], None).expect("Should compile");
    assert!((eval.evaluate(&[3.0]) - 16.0).abs() < 1e-10);
}

#[test]
fn test_trig() {
    let expr = parse_expr("sin(x)^2 + cos(x)^2");
    let eval = CompiledEvaluator::compile(&expr, &["x"], None).expect("Should compile");
    // Should always equal 1
    assert!((eval.evaluate(&[0.5]) - 1.0).abs() < 1e-10);
    assert!((eval.evaluate(&[1.23]) - 1.0).abs() < 1e-10);
}

#[test]
fn test_constants() {
    let expr = parse_expr("pi * e");
    let eval = CompiledEvaluator::compile_auto(&expr, None).expect("Should compile");
    let expected = std::f64::consts::PI * std::f64::consts::E;
    assert!((eval.evaluate(&[]) - expected).abs() < 1e-10);
}

#[test]
fn test_multi_var() {
    let expr = parse_expr("x * y + z");
    let eval = CompiledEvaluator::compile(&expr, &["x", "y", "z"], None).expect("Should compile");
    assert!((eval.evaluate(&[2.0, 3.0, 4.0]) - 10.0).abs() < 1e-10);
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_unbound_variable_error() {
    let expr = parse_expr("x + y");
    let result = CompiledEvaluator::compile(&expr, &["x"], None);
    assert!(matches!(result, Err(DiffError::UnboundVariable(_))));
}

#[test]
fn test_compile_auto() {
    let expr = parse_expr("x^2 + y");
    let eval = CompiledEvaluator::compile_auto(&expr, None).expect("Should compile");
    // Auto compilation sorts parameters alphabetically
    assert_eq!(eval.param_names(), &["x", "y"]);
}

// =============================================================================
// Batch/SIMD Evaluation Tests
// =============================================================================

#[test]
fn test_eval_batch_simd_path() {
    // Tests the SIMD path (4+ points processed with f64x4)
    let expr = parse_expr("x^2 + 2*x + 1");
    let eval = CompiledEvaluator::compile(&expr, &["x"], None).expect("Should compile");

    // 8 points to ensure full SIMD chunks
    let x_vals: Vec<f64> = (0..8).map(f64::from).collect();
    let columns: Vec<&[f64]> = vec![&x_vals];
    let mut output = vec![0.0; 8];

    eval.eval_batch(&columns, &mut output, None)
        .expect("Should pass");

    // Verify each result: (x+1)^2
    for (i, &result) in output.iter().enumerate() {
        let x = i as f64;
        let expected = (x + 1.0).powi(2);
        assert!(
            (result - expected).abs() < 1e-10,
            "Mismatch at i={i}: got {result}, expected {expected}"
        );
    }
}

#[test]
fn test_eval_batch_remainder_path() {
    // Tests the scalar remainder path (points not divisible by 4)
    let expr = parse_expr("sin(x) + cos(x)");
    let eval = CompiledEvaluator::compile(&expr, &["x"], None).expect("Should compile");

    // 6 points: 4 SIMD + 2 remainder
    let x_vals: Vec<f64> = vec![0.0, 0.5, 1.0, 1.5, 2.0, 2.5];
    let columns: Vec<&[f64]> = vec![&x_vals];
    let mut output = vec![0.0; 6];

    eval.eval_batch(&columns, &mut output, None)
        .expect("Should pass");

    for (i, &result) in output.iter().enumerate() {
        let x = x_vals[i];
        let expected = x.sin() + x.cos();
        assert!(
            (result - expected).abs() < 1e-10,
            "Mismatch at i={i}: got {result}, expected {expected}"
        );
    }
}

#[test]
fn test_eval_batch_multi_var() {
    // Tests batch evaluation with multiple variables
    let expr = parse_expr("x * y + z");
    let eval = CompiledEvaluator::compile(&expr, &["x", "y", "z"], None).expect("Should compile");

    let x_vals = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_vals = vec![2.0, 3.0, 4.0, 5.0, 6.0];
    let z_vals = vec![0.5, 1.0, 1.5, 2.0, 2.5];
    let columns: Vec<&[f64]> = vec![&x_vals, &y_vals, &z_vals];
    let mut output = vec![0.0; 5];

    eval.eval_batch(&columns, &mut output, None)
        .expect("Should pass");

    for i in 0..5 {
        let expected = x_vals[i].mul_add(y_vals[i], z_vals[i]);
        assert!(
            (output[i] - expected).abs() < 1e-10,
            "Mismatch at i={}: got {}, expected {}",
            i,
            output[i],
            expected
        );
    }
}

#[test]
fn test_eval_batch_special_functions() {
    // Tests SIMD slow path for special functions
    let expr = parse_expr("exp(x) + sqrt(x)");
    let eval = CompiledEvaluator::compile(&expr, &["x"], None).expect("Should compile");

    let x_vals: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0];
    let columns: Vec<&[f64]> = vec![&x_vals];
    let mut output = vec![0.0; 4];

    eval.eval_batch(&columns, &mut output, None)
        .expect("Should pass");

    for (i, &result) in output.iter().enumerate() {
        let x = x_vals[i];
        let expected = x.exp() + x.sqrt();
        assert!(
            (result - expected).abs() < 1e-10,
            "Mismatch at i={i}: got {result}, expected {expected}"
        );
    }
}

#[test]
fn test_eval_batch_single_point() {
    // Edge case: single point (no SIMD, just remainder)
    let expr = parse_expr("x^2");
    let eval = CompiledEvaluator::compile(&expr, &["x"], None).expect("Should compile");

    let x_vals = vec![3.0];
    let columns: Vec<&[f64]> = vec![&x_vals];
    let mut output = vec![0.0; 1];

    eval.eval_batch(&columns, &mut output, None)
        .expect("Should pass");

    assert!((output[0] - 9.0).abs() < 1e-10);
}

#[test]
fn test_eval_batch_constant_expr() {
    // Edge case: expression with no variables
    let expr = parse_expr("pi * 2");
    let eval = CompiledEvaluator::compile_auto(&expr, None).expect("Should compile");

    let columns: Vec<&[f64]> = vec![];
    let mut output = vec![0.0; 1];

    eval.eval_batch(&columns, &mut output, None)
        .expect("Should pass");

    let expected = std::f64::consts::PI * 2.0;
    assert!((output[0] - expected).abs() < 1e-10);
}

#[test]
fn test_eval_batch_vs_single() {
    // Verify batch and single evaluation produce identical results
    let expr = parse_expr("sin(x) * cos(y) + exp(x/y)");
    let eval = CompiledEvaluator::compile(&expr, &["x", "y"], None).expect("Should compile");

    let x_vals: Vec<f64> = (1..=8).map(|i| f64::from(i) * 0.5).collect();
    let y_vals: Vec<f64> = (1..=8).map(|i| f64::from(i).mul_add(0.3, 0.1)).collect();
    let columns: Vec<&[f64]> = vec![&x_vals, &y_vals];
    let mut batch_output = vec![0.0; 8];

    eval.eval_batch(&columns, &mut batch_output, None)
        .expect("Should pass");

    // Compare with single evaluations
    for i in 0..8 {
        let single_result = eval.evaluate(&[x_vals[i], y_vals[i]]);
        assert!(
            (batch_output[i] - single_result).abs() < 1e-10,
            "Batch/single mismatch at i={}: batch={}, single={}",
            i,
            batch_output[i],
            single_result
        );
    }
}

// =============================================================================
// User Function Tests
// =============================================================================

#[test]
fn test_user_function_expansion() {
    use crate::core::unified_context::{Context, UserFunction};

    // Define f(x) = x^2 + 1
    let ctx = Context::new().with_function(
        "f",
        UserFunction::new(1..=1).body(|args| {
            let x = (*args[0]).clone();
            x.pow(2.0) + 1.0
        }),
    );

    // Create expression: f(x) + 2
    let x = crate::symb("x");
    let expr = Expr::func("f", x.to_expr()) + 2.0;

    // Compile with context - user function should be expanded
    let eval = CompiledEvaluator::compile(&expr, &["x"], Some(&ctx)).expect("Should compile");

    // f(3) + 2 = (3^2 + 1) + 2 = 10 + 2 = 12
    let result = eval.evaluate(&[3.0]);
    assert!((result - 12.0).abs() < 1e-10, "Expected 12.0, got {result}");

    // f(0) + 2 = (0^2 + 1) + 2 = 1 + 2 = 3
    let result2 = eval.evaluate(&[0.0]);
    assert!((result2 - 3.0).abs() < 1e-10, "Expected 3.0, got {result2}");
}

#[test]
fn test_nested_user_function_expansion() {
    use crate::core::unified_context::{Context, UserFunction};

    // Define g(x) = 2*x
    // Define f(x) = g(x) + 1  (nested call)
    let ctx = Context::new()
        .with_function(
            "g",
            UserFunction::new(1..=1).body(|args| 2.0 * (*args[0]).clone()),
        )
        .with_function(
            "f",
            UserFunction::new(1..=1).body(|args| {
                // f(x) = g(x) + 1
                Expr::func("g", (*args[0]).clone()) + 1.0
            }),
        );

    // Create expression: f(x)
    let x = crate::symb("x");
    let expr = Expr::func("f", x.to_expr());

    // Compile with context - nested function calls should be expanded
    let eval = CompiledEvaluator::compile(&expr, &["x"], Some(&ctx)).expect("Should compile");

    // f(5) = g(5) + 1 = 2*5 + 1 = 11
    let result = eval.evaluate(&[5.0]);
    assert!((result - 11.0).abs() < 1e-10, "Expected 11.0, got {result}");
}

// =============================================================================
// Fused Operations Tests
// =============================================================================

#[test]
fn test_fused_square() {
    let expr = parse_expr("x^2");
    let eval = CompiledEvaluator::compile(&expr, &["x"], None).expect("Should compile");

    // Verify the Square instruction is used (should have fewer instructions than general pow)
    assert!(
        eval.instruction_count() <= 3,
        "Should use fused Square instruction"
    );

    assert!((eval.evaluate(&[5.0]) - 25.0).abs() < 1e-10);
}

#[test]
fn test_fused_cube() {
    let expr = parse_expr("x^3");
    let eval = CompiledEvaluator::compile(&expr, &["x"], None).expect("Should compile");

    assert!((eval.evaluate(&[3.0]) - 27.0).abs() < 1e-10);
}

#[test]
fn test_fused_recip() {
    let expr = parse_expr("x^(-1)");
    let eval = CompiledEvaluator::compile(&expr, &["x"], None).expect("Should compile");

    assert!((eval.evaluate(&[4.0]) - 0.25).abs() < 1e-10);
}

#[test]
fn test_fused_sqrt() {
    let expr = parse_expr("x^0.5");
    let eval = CompiledEvaluator::compile(&expr, &["x"], None).expect("Should compile");

    assert!((eval.evaluate(&[9.0]) - 3.0).abs() < 1e-10);
}

// =============================================================================
// CSE Tests
// =============================================================================

#[test]
fn test_cse_repeated_subexpr() {
    // sin(x) appears twice - should be cached
    let expr = parse_expr("sin(x) + sin(x)");
    let eval = CompiledEvaluator::compile(&expr, &["x"], None).expect("Should compile");

    // CSE should cache sin(x)
    assert!(eval.cache_size > 0, "Should use CSE cache");

    let result = eval.evaluate(&[1.0]);
    let expected = 2.0 * 1.0_f64.sin();
    assert!((result - expected).abs() < 1e-10);
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_sinc_at_zero() {
    let expr = parse_expr("sin(x)/x");
    let eval = CompiledEvaluator::compile(&expr, &["x"], None).expect("Should compile");

    // sinc(0) should be 1, not NaN
    let result = eval.evaluate(&[0.0]);
    assert!(
        (result - 1.0).abs() < 1e-10,
        "sinc(0) should be 1, got {result}"
    );
}

#[test]
fn test_division_same_expr() {
    let expr = parse_expr("x/x");
    let eval = CompiledEvaluator::compile(&expr, &["x"], None).expect("Should compile");

    // x/x should fold to 1
    let result = eval.evaluate(&[0.0]); // Even at x=0
    assert!(
        (result - 1.0).abs() < 1e-10,
        "x/x should be 1, got {result}"
    );
}
