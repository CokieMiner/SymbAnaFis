//! Flamegraph profiling for specific benchmark targets.
//!
//! Usage: cargo flamegraph --example `flamegraph_benchmarks` -- `<benchmark_id>`
//! Example: cargo flamegraph --example `flamegraph_benchmarks` -- `large_expr_300_raw`
//!
//! Available benchmark IDs:
//! - `large_expr_300_raw`
//! - `large_expr_300_simplified`
//! - `large_expr_100_raw`
//! - `eval_methods_tree_walk_gaussian2d`
//! - `diff_light_gaussian2d`
//! - `diff_light_lorentz`
//! - `full_pipeline_planck`
//! - `full_pipeline_normal`
//! - `parsing_small_exprs`

#![allow(clippy::print_stderr, reason = "profile output")]
#![allow(clippy::use_debug, reason = "timing display")]
#![allow(clippy::non_ascii_literal, reason = "profile output")]
#![allow(clippy::cast_precision_loss, reason = "profile output")]
#![allow(clippy::missing_docs_in_private_items, reason = "profile output")]
#![allow(clippy::unwrap_used, reason = "profile output")]
#![allow(clippy::too_many_lines, reason = "profiling harness kept in one place")]

use std::collections::HashSet;
use std::fmt::Write;
use std::hint::black_box;
use std::time::Instant;

use symb_anafis::{CompiledEvaluator, Diff, parse, symb};

mod expressions {
    // Copy of the expression definitions from benches/rust/expressions.rs
    // Normal probability density function
    pub const NORMAL_PDF: &str = "exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)";
    pub const NORMAL_PDF_VAR: &str = "x";
    pub const NORMAL_PDF_FIXED: &[&str] = &["mu", "sigma", "pi"];

    // 2D Gaussian distribution
    pub const GAUSSIAN_2D: &str = "exp(-((x - x0)^2 + (y - y0)^2) / (2 * s^2)) / (2 * pi * s^2)";
    pub const GAUSSIAN_2D_VAR: &str = "x";
    pub const GAUSSIAN_2D_FIXED: &[&str] = &["y", "x0", "y0", "s", "pi"];

    // Maxwell-Boltzmann speed distribution
    pub const MAXWELL_BOLTZMANN: &str =
        "4 * pi * (m / (2 * pi * k * T))^(3/2) * v^2 * exp(-m * v^2 / (2 * k * T))";
    pub const MAXWELL_BOLTZMANN_VAR: &str = "v";
    pub const MAXWELL_BOLTZMANN_FIXED: &[&str] = &["m", "k", "T", "pi"];

    // Lorentz factor
    pub const LORENTZ_FACTOR: &str = "1 / sqrt(1 - v^2 / c^2)";
    pub const LORENTZ_FACTOR_VAR: &str = "v";
    pub const LORENTZ_FACTOR_FIXED: &[&str] = &["c"];

    // Lennard-Jones potential
    pub const LENNARD_JONES: &str = "4 * epsilon * ((sigma / r)^12 - (sigma / r)^6)";
    pub const LENNARD_JONES_VAR: &str = "r";
    pub const LENNARD_JONES_FIXED: &[&str] = &["epsilon", "sigma"];

    // Logistic sigmoid function
    pub const LOGISTIC_SIGMOID: &str = "1 / (1 + exp(-k * (x - x0)))";
    pub const LOGISTIC_SIGMOID_VAR: &str = "x";
    pub const LOGISTIC_SIGMOID_FIXED: &[&str] = &["k", "x0"];

    // Damped harmonic oscillator
    pub const DAMPED_OSCILLATOR: &str = "A * exp(-gamma * t) * cos(omega * t + phi)";
    pub const DAMPED_OSCILLATOR_VAR: &str = "t";
    pub const DAMPED_OSCILLATOR_FIXED: &[&str] = &["A", "gamma", "omega", "phi"];

    // Planck blackbody radiation law
    pub const PLANCK_BLACKBODY: &str = "2 * h * nu^3 / c^2 * 1 / (exp(h * nu / (k * T)) - 1)";
    pub const PLANCK_BLACKBODY_VAR: &str = "nu";
    pub const PLANCK_BLACKBODY_FIXED: &[&str] = &["h", "c", "k", "T"];

