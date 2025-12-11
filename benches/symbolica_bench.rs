use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use symbolica::{atom::AtomCore, parse, symbol};

// ==============================================================================
// Symbolica Benchmarks for Comparison with symb_anafis
// ==============================================================================

// Benchmark parsing separately
fn bench_symbolica_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("symbolica_parsing");

    group.bench_function("parse_poly_x^3+2x^2+x", |b| {
        b.iter(|| parse!(black_box("x^3 + 2*x^2 + x")))
    });

    group.bench_function("parse_trig_sin(x)*cos(x)", |b| {
        b.iter(|| parse!(black_box("sin(x) * cos(x)")))
    });

    group.bench_function("parse_complex_x^2*sin(x)*exp(x)", |b| {
        b.iter(|| parse!(black_box("x^2 * sin(x) * exp(x)")))
    });

    group.bench_function("parse_nested_sin(cos(tan(x)))", |b| {
        b.iter(|| parse!(black_box("sin(cos(tan(x)))")))
    });

    group.finish();
}

// Benchmark differentiation on pre-parsed AST
fn bench_symbolica_diff_ast(c: &mut Criterion) {
    let mut group = c.benchmark_group("symbolica_diff_ast");

    // Pre-parse expressions
    let poly = parse!("x^3 + 2*x^2 + x");
    let trig = parse!("sin(x) * cos(x)");
    let complex = parse!("x^2 * sin(x) * exp(x)");
    let nested = parse!("sin(cos(tan(x)))");
    let x = symbol!("x");

    group.bench_function("diff_ast_poly", |b| {
        b.iter(|| black_box(&poly).derivative(x))
    });

    group.bench_function("diff_ast_trig", |b| {
        b.iter(|| black_box(&trig).derivative(x))
    });

    group.bench_function("diff_ast_complex", |b| {
        b.iter(|| black_box(&complex).derivative(x))
    });

    group.bench_function("diff_ast_nested", |b| {
        b.iter(|| black_box(&nested).derivative(x))
    });

    group.finish();
}

// Benchmark differentiation including parsing (full pipeline)
fn bench_symbolica_differentiation(c: &mut Criterion) {
    let mut group = c.benchmark_group("symbolica_differentiation");
    let x = symbol!("x");

    // Simple polynomial
    group.bench_function("poly_x^3+2x^2+x", |b| {
        b.iter(|| {
            let expr = parse!(black_box("x^3 + 2*x^2 + x"));
            expr.derivative(x)
        })
    });

    // Trigonometric
    group.bench_function("trig_sin(x)*cos(x)", |b| {
        b.iter(|| {
            let expr = parse!(black_box("sin(x) * cos(x)"));
            expr.derivative(x)
        })
    });

    // Chain rule
    group.bench_function("chain_sin(x^2)", |b| {
        b.iter(|| {
            let expr = parse!(black_box("sin(x^2)"));
            expr.derivative(x)
        })
    });

    // Exponential
    group.bench_function("exp_e^(x^2)", |b| {
        b.iter(|| {
            let expr = parse!(black_box("exp(x^2)"));
            expr.derivative(x)
        })
    });

    // Complex expression
    group.bench_function("complex_x^2*sin(x)*exp(x)", |b| {
        b.iter(|| {
            let expr = parse!(black_box("x^2 * sin(x) * exp(x)"));
            expr.derivative(x)
        })
    });

    // Quotient
    group.bench_function("quotient_(x^2+1)/(x-1)", |b| {
        b.iter(|| {
            let expr = parse!(black_box("(x^2 + 1) / (x - 1)"));
            expr.derivative(x)
        })
    });

    // Nested functions
    group.bench_function("nested_sin(cos(tan(x)))", |b| {
        b.iter(|| {
            let expr = parse!(black_box("sin(cos(tan(x)))"));
            expr.derivative(x)
        })
    });

    // Power rule with variable exponent
    group.bench_function("power_x^x", |b| {
        b.iter(|| {
            let expr = parse!(black_box("x^x"));
            expr.derivative(x)
        })
    });

    group.finish();
}

// Note: Symbolica evaluation benchmarks not included.
// The Symbolica 1.0.1 evaluate API requires a complex type signature with
// closures for coefficient conversion that's incompatible with simple HashMap-based
// benchmarking. For evaluation comparison, see SymbAnaFis and SymPy benchmarks.
//
// Key difference: Symbolica focuses on compiled evaluators for high-performance
// repeated evaluation (code generation), while SymbAnaFis focuses on direct
// evaluation of the AST.

criterion_group!(
    benches,
    bench_symbolica_parsing,
    bench_symbolica_diff_ast,
    bench_symbolica_differentiation,
);
criterion_main!(benches);
