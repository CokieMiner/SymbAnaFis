// Benchmark requirements: unwrap for setup, stdout for progress, similar names for math variables
#![allow(
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::similar_names,
    reason = "Benchmark requirements: unwrap for setup, stdout for progress, similar names for math variables"
)]
//! Comprehensive `SymbAnaFis` Benchmarks

//!
//! Tests the full pipeline: parse → diff → simplify → compile → evaluate
//! Uses realistic medium-sized expressions from physics and ML.

mod expressions;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use expressions::ALL_EXPRESSIONS;
use std::collections::HashSet;
use std::hint::black_box;
use symb_anafis::{CompiledEvaluator, Diff, Simplify, parse, symb};

// =============================================================================
// Parsing Benchmarks
// =============================================================================

/// Benchmark parsing performance for all expressions
fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("1_parse");
    let empty = HashSet::new();

    for (name, expr, _, _) in ALL_EXPRESSIONS {
        group.bench_with_input(BenchmarkId::new("symb_anafis", name), expr, |b, expr| {
            b.iter(|| parse(black_box(expr), &empty, &empty, None));
        });
    }

    group.finish();
}

// =============================================================================
// Differentiation Benchmarks
// =============================================================================

/// Benchmark differentiation performance for all expressions
fn bench_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("2_diff");
    let empty = HashSet::new();

    for (name, expr_str, var, _fixed) in ALL_EXPRESSIONS {
        // Pre-parse the expression
        let expr = parse(expr_str, &empty, &empty, None).unwrap();
        let var_sym = symb(var);

        // SymbAnaFis: diff only (skip full simplification, similar to Symbolica)
        let diff_light = Diff::new().skip_simplification(true);

        group.bench_with_input(
            BenchmarkId::new("symb_anafis_light", name),
            &expr,
            |b, expr| b.iter(|| diff_light.differentiate(black_box(expr), &var_sym)),
        );
    }

    group.finish();
}

// =============================================================================
// Differentiation + Simplification Benchmarks
// =============================================================================

/// Benchmark differentiation with simplification performance for all expressions
fn bench_diff_simplified(c: &mut Criterion) {
    let mut group = c.benchmark_group("3_diff_simplified");
    let empty = HashSet::new();

    for (name, expr_str, var, _fixed) in ALL_EXPRESSIONS {
        // Pre-parse the expression
        let expr = parse(expr_str, &empty, &empty, None).unwrap();
        let var_sym = symb(var);

        // SymbAnaFis: diff + full simplification (default)
        let diff_full = Diff::new();

        group.bench_with_input(
            BenchmarkId::new("symb_anafis_full", name),
            &expr,
            |b, expr| b.iter(|| diff_full.differentiate(black_box(expr), &var_sym)),
        );
    }

    group.finish();
}

// =============================================================================
// Simplification Only Benchmarks
// =============================================================================

/// Benchmark simplification performance for all expressions
fn bench_simplify_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("4_simplify");
    let empty = HashSet::new();

    for (name, expr_str, var, _fixed) in ALL_EXPRESSIONS {
        // First differentiate to get a complex expression to simplify
        let expr = parse(expr_str, &empty, &empty, None).unwrap();
        let var_sym = symb(var);

        let diff_builder = Diff::new().skip_simplification(true);
        let diff_result = diff_builder.differentiate(&expr, &var_sym).unwrap();

        let simplify_builder = Simplify::new();

        group.bench_with_input(
            BenchmarkId::new("symb_anafis", name),
            &diff_result,
            |b, diffed_expr| b.iter(|| simplify_builder.simplify(black_box(diffed_expr))),
        );
    }

    group.finish();
}

// =============================================================================
// Compilation Benchmarks
// =============================================================================

