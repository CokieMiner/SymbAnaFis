use crate::parser::parse;
use crate::{Diff, Simplify, diff, symb};
use std::collections::HashSet;

#[test]
fn test_gaussian_2d_diff_cycle() {
    // Current Gaussian 2D expression from benchmarks
    // const GAUSSIAN_2D: &str = "exp(-((x - x0)^2 + (y - y0)^2) / (2 * s^2)) / (2 * pi * s^2)";
    let expr_str = "exp(-((x - x0)^2 + (y - y0)^2) / (2 * s^2)) / (2 * pi * s^2)";
    let var = "x";
    let fixed = ["x0", "y0", "s", "pi"];

    println!("Differentiating Gaussian 2D...");

    // Test 1: Using high-level diff() which includes simplification
    // This is where the cycle is happening (max iterations exceeded)
    let result = diff(expr_str, var, &fixed, None);

    assert!(result.is_ok(), "Differentiation failed: {:?}", result.err());
    let result_str = result.unwrap();
    println!("Result: {}", result_str);

    // Test 2: Step-by-step to see where it cycles
    let expr = parse(expr_str, &HashSet::new(), &HashSet::new(), None).unwrap();
    let x = symb("x");

    // 2a. Raw differentiation
    println!("\nRaw differentiation...");
    let diff_builder = Diff::new().skip_simplification(true);

    let diff_raw = diff_builder.differentiate(&expr, &x).unwrap();
    println!("Raw diff nodes: {}", diff_raw.node_count());

    // 2b. Simplify with tracing
    println!("\nSimplifying...");
    let simplify_builder = Simplify::new();

    // We can enable debug print in Simplify if needed, but for now
    // let's just assert it finishes
    let simplified = simplify_builder.simplify(&diff_raw).unwrap();
    println!("Simplified: {}", simplified);
}
