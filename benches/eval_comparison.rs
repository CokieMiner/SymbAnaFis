//! Evaluation comparison benchmark - outputs timing AND results for precision verification
//! Run with: cargo bench --bench eval_comparison
//!
//! This benchmark tests both common functions and special functions with custom implementations
//! to help identify whether performance issues are in the evaluator engine or specific functions.

use criterion::{Criterion, criterion_group, criterion_main};
use std::collections::{HashMap, HashSet};
use std::hint::black_box;
use symb_anafis::{ExprKind, parse};

const TEST_VALUE: f64 = 2.5;

fn eval_and_print(name: &str, expr_str: &str) -> Option<f64> {
    let empty: HashSet<String> = HashSet::new();

    let expr = match parse(expr_str, &empty, &empty) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("{:<40} PARSE FAILED: {:?}", name, err);
            return None;
        }
    };

    let mut vars: HashMap<&str, f64> = HashMap::new();
    vars.insert("x", TEST_VALUE);
    vars.insert("s", TEST_VALUE);

    let result = expr.evaluate(&vars);
    if let ExprKind::Number(n) = result.kind {
        Some(n)
    } else {
        None
    }
}

fn bench_common_functions(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_common");
    let empty: HashSet<String> = HashSet::new();

    let expressions = [
        ("polynomial", "x^3 + 2*x^2 + x + 1"),
        ("sin", "sin(x)"),
        ("cos", "cos(x)"),
        ("tan", "tan(x)"),
        ("exp", "exp(x)"),
        ("ln", "ln(x)"),
        ("sqrt", "sqrt(x)"),
        ("sin*cos", "sin(x)*cos(x)"),
        ("exp*sin", "exp(x)*sin(x)"),
        ("nested_trig", "sin(cos(tan(x)))"),
    ];

    let mut vars: HashMap<&str, f64> = HashMap::new();
    vars.insert("x", TEST_VALUE);

    // Print results first
    println!("\n=== COMMON FUNCTIONS (x = {}) ===", TEST_VALUE);
    for (name, expr_str) in &expressions {
        if let Some(result) = eval_and_print(name, expr_str) {
            println!("{:<20} = {:.10}", name, result);
        }
    }

    // Then benchmark
    for (name, expr_str) in &expressions {
        let expr = parse(expr_str, &empty, &empty).unwrap();
        group.bench_function(*name, |b| b.iter(|| black_box(&expr).evaluate(&vars)));
    }

    group.finish();
}

fn bench_gamma_family(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_gamma_family");
    let empty: HashSet<String> = HashSet::new();

    let expressions = [
        ("gamma", "gamma(x)"),
        ("digamma", "digamma(x)"),
        ("trigamma", "trigamma(x)"),
        ("polygamma_2", "polygamma(2, x)"),
        ("polygamma_3", "polygamma(3, x)"),
        ("polygamma_4", "polygamma(4, x)"),
    ];

    let mut vars: HashMap<&str, f64> = HashMap::new();
    vars.insert("x", TEST_VALUE);

    // Print results first
    println!("\n=== GAMMA FAMILY (x = {}) ===", TEST_VALUE);
    for (name, expr_str) in &expressions {
        if let Some(result) = eval_and_print(name, expr_str) {
            println!("{:<20} = {:.10}", name, result);
        }
    }

    // Then benchmark
    for (name, expr_str) in &expressions {
        let expr = parse(expr_str, &empty, &empty).unwrap();
        group.bench_function(*name, |b| b.iter(|| black_box(&expr).evaluate(&vars)));
    }

    group.finish();
}

