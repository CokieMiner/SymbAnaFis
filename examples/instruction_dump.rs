#![allow(
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::use_debug,
    reason = "Essential for examples: unwrap for simplicity, stdout for demonstration"
)]
//! Instruction Sequence Dumper for Benchmark Expressions
//!
//! This example compiles all benchmark expressions (their derivatives) and outputs their
//! instruction sequences for analysis.
//!
//! Run with: cargo run --example `instruction_dump`

use std::collections::HashSet;
use symb_anafis::{CompiledEvaluator, Diff, parse, symb};

// Benchmark expressions (copied from benches/rust/expressions.rs)
/// Array of benchmark expressions with names, formulas, differentiation variables, and parameters
const ALL_EXPRESSIONS: &[(&str, &str, &str, &[&str])] = &[
    (
        "Normal PDF",
        "exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)",
        "x",
        &["mu", "sigma", "pi"],
    ),
    (
        "Gaussian 2D",
        "exp(-((x - x0)^2 + (y - y0)^2) / (2 * s^2)) / (2 * pi * s^2)",
        "x",
        &["y", "x0", "y0", "s", "pi"],
    ),
    (
        "Maxwell-Boltzmann",
        "4 * pi * (m / (2 * pi * k * T))^(3/2) * v^2 * exp(-m * v^2 / (2 * k * T))",
        "v",
        &["m", "k", "T", "pi"],
    ),
    ("Lorentz Factor", "1 / sqrt(1 - v^2 / c^2)", "v", &["c"]),
    (
        "Lennard-Jones",
        "4 * epsilon * ((sigma / r)^12 - (sigma / r)^6)",
        "r",
        &["epsilon", "sigma"],
    ),
    (
        "Logistic Sigmoid",
        "1 / (1 + exp(-k * (x - x0)))",
        "x",
        &["k", "x0"],
    ),
    (
        "Damped Oscillator",
        "A * exp(-gamma * t) * cos(omega * t + phi)",
        "t",
        &["A", "gamma", "omega", "phi"],
    ),
    (
        "Planck Blackbody",
        "2 * h * nu^3 / c^2 * 1 / (exp(h * nu / (k * T)) - 1)",
        "nu",
        &["h", "c", "k", "T"],
    ),
    (
        "Bessel Wave",
        "A * besselj(0, k * r) * exp(-alpha * r)",
        "r",
        &["A", "k", "alpha"],
    ),
];

fn main() {
    let empty = HashSet::new();

    for (name, expr_str, var, fixed) in ALL_EXPRESSIONS {
        println!("=== {name} ===");
        println!("Original Expression: {expr_str}");
        println!("Variable: {var}");
        println!("Fixed: {fixed:?}");

        // Parse the expression
        let expr = parse(expr_str, &empty, &empty, None).unwrap();
        let var_sym = symb(var);

        // Differentiate (raw, no simplification like in benchmarks)
        let diff_builder_raw = Diff::new().skip_simplification(true);
        let derivative_raw = diff_builder_raw.differentiate(&expr, &var_sym).unwrap();

        // Differentiate (simplified)
        let diff_builder_simp = Diff::new().skip_simplification(false);
        let derivative_simp = diff_builder_simp.differentiate(&expr, &var_sym).unwrap();

        println!("Raw Derivative: {derivative_raw}");
        println!("Simplified Derivative: {derivative_simp}");

        // Collect all parameters (sorted like in benchmarks)
        let mut params = vec![*var];
        params.extend_from_slice(fixed);
        params.sort_unstable();

        // Compile Raw
        let compiled_raw = CompiledEvaluator::compile(&derivative_raw, &params, None).unwrap();
        // Compile Simplified
        let compiled_simp = CompiledEvaluator::compile(&derivative_simp, &params, None).unwrap();

        println!(
            "Raw Instructions ({} total):",
            compiled_raw.instructions.len()
        );
        println!(
            "Simplified Instructions ({} total):",
            compiled_simp.instructions.len()
        );

        println!("\n--- Simplified Instructions Details ---");
        for (i, instr) in compiled_simp.instructions.iter().enumerate() {
            println!("  {i:3}: {instr:?}");
        }

        println!("\n--- Raw Instructions Details ---");
        for (i, instr) in compiled_raw.instructions.iter().enumerate() {
            println!("  {i:3}: {instr:?}");
        }

        println!("Constants (Simp): {:?}", compiled_simp.constants);
        println!("Stack size (Simp): {}", compiled_simp.stack_size);
        println!("Cache size (Simp): {}", compiled_simp.cache_size);
        println!();
    }
}
