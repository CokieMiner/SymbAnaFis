//! SymbAnaFis vs Symbolica Benchmark Comparison
//!
//! Head-to-head comparison of parsing and differentiation performance.

use criterion::{Criterion, criterion_group, criterion_main};
use std::collections::HashSet;
use std::hint::black_box;
use symb_anafis::{Diff, diff};

// Load .env file for SYMBOLICA_LICENSE
fn init() {
    let _ = dotenvy::dotenv();
}

use symbolica::{atom::AtomCore, parse, symbol};

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

// =============================================================================
// Parsing Comparison
// =============================================================================

fn bench_parsing_comparison(c: &mut Criterion) {
    init(); // Load license from .env
    let mut group = c.benchmark_group("parsing_comparison");
    let empty_set = HashSet::new();

    // SymbAnaFis parsing
    group.bench_function("symb_anafis/polynomial", |b| {
        b.iter(|| symb_anafis::parse(black_box(POLYNOMIAL), &empty_set, &empty_set))
    });

    group.bench_function("symb_anafis/trig_simple", |b| {
        b.iter(|| symb_anafis::parse(black_box(TRIG_SIMPLE), &empty_set, &empty_set))
    });

    group.bench_function("symb_anafis/complex_expr", |b| {
        b.iter(|| symb_anafis::parse(black_box(COMPLEX_EXPR), &empty_set, &empty_set))
    });

    group.bench_function("symb_anafis/nested_trig", |b| {
        b.iter(|| symb_anafis::parse(black_box(NESTED_TRIG), &empty_set, &empty_set))
    });

    // Symbolica parsing
    group.bench_function("symbolica/polynomial", |b| {
        b.iter(|| parse!(black_box(POLYNOMIAL)))
    });

    group.bench_function("symbolica/trig_simple", |b| {
        b.iter(|| parse!(black_box(TRIG_SIMPLE)))
    });

    group.bench_function("symbolica/complex_expr", |b| {
        b.iter(|| parse!(black_box(COMPLEX_EXPR)))
    });

    group.bench_function("symbolica/nested_trig", |b| {
        b.iter(|| parse!(black_box(NESTED_TRIG)))
    });

    group.finish();
}

// =============================================================================
// Differentiation AST Only Comparison
// =============================================================================

fn bench_diff_ast_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_ast_comparison");
    let empty_set = HashSet::new();

    // Pre-parse for SymbAnaFis
    let poly_expr = symb_anafis::parse(POLYNOMIAL, &empty_set, &empty_set).unwrap();
    let trig_expr = symb_anafis::parse(TRIG_SIMPLE, &empty_set, &empty_set).unwrap();
    let complex_expr = symb_anafis::parse(COMPLEX_EXPR, &empty_set, &empty_set).unwrap();
    let nested_expr = symb_anafis::parse(NESTED_TRIG, &empty_set, &empty_set).unwrap();

    // Pre-parse for Symbolica
    let x_sym = symbol!("x");
    let poly_atom = parse!(POLYNOMIAL);
    let trig_atom = parse!(TRIG_SIMPLE);
    let complex_atom = parse!(COMPLEX_EXPR);
    let nested_atom = parse!(NESTED_TRIG);

    let x = symb_anafis::symb("x");

    // SymbAnaFis differentiation
    group.bench_function("symb_anafis/polynomial", |b| {
        b.iter(|| Diff::new().differentiate(black_box(poly_expr.clone()), black_box(&x)))
    });

    group.bench_function("symb_anafis/trig_simple", |b| {
        b.iter(|| Diff::new().differentiate(black_box(trig_expr.clone()), black_box(&x)))
    });

    group.bench_function("symb_anafis/complex_expr", |b| {
        b.iter(|| Diff::new().differentiate(black_box(complex_expr.clone()), black_box(&x)))
    });

    group.bench_function("symb_anafis/nested_trig", |b| {
        b.iter(|| Diff::new().differentiate(black_box(nested_expr.clone()), black_box(&x)))
    });

    // Symbolica differentiation
    group.bench_function("symbolica/polynomial", |b| {
        b.iter(|| black_box(&poly_atom).derivative(black_box(x_sym)))
    });

    group.bench_function("symbolica/trig_simple", |b| {
        b.iter(|| black_box(&trig_atom).derivative(black_box(x_sym)))
    });

    group.bench_function("symbolica/complex_expr", |b| {
        b.iter(|| black_box(&complex_atom).derivative(black_box(x_sym)))
    });

    group.bench_function("symbolica/nested_trig", |b| {
        b.iter(|| black_box(&nested_atom).derivative(black_box(x_sym)))
    });

    group.finish();
}

// =============================================================================
// Differentiation Full Pipeline Comparison
// =============================================================================

