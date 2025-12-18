//! Large Expression Benchmark
//!
//! Benchmarks for expressions with many mixed terms (N >= 300).
//! Compares SymbAnaFis vs Symbolica.

use criterion::{Criterion, criterion_group, criterion_main};
use std::collections::HashSet;
use std::fmt::Write;
use std::hint::black_box;
use symb_anafis::{Diff, diff};

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
            // Mix operators: +, -, *
            if i % 3 == 0 {
                write!(s, " + ").unwrap();
            } else if i % 3 == 1 {
                write!(s, " - ").unwrap();
            } else {
                write!(s, " + ").unwrap(); // Avoid too many multiplications to prevent explosion
            }
        }

        // Mix term types based on index
        match i % 5 {
            0 => {
                // Polynomial term: i*x^i
                write!(s, "{}*x^{}", i, i % 10 + 1).unwrap();
            }
            1 => {
                // Trig term: sin(i*x) * cos(x)
                write!(s, "sin({}*x)*cos(x)", i).unwrap();
            }
            2 => {
                // Exponential/Log: exp(x/i) + ln(x + i)
                write!(s, "(exp(x/{}) + ln(x + {}))", i, i).unwrap();
            }
            3 => {
                // Rational: (x^2 + i) / (x + i)
                write!(s, "(x^2 + {})/(x + {})", i, i).unwrap();
            }
            4 => {
                // Nested: sin(exp(x) + i)
                write!(s, "sin(exp(x) + {})", i).unwrap();
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
    let mut group = c.benchmark_group("complex_expressions_300");
    group.sample_size(10); // Very large exprs, reduce samples further
    group.measurement_time(std::time::Duration::from_secs(15)); // Allocate more time

    let n = 300;

    // Generate huge mixed expression
    let mixed_str = generate_mixed_complex(n);

    let empty_set = HashSet::new();
    let x_sym = symbol!("x");
    let x_symb = symb_anafis::symb("x");

    // -------------------------------------------------------------------------
    // Parsing Benchmarks
    // -------------------------------------------------------------------------

    group.bench_function("symb_anafis/parse_mixed_300", |b| {
        b.iter(|| symb_anafis::parse(black_box(&mixed_str), &empty_set, &empty_set, None))
    });

    group.bench_function("symbolica/parse_mixed_300", |b| {
        b.iter(|| parse!(black_box(&mixed_str)))
    });

    // -------------------------------------------------------------------------
    // Differentiation Benchmarks (AST Reuse)
    // -------------------------------------------------------------------------

    // Pre-parse
    let mixed_expr = symb_anafis::parse(&mixed_str, &empty_set, &empty_set, None).unwrap();
    let mixed_atom = parse!(&mixed_str);

    group.bench_function("symb_anafis/diff_mixed_300", |b| {
        b.iter(|| Diff::new().differentiate(black_box(mixed_expr.clone()), black_box(&x_symb)))
    });

    group.bench_function("symbolica/diff_mixed_300", |b| {
        b.iter(|| black_box(&mixed_atom).derivative(black_box(x_sym)))
    });

    // -------------------------------------------------------------------------
    // Full Pipeline (Parse + Diff)
    // -------------------------------------------------------------------------

    group.bench_function("symb_anafis/full_mixed_300", |b| {
        b.iter(|| diff(black_box(&mixed_str), "x", None, None))
    });

    group.bench_function("symbolica/full_mixed_300", |b| {
        b.iter(|| {
            let atom = parse!(black_box(&mixed_str));
            atom.derivative(x_sym)
        })
    });

    group.finish();
}

criterion_group!(benches, bench_large_expressions);

criterion_main!(benches);
