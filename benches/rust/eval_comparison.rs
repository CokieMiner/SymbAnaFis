//! Evaluation Benchmarks
//!
//! Benchmarks for numerical evaluation of expressions.

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use symb_anafis::evaluate_str;

// =============================================================================
// Test Values
// =============================================================================

const X_VAL: f64 = 2.5;
const Y_VAL: f64 = 1.5;

// =============================================================================
// Basic Function Evaluation
// =============================================================================

fn bench_eval_basic(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_basic");

    group.bench_function("polynomial", |b| {
        b.iter(|| evaluate_str(black_box("x^3 + 2*x^2 + x + 1"), &[("x", X_VAL)]))
    });

    group.bench_function("sin", |b| {
        b.iter(|| evaluate_str(black_box("sin(x)"), &[("x", X_VAL)]))
    });

    group.bench_function("cos", |b| {
        b.iter(|| evaluate_str(black_box("cos(x)"), &[("x", X_VAL)]))
    });

    group.bench_function("exp", |b| {
        b.iter(|| evaluate_str(black_box("exp(x)"), &[("x", X_VAL)]))
    });

    group.bench_function("ln", |b| {
        b.iter(|| evaluate_str(black_box("ln(x)"), &[("x", X_VAL)]))
    });

    group.bench_function("sqrt", |b| {
        b.iter(|| evaluate_str(black_box("sqrt(x)"), &[("x", X_VAL)]))
    });

    group.finish();
}

// =============================================================================
// Special Function Evaluation
// =============================================================================

fn bench_eval_special(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_special");

    group.bench_function("gamma", |b| {
        b.iter(|| evaluate_str(black_box("gamma(x)"), &[("x", X_VAL)]))
    });

    group.bench_function("digamma", |b| {
        b.iter(|| evaluate_str(black_box("digamma(x)"), &[("x", X_VAL)]))
    });

    group.bench_function("trigamma", |b| {
        b.iter(|| evaluate_str(black_box("trigamma(x)"), &[("x", X_VAL)]))
    });

    group.bench_function("erf", |b| {
        b.iter(|| evaluate_str(black_box("erf(x)"), &[("x", X_VAL)]))
    });

    group.bench_function("erfc", |b| {
        b.iter(|| evaluate_str(black_box("erfc(x)"), &[("x", X_VAL)]))
    });

    group.bench_function("zeta", |b| {
        b.iter(|| evaluate_str(black_box("zeta(x)"), &[("x", X_VAL)]))
    });

    group.bench_function("lambertw", |b| {
        b.iter(|| evaluate_str(black_box("LambertW(x)"), &[("x", X_VAL)]))
    });

    group.finish();
}

// =============================================================================
// Bessel Function Evaluation
// =============================================================================

fn bench_eval_bessel(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_bessel");

    group.bench_function("besselj_0", |b| {
        b.iter(|| evaluate_str(black_box("besselj(0, x)"), &[("x", X_VAL)]))
    });

    group.bench_function("besselj_1", |b| {
        b.iter(|| evaluate_str(black_box("besselj(1, x)"), &[("x", X_VAL)]))
    });

    group.bench_function("bessely_0", |b| {
        b.iter(|| evaluate_str(black_box("bessely(0, x)"), &[("x", X_VAL)]))
    });

    group.bench_function("bessely_1", |b| {
        b.iter(|| evaluate_str(black_box("bessely(1, x)"), &[("x", X_VAL)]))
    });

    group.bench_function("besseli_0", |b| {
        b.iter(|| evaluate_str(black_box("besseli(0, x)"), &[("x", X_VAL)]))
    });

    group.bench_function("besselk_0", |b| {
        b.iter(|| evaluate_str(black_box("besselk(0, x)"), &[("x", X_VAL)]))
    });

    group.finish();
}

// =============================================================================
// Polygamma Function Evaluation
// =============================================================================

fn bench_eval_polygamma(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_polygamma");

    group.bench_function("polygamma_2", |b| {
        b.iter(|| evaluate_str(black_box("polygamma(2, x)"), &[("x", X_VAL)]))
    });

    group.bench_function("polygamma_3", |b| {
        b.iter(|| evaluate_str(black_box("polygamma(3, x)"), &[("x", X_VAL)]))
    });

    group.bench_function("polygamma_4", |b| {
        b.iter(|| evaluate_str(black_box("polygamma(4, x)"), &[("x", X_VAL)]))
    });

    group.bench_function("tetragamma", |b| {
        b.iter(|| evaluate_str(black_box("tetragamma(x)"), &[("x", X_VAL)]))
    });

    group.finish();
}

// =============================================================================
// Complex Expression Evaluation
// =============================================================================

fn bench_eval_complex(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_complex");

    group.bench_function("trig_combo", |b| {
        b.iter(|| evaluate_str(black_box("sin(x)^2 + cos(x)^2"), &[("x", X_VAL)]))
    });

    group.bench_function("exp_poly", |b| {
        b.iter(|| evaluate_str(black_box("exp(x^2) * sin(x)"), &[("x", X_VAL)]))
    });

    group.bench_function("nested", |b| {
        b.iter(|| evaluate_str(black_box("sin(cos(tan(x)))"), &[("x", X_VAL)]))
    });

    group.bench_function("multivar", |b| {
        b.iter(|| {
            evaluate_str(
                black_box("x^2 + y^2 + 2*x*y"),
                &[("x", X_VAL), ("y", Y_VAL)],
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
    bench_eval_basic,
    bench_eval_special,
    bench_eval_bessel,
    bench_eval_polygamma,
    bench_eval_complex,
);

criterion_main!(benches);