    // Bessel function example
    pub const BESSEL_WAVE: &str = "A * besselj(0, k * r) * exp(-alpha * r)";
    pub const BESSEL_WAVE_VAR: &str = "r";
    pub const BESSEL_WAVE_FIXED: &[&str] = &["A", "k", "alpha"];

    // All expressions as a slice for iteration
    pub const ALL_EXPRESSIONS: &[(&str, &str, &str, &[&str])] = &[
        ("Normal PDF", NORMAL_PDF, NORMAL_PDF_VAR, NORMAL_PDF_FIXED),
        ("Gaussian 2D", GAUSSIAN_2D, GAUSSIAN_2D_VAR, GAUSSIAN_2D_FIXED),
        ("Maxwell-Boltzmann", MAXWELL_BOLTZMANN, MAXWELL_BOLTZMANN_VAR, MAXWELL_BOLTZMANN_FIXED),
        ("Lorentz Factor", LORENTZ_FACTOR, LORENTZ_FACTOR_VAR, LORENTZ_FACTOR_FIXED),
        ("Lennard-Jones", LENNARD_JONES, LENNARD_JONES_VAR, LENNARD_JONES_FIXED),
        ("Logistic Sigmoid", LOGISTIC_SIGMOID, LOGISTIC_SIGMOID_VAR, LOGISTIC_SIGMOID_FIXED),
        ("Damped Oscillator", DAMPED_OSCILLATOR, DAMPED_OSCILLATOR_VAR, DAMPED_OSCILLATOR_FIXED),
        ("Planck Blackbody", PLANCK_BLACKBODY, PLANCK_BLACKBODY_VAR, PLANCK_BLACKBODY_FIXED),
        ("Bessel Wave", BESSEL_WAVE, BESSEL_WAVE_VAR, BESSEL_WAVE_FIXED),
    ];
}

/// Generates a complex mixed expression with N terms (copied from `large_expr.rs`)
fn generate_mixed_complex(n: usize) -> String {
    let mut s = String::with_capacity(n * 50);
    for i in 1..=n {
        if i > 1 {
            if i % 4 == 0 {
                write!(s, " - ").unwrap();
            } else {
                write!(s, " + ").unwrap();
            }
        }

        match i % 5 {
            0 => write!(s, "{}*x^{}", i, i % 10 + 1).unwrap(),
            1 => write!(s, "sin({i}*x)*cos(x)").unwrap(),
            2 => write!(s, "(exp(x/{i}) + sqrt(x + {i}))").unwrap(),
            3 => write!(s, "(x^2 + {i})/(x + {i})").unwrap(),
            4 => write!(s, "sin(exp(x) + {i})").unwrap(),
            #[allow(clippy::unreachable, reason = "match covers all possibilities")]
            _ => unreachable!(),
        }
    }
    s
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <benchmark_id>", args[0]);
        eprintln!("Available IDs: large_expr_300_raw, large_expr_300_simplified, large_expr_100_raw, eval_methods_tree_walk_gaussian2d, diff_light_gaussian2d, diff_light_lorentz, full_pipeline_planck, full_pipeline_normal, parsing_small_exprs");
        std::process::exit(1);
    }
    let bench_id = &args[1];
    eprintln!("Starting flamegraph profiling for benchmark: {bench_id}");

    match bench_id.as_str() {
        "large_expr_300_raw" => bench_large_expr_eval_raw(300),
        "large_expr_300_simplified" => bench_large_expr_eval_simplified(300),
        "large_expr_100_raw" => bench_large_expr_eval_raw(100),
        "eval_methods_tree_walk_gaussian2d" => bench_eval_methods_tree_walk_gaussian2d(),
        "diff_light_gaussian2d" => bench_diff_light_gaussian2d(),
        "diff_light_lorentz" => bench_diff_light_lorentz(),
        "full_pipeline_planck" => bench_full_pipeline_planck(),
        "full_pipeline_normal" => bench_full_pipeline_normal(),
        "parsing_small_exprs" => bench_parsing_small_exprs(),
        _ => {
            eprintln!("Unknown benchmark ID: {bench_id}");
            std::process::exit(1);
        }
    }

    eprintln!("Profiling completed.");
}

