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
    clippy::no_effect_underscore_binding,
    reason = "Standard test relaxations"
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

#[test]
fn test_evaluate_missing_params_default_to_zero() {
    let expr = parse_expr("x * y + z");
    let eval = CompiledEvaluator::compile(&expr, &["x", "y", "z"], None).expect("Should compile");

    // Missing y,z -> y=0, z=0 => x*0 + 0 = 0
    assert!((eval.evaluate(&[2.0]) - 0.0).abs() < 1e-10);

    // Missing z -> z=0 => x*y + 0 = 6
    assert!((eval.evaluate(&[2.0, 3.0]) - 6.0).abs() < 1e-10);

    // Extra params are ignored
    assert!((eval.evaluate(&[2.0, 3.0, 4.0, 99.0]) - 10.0).abs() < 1e-10);
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
fn test_eval_batch_neg_muladd() {
    // Tests NegMulAdd in SIMD path: -x*y + z
    let expr = parse_expr("-x * y + z");
    let eval = CompiledEvaluator::compile(&expr, &["x", "y", "z"], None).expect("Should compile");

    // Ensure NegMulAdd was actually emitted
    assert!(
        eval.instructions
            .iter()
            .any(|i| matches!(i, Instruction::NegMulAdd))
    );

    let x_vals = vec![1.0, 2.0, 3.0, 4.0];
    let y_vals = vec![2.0, 3.0, 4.0, 5.0];
    let z_vals = vec![10.0, 20.0, 30.0, 40.0];
    let columns: Vec<&[f64]> = vec![&x_vals, &y_vals, &z_vals];
    let mut output = vec![0.0; 4];

    eval.eval_batch(&columns, &mut output, None)
        .expect("Should pass");

    for i in 0..4 {
        let expected = (-x_vals[i]).mul_add(y_vals[i], z_vals[i]);
        assert!((output[i] - expected).abs() < 1e-10);
    }
}

#[test]
fn test_eval_batch_mulsub() {
    // Tests MulSub in SIMD path: x*y - z
    let expr = parse_expr("x * y - z");
    let eval = CompiledEvaluator::compile(&expr, &["x", "y", "z"], None).expect("Should compile");

    // Ensure MulSub was actually emitted (fused by optimize_instructions or compiler)
    assert!(
        eval.instructions
            .iter()
            .any(|i| matches!(i, Instruction::MulSub))
    );

    let x_vals = vec![1.0, 2.0, 3.0, 4.0];
    let y_vals = vec![2.0, 3.0, 4.0, 5.0];
    let z_vals = vec![0.5, 1.0, 1.5, 2.0];
    let columns: Vec<&[f64]> = vec![&x_vals, &y_vals, &z_vals];
    let mut output = vec![0.0; 4];

    eval.eval_batch(&columns, &mut output, None)
        .expect("Should pass");

    for i in 0..4 {
        let expected = x_vals[i].mul_add(y_vals[i], -z_vals[i]);
        assert!((output[i] - expected).abs() < 1e-10);
    }
}

#[test]
fn test_compile_neg_add_is_lowered_to_sub() {
    // Peephole pattern: [LoadA, Neg, LoadB, Add] -> [LoadB, LoadA, Sub]
    let mut instructions = vec![
        Instruction::LoadParam(0),
        Instruction::Neg,
        Instruction::LoadParam(1),
        Instruction::Add,
    ];
    let mut constants = Vec::new();

    CompiledEvaluator::peephole_optimize(&mut instructions, &mut constants);

    assert_eq!(
        instructions,
        vec![
            Instruction::LoadParam(1),
            Instruction::LoadParam(0),
            Instruction::Sub,
        ]
    );
}

#[test]
fn test_normal_pdf_derivative_uses_expneg_fusion() {
    // Pattern from normal-PDF derivative: exp(-(...)/(2*sigma^2))
    // should lower to ExpNeg after optimization.
    let expr = parse_expr("-exp(-(-mu + x)^2/(2*sigma^2))*(-mu + x)/(sigma^2*abs(sigma)*sqrt(2*pi))");
    let eval =
        CompiledEvaluator::compile(&expr, &["mu", "sigma", "x"], None).expect("Should compile");

    assert!(
        eval.instructions
            .iter()
            .any(|i| matches!(i, Instruction::ExpNeg))
    );
}

#[test]
fn test_normal_pdf_raw_derivative_prefers_sub_for_x_minus_mu() {
    let expr = parse_expr("exp(-(-mu + x)^2/(2*sigma^2))*-2*(-mu + x)/(2*sigma^2)/sqrt(2*pi*sigma^2)");
    let eval = CompiledEvaluator::compile(&expr, &["mu", "pi", "sigma", "x"], None)
        .expect("Should compile");

    assert!(
        eval.instructions
            .iter()
            .any(|i| matches!(i, Instruction::Sub))
    );
}

#[test]
fn test_eval_batch_trig_nonfinite_matches_scalar() {
    // `coth(0)` evaluates to +inf, and `sin(+inf)` must be NaN.
    // SIMD path should match scalar IEEE behavior lane-by-lane.
    let expr = parse_expr("sin(coth(x))");
    let eval = CompiledEvaluator::compile(&expr, &["x"], None).expect("Should compile");

    let x_vals = vec![0.0, 0.5, 1.0, 2.0];
    let columns: Vec<&[f64]> = vec![&x_vals];
    let mut output = vec![0.0; x_vals.len()];
    eval.eval_batch(&columns, &mut output, None)
        .expect("Should pass");

    for (i, x) in x_vals.iter().enumerate() {
        let scalar = eval.evaluate(&[*x]);
        let simd = output[i];
        if scalar.is_nan() {
            assert!(
                simd.is_nan(),
                "Mismatch at i={i}: expected NaN for x={x}, got {simd}"
            );
        } else {
            assert!(
                (simd - scalar).abs() < 1e-12,
                "Mismatch at i={i}: x={x}, scalar={scalar}, simd={simd}"
            );
        }
    }
}

#[test]
#[allow(
    clippy::excessive_precision,
    clippy::unreadable_literal,
    reason = "Regression test uses exact reproducer literals from a fuzz failure"
)]
fn test_eval_batch_regression_nan_to_besseli() {
    let expr = parse_expr(
        "besseli(0.1547707167434922, sin(coth(assoc_legendre(beta(3.5446259250558008, 2.8050545544522407), x1, log(-0.45401117869232976, 6.215453824346511))))/csch(polygamma(assoc_legendre(csch(x4), 5.738965850904144 + x0, log(x3, x4)), assoc_legendre(acsch(-1.7901751475351624), csch(x3), floor(x4)))))",
    );
    let eval =
        CompiledEvaluator::compile(&expr, &["x0", "x1", "x2", "x3", "x4"], None).expect("compile");

    let x0 = vec![
        -2.2247003709803708,
        -2.1247003709803707,
        -2.024700370980371,
        -2.3247003709803706,
    ];
    let x1 = vec![
        -3.547937729368135,
        -3.447937729368135,
        -3.347937729368135,
        -3.647937729368135,
    ];
    let x2 = vec![
        -2.2202727238738222,
        -2.120272723873822,
        -2.3202727238738223,
        -2.4202727238738224,
    ];
    let x3 = vec![
        3.75795451518241,
        3.85795451518241,
        3.65795451518241,
        3.55795451518241,
    ];
    let x4 = vec![
        0.12856467048791584,
        0.22856467048791584,
        0.02856467048791584,
        0.32856467048791584,
    ];
    let columns: Vec<&[f64]> = vec![&x0, &x1, &x2, &x3, &x4];
    let mut out = vec![0.0; 4];
    eval.eval_batch(&columns, &mut out, None)
        .expect("eval_batch");

    for i in 0..4 {
        let scalar = eval.evaluate(&[x0[i], x1[i], x2[i], x3[i], x4[i]]);
        let simd = out[i];
        if scalar.is_nan() {
            assert!(simd.is_nan(), "row {i}: expected NaN, got {simd}");
        } else {
            assert!(
                (simd - scalar).abs() < 1e-10,
                "row {i}: scalar={scalar}, simd={simd}"
            );
        }
    }
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
