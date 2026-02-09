// Benchmark requirements: unwrap for setup, stdout for progress, similar names for math variables
#![allow(
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::similar_names,
    reason = "Benchmark requirements: unwrap for setup, stdout for progress, similar names for math variables"
)]
//! Symbolica Benchmarks (Comparison)

//!
//! Replicates the structure of benchmark.rs but using Symbolica's API.
//! Tests: parse → diff → simplify → compile → evaluate

mod expressions;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use dotenvy::dotenv;
use expressions::ALL_EXPRESSIONS;
use std::convert::TryInto;
use std::env;
use std::hint::black_box;
use symbolica::{
    LicenseManager,
    atom::{Atom, AtomCore, Indeterminate},
    evaluate::{FunctionMap, OptimizationSettings},
    parser::ParseSettings,
    wrap_input,
};

// Setup license
/// Setup Symbolica license from environment variable
fn setup_license() {
    dotenv().ok();
    if let Ok(key) = env::var("SYMBOLICA_LICENSE") {
        LicenseManager::set_license_key(&key).ok();
    }
}

// =============================================================================
// Parsing Benchmarks
// =============================================================================

/// Benchmark parsing performance for all expressions using Symbolica
fn bench_parse(c: &mut Criterion) {
    setup_license();
    let mut group = c.benchmark_group("1_parse");

    for (name, expr, _, _) in ALL_EXPRESSIONS {
        group.bench_with_input(BenchmarkId::new("symbolica", name), expr, |b, expr| {
            b.iter(|| Atom::parse(wrap_input!(black_box(expr)), ParseSettings::default()));
        });
    }

    group.finish();
}

// =============================================================================
// Differentiation (String Pipeline)
// =============================================================================

/// Benchmark differentiation performance for all expressions using Symbolica
fn bench_diff(c: &mut Criterion) {
    setup_license();
    let mut group = c.benchmark_group("2_diff");

    for (name, expr_str, var, _) in ALL_EXPRESSIONS {
        // Pre-parse for fair comparison
        let expr = Atom::parse(wrap_input!(expr_str), ParseSettings::default()).unwrap();
        let var_atom = Atom::parse(wrap_input!(var), ParseSettings::default()).unwrap();
        let var_indet: Indeterminate = var_atom.try_into().unwrap();

        // Symbolica: diff only (auto light-simplifies)
        group.bench_with_input(BenchmarkId::new("symbolica", name), &expr, |b, expr| {
            b.iter(|| black_box(expr.derivative(var_indet.clone())));
        });
    }

    group.finish();
}

// =============================================================================
// Simplification Only Benchmarks
// =============================================================================

/// Benchmark simplification performance (not applicable for Symbolica)
fn bench_simplify_only(_c: &mut Criterion) {
    setup_license();
    // Symbolica does not have explicit simplify API separate from other ops
}

// =============================================================================
// Compilation Benchmarks
// =============================================================================

/// Benchmark compilation performance for expressions using Symbolica
fn bench_compile(c: &mut Criterion) {
    setup_license();
    let mut group = c.benchmark_group("5_compile");

    for (name, expr_str, var, fixed_vars) in ALL_EXPRESSIONS {
        if *name == "Bessel Wave" {
            continue;
        }
        // Pre-parse to isolate compile time
        let expr = Atom::parse(wrap_input!(expr_str), ParseSettings::default()).unwrap();

        // Handle extra variables that are not in fixed_vars but are in the expression
        let mut all_fixed_vars = fixed_vars.to_vec();
        if *name == "Gaussian 2D" {
            all_fixed_vars.push("y");
        }

        let var_atom = Atom::parse(wrap_input!(var), ParseSettings::default()).unwrap();
        let mut params = vec![var_atom];

        // Add fixed vars to params
        for f_var in all_fixed_vars {
            let f_atom = Atom::parse(wrap_input!(f_var), ParseSettings::default()).unwrap();
            params.push(f_atom);
        }

        let func_map = FunctionMap::new();

        group.bench_with_input(BenchmarkId::new("symbolica", name), &expr, |b, expr| {
            b.iter(|| {
                // creating the evaluator is the "compilation" step here
                expr.evaluator(
                    black_box(&func_map),
                    black_box(&params),
                    black_box(OptimizationSettings::default()),
                )
                .expect("Failed to create evaluator")
            });
        });
    }
    group.finish();
}

// =============================================================================
// Evaluation Benchmarks
// =============================================================================