fn bench_bessel(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_bessel");
    let empty: HashSet<String> = HashSet::new();

    let expressions = [
        ("besselj_0", "besselj(0, x)"),
        ("besselj_1", "besselj(1, x)"),
        ("besselj_2", "besselj(2, x)"),
        ("bessely_0", "bessely(0, x)"),
        ("bessely_1", "bessely(1, x)"),
    ];

    let mut vars: HashMap<&str, f64> = HashMap::new();
    vars.insert("x", TEST_VALUE);

    // Print results first
    println!("\n=== BESSEL FUNCTIONS (x = {}) ===", TEST_VALUE);
    for (name, expr_str) in &expressions {
        if let Some(result) = eval_and_print(name, expr_str) {
            println!("{:<20} = {:.10}", name, result);
        }
    }

    // Then benchmark
    for (name, expr_str) in &expressions {
        let expr = parse(expr_str, &empty, &empty).unwrap();
        group.bench_function(*name, |b| b.iter(|| black_box(&expr).evaluate(&vars)));
    }

    group.finish();
}

fn bench_zeta(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_zeta");
    let empty: HashSet<String> = HashSet::new();

    let expressions = [
        ("zeta", "zeta(x)"),
        ("zeta_deriv_1", "zeta_deriv(1, x)"),
        ("zeta_deriv_2", "zeta_deriv(2, x)"),
        ("zeta_deriv_3", "zeta_deriv(3, x)"),
    ];

    let mut vars: HashMap<&str, f64> = HashMap::new();
    vars.insert("x", TEST_VALUE);

    // Print results first
    println!("\n=== ZETA FUNCTION (x = {}) ===", TEST_VALUE);
    for (name, expr_str) in &expressions {
        if let Some(result) = eval_and_print(name, expr_str) {
            println!("{:<20} = {:.10}", name, result);
        }
    }

    // Then benchmark
    for (name, expr_str) in &expressions {
        let expr = parse(expr_str, &empty, &empty).unwrap();
        group.bench_function(*name, |b| b.iter(|| black_box(&expr).evaluate(&vars)));
    }

    group.finish();
}

fn bench_other_special(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_other_special");
    let empty: HashSet<String> = HashSet::new();

    let expressions = [
        ("erf", "erf(x)"),
        ("erfc", "erfc(x)"),
        ("lambertw", "lambertw(x)"),
    ];

    let mut vars: HashMap<&str, f64> = HashMap::new();
    vars.insert("x", TEST_VALUE);

    // Print results first
    println!("\n=== OTHER SPECIAL FUNCTIONS (x = {}) ===", TEST_VALUE);
    for (name, expr_str) in &expressions {
        if let Some(result) = eval_and_print(name, expr_str) {
            println!("{:<20} = {:.10}", name, result);
        }
    }

    // Then benchmark
    for (name, expr_str) in &expressions {
        let expr = parse(expr_str, &empty, &empty).unwrap();
        group.bench_function(*name, |b| b.iter(|| black_box(&expr).evaluate(&vars)));
    }

    group.finish();
}

fn bench_complex_combinations(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_complex");
    let empty: HashSet<String> = HashSet::new();

    let expressions = [
        ("gamma*bessel+erf", "gamma(x)*besselj(0, x)+erf(x)"),
        ("polygamma*zeta", "polygamma(2, x)*zeta(x)"),
        ("zeta_deriv3*gamma", "zeta_deriv(3, x)*gamma(x)"),
    ];

    let mut vars: HashMap<&str, f64> = HashMap::new();
    vars.insert("x", TEST_VALUE);

    // Print results first
    println!("\n=== COMPLEX COMBINATIONS (x = {}) ===", TEST_VALUE);
    for (name, expr_str) in &expressions {
        if let Some(result) = eval_and_print(name, expr_str) {
            println!("{:<20} = {:.10}", name, result);
        }
    }

    // Then benchmark
    for (name, expr_str) in &expressions {
        let expr = parse(expr_str, &empty, &empty).unwrap();
        group.bench_function(*name, |b| b.iter(|| black_box(&expr).evaluate(&vars)));
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_common_functions,
    bench_gamma_family,
    bench_bessel,
    bench_zeta,
    bench_other_special,
    bench_complex_combinations,
);
criterion_main!(benches);
