//! Parallel Evaluation Benchmarks
//!
//! Compares evaluation methods in the `parallel` feature:
//! - `eval_f64` (high-perf batch, columnar data)
//! - `eval_batch` (low-level CompiledEvaluator method)
//! - `evaluate` loop (baseline, single-point calls)
//!
//! Run with: cargo bench --bench benchmark_parallel --features parallel

mod expressions;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use expressions::ALL_EXPRESSIONS;
use std::collections::HashSet;
use std::hint::black_box;
use symb_anafis::{CompiledEvaluator, parse};

// =============================================================================
// Evaluation Method Comparison (1000 points)
// =============================================================================

fn bench_eval_methods(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_methods_1000pts");
    let empty = HashSet::new();

    // Generate 1000 test points
    let test_points: Vec<f64> = (0..1000).map(|i| 0.1 + i as f64 * 0.01).collect();

    for (name, expr_str, var, fixed) in ALL_EXPRESSIONS {
        // Get simplified derivative
        let diff_str = symb_anafis::diff(expr_str, var, fixed, None).unwrap();
        let diff_expr = parse(&diff_str, &empty, &empty, None).unwrap();

        // Use compile_auto to include all variables
        let evaluator = CompiledEvaluator::compile_auto(&diff_expr);
        if evaluator.is_err() {
            eprintln!("Skipping {} - compile error", name);
            continue;
        }
        let evaluator = evaluator.unwrap();
        let var_names = evaluator.param_names();

        // Build parameter array with fixed values for all except the main var
        let var_idx = var_names.iter().position(|v| v == var);
        if var_idx.is_none() {
            eprintln!("Skipping {} - main var not found in compiled vars", name);
            continue;
        }
        let var_idx = var_idx.unwrap();

        // Create base values array (fixed vars = 1.0)
        let base_values: Vec<f64> = var_names.iter().map(|_| 1.0).collect();

        // ---------------------------------------------------------------------
        // Method 0: Tree-walk evaluate (generalized, symbolic)
        // ---------------------------------------------------------------------
        group.bench_with_input(
            BenchmarkId::new("tree_walk_evaluate", name),
            &test_points,
            |b, points| {
                let mut vars = std::collections::HashMap::new();
                // Set fixed vars to 1.0
                for f in *fixed {
                    vars.insert(*f, 1.0);
                }
                b.iter(|| {
                    let mut sum = 0.0;
                    for &x in points {
                        vars.insert(*var, x);
                        // as_number() extracts f64 from Expr::Number
                        sum += diff_expr
                            .evaluate(&vars, &std::collections::HashMap::new())
                            .as_number()
                            .unwrap_or(0.0);
                    }
                    black_box(sum)
                })
            },
        );

        // ---------------------------------------------------------------------
        // Method 1: Compiled evaluate loop
        // ---------------------------------------------------------------------
        group.bench_with_input(
            BenchmarkId::new("compiled_loop", name),
            &test_points,
            |b, points| {
                let mut values = base_values.clone();
                b.iter(|| {
                    let mut sum = 0.0;
                    for &x in points {
                        values[var_idx] = x;
                        sum += evaluator.evaluate(&values);
                    }
                    black_box(sum)
                })
            },
        );

        // ---------------------------------------------------------------------
        // Method 2: eval_batch (loop inside VM)
        // ---------------------------------------------------------------------
        // Note: eval_batch requires columnar data format for all variables
        // For simplicity, we skip this for multi-variable expressions
        if var_names.len() == 1 {
            group.bench_with_input(
                BenchmarkId::new("eval_batch", name),
                &test_points,
                |b, points| {
                    let columns: Vec<&[f64]> = vec![&points[..]];
                    let mut output = vec![0.0; points.len()];
                    b.iter(|| {
                        evaluator.eval_batch(&columns, &mut output).unwrap();
                        black_box(output.iter().sum::<f64>())
                    })
                },
            );
        }
    }

    group.finish();
}

// =============================================================================
// Scaling Benchmarks (varying point counts)
// =============================================================================

fn bench_eval_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_scaling");
    let empty = HashSet::new();

    // Use a single-variable expression after differentiation for cleaner benchmarking
    // We use Lorentz factor: 1/sqrt(1 - v^2/c^2) with c=1, so only v is variable
    let expr_str = "1 / sqrt(1 - v^2)";
    let var = "v";

    let diff_str = symb_anafis::diff(expr_str, var, &[], None).unwrap();
    let diff_expr = parse(&diff_str, &empty, &empty, None).unwrap();
    let evaluator = CompiledEvaluator::compile(&diff_expr, &[var]).unwrap();

    let point_counts = [100, 1000, 10000, 100000];

    for n in point_counts {
        // Keep v values in valid range (|v| < 1)
        let test_points: Vec<f64> = (0..n)
            .map(|i| 0.01 + 0.98 * (i as f64) / (n as f64))
            .collect();

        // Loop evaluate
        group.bench_with_input(
            BenchmarkId::new("loop_evaluate", n),
            &test_points,
            |b, points| {
                b.iter(|| {
                    let mut sum = 0.0;
                    for &x in points {
                        sum += evaluator.evaluate(&[x]);
                    }
                    sum
                })
            },
        );

        // eval_batch
        group.bench_with_input(
            BenchmarkId::new("eval_batch", n),
            &test_points,
            |b, points| {
                let columns: Vec<&[f64]> = vec![&points[..]];
                let mut output = vec![0.0; points.len()];
                b.iter(|| {
                    evaluator.eval_batch(&columns, &mut output).unwrap();
                    black_box(output.iter().sum::<f64>())
                })
            },
        );
    }

    group.finish();
}