/// Benchmark evaluation performance for 1000 points using Symbolica
fn bench_eval(c: &mut Criterion) {
    setup_license();
    let mut group = c.benchmark_group("6_eval_1000pts");
    let test_points: Vec<f64> = (0..1000).map(|i| f64::from(i).mul_add(0.01, 0.1)).collect();

    for (name, expr_str, var, fixed_vars) in ALL_EXPRESSIONS {
        // Symbolica evaluator lacks explicit besselj support out-of-the-box or requires custom registration
        if *name == "Bessel Wave" {
            continue;
        }

        // Pre-parse to isolate compile time
        let expr = Atom::parse(wrap_input!(expr_str), ParseSettings::default()).unwrap();

        // Handle extra variables that are not in fixed_vars but are in the expression
        let mut all_fixed_vars = fixed_vars.to_vec();
        if *name == "Gaussian 2D" {
            all_fixed_vars.push("y");
        }

        let var_atom = Atom::parse(wrap_input!(var), ParseSettings::default()).unwrap();
        let mut params = vec![var_atom];

        // Add fixed vars to params
        for f_var in &all_fixed_vars {
            let f_atom = Atom::parse(wrap_input!(f_var), ParseSettings::default()).unwrap();
            params.push(f_atom);
        }

        let func_map = FunctionMap::new();
        let settings = OptimizationSettings::default();

        // Create evaluator (compile)
        // Use cleaner conversion pattern from large_expr.rs
        let mut evaluator = expr
            .evaluator(&func_map, &params, settings)
            .expect("Failed to create evaluator")
            .map_coeff(&|coeff| coeff.to_real().map_or(1.0, std::convert::Into::into));

        // Prepare constants (1.0 for all fixed vars)
        let fixed_vals: Vec<f64> = vec![1.0; all_fixed_vars.len()];

        group.bench_with_input(
            BenchmarkId::new("symbolica", name),
            &test_points,
            |b, points| {
                b.iter(|| {
                    let mut sum = 0.0;
                    for &x in points {
                        // MATCHING BEHAVIOR: allocating per point as done in benchmark.rs
                        // This ensures fair comparison including allocation overhead.
                        let mut input = Vec::with_capacity(1 + fixed_vals.len());
                        input.push(x);
                        input.extend_from_slice(&fixed_vals);

                        // evaluate_single takes a slice matching params
                        sum += evaluator.evaluate_single(&input);
                    }
                    sum
                });
            },
        );
    }
    group.finish();
}

// =============================================================================
// Full Pipeline
// =============================================================================

/// Benchmark full pipeline performance using Symbolica
fn bench_full_pipeline(c: &mut Criterion) {
    setup_license();
    let mut group = c.benchmark_group("7_full_pipeline");

    let test_points: Vec<f64> = (0..1000).map(|i| f64::from(i).mul_add(0.01, 0.1)).collect();

    for (name, expr_str, var, fixed_vars) in ALL_EXPRESSIONS {
        if *name == "Bessel Wave" {
            continue;
        }

        group.bench_with_input(
            BenchmarkId::new("symbolica", name),
            expr_str,
            |b, expr_str| {
                b.iter(|| {
                    // 1. Parse
                    let expr =
                        Atom::parse(wrap_input!(expr_str), ParseSettings::default()).unwrap();
                    let var_atom = Atom::parse(wrap_input!(var), ParseSettings::default()).unwrap();
                    let var_indet: Indeterminate = var_atom.try_into().unwrap();

                    // 2. Diff
                    let diff = expr.derivative(var_indet);

                    // 3. Compile (Create Evaluator)
                    // Need to prepare params for the differentiated expression
                    // The derivative might contain the differentiation variable and fixed vars
                    // plus potentially "y" for Gaussian 2D.

                    let all_fixed_vars = fixed_vars.to_vec();

                    // Re-parse var for params list (as Atom)
                    let var_param =
                        Atom::parse(wrap_input!(var), ParseSettings::default()).unwrap();
                    let mut params = vec![var_param];
                    for f_var in &all_fixed_vars {
                        let f_atom =
                            Atom::parse(wrap_input!(f_var), ParseSettings::default()).unwrap();
                        params.push(f_atom);
                    }

                    let func_map = FunctionMap::new();
                    let settings = OptimizationSettings::default();

                    let mut evaluator = diff
                        .evaluator(&func_map, &params, settings)
                        .expect("Failed to create evaluator")
                        .map_coeff(&|coeff| coeff.to_real().map_or(1.0, std::convert::Into::into));

                    // 4. Evaluate (1000 points)
                    let fixed_vals: Vec<f64> = vec![1.0; all_fixed_vars.len()];
                    let mut sum = 0.0;

                    // Re-use buffer to avoid 1000 allocations
                    let mut input = Vec::with_capacity(1 + fixed_vals.len());
                    // Initial setup (push placeholders or just use clear/extend approach)
                    input.push(0.0);
                    input.extend_from_slice(&fixed_vals);

                    for &x in &test_points {
                        // Update the variable part (assuming it's at index 0 because we pushed it first)
                        // Note: Symbolica might expect params in specific order.
                        // In "Compile" step above: params = vec![var_param]; params.push(fixed)...
                        // So input matches: [var, fixed...]
                        input[0] = x;
                        sum += evaluator.evaluate_single(&input);
                    }
                    sum
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_parse,
    bench_diff,
    bench_simplify_only,
    bench_compile,
    bench_eval,
    bench_full_pipeline,
);

criterion_main!(benches);