// -----------------------------------------------------------------------------
// Large expression evaluation (raw derivative)
// -----------------------------------------------------------------------------
fn bench_large_expr_eval_raw(n: usize) {
    eprintln!("Generating large expression with {n} terms...");
    let expr_str = generate_mixed_complex(n);
    let empty = HashSet::new();
    let x_sym = symb("x");

    // Parse once (outside profiling loop)
    let expr = parse(&expr_str, &empty, &empty, None).unwrap();

    // Differentiate with skip_simplification (raw)
    let diff_expr = Diff::new()
        .skip_simplification(true)
        .differentiate(&expr, &x_sym)
        .unwrap();

    // Compile raw derivative
    let evaluator = CompiledEvaluator::compile(&diff_expr, &["x"], None).unwrap();

    // Generate 1000 test points (same as benchmark)
    let test_points: Vec<f64> = (0..1000).map(|i| f64::from(i).mul_add(0.01, 0.1)).collect();

    eprintln!("Running evaluation loop for flamegraph...");
    let start = Instant::now();
    let iterations = 50; // Enough for flamegraph
    for _ in 0..iterations {
        for &x in &test_points {
            black_box(evaluator.evaluate(&[x]));
        }
    }
    let elapsed = start.elapsed();
    eprintln!("Completed {} iterations in {:?} ({:?}/iteration)", iterations, elapsed, elapsed / iterations);
}

// -----------------------------------------------------------------------------
// Large expression evaluation (simplified derivative)
// -----------------------------------------------------------------------------
fn bench_large_expr_eval_simplified(n: usize) {
    eprintln!("Generating large expression with {n} terms...");
    let expr_str = generate_mixed_complex(n);
    let empty = HashSet::new();
    let x_sym = symb("x");

    let expr = parse(&expr_str, &empty, &empty, None).unwrap();

    // Differentiate with full simplification
    let diff_expr = Diff::new().differentiate(&expr, &x_sym).unwrap();

    // Compile simplified derivative
    let evaluator = CompiledEvaluator::compile(&diff_expr, &["x"], None).unwrap();

    let test_points: Vec<f64> = (0..1000).map(|i| f64::from(i).mul_add(0.01, 0.1)).collect();

    eprintln!("Running evaluation loop for flamegraph...");
    let start = Instant::now();
    let iterations = 50;
    for _ in 0..iterations {
        for &x in &test_points {
            black_box(evaluator.evaluate(&[x]));
        }
    }
    let elapsed = start.elapsed();
    eprintln!("Completed {} iterations in {:?} ({:?}/iteration)", iterations, elapsed, elapsed / iterations);
}

// -----------------------------------------------------------------------------
// Tree-walk evaluation for Gaussian 2D (eval_methods_1000pts/tree_walk_evaluate)
// -----------------------------------------------------------------------------
fn bench_eval_methods_tree_walk_gaussian2d() {
    use std::collections::HashMap;
    eprintln!("Tree-walk evaluation for Gaussian 2D...");
    let empty = HashSet::new();
    let expr_str = expressions::GAUSSIAN_2D;
    let var = expressions::GAUSSIAN_2D_VAR;
    let fixed = expressions::GAUSSIAN_2D_FIXED;

    // Get simplified derivative (same as benchmark)
    let diff_str = symb_anafis::diff(expr_str, var, fixed, None).unwrap();
    let diff_expr = parse(&diff_str, &empty, &empty, None).unwrap();

    // Generate 1000 test points
    let test_points: Vec<f64> = (0..1000).map(|i| f64::from(i).mul_add(0.01, 0.1)).collect();

    // Pre-set fixed vars to 1.0
    let mut vars = HashMap::new();
    for &f in fixed {
        vars.insert(f, 1.0);
    }

    eprintln!("Running tree‑walk evaluation loop for flamegraph...");
    let start = Instant::now();
    let iterations = 20; // Enough for flamegraph
    for _ in 0..iterations {
        let mut sum = 0.0;
        for &x in &test_points {
            vars.insert(var, x);
            // evaluate with empty function map
            sum += diff_expr
                .evaluate(&vars, &HashMap::new())
                .as_number()
                .unwrap_or(0.0);
        }
        black_box(sum);
    }
    let elapsed = start.elapsed();
    eprintln!("Completed {} iterations in {:?} ({:?}/iteration)", iterations, elapsed, elapsed / iterations);
}

