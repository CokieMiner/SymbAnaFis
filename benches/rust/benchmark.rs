//! Comprehensive SymbAnaFis Benchmarks
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

fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("1_parse");
    let empty = HashSet::new();

    for (name, expr, _, _) in ALL_EXPRESSIONS {
        group.bench_with_input(BenchmarkId::new("symb_anafis", name), expr, |b, expr| {
            b.iter(|| parse(black_box(expr), &empty, &empty, None))
        });
    }

    group.finish();
}

// =============================================================================
// Differentiation Benchmarks (Raw - no simplification)
// =============================================================================

fn bench_diff_raw(c: &mut Criterion) {
    let mut group = c.benchmark_group("2_diff_raw");
    let empty = HashSet::new();

    for (name, expr_str, var, fixed) in ALL_EXPRESSIONS {
        // Pre-parse the expression
        let expr = parse(expr_str, &empty, &empty, None).unwrap();
        let var_sym = symb(var);

        // Build fixed var symbols
        let fixed_syms: Vec<_> = fixed.iter().map(|s| symb(s)).collect();
        let fixed_refs: Vec<_> = fixed_syms.iter().collect();

        // Build Diff with fixed vars but skip simplification
        let diff_builder = Diff::new()
            .fixed_vars(&fixed_refs)
            .skip_simplification(true);

        group.bench_with_input(BenchmarkId::new("symb_anafis", name), &expr, |b, expr| {
            b.iter(|| diff_builder.differentiate(black_box(expr.clone()), &var_sym))
        });
    }

    group.finish();
}

// =============================================================================
// Differentiation + Simplification Benchmarks
// =============================================================================

fn bench_diff_simplified(c: &mut Criterion) {
    let mut group = c.benchmark_group("3_diff_simplified");
    let empty = HashSet::new();

    for (name, expr_str, var, fixed) in ALL_EXPRESSIONS {
        // Pre-parse the expression
        let expr = parse(expr_str, &empty, &empty, None).unwrap();
        let var_sym = symb(var);

        // Build fixed var symbols
        let fixed_syms: Vec<_> = fixed.iter().map(|s| symb(s)).collect();
        let fixed_refs: Vec<_> = fixed_syms.iter().collect();

        // SymbAnaFis: diff only (skip full simplification, similar to Symbolica)
        let diff_light = Diff::new()
            .fixed_vars(&fixed_refs)
            .skip_simplification(true);

        group.bench_with_input(
            BenchmarkId::new("symb_anafis_diff_only", name),
            &expr,
            |b, expr| b.iter(|| diff_light.differentiate(black_box(expr.clone()), &var_sym)),
        );

        // SymbAnaFis: diff + full simplification
        let diff_full = Diff::new().fixed_vars(&fixed_refs);

        group.bench_with_input(
            BenchmarkId::new("symb_anafis_diff+simplify", name),
            &expr,
            |b, expr| b.iter(|| diff_full.differentiate(black_box(expr.clone()), &var_sym)),
        );
    }

    group.finish();
}

// =============================================================================
// Simplification Only Benchmarks
// =============================================================================

fn bench_simplify_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("4_simplify");
    let empty = HashSet::new();

    for (name, expr_str, var, fixed) in ALL_EXPRESSIONS {
        // First differentiate to get a complex expression to simplify
        let expr = parse(expr_str, &empty, &empty, None).unwrap();
        let var_sym = symb(var);

        let fixed_syms: Vec<_> = fixed.iter().map(|s| symb(s)).collect();
        let fixed_refs: Vec<_> = fixed_syms.iter().collect();

        let diff_builder = Diff::new()
            .fixed_vars(&fixed_refs)
            .skip_simplification(true);
        let diff_result = diff_builder.differentiate(expr, &var_sym).unwrap();

        let simplify_builder = Simplify::new().fixed_vars(&fixed_refs);

        group.bench_with_input(
            BenchmarkId::new("symb_anafis", name),
            &diff_result,
            |b, expr| b.iter(|| simplify_builder.simplify(black_box(expr.clone()))),
        );
    }

    group.finish();
}

// =============================================================================
// Compilation Benchmarks
// =============================================================================

