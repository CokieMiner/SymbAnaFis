//! Core SymbAnaFis Benchmarks
//!
//! Benchmarks for parsing, differentiation, simplification, and combined operations.

use criterion::{Criterion, criterion_group, criterion_main};
use std::collections::HashSet;
use std::hint::black_box;
use symb_anafis::{Diff, diff, parse, simplify};

// =============================================================================
// Test Expressions
// =============================================================================

const POLYNOMIAL: &str = "x^3 + 2*x^2 + x + 1";
const TRIG_SIMPLE: &str = "sin(x) * cos(x)";
const COMPLEX_EXPR: &str = "x^2 * sin(x) * exp(x)";
const NESTED_TRIG: &str = "sin(cos(tan(x)))";
const CHAIN_SIN: &str = "sin(x^2)";
const EXP_SQUARED: &str = "exp(x^2)";
const QUOTIENT: &str = "(x^2 + 1) / (x - 1)";
const POWER_XX: &str = "x^x";

// Simplification expressions
const PYTHAGOREAN: &str = "sin(x)^2 + cos(x)^2";
const PERFECT_SQUARE: &str = "x^2 + 2*x + 1";
const FRACTION_CANCEL: &str = "(x^2 - 1) / (x - 1)";
const EXP_COMBINE: &str = "exp(x) * exp(y)";
const LIKE_TERMS: &str = "2*x + 3*x + x";
const HYPERBOLIC: &str = "(exp(x) - exp(-x)) / 2";
const FRAC_ADD: &str = "(x^2 + 1) / (x + 1) + (x - 1) / (x + 1)";
const POWER_COMBINE: &str = "x^2 * x^3";

// Large physics expressions (for Rayon parallelism testing)
const NORMAL_PDF: &str = "exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)";
const WAVE_EQUATION: &str = "A * sin(k*x - omega*t) * exp(-gamma*t)";
const GAUSSIAN_2D: &str = "exp(-((x - x0)^2 + (y - y0)^2) / (2 * sigma^2)) / (2 * pi * sigma^2)";
const MAXWELL_BOLTZMANN: &str =
    "4 * pi * (m / (2 * pi * k * T))^(3/2) * v^2 * exp(-m * v^2 / (2 * k * T))";
const ORBITAL_ENERGY: &str = "-G * M * m / (2 * a) + L^2 / (2 * m * r^2) - G * M * m / r";
const SCHRODINGER_1D: &str = "(-hbar^2 / (2 * m)) * d2psi + V * psi";

// =============================================================================
// Parsing Benchmarks
// =============================================================================

fn bench_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing");
    let empty_set = HashSet::new();

    group.bench_function("polynomial", |b| {
        b.iter(|| parse(black_box(POLYNOMIAL), &empty_set, &empty_set, None))
    });

    group.bench_function("trig_simple", |b| {
        b.iter(|| parse(black_box(TRIG_SIMPLE), &empty_set, &empty_set, None))
    });

    group.bench_function("complex_expr", |b| {
        b.iter(|| parse(black_box(COMPLEX_EXPR), &empty_set, &empty_set, None))
    });

    group.bench_function("nested_trig", |b| {
        b.iter(|| parse(black_box(NESTED_TRIG), &empty_set, &empty_set, None))
    });

    group.finish();
}

// =============================================================================
// Differentiation Benchmarks (AST Only - Using Diff builder)
// =============================================================================

fn bench_diff_ast_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_ast_only");
    let empty_set = HashSet::new();

    // Pre-parse expressions for AST-only benchmarking
    let poly_expr = parse(POLYNOMIAL, &empty_set, &empty_set, None).unwrap();
    let trig_expr = parse(TRIG_SIMPLE, &empty_set, &empty_set, None).unwrap();
    let complex_expr = parse(COMPLEX_EXPR, &empty_set, &empty_set, None).unwrap();
    let nested_expr = parse(NESTED_TRIG, &empty_set, &empty_set, None).unwrap();

    let x = symb_anafis::symb("x");

    group.bench_function("polynomial", |b| {
        b.iter(|| Diff::new().differentiate(black_box(poly_expr.clone()), black_box(&x)))
    });

    group.bench_function("trig_simple", |b| {
        b.iter(|| Diff::new().differentiate(black_box(trig_expr.clone()), black_box(&x)))
    });

    group.bench_function("complex_expr", |b| {
        b.iter(|| Diff::new().differentiate(black_box(complex_expr.clone()), black_box(&x)))
    });

    group.bench_function("nested_trig", |b| {
        b.iter(|| Diff::new().differentiate(black_box(nested_expr.clone()), black_box(&x)))
    });

    group.finish();
}

// =============================================================================
// Differentiation Benchmarks (Full Pipeline)
// =============================================================================

fn bench_diff_full(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_full");

    group.bench_function("polynomial", |b| {
        b.iter(|| diff(black_box(POLYNOMIAL), "x", None, None))
    });

    group.bench_function("trig_simple", |b| {
        b.iter(|| diff(black_box(TRIG_SIMPLE), "x", None, None))
    });

    group.bench_function("chain_sin", |b| {
        b.iter(|| diff(black_box(CHAIN_SIN), "x", None, None))
    });

    group.bench_function("exp_squared", |b| {
        b.iter(|| diff(black_box(EXP_SQUARED), "x", None, None))
    });

    group.bench_function("complex_expr", |b| {
        b.iter(|| diff(black_box(COMPLEX_EXPR), "x", None, None))
    });

    group.bench_function("quotient", |b| {
        b.iter(|| diff(black_box(QUOTIENT), "x", None, None))
    });

    group.bench_function("nested_trig", |b| {
        b.iter(|| diff(black_box(NESTED_TRIG), "x", None, None))
    });

    group.bench_function("power_xx", |b| {
        b.iter(|| diff(black_box(POWER_XX), "x", None, None))
    });

    group.finish();
}

// =============================================================================
// Simplification Benchmarks
// =============================================================================

fn bench_simplification(c: &mut Criterion) {
    let mut group = c.benchmark_group("simplification");

    group.bench_function("pythagorean", |b| {
        b.iter(|| simplify(black_box(PYTHAGOREAN), None, None))
    });

    group.bench_function("perfect_square", |b| {
        b.iter(|| simplify(black_box(PERFECT_SQUARE), None, None))
    });

    group.bench_function("fraction_cancel", |b| {
        b.iter(|| simplify(black_box(FRACTION_CANCEL), None, None))
    });

    group.bench_function("exp_combine", |b| {
        b.iter(|| simplify(black_box(EXP_COMBINE), None, None))
    });

    group.bench_function("like_terms", |b| {
        b.iter(|| simplify(black_box(LIKE_TERMS), None, None))
    });

    group.bench_function("hyperbolic", |b| {
        b.iter(|| simplify(black_box(HYPERBOLIC), None, None))
    });

    group.bench_function("frac_add", |b| {
        b.iter(|| simplify(black_box(FRAC_ADD), None, None))
    });

    group.bench_function("power_combine", |b| {
        b.iter(|| simplify(black_box(POWER_COMBINE), None, None))
    });

    group.finish();
}

// =============================================================================
// Large Expression Benchmarks (Testing Rayon parallelism benefits)
// =============================================================================

fn bench_large_expressions(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_expr");

    // Normal PDF - ~30 nodes
    group.bench_function("normal_pdf", |b| {
        b.iter(|| diff(black_box(NORMAL_PDF), "x", Some(&["mu", "sigma"]), None))
    });

    // Wave equation - ~20 nodes
    group.bench_function("wave_equation", |b| {
        b.iter(|| {
            diff(
                black_box(WAVE_EQUATION),
                "x",
                Some(&["A", "k", "omega", "gamma"]),
                None,
            )
        })
    });

    // 2D Gaussian - ~40 nodes
    group.bench_function("gaussian_2d", |b| {
        b.iter(|| {
            diff(
                black_box(GAUSSIAN_2D),
                "x",
                Some(&["x0", "y0", "sigma"]),
                None,
            )
        })
    });

    // Maxwell-Boltzmann - ~50 nodes
    group.bench_function("maxwell_boltzmann", |b| {
        b.iter(|| {
            diff(
                black_box(MAXWELL_BOLTZMANN),
                "v",
                Some(&["m", "k", "T"]),
                None,
            )
        })
    });

    // Orbital energy - ~35 nodes
    group.bench_function("orbital_energy", |b| {
        b.iter(|| {
            diff(
                black_box(ORBITAL_ENERGY),
                "r",
                Some(&["G", "M", "m", "a", "L"]),
                None,
            )
        })
    });

    // Schrodinger 1D - ~25 nodes (with second derivative as symbol)
    group.bench_function("schrodinger_1d", |b| {
        b.iter(|| {
            diff(
                black_box(SCHRODINGER_1D),
                "x",
                Some(&["hbar", "m", "V", "psi", "d2psi"]),
                None,
            )
        })
    });

    group.finish();
}

// =============================================================================
// Combined Operations (Real-World Scenarios)
// =============================================================================

fn bench_combined(c: &mut Criterion) {
    let mut group = c.benchmark_group("combined");

    // Differentiate and simplify sin^2(x)
    group.bench_function("diff_sin_squared", |b| {
        b.iter(|| diff(black_box("sin(x)^2"), "x", None, None))
    });

    // Differentiate and simplify quotient
    group.bench_function("diff_quotient", |b| {
        b.iter(|| diff(black_box(QUOTIENT), "x", None, None))
    });

    // Physics: RC circuit voltage derivative
    group.bench_function("physics_rc_circuit", |b| {
        b.iter(|| {
            diff(
                black_box("V0 * exp(-t / (R * C))"),
                "t",
                Some(&["V0", "R", "C"]),
                None,
            )
        })
    });

    // Statistics: Normal distribution derivative
    group.bench_function("stats_normal_pdf", |b| {
        b.iter(|| {
            diff(
                black_box("exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)"),
                "x",
                Some(&["mu", "sigma"]),
                None,
            )
        })
    });

    group.finish();
}

// =============================================================================
// Criterion Setup
// =============================================================================

criterion_group!(
    benches,
    bench_parsing,
    bench_diff_ast_only,
    bench_diff_full,
    bench_simplification,
    bench_large_expressions,
    bench_combined,
);

criterion_main!(benches);