// -----------------------------------------------------------------------------
// Differentiation light for Gaussian 2D
// -----------------------------------------------------------------------------
fn bench_diff_light_gaussian2d() {
    eprintln!("Differentiation light for Gaussian 2D...");
    let empty = HashSet::new();
    let expr_str = expressions::GAUSSIAN_2D;
    let var = expressions::GAUSSIAN_2D_VAR;
    #[allow(unused, reason = "kept for consistency with expression definitions")]
    #[allow(clippy::no_effect_underscore_binding, reason = "binding unused but kept for consistency")]
    let _fixed = expressions::GAUSSIAN_2D_FIXED;

    // Parse once (outside loop)
    let expr = parse(expr_str, &empty, &empty, None).unwrap();
    let var_sym = symb(var);

    let diff_light = Diff::new().skip_simplification(true);

    eprintln!("Running differentiation light loop for flamegraph...");
    let start = Instant::now();
    let iterations = 5000; // Enough for flamegraph
    for _ in 0..iterations {
        black_box(diff_light.differentiate(&expr, &var_sym).unwrap());
    }
    let elapsed = start.elapsed();
    eprintln!("Completed {} iterations in {:?} ({:?}/iteration)", iterations, elapsed, elapsed / iterations);
}

// -----------------------------------------------------------------------------
// Differentiation light for Lorentz Factor
// -----------------------------------------------------------------------------
fn bench_diff_light_lorentz() {
    eprintln!("Differentiation light for Lorentz Factor...");
    let empty = HashSet::new();
    let expr_str = expressions::LORENTZ_FACTOR;
    let var = expressions::LORENTZ_FACTOR_VAR;
    #[allow(unused, reason = "kept for consistency with expression definitions")]
    #[allow(clippy::no_effect_underscore_binding, reason = "binding unused but kept for consistency")]
    let _fixed = expressions::LORENTZ_FACTOR_FIXED;

    // Parse once (outside loop)
    let expr = parse(expr_str, &empty, &empty, None).unwrap();
    let var_sym = symb(var);

    let diff_light = Diff::new().skip_simplification(true);

    eprintln!("Running differentiation light loop for flamegraph...");
    let start = Instant::now();
    let iterations = 5000; // Enough for flamegraph
    for _ in 0..iterations {
        black_box(diff_light.differentiate(&expr, &var_sym).unwrap());
    }
    let elapsed = start.elapsed();
    eprintln!("Completed {} iterations in {:?} ({:?}/iteration)", iterations, elapsed, elapsed / iterations);
}