fn bench_compile(c: &mut Criterion) {
    let mut group = c.benchmark_group("5_compile");
    let empty = HashSet::new();

    for (name, expr_str, var, fixed) in ALL_EXPRESSIONS {
        // Raw differentiation result
        let expr = parse(expr_str, &empty, &empty, None).unwrap();
        let var_sym = symb(var);

        let fixed_syms: Vec<_> = fixed.iter().map(|s| symb(s)).collect();
        let fixed_refs: Vec<_> = fixed_syms.iter().collect();

        let diff_builder = Diff::new()
            .fixed_vars(&fixed_refs)
            .skip_simplification(true);
        let diff_raw = diff_builder.differentiate(expr.clone(), &var_sym).unwrap();

        // Simplified result
        let diff_simplified_str = symb_anafis::diff(expr_str, var, Some(fixed), None).unwrap();
        let diff_simplified = parse(&diff_simplified_str, &empty, &empty, None).unwrap();

        // Benchmark: compile raw
        group.bench_with_input(BenchmarkId::new("raw", name), &diff_raw, |b, expr| {
            b.iter(|| CompiledEvaluator::compile_auto(black_box(expr)))
        });

        // Benchmark: compile simplified
        group.bench_with_input(
            BenchmarkId::new("simplified", name),
            &diff_simplified,
            |b, expr| b.iter(|| CompiledEvaluator::compile_auto(black_box(expr))),
        );
    }

    group.finish();
}

// =============================================================================
// Evaluation Benchmarks (1000 points)
// =============================================================================

fn bench_eval(c: &mut Criterion) {
    let mut group = c.benchmark_group("6_eval_1000pts");
    let empty = HashSet::new();

    // Generate 1000 test points
    let test_points: Vec<f64> = (0..1000).map(|i| 0.1 + i as f64 * 0.01).collect();

    for (name, expr_str, var, fixed) in ALL_EXPRESSIONS {
        // Get raw and simplified derivatives
        let expr = parse(expr_str, &empty, &empty, None).unwrap();
        let var_sym = symb(var);

        let fixed_syms: Vec<_> = fixed.iter().map(|s| symb(s)).collect();
        let fixed_refs: Vec<_> = fixed_syms.iter().collect();

        let diff_builder = Diff::new()
            .fixed_vars(&fixed_refs)
            .skip_simplification(true);
        let diff_raw = diff_builder.differentiate(expr, &var_sym).unwrap();

        let diff_simplified_str = symb_anafis::diff(expr_str, var, Some(fixed), None).unwrap();
        let diff_simplified = parse(&diff_simplified_str, &empty, &empty, None).unwrap();

        // Compile both versions
        let compiled_raw = CompiledEvaluator::compile_auto(&diff_raw);
        let compiled_simplified = CompiledEvaluator::compile_auto(&diff_simplified);

        // Benchmark: evaluate compiled (raw)
        if let Ok(ref evaluator) = compiled_raw {
            let param_count = evaluator.param_count();
            group.bench_with_input(
                BenchmarkId::new("compiled_raw", name),
                &test_points,
                |b, points| {
                    b.iter(|| {
                        let mut sum = 0.0;
                        for &x in points {
                            // Build values: x for var, 1.0 for all fixed
                            let mut values = vec![1.0; param_count];
                            values[0] = x; // First param is usually the variable
                            sum += evaluator.evaluate(&values);
                        }
                        sum
                    })
                },
            );
        }

        // Benchmark: evaluate compiled (simplified)
        if let Ok(ref evaluator) = compiled_simplified {
            let param_count = evaluator.param_count();
            group.bench_with_input(
                BenchmarkId::new("compiled_simplified", name),
                &test_points,
                |b, points| {
                    b.iter(|| {
                        let mut sum = 0.0;
                        for &x in points {
                            let mut values = vec![1.0; param_count];
                            values[0] = x;
                            sum += evaluator.evaluate(&values);
                        }
                        sum
                    })
                },
            );
        }
    }

    group.finish();
}

// =============================================================================
// Full Pipeline Benchmark (End-to-End)
// =============================================================================

fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("7_full_pipeline");

    // Test: parse → diff → simplify → compile → eval 1000 points
    let test_points: Vec<f64> = (0..1000).map(|i| 0.1 + i as f64 * 0.01).collect();
    let empty = HashSet::new();

    for (name, expr_str, var, fixed) in ALL_EXPRESSIONS {
        group.bench_with_input(
            BenchmarkId::new("symb_anafis", name),
            expr_str,
            |b, expr| {
                b.iter(|| {
                    // Full pipeline
                    let diff_str =
                        symb_anafis::diff(black_box(expr), var, Some(fixed), None).unwrap();
                    let diff_expr = parse(&diff_str, &empty, &empty, None).unwrap();
                    let compiled = CompiledEvaluator::compile_auto(&diff_expr).unwrap();

                    // Evaluate at 1000 points
                    let param_count = compiled.param_count();
                    let mut sum = 0.0;
                    for &x in &test_points {
                        let mut values = vec![1.0; param_count];
                        values[0] = x;
                        sum += compiled.evaluate(&values);
                    }
                    sum
                })
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
    bench_diff_raw,
    bench_diff_simplified,
    bench_simplify_only,
    bench_compile,
    bench_eval,
    bench_full_pipeline,
);

criterion_main!(benches);