fn bench_diff_full_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_full_comparison");

    let x_sym = symbol!("x");

    // SymbAnaFis full pipeline (parse + diff + simplify)
    group.bench_function("symb_anafis/polynomial", |b| {
        b.iter(|| diff(black_box(POLYNOMIAL), "x", None, None))
    });

    group.bench_function("symb_anafis/trig_simple", |b| {
        b.iter(|| diff(black_box(TRIG_SIMPLE), "x", None, None))
    });

    group.bench_function("symb_anafis/chain_sin", |b| {
        b.iter(|| diff(black_box(CHAIN_SIN), "x", None, None))
    });

    group.bench_function("symb_anafis/exp_squared", |b| {
        b.iter(|| diff(black_box(EXP_SQUARED), "x", None, None))
    });

    group.bench_function("symb_anafis/complex_expr", |b| {
        b.iter(|| diff(black_box(COMPLEX_EXPR), "x", None, None))
    });

    group.bench_function("symb_anafis/quotient", |b| {
        b.iter(|| diff(black_box(QUOTIENT), "x", None, None))
    });

    group.bench_function("symb_anafis/nested_trig", |b| {
        b.iter(|| diff(black_box(NESTED_TRIG), "x", None, None))
    });

    group.bench_function("symb_anafis/power_xx", |b| {
        b.iter(|| diff(black_box(POWER_XX), "x", None, None))
    });

    // Symbolica full pipeline (parse + diff with auto-normalize)
    group.bench_function("symbolica/polynomial", |b| {
        b.iter(|| {
            let atom = parse!(black_box(POLYNOMIAL));
            atom.derivative(x_sym)
        })
    });

    group.bench_function("symbolica/trig_simple", |b| {
        b.iter(|| {
            let atom = parse!(black_box(TRIG_SIMPLE));
            atom.derivative(x_sym)
        })
    });

    group.bench_function("symbolica/chain_sin", |b| {
        b.iter(|| {
            let atom = parse!(black_box(CHAIN_SIN));
            atom.derivative(x_sym)
        })
    });

    group.bench_function("symbolica/exp_squared", |b| {
        b.iter(|| {
            let atom = parse!(black_box(EXP_SQUARED));
            atom.derivative(x_sym)
        })
    });

    group.bench_function("symbolica/complex_expr", |b| {
        b.iter(|| {
            let atom = parse!(black_box(COMPLEX_EXPR));
            atom.derivative(x_sym)
        })
    });

    group.bench_function("symbolica/quotient", |b| {
        b.iter(|| {
            let atom = parse!(black_box(QUOTIENT));
            atom.derivative(x_sym)
        })
    });

    group.bench_function("symbolica/nested_trig", |b| {
        b.iter(|| {
            let atom = parse!(black_box(NESTED_TRIG));
            atom.derivative(x_sym)
        })
    });

    group.bench_function("symbolica/power_xx", |b| {
        b.iter(|| {
            let atom = parse!(black_box(POWER_XX));
            atom.derivative(x_sym)
        })
    });

    group.finish();
}

// =============================================================================
// Large Expression Comparison (Testing Rayon parallelism vs Symbolica)
// =============================================================================

const NORMAL_PDF: &str = "exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)";
const WAVE_EQUATION: &str = "A * sin(k*x - omega*t) * exp(-gamma*t)";
const GAUSSIAN_2D: &str = "exp(-((x - x0)^2 + (y - y0)^2) / (2 * sigma^2)) / (2 * pi * sigma^2)";
const MAXWELL_BOLTZMANN: &str =
    "4 * pi * (m / (2 * pi * k * T))^(3/2) * v^2 * exp(-m * v^2 / (2 * k * T))";
const ORBITAL_ENERGY: &str = "-G * M * m / (2 * a) + L^2 / (2 * m * r^2) - G * M * m / r";

fn bench_large_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_expr_comparison");

    let x_sym = symbol!("x");
    let v_sym = symbol!("v");
    let r_sym = symbol!("r");

    // SymbAnaFis large expressions
    group.bench_function("symb_anafis/normal_pdf", |b| {
        b.iter(|| diff(black_box(NORMAL_PDF), "x", Some(&["mu", "sigma"]), None))
    });

    group.bench_function("symb_anafis/wave_equation", |b| {
        b.iter(|| {
            diff(
                black_box(WAVE_EQUATION),
                "x",
                Some(&["A", "k", "omega", "gamma"]),
                None,
            )
        })
    });

    group.bench_function("symb_anafis/gaussian_2d", |b| {
        b.iter(|| {
            diff(
                black_box(GAUSSIAN_2D),
                "x",
                Some(&["x0", "y0", "sigma"]),
                None,
            )
        })
    });

    group.bench_function("symb_anafis/maxwell_boltzmann", |b| {
        b.iter(|| {
            diff(
                black_box(MAXWELL_BOLTZMANN),
                "v",
                Some(&["m", "k", "T"]),
                None,
            )
        })
    });

    group.bench_function("symb_anafis/orbital_energy", |b| {
        b.iter(|| {
            diff(
                black_box(ORBITAL_ENERGY),
                "r",
                Some(&["G", "M", "m", "a", "L"]),
                None,
            )
        })
    });

    // Symbolica large expressions
    group.bench_function("symbolica/normal_pdf", |b| {
        b.iter(|| {
            let atom = parse!(black_box(NORMAL_PDF));
            atom.derivative(x_sym)
        })
    });

    group.bench_function("symbolica/wave_equation", |b| {
        b.iter(|| {
            let atom = parse!(black_box(WAVE_EQUATION));
            atom.derivative(x_sym)
        })
    });

    group.bench_function("symbolica/gaussian_2d", |b| {
        b.iter(|| {
            let atom = parse!(black_box(GAUSSIAN_2D));
            atom.derivative(x_sym)
        })
    });

    group.bench_function("symbolica/maxwell_boltzmann", |b| {
        b.iter(|| {
            let atom = parse!(black_box(MAXWELL_BOLTZMANN));
            atom.derivative(v_sym)
        })
    });

    group.bench_function("symbolica/orbital_energy", |b| {
        b.iter(|| {
            let atom = parse!(black_box(ORBITAL_ENERGY));
            atom.derivative(r_sym)
        })
    });

    group.finish();
}

// =============================================================================
// Criterion Setup
// =============================================================================

criterion_group!(
    benches,
    bench_parsing_comparison,
    bench_diff_ast_comparison,
    bench_diff_full_comparison,
    bench_large_comparison,
);

criterion_main!(benches);