/// Benchmark compilation performance for raw and simplified expressions
fn bench_compile(c: &mut Criterion) {
    let mut group = c.benchmark_group("5_compile");
    let empty = HashSet::new();

    for (name, expr_str, var, fixed) in ALL_EXPRESSIONS {
        // Raw differentiation result
        let expr = parse(expr_str, &empty, &empty, None).unwrap();
        let var_sym = symb(var);

        let diff_builder = Diff::new().skip_simplification(true);
        let diff_raw = diff_builder.differentiate(&expr, &var_sym).unwrap();

        // Simplified result
        let diff_simplified_str = symb_anafis::diff(expr_str, var, fixed, None).unwrap();
        let diff_simplified = parse(&diff_simplified_str, &empty, &empty, None).unwrap();

        // Create sorted parameter list
        let mut params = vec![var];
        params.extend(fixed.iter());
        params.sort();

        // Benchmark: compile raw
        group.bench_with_input(BenchmarkId::new("raw", name), &diff_raw, |b, raw_expr| {
            b.iter(|| CompiledEvaluator::compile(black_box(raw_expr), &params, None));
        });

        // Benchmark: compile simplified
        group.bench_with_input(
            BenchmarkId::new("simplified", name),
            &diff_simplified,
            |b, simplified_expr| {
                b.iter(|| CompiledEvaluator::compile(black_box(simplified_expr), &params, None));
            },
        );
    }

    group.finish();
}

// =============================================================================
// Evaluation Benchmarks (1000 points)
// =============================================================================

/// Benchmark evaluation performance for 1000 points on compiled expressions
fn bench_eval(c: &mut Criterion) {
    let mut group = c.benchmark_group("6_eval_1000pts");
    let empty = HashSet::new();

    // Generate 1000 test points
    let test_points: Vec<f64> = (0..1000).map(|i| f64::from(i).mul_add(0.01, 0.1)).collect();

    for (name, expr_str, var, fixed) in ALL_EXPRESSIONS {
        // Get raw and simplified derivatives
        let expr = parse(expr_str, &empty, &empty, None).unwrap();
        let var_sym = symb(var);

        let diff_builder = Diff::new().skip_simplification(true);
        let diff_raw = diff_builder.differentiate(&expr, &var_sym).unwrap();

        let diff_simplified_str = symb_anafis::diff(expr_str, var, fixed, None).unwrap();
        let diff_simplified = parse(&diff_simplified_str, &empty, &empty, None).unwrap();

        // Create sorted parameter list
        let mut params = vec![var];
        params.extend(fixed.iter());
        params.sort();

        // Compile both versions
        let compiled_raw = CompiledEvaluator::compile(&diff_raw, &params, None);
        let compiled_simplified = CompiledEvaluator::compile(&diff_simplified, &params, None);

        // Benchmark: evaluate compiled (raw)
        if let Ok(ref evaluator) = compiled_raw {
            let param_count = evaluator.param_count();
            let param_names = evaluator.param_names();
            let var_idx = param_names.iter().position(|p| p == var).unwrap_or(0);

            group.bench_with_input(
                BenchmarkId::new("compiled_raw", name),
                &test_points,
                |b, points| {
                    b.iter(|| {
                        let mut sum = 0.0;
                        // Re-use buffer to avoid 1000 allocations
                        let mut values = vec![1.0; param_count];
                        for &x in points {
                            values[var_idx] = x;
                            sum += evaluator.evaluate(&values);
                        }
                        sum
                    });
                },
            );
        }

        // Benchmark: evaluate compiled (simplified)
        if let Ok(ref evaluator) = compiled_simplified {
            let param_count = evaluator.param_count();
            let param_names = evaluator.param_names();
            let var_idx = param_names.iter().position(|p| p == var).unwrap_or(0);

            group.bench_with_input(
                BenchmarkId::new("compiled_simplified", name),
                &test_points,
                |b, points| {
                    b.iter(|| {
                        let mut sum = 0.0;
                        // Re-use buffer to avoid 1000 allocations
                        let mut values = vec![1.0; param_count];
                        for &x in points {
                            values[var_idx] = x;
                            sum += evaluator.evaluate(&values);
                        }
                        sum
                    });
                },
            );
        }
    }

    group.finish();
}

// =============================================================================
// Full Pipeline Benchmark (End-to-End)
// =============================================================================

/// Benchmark full pipeline performance: parse, diff, simplify, compile, eval 1000 points
fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("7_full_pipeline");

    // Test: parse → diff → simplify → compile → eval 1000 points
    let test_points: Vec<f64> = (0..1000).map(|i| f64::from(i).mul_add(0.01, 0.1)).collect();
    let empty = HashSet::new();

    for (name, expr_str, var, fixed) in ALL_EXPRESSIONS {
        group.bench_with_input(
            BenchmarkId::new("symb_anafis", name),
            expr_str,
            |b, expr| {
                // Pre-compute fixed set to avoid allocation in loop
                let fixed_set: HashSet<String> = fixed.iter().copied().map(String::from).collect();

                b.iter(|| {
                    // Full pipeline using expr API
                    let expr = parse(black_box(expr), &empty, &fixed_set, None).unwrap();
                    let var_sym = symb(var);
                    let diff_expr = Diff::new().differentiate(&expr, &var_sym).unwrap();

                    // Create sorted parameter list
                    let mut params = vec![var];
                    params.extend(fixed.iter());
                    params.sort();

                    let compiled = CompiledEvaluator::compile(&diff_expr, &params, None).unwrap();

                    // Evaluate at 1000 points
                    let param_count = compiled.param_count();
                    let param_names = compiled.param_names();
                    let var_idx = param_names.iter().position(|p| p == var).unwrap_or(0);
                    let mut sum = 0.0;

                    // Re-use buffer to avoid 1000 allocations
                    let mut values = vec![1.0; param_count];

                    for &x in &test_points {
                        values[var_idx] = x;
                        sum += compiled.evaluate(&values);
                    }
                    sum
                });
            },
        );
    }

    group.finish();
}

/// Benchmark full pipeline performance without simplification: parse, `diff(no_simp)`, compile, eval 1000 points
fn bench_full_pipeline_no_simp(c: &mut Criterion) {
    let mut group = c.benchmark_group("8_full_pipeline_no_simp");

    // Test: parse → diff(no_simp) → compile → eval 1000 points
    let test_points: Vec<f64> = (0..1000).map(|i| f64::from(i).mul_add(0.01, 0.1)).collect();
    let empty = HashSet::new();

    for (name, expr_str, var, fixed) in ALL_EXPRESSIONS {
        group.bench_with_input(
            BenchmarkId::new("symb_anafis", name),
            expr_str,
            |b, expr| {
                // Pre-compute fixed set to avoid allocation in loop
                let fixed_set: HashSet<String> = fixed.iter().copied().map(String::from).collect();

                b.iter(|| {
                    // Full pipeline using expr API, but skipping simplification in diff
                    let expr = parse(black_box(expr), &empty, &fixed_set, None).unwrap();
                    let var_sym = symb(var);
                    // SKIP SIMPLIFICATION HERE
                    let diff_expr = Diff::new()
                        .skip_simplification(true)
                        .differentiate(&expr, &var_sym)
                        .unwrap();

                    // Create sorted parameter list
                    let mut params = vec![var];
                    params.extend(fixed.iter());
                    params.sort();

                    let compiled = CompiledEvaluator::compile(&diff_expr, &params, None).unwrap();

                    // Evaluate at 1000 points
                    let param_count = compiled.param_count();
                    let param_names = compiled.param_names();
                    let var_idx = param_names.iter().position(|p| p == var).unwrap_or(0);
                    let mut sum = 0.0;

                    // Re-use buffer to avoid 1000 allocations
                    let mut values = vec![1.0; param_count];

                    for &x in &test_points {
                        values[var_idx] = x;
                        sum += compiled.evaluate(&values);
                    }
                    sum
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Criterion Setup
// =============================================================================

criterion_group!(
    benches,
    bench_parse,
    bench_diff,
    bench_diff_simplified,
    bench_simplify_only,
    bench_compile,
    bench_eval,
    bench_full_pipeline,
    bench_full_pipeline_no_simp,
);

criterion_main!(benches);