// =============================================================================
// Multi-Expression Batch Benchmarks
// =============================================================================

fn bench_multi_expr(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_expr_batch");
    let empty = HashSet::new();

    // 3 single-variable expressions (no fixed constants to avoid compilation issues)
    let exprs_str = [
        ("Lorentz", "1 / sqrt(1 - v^2)", "v"),
        ("Quadratic", "3*x^2 + 2*x + 1", "x"),
        ("Trig", "sin(t) * cos(t)", "t"),
    ];

    let n_points = 1000;

    // Pre-differentiate and compile all expressions
    let mut diff_exprs = Vec::new();
    let mut evaluators = Vec::new();
    let mut vars = Vec::new();
    let mut test_data: Vec<Vec<f64>> = Vec::new();

    for (_, expr_str, var) in &exprs_str {
        let diff_str = symb_anafis::diff(expr_str, var, &[], None).unwrap();
        let diff_expr = parse(&diff_str, &empty, &empty, None).unwrap();
        if let Ok(eval) = CompiledEvaluator::compile(&diff_expr, &[var]) {
            diff_exprs.push(diff_expr);
            evaluators.push(eval);
            vars.push(*var);
            // Generate test data in valid domain for each expression
            let data: Vec<f64> = if *var == "v" {
                // Lorentz: |v| < 1
                (0..n_points)
                    .map(|i| 0.01 + 0.98 * (i as f64 / n_points as f64))
                    .collect()
            } else {
                // General: 0.1 to 10.1
                (0..n_points).map(|i| 0.1 + i as f64 * 0.01).collect()
            };
            test_data.push(data);
        }
    }

    // Skip if no evaluators compiled
    if evaluators.is_empty() {
        group.finish();
        return;
    }

    // Benchmark: sequential loop over expressions
    group.bench_function("sequential_loops", |b| {
        b.iter(|| {
            let mut total = 0.0;
            for (i, eval) in evaluators.iter().enumerate() {
                for &x in &test_data[i] {
                    total += eval.evaluate(&[x]);
                }
            }
            black_box(total)
        })
    });

    // Benchmark: eval_batch per expression
    let mut outputs: Vec<Vec<f64>> = evaluators.iter().map(|_| vec![0.0; n_points]).collect();

    group.bench_function("eval_batch_per_expr", |b| {
        b.iter(|| {
            let mut total = 0.0;
            for (i, eval) in evaluators.iter().enumerate() {
                let columns: Vec<&[f64]> = vec![&test_data[i][..]];
                eval.eval_batch(&columns, &mut outputs[i]).unwrap();
                total += outputs[i].iter().sum::<f64>();
            }
            black_box(total)
        })
    });

    // Benchmark: eval_f64 for each expression individually
    group.bench_function("eval_f64_per_expr", |b| {
        b.iter(|| {
            let mut total = 0.0;
            for (i, diff_expr) in diff_exprs.iter().enumerate() {
                let var = vars[i];
                let result =
                    symb_anafis::eval_f64(&[diff_expr], &[&[var]], &[&[&test_data[i][..]]])
                        .unwrap();
                total += result[0].iter().sum::<f64>();
            }
            black_box(total)
        })
    });

    group.finish();
}

// =============================================================================
// eval_f64 vs evaluate_parallel Comparison
// =============================================================================

fn bench_eval_apis(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_api_comparison");
    let empty = HashSet::new();

    // Use a single-variable expression for fair comparison
    let expr_str = "1 / sqrt(1 - v^2)";
    let var = "v";

    let diff_str = symb_anafis::diff(expr_str, var, &[], None).unwrap();
    let diff_expr = parse(&diff_str, &empty, &empty, None).unwrap();

    // Generate 10000 test points in valid domain
    let n_points = 10000;
    let test_points: Vec<f64> = (0..n_points)
        .map(|i| 0.01 + 0.98 * (i as f64 / n_points as f64))
        .collect();

    // ---------------------------------------------------------------------
    // eval_f64 API (optimized for f64, columnar data)
    // ---------------------------------------------------------------------
    group.bench_function("eval_f64", |b| {
        b.iter(|| {
            let result =
                symb_anafis::eval_f64(&[&diff_expr], &[&[var]], &[&[&test_points[..]]]).unwrap();
            black_box(result[0].iter().sum::<f64>())
        })
    });

    // ---------------------------------------------------------------------
    // evaluate_parallel API (general, returns EvalResult)
    // ---------------------------------------------------------------------
    group.bench_function("evaluate_parallel", |b| {
        // Prepare inputs in the format evaluate_parallel expects
        use symb_anafis::parallel::{EvalResult, ExprInput, Value, VarInput, evaluate_parallel};

        let exprs = vec![ExprInput::Parsed(diff_expr.clone())];

        b.iter(|| {
            // Build values for each point
            let var_names = vec![vec![VarInput::from(var)]];
            let values: Vec<Vec<Vec<Value>>> =
                vec![test_points.iter().map(|&v| vec![Value::Num(v)]).collect()];

            let result = evaluate_parallel(exprs.clone(), var_names, values).unwrap();
            // Sum results (extract f64 from EvalResult)
            let sum: f64 = result[0]
                .iter()
                .map(|r| match r {
                    EvalResult::Expr(e) => e.as_number().unwrap_or(0.0),
                    EvalResult::String(s) => s.parse::<f64>().unwrap_or(0.0),
                })
                .sum();
            black_box(sum)
        })
    });

    group.finish();
}

// =============================================================================
// Criterion Setup
// =============================================================================

criterion_group!(
    benches,
    bench_eval_methods,
    bench_eval_scaling,
    bench_multi_expr,
    bench_eval_apis,
);

criterion_main!(benches);
