//! Large Expression Benchmark
//!
//! Benchmarks for expressions with many mixed terms (N >= 300).
//! Compares `SymbAnaFis` vs Symbolica across: Parse, Diff, Compile, Eval.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::collections::HashSet;
use std::fmt::Write;
use std::hint::black_box;
use symb_anafis::{CompiledEvaluator, Diff};

// Load .env file for SYMBOLICA_LICENSE
fn init() {
    let _ = dotenvy::dotenv();
}

use symbolica::{atom::AtomCore, parse, symbol};

// =============================================================================
// Complex Expression Generator
// =============================================================================

/// Generates a complex mixed expression with N terms
/// Includes: polynomials, trig, exponentials, fractions, and nested functions
fn generate_mixed_complex(n: usize) -> String {
    let mut s = String::with_capacity(n * 50);
    for i in 1..=n {
        if i > 1 {
            // Mix operators: mostly + to avoid expression explosion
            if i % 4 == 0 {
                write!(s, " - ").unwrap();
            } else {
                write!(s, " + ").unwrap();
            }
        }

        // Mix term types based on index
        match i % 5 {
            0 => {
                // Polynomial term: i*x^(i mod 10 + 1)
                write!(s, "{}*x^{}", i, i % 10 + 1).unwrap();
            }
            1 => {
                // Trig term: sin(i*x) * cos(x)
                write!(s, "sin({i}*x)*cos(x)").unwrap();
            }
            2 => {
                // Exponential/Sqrt: exp(x/i) + sqrt(x + i)
                write!(s, "(exp(x/{i}) + sqrt(x + {i}))").unwrap();
            }
            3 => {
                // Rational: (x^2 + i) / (x + i)
                write!(s, "(x^2 + {i})/(x + {i})").unwrap();
            }
            4 => {
                // Nested: sin(exp(x) + i)
                write!(s, "sin(exp(x) + {i})").unwrap();
            }
            _ => unreachable!(),
        }
    }
    s
}

// =============================================================================
// Benchmarks
// =============================================================================

fn bench_large_expressions(c: &mut Criterion) {
    init();

    let sizes = [100, 300];

    for n in sizes {
        let group_name = format!("large_expr_{n}");
        let mut group = c.benchmark_group(&group_name);
        group.sample_size(10);
        group.measurement_time(std::time::Duration::from_secs(10));

        let expr_str = generate_mixed_complex(n);
        let empty_set = HashSet::new();
        let x_sym = symbol!("x");
        let x_symb = symb_anafis::symb("x");

        // ---------------------------------------------------------------------
        // 1. PARSING
        // ---------------------------------------------------------------------

        group.bench_function("1_parse/symb_anafis", |b| {
            b.iter(|| symb_anafis::parse(black_box(&expr_str), &empty_set, &empty_set, None));
        });

        group.bench_function("1_parse/symbolica", |b| {
            b.iter(|| parse!(black_box(&expr_str)));
        });

        // Pre-parse for subsequent benchmarks
        let sa_expr = symb_anafis::parse(&expr_str, &empty_set, &empty_set, None).unwrap();
        let sy_expr = parse!(&expr_str);

        // ---------------------------------------------------------------------
        // 2. DIFFERENTIATION (from pre-parsed AST)
        // ---------------------------------------------------------------------

        // SymbAnaFis: diff only (no simplification) - fair comparison with Symbolica
        group.bench_function("2_diff/symb_anafis_diff_only", |b| {
            b.iter(|| {
                Diff::new()
                    .skip_simplification(true)
                    .differentiate(black_box(&sa_expr), black_box(&x_symb))
            });
        });

        // SymbAnaFis: diff + full simplification
        group.bench_function("2_diff/symb_anafis_diff+simplify", |b| {
            b.iter(|| Diff::new().differentiate(black_box(&sa_expr), black_box(&x_symb)));
        });

        // Symbolica: diff (light auto-simplification)
        group.bench_function("2_diff/symbolica", |b| {
            b.iter(|| black_box(&sy_expr).derivative(black_box(x_sym)));
        });

        // ---------------------------------------------------------------------
        // 3. COMPILATION (derivative -> evaluator)
        // ---------------------------------------------------------------------

        // Pre-compute derivatives
        let sa_deriv_light = Diff::new()
            .skip_simplification(true)
            .differentiate(&sa_expr, &x_symb)
            .unwrap();
        let sa_deriv_full = Diff::new().differentiate(&sa_expr, &x_symb).unwrap();
        let sy_deriv = sy_expr.derivative(x_sym);

        group.bench_function("3_compile/symb_anafis_raw", |b| {
            b.iter(|| CompiledEvaluator::compile_auto(black_box(&sa_deriv_light), None));
        });

        group.bench_function("3_compile/symb_anafis_simplified", |b| {
            b.iter(|| CompiledEvaluator::compile_auto(black_box(&sa_deriv_full), None));
        });

        let fn_map = symbolica::evaluate::FunctionMap::new();
        let params = vec![x_sym.into()];

        group.bench_function("3_compile/symbolica", |b| {
            b.iter(|| {
                sy_deriv.as_view().evaluator(
                    black_box(&fn_map),
                    black_box(&params),
                    symbolica::evaluate::OptimizationSettings::default(),
                )
            });
        });

        // ---------------------------------------------------------------------
        // 4. EVALUATION (1000 points, compiled)
        // ---------------------------------------------------------------------

        let test_points: Vec<f64> = (0..1000).map(|i| f64::from(i).mul_add(0.01, 0.1)).collect();

        // SymbAnaFis compiled evaluators
        let sa_eval_raw = CompiledEvaluator::compile_auto(&sa_deriv_light, None).unwrap();
        let sa_eval_simp = CompiledEvaluator::compile_auto(&sa_deriv_full, None).unwrap();

        group.bench_with_input(
            BenchmarkId::new("4_eval_1000pts/symb_anafis_raw", ""),
            &test_points,
            |b, points| {
                b.iter(|| {
                    let mut sum = 0.0;
                    for &x in points {
                        sum += sa_eval_raw.evaluate(&[x]);
                    }
                    sum
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("4_eval_1000pts/symb_anafis_simplified", ""),
            &test_points,
            |b, points| {
                b.iter(|| {
                    let mut sum = 0.0;
                    for &x in points {
                        sum += sa_eval_simp.evaluate(&[x]);
                    }
                    sum
                });
            },
        );

        // Symbolica compiled evaluator
        let sy_evaluator = sy_deriv
            .as_view()
            .evaluator(
                &fn_map,
                &params,
                symbolica::evaluate::OptimizationSettings::default(),
            )
            .unwrap();
        let mut sy_eval_f64 = sy_evaluator.map_coeff(&|x| x.to_real().unwrap().into());

        group.bench_with_input(
            BenchmarkId::new("4_eval_1000pts/symbolica", ""),
            &test_points,
            |b, points| {
                b.iter(|| {
                    let mut sum = 0.0;
                    let mut output = [0.0_f64];
                    for &x in points {
                        sy_eval_f64.evaluate(&[x], &mut output);
                        sum += output[0];
                    }
                    sum
                });
            },
        );

        group.finish();
    }
}

criterion_group!(benches, bench_large_expressions);

criterion_main!(benches);
