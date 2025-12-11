use criterion::{Criterion, criterion_group, criterion_main};
use std::collections::{HashMap, HashSet};
use std::hint::black_box;
use symb_anafis::{diff, parse, simplify, simplify_expr};

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

// Benchmark differentiation on pre-parsed AST (raw derive only, no simplification)
fn bench_diff_ast(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_ast_only");
    let empty: HashSet<String> = HashSet::new();

    // Pre-parse expressions
    let poly = parse("x^3 + 2*x^2 + x", &empty, &empty).unwrap();
    let trig = parse("sin(x) * cos(x)", &empty, &empty).unwrap();
    let complex = parse("x^2 * sin(x) * exp(x)", &empty, &empty).unwrap();
    let nested = parse("sin(cos(tan(x)))", &empty, &empty).unwrap();

    group.bench_function("diff_ast_poly", |b| {
        b.iter(|| black_box(poly.clone()).derive_raw("x"))
    });

    group.bench_function("diff_ast_trig", |b| {
        b.iter(|| black_box(trig.clone()).derive_raw("x"))
    });

    group.bench_function("diff_ast_complex", |b| {
        b.iter(|| black_box(complex.clone()).derive_raw("x"))
    });

    group.bench_function("diff_ast_nested", |b| {
        b.iter(|| black_box(nested.clone()).derive_raw("x"))
    });

    group.finish();
}