// -----------------------------------------------------------------------------
// Full pipeline for Planck Blackbody
// -----------------------------------------------------------------------------
fn bench_full_pipeline_planck() {
    eprintln!("Full pipeline for Planck Blackbody...");
    let empty = HashSet::new();
    let expr_str = expressions::PLANCK_BLACKBODY;
    let var = expressions::PLANCK_BLACKBODY_VAR;
    let fixed = expressions::PLANCK_BLACKBODY_FIXED;

    // Generate 1000 test points
    let test_points: Vec<f64> = (0..1000).map(|i| f64::from(i).mul_add(0.01, 0.1)).collect();

    // Pre‑compute fixed set
    let fixed_set: HashSet<String> = fixed.iter().copied().map(String::from).collect();

    eprintln!("Running full pipeline loop for flamegraph...");
    let start = Instant::now();
    let iterations = 5; // Heavy pipeline, fewer iterations
    for _ in 0..iterations {
        // Parse
        let expr = parse(expr_str, &empty, &fixed_set, None).unwrap();
        let var_sym = symb(var);
        // Differentiate with full simplification (default)
        let diff_expr = Diff::new().differentiate(&expr, &var_sym).unwrap();

        // Create sorted parameter list
        let mut params = vec![var];
        params.extend(fixed.iter());
        params.sort_unstable();

        let compiled = CompiledEvaluator::compile(&diff_expr, &params, None).unwrap();

        // Evaluate at 1000 points
        let param_count = compiled.param_count();
        let param_names = compiled.param_names();
        let var_idx = param_names.iter().position(|p| p == var).unwrap_or(0);
        let mut sum = 0.0;
        let mut values = vec![1.0; param_count];
        for &x in &test_points {
            values[var_idx] = x;
            sum += compiled.evaluate(&values);
        }
        black_box(sum);
    }
    let elapsed = start.elapsed();
    eprintln!("Completed {} iterations in {:?} ({:?}/iteration)", iterations, elapsed, elapsed / iterations);
}

// -----------------------------------------------------------------------------
// Full pipeline for Normal PDF
// -----------------------------------------------------------------------------
fn bench_full_pipeline_normal() {
    eprintln!("Full pipeline for Normal PDF...");
    let empty = HashSet::new();
    let expr_str = expressions::NORMAL_PDF;
    let var = expressions::NORMAL_PDF_VAR;
    let fixed = expressions::NORMAL_PDF_FIXED;

    // Generate 1000 test points
    let test_points: Vec<f64> = (0..1000).map(|i| f64::from(i).mul_add(0.01, 0.1)).collect();

    // Pre‑compute fixed set
    let fixed_set: HashSet<String> = fixed.iter().copied().map(String::from).collect();

    eprintln!("Running full pipeline loop for flamegraph...");
    let start = Instant::now();
    let iterations = 5; // Heavy pipeline, fewer iterations
    for _ in 0..iterations {
        // Parse
        let expr = parse(expr_str, &empty, &fixed_set, None).unwrap();
        let var_sym = symb(var);
        // Differentiate with full simplification (default)
        let diff_expr = Diff::new().differentiate(&expr, &var_sym).unwrap();

        // Create sorted parameter list
        let mut params = vec![var];
        params.extend(fixed.iter());
        params.sort_unstable();

        let compiled = CompiledEvaluator::compile(&diff_expr, &params, None).unwrap();

        // Evaluate at 1000 points
        let param_count = compiled.param_count();
        let param_names = compiled.param_names();
        let var_idx = param_names.iter().position(|p| p == var).unwrap_or(0);
        let mut sum = 0.0;
        let mut values = vec![1.0; param_count];
        for &x in &test_points {
            values[var_idx] = x;
            sum += compiled.evaluate(&values);
        }
        black_box(sum);
    }
    let elapsed = start.elapsed();
    eprintln!("Completed {} iterations in {:?} ({:?}/iteration)", iterations, elapsed, elapsed / iterations);
}

// -----------------------------------------------------------------------------
// Parsing and evaluation of small expressions
// -----------------------------------------------------------------------------
fn bench_parsing_small_exprs() {
    use std::collections::HashMap;
    eprintln!("Parsing and evaluation of small expressions...");
    let empty = HashSet::new();

    eprintln!("Running parse+eval loop for flamegraph...");
    let start = Instant::now();
    let iterations = 100; // Many iterations because each expression is small
    for _ in 0..iterations {
        for (_, expr_str, var, fixed) in expressions::ALL_EXPRESSIONS {
            // Parse
            let expr = parse(expr_str, &empty, &empty, None).unwrap();
            // Prepare variable values (all 1.0)
            let mut vars = HashMap::new();
            vars.insert(*var, 1.0);
            for &f in *fixed {
                vars.insert(f, 1.0);
            }
            // Evaluate
            black_box(expr.evaluate(&vars, &HashMap::new()));
        }
    }
    let elapsed = start.elapsed();
    eprintln!("Completed {} iterations in {:?} ({:?}/iteration)", iterations, elapsed, elapsed / iterations);
}