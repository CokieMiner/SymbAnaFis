use criterion::{Criterion, criterion_group, criterion_main};
use std::collections::HashSet;
use std::hint::black_box;
use symb_anafis::{diff, parse, simplify, simplify_ast};

// Benchmark parsing separately
fn bench_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing");
    let empty: HashSet<String> = HashSet::new();

    group.bench_function("parse_poly_x^3+2x^2+x", |b| {
        b.iter(|| parse(black_box("x^3 + 2*x^2 + x"), &empty, &empty))
    });

    group.bench_function("parse_trig_sin(x)*cos(x)", |b| {
        b.iter(|| parse(black_box("sin(x) * cos(x)"), &empty, &empty))
    });

    group.bench_function("parse_complex_x^2*sin(x)*exp(x)", |b| {
        b.iter(|| parse(black_box("x^2 * sin(x) * exp(x)"), &empty, &empty))
    });

    group.bench_function("parse_nested_sin(cos(tan(x)))", |b| {
        b.iter(|| parse(black_box("sin(cos(tan(x)))"), &empty, &empty))
    });

    group.finish();
}

// Benchmark differentiation on pre-parsed AST (derive method)
fn bench_diff_ast(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_ast_only");
    let empty: HashSet<String> = HashSet::new();

    // Pre-parse expressions
    let poly = parse("x^3 + 2*x^2 + x", &empty, &empty).unwrap();
    let trig = parse("sin(x) * cos(x)", &empty, &empty).unwrap();
    let complex = parse("x^2 * sin(x) * exp(x)", &empty, &empty).unwrap();
    let nested = parse("sin(cos(tan(x)))", &empty, &empty).unwrap();

    group.bench_function("diff_ast_poly", |b| {
        b.iter(|| {
            let expr = black_box(&poly);
            expr.derive("x", &empty)
        })
    });

    group.bench_function("diff_ast_trig", |b| {
        b.iter(|| {
            let expr = black_box(&trig);
            expr.derive("x", &empty)
        })
    });

    group.bench_function("diff_ast_complex", |b| {
        b.iter(|| {
            let expr = black_box(&complex);
            expr.derive("x", &empty)
        })
    });

    group.bench_function("diff_ast_nested", |b| {
        b.iter(|| {
            let expr = black_box(&nested);
            expr.derive("x", &empty)
        })
    });

    group.finish();
}

// Benchmark simplification on pre-parsed AST
fn bench_simplify_ast(c: &mut Criterion) {
    let mut group = c.benchmark_group("simplify_ast_only");
    let empty: HashSet<String> = HashSet::new();

    // Pre-parse expressions
    let pythag = parse("sin(x)^2 + cos(x)^2", &empty, &empty).unwrap();
    let perfect = parse("x^2 + 2*x + 1", &empty, &empty).unwrap();
    let frac = parse("(x + 1)^2 / (x + 1)", &empty, &empty).unwrap();
    let exp_comb = parse("exp(x) * exp(y)", &empty, &empty).unwrap();

    group.bench_function("simplify_ast_pythagorean", |b| {
        b.iter(|| simplify_ast(black_box(pythag.clone())))
    });

    group.bench_function("simplify_ast_perfect_square", |b| {
        b.iter(|| simplify_ast(black_box(perfect.clone())))
    });

    group.bench_function("simplify_ast_fraction", |b| {
        b.iter(|| simplify_ast(black_box(frac.clone())))
    });

    group.bench_function("simplify_ast_exp_combine", |b| {
        b.iter(|| simplify_ast(black_box(exp_comb.clone())))
    });

    group.finish();
}

fn bench_differentiation(c: &mut Criterion) {
    let mut group = c.benchmark_group("differentiation");

    // Simple polynomial
    group.bench_function("poly_x^3+2x^2+x", |b| {
        b.iter(|| {
            diff(
                black_box("x^3 + 2*x^2 + x".to_string()),
                "x".to_string(),
                None,
                None,
            )
        })
    });

    // Trigonometric
    group.bench_function("trig_sin(x)*cos(x)", |b| {
        b.iter(|| {
            diff(
                black_box("sin(x) * cos(x)".to_string()),
                "x".to_string(),
                None,
                None,
            )
        })
    });

    // Chain rule
    group.bench_function("chain_sin(x^2)", |b| {
        b.iter(|| {
            diff(
                black_box("sin(x^2)".to_string()),
                "x".to_string(),
                None,
                None,
            )
        })
    });

    // Exponential
    group.bench_function("exp_e^(x^2)", |b| {
        b.iter(|| {
            diff(
                black_box("exp(x^2)".to_string()),
                "x".to_string(),
                None,
                None,
            )
        })
    });

    // Complex expression
    group.bench_function("complex_x^2*sin(x)*exp(x)", |b| {
        b.iter(|| {
            diff(
                black_box("x^2 * sin(x) * exp(x)".to_string()),
                "x".to_string(),
                None,
                None,
            )
        })
    });

    // Quotient rule
    group.bench_function("quotient_(x^2+1)/(x-1)", |b| {
        b.iter(|| {
            diff(
                black_box("(x^2 + 1) / (x - 1)".to_string()),
                "x".to_string(),
                None,
                None,
            )
        })
    });

    // Nested functions
    group.bench_function("nested_sin(cos(tan(x)))", |b| {
        b.iter(|| {
            diff(
                black_box("sin(cos(tan(x)))".to_string()),
                "x".to_string(),
                None,
                None,
            )
        })
    });

    // Power rule with variable exponent
    group.bench_function("power_x^x", |b| {
        b.iter(|| diff(black_box("x^x".to_string()), "x".to_string(), None, None))
    });

    group.finish();
}

fn bench_simplification(c: &mut Criterion) {
    let mut group = c.benchmark_group("simplification");

    // Pythagorean identity
    group.bench_function("pythagorean_sin^2+cos^2", |b| {
        b.iter(|| simplify(black_box("sin(x)^2 + cos(x)^2".to_string()), None, None))
    });

    // Perfect square
    group.bench_function("perfect_square_x^2+2x+1", |b| {
        b.iter(|| simplify(black_box("x^2 + 2*x + 1".to_string()), None, None))
    });

    // Fraction cancellation
    group.bench_function("fraction_(x+1)^2/(x+1)", |b| {
        b.iter(|| simplify(black_box("(x + 1)^2 / (x + 1)".to_string()), None, None))
    });

    // Exponential combination
    group.bench_function("exp_combine_e^x*e^y", |b| {
        b.iter(|| simplify(black_box("exp(x) * exp(y)".to_string()), None, None))
    });

    // Like terms
    group.bench_function("like_terms_2x+3x+x", |b| {
        b.iter(|| simplify(black_box("2*x + 3*x + x".to_string()), None, None))
    });

    // Complex fraction addition
    group.bench_function("frac_add_(x^2+1)/(x^2-1)+1/(x+1)", |b| {
        b.iter(|| {
            simplify(
                black_box("(x^2 + 1)/(x^2 - 1) + 1/(x + 1)".to_string()),
                None,
                None,
            )
        })
    });

    // Hyperbolic from exponential
    group.bench_function("hyp_sinh_(e^x-e^-x)/2", |b| {
        b.iter(|| simplify(black_box("(exp(x) - exp(-x)) / 2".to_string()), None, None))
    });

    // Power simplification
    group.bench_function("power_x^2*x^3/x", |b| {
        b.iter(|| simplify(black_box("x^2 * x^3 / x".to_string()), None, None))
    });

    group.finish();
}

fn bench_combined(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_and_simplify");

    // Differentiate and simplify
    group.bench_function("d/dx[sin(x)^2]_simplified", |b| {
        b.iter(|| {
            let d = diff(
                black_box("sin(x)^2".to_string()),
                "x".to_string(),
                None,
                None,
            )
            .unwrap();
            simplify(d, None, None)
        })
    });

    group.bench_function("d/dx[(x^2+1)/(x-1)]_simplified", |b| {
        b.iter(|| {
            let d = diff(
                black_box("(x^2 + 1) / (x - 1)".to_string()),
                "x".to_string(),
                None,
                None,
            )
            .unwrap();
            simplify(d, None, None)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parsing,
    bench_diff_ast,
    bench_simplify_ast,
    bench_differentiation,
    bench_simplification,
    bench_combined
);
criterion_main!(benches);