// Benchmark differentiation with caching
fn bench_diff_cached(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_cached");
    let empty: HashSet<String> = HashSet::new();

    // Pre-parse expressions
    let poly = parse("x^3 + 2*x^2 + x", &empty, &empty).unwrap();
    let trig = parse("sin(x) * cos(x)", &empty, &empty).unwrap();
    let complex = parse("x^2 * sin(x) * exp(x)", &empty, &empty).unwrap();
    let nested = parse("sin(cos(tan(x)))", &empty, &empty).unwrap();
    // Pathological cases for repeated subexpressions
    let repeated = parse("(x+1)*(x+1)*(x+1)*(x+1)", &empty, &empty).unwrap();

    group.bench_function("diff_cached_poly", |b| {
        b.iter(|| {
            let mut cache = std::collections::HashMap::new();
            black_box(poly.clone()).derive_cached("x", &mut cache)
        })
    });

    group.bench_function("diff_cached_trig", |b| {
        b.iter(|| {
            let mut cache = std::collections::HashMap::new();
            black_box(trig.clone()).derive_cached("x", &mut cache)
        })
    });

    group.bench_function("diff_cached_complex", |b| {
        b.iter(|| {
            let mut cache = std::collections::HashMap::new();
            black_box(complex.clone()).derive_cached("x", &mut cache)
        })
    });

    group.bench_function("diff_cached_nested", |b| {
        b.iter(|| {
            let mut cache = std::collections::HashMap::new();
            black_box(nested.clone()).derive_cached("x", &mut cache)
        })
    });

    group.bench_function("diff_cached_repeated", |b| {
        b.iter(|| {
            let mut cache = std::collections::HashMap::new();
            black_box(repeated.clone()).derive_cached("x", &mut cache)
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
        b.iter(|| simplify_expr(black_box(pythag.clone()), HashSet::new()))
    });

    group.bench_function("simplify_ast_perfect_square", |b| {
        b.iter(|| simplify_expr(black_box(perfect.clone()), HashSet::new()))
    });

    group.bench_function("simplify_ast_fraction", |b| {
        b.iter(|| simplify_expr(black_box(frac.clone()), HashSet::new()))
    });

    group.bench_function("simplify_ast_exp_combine", |b| {
        b.iter(|| simplify_expr(black_box(exp_comb.clone()), HashSet::new()))
    });

    group.finish();
}

fn bench_differentiation(c: &mut Criterion) {
    let mut group = c.benchmark_group("differentiation");

    // Simple polynomial
    group.bench_function("poly_x^3+2x^2+x", |b| {
        b.iter(|| diff(black_box("x^3 + 2*x^2 + x"), "x", None, None))
    });

    // Trigonometric
    group.bench_function("trig_sin(x)*cos(x)", |b| {
        b.iter(|| diff(black_box("sin(x) * cos(x)"), "x", None, None))
    });

    // Chain rule
    group.bench_function("chain_sin(x^2)", |b| {
        b.iter(|| diff(black_box("sin(x^2)"), "x", None, None))
    });

    // Exponential
    group.bench_function("exp_e^(x^2)", |b| {
        b.iter(|| diff(black_box("exp(x^2)"), "x", None, None))
    });

    // Complex expression
    group.bench_function("complex_x^2*sin(x)*exp(x)", |b| {
        b.iter(|| diff(black_box("x^2 * sin(x) * exp(x)"), "x", None, None))
    });

    // Quotient rule
    group.bench_function("quotient_(x^2+1)/(x-1)", |b| {
        b.iter(|| diff(black_box("(x^2 + 1) / (x - 1)"), "x", None, None))
    });

    // Nested functions
    group.bench_function("nested_sin(cos(tan(x)))", |b| {
        b.iter(|| diff(black_box("sin(cos(tan(x)))"), "x", None, None))
    });

    // Power rule with variable exponent
    group.bench_function("power_x^x", |b| {
        b.iter(|| diff(black_box("x^x"), "x", None, None))
    });

    group.finish();
}

fn bench_simplification(c: &mut Criterion) {
    let mut group = c.benchmark_group("simplification");

    // Pythagorean identity
    group.bench_function("pythagorean_sin^2+cos^2", |b| {
        b.iter(|| simplify(black_box("sin(x)^2 + cos(x)^2"), None, None))
    });

    // Perfect square
    group.bench_function("perfect_square_x^2+2x+1", |b| {
        b.iter(|| simplify(black_box("x^2 + 2*x + 1"), None, None))
    });

    // Fraction cancellation
    group.bench_function("fraction_(x+1)^2/(x+1)", |b| {
        b.iter(|| simplify(black_box("(x + 1)^2 / (x + 1)"), None, None))
    });

    // Exponential combination
    group.bench_function("exp_combine_e^x*e^y", |b| {
        b.iter(|| simplify(black_box("exp(x) * exp(y)"), None, None))
    });

    // Like terms
    group.bench_function("like_terms_2x+3x+x", |b| {
        b.iter(|| simplify(black_box("2*x + 3*x + x"), None, None))
    });

    // Complex fraction addition
    group.bench_function("frac_add_(x^2+1)/(x^2-1)+1/(x+1)", |b| {
        b.iter(|| simplify(black_box("(x^2 + 1)/(x^2 - 1) + 1/(x + 1)"), None, None))
    });

    // Hyperbolic from exponential
    group.bench_function("hyp_sinh_(e^x-e^-x)/2", |b| {
        b.iter(|| simplify(black_box("(exp(x) - exp(-x)) / 2"), None, None))
    });

    // Power simplification
    group.bench_function("power_x^2*x^3/x", |b| {
        b.iter(|| simplify(black_box("x^2 * x^3 / x"), None, None))
    });

    group.finish();
}

fn bench_combined(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_and_simplify");

    // Differentiate and simplify
    group.bench_function("d/dx[sin(x)^2]_simplified", |b| {
        b.iter(|| {
            let d = diff(black_box("sin(x)^2"), "x", None, None).unwrap();
            simplify(&d, None, None)
        })
    });

    group.bench_function("d/dx[(x^2+1)/(x-1)]_simplified", |b| {
        b.iter(|| {
            let d = diff(black_box("(x^2 + 1) / (x - 1)"), "x", None, None).unwrap();
            simplify(&d, None, None)
        })
    });

    group.finish();
}

// Benchmark evaluation (Expression + values -> Number)
fn bench_evaluation(c: &mut Criterion) {
    let mut group = c.benchmark_group("evaluation");
    let empty: HashSet<String> = HashSet::new();

    // Pre-parse expressions - focus on special functions with custom implementations
    let poly = parse("x^3 + 2*x^2 + x + 1", &empty, &empty).unwrap();
    let gamma_expr = parse("gamma(x)", &empty, &empty).unwrap();
    let digamma_expr = parse("digamma(x)", &empty, &empty).unwrap();
    let trigamma_expr = parse("trigamma(x)", &empty, &empty).unwrap();
    let bessel_expr = parse("besselj(0, x)", &empty, &empty).unwrap();
    let bessel_y_expr = parse("bessely(1, x)", &empty, &empty).unwrap();
    let zeta_expr = parse("zeta(x)", &empty, &empty).unwrap();
    let zeta_deriv_expr = parse("zeta_deriv(1, x)", &empty, &empty).unwrap();
    let erf_expr = parse("erf(x)", &empty, &empty).unwrap();
    let lambertw_expr = parse("lambertw(x)", &empty, &empty).unwrap();
    let complex_special = parse("gamma(x) * besselj(0, x) + erf(x)", &empty, &empty).unwrap();

    // Pre-build variable maps
    let mut vars: HashMap<&str, f64> = HashMap::new();
    vars.insert("x", 2.5);

    group.bench_function("eval_polynomial", |b| {
        b.iter(|| black_box(&poly).evaluate(&vars))
    });

    group.bench_function("eval_gamma", |b| {
        b.iter(|| black_box(&gamma_expr).evaluate(&vars))
    });

    group.bench_function("eval_digamma", |b| {
        b.iter(|| black_box(&digamma_expr).evaluate(&vars))
    });

    group.bench_function("eval_trigamma", |b| {
        b.iter(|| black_box(&trigamma_expr).evaluate(&vars))
    });

    group.bench_function("eval_besselj(0,x)", |b| {
        b.iter(|| black_box(&bessel_expr).evaluate(&vars))
    });

    group.bench_function("eval_bessely(1,x)", |b| {
        b.iter(|| black_box(&bessel_y_expr).evaluate(&vars))
    });

    group.bench_function("eval_zeta", |b| {
        b.iter(|| black_box(&zeta_expr).evaluate(&vars))
    });

    group.bench_function("eval_zeta_deriv(1,x)", |b| {
        b.iter(|| black_box(&zeta_deriv_expr).evaluate(&vars))
    });

    group.bench_function("eval_erf", |b| {
        b.iter(|| black_box(&erf_expr).evaluate(&vars))
    });

    group.bench_function("eval_lambertw", |b| {
        b.iter(|| black_box(&lambertw_expr).evaluate(&vars))
    });

    group.bench_function("eval_complex_special", |b| {
        b.iter(|| black_box(&complex_special).evaluate(&vars))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parsing,
    bench_diff_ast,
    bench_diff_cached,
    bench_simplify_ast,
    bench_differentiation,
    bench_simplification,
    bench_combined,
    bench_evaluation
);
criterion_main!(benches);
