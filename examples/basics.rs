/// 01 - Getting Started with SymbAnaFis
///
/// This example demonstrates the core functionality of the library.
/// Perfect for first-time users to understand the basic API.
///
/// Run with: cargo run --example 01_basics
use symb_anafis::{diff, simplify};

fn main() {
    println!("=== SYMB ANAFIS: GETTING STARTED ===\n");

    // ===================================
    // BASIC DIFFERENTIATION
    // ===================================
    println!("📌 BASIC DIFFERENTIATION\n");

    println!("Example 1: Power Rule");
    demo_diff("x^2", "x");
    demo_diff("x^3", "x");

    println!("\nExample 2: Product of Functions");
    demo_diff("x * sin(x)", "x");

    println!("\nExample 3: Quotient Rule");
    demo_diff("x / (x + 1)", "x");

    println!("\nExample 4: Chain Rule");
    demo_diff("sin(x^2)", "x");
    demo_diff("exp(2*x)", "x");

    println!("\nExample 5: Simple polynomial");
    demo_diff("x^2 + 3*x + 5", "x");

    println!("\nExample 6: Trigonometric product");
    demo_diff("sin(x) * cos(x)", "x");

    println!("\nExample 8: Complex polynomial");
    demo_diff("(x^2 + 3*x + 1)^2", "x");

    println!("\nExample 9: Mixed functions");
    demo_diff("x^2 * sin(x) + exp(x) * cos(x)", "x");

    println!("\nExample 10: Nested exponentials");
    demo_diff("exp(exp(x))", "x");

    println!("\nExample 11: Cubic polynomial");
    demo_diff("x^3 + 2*x^2 + 3*x + 1", "x");

    println!("\nExample 12: Polynomial factored form");
    demo_diff("(x + 1)^3", "x");

    println!("\nExample 13: Rational function with polynomial");
    demo_diff("(x^2 + 1) / (x + 1)", "x");
    println!("\n📌 BASIC SIMPLIFICATION\n");

    println!("Example 1: Combining Like Terms");
    demo_simplify("2*x + 3*x");
    demo_simplify("x * x");

    println!("\nExample 2: Perfect Square Factoring");
    demo_simplify("x^2 + 2*x + 1");

    println!("\nExample 3: Trigonometric Identity");
    demo_simplify("sin(x)^2 + cos(x)^2");

    println!("\nExample 5: Complex fractions");
    demo_simplify("(x^2 + 1)/(x^2 - 1) + 1/x");

    println!("\nExample 6: Trigonometric expansions");
    demo_simplify("sin(x)^2 + cos(x)^2 + tan(x)^2");

    println!("\nExample 7: Exponential identities");
    demo_simplify("exp(x) * exp(y) * exp(z)");

    println!("\nExample 8: Power manipulations");
    demo_simplify("(x^2)^3 * x^4 / x^5");

    println!("\nExample 9: Mixed operations");
    demo_simplify("2*x + 3*x - x + x^2 * x");

    println!("\nExample 10: Nested expressions");
    demo_simplify("e^(ln(x^2)) + sqrt(x^4)");

    println!("\nExample 11: Polynomial expansion");
    demo_simplify("(x + 1)^2");

    println!("\nExample 12: Polynomial with common factors");
    demo_simplify("x^3 + x^2");

    println!("\nExample 13: Difference of cubes approximation");
    demo_simplify("(x + 1)^3 - (x - 1)^3");

    // ===================================
    // FIXED VARIABLES & CUSTOM FUNCTIONS
    // ===================================
    println!("\n📌 FIXED VARIABLES (Constants)\n");

    println!("Example: Treating 'a' and 'b' as constants");
    let expr = "a * x^2 + b * x";
    match diff(
        expr.to_string(),
        "x".to_string(),
        Some(&["a".to_string(), "b".to_string()]),
        None,
    ) {
        Ok(result) => println!("  d/dx [{}] = {}", expr, result),
        Err(e) => println!("  Error: {}", e),
    }

    println!("\n📌 ADVANCED FIXED VARIABLES\n");

    println!("Example: Multiple constants in complex expression");
    let expr = "a*x^3 + b*x^2 + c*x + d";
    match diff(
        expr.to_string(),
        "x".to_string(),
        Some(&[
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
        ]),
        None,
    ) {
        Ok(result) => println!("  d/dx [{}] = {}", expr, result),
        Err(e) => println!("  Error: {}", e),
    }

    println!("\n📌 ADVANCED CUSTOM FUNCTIONS\n");

    println!("Example: Chain of custom functions");
    let expr = "f(g(h(x)))";
    match diff(
        expr.to_string(),
        "x".to_string(),
        None,
        Some(&["f".to_string(), "g".to_string(), "h".to_string()]),
    ) {
        Ok(result) => println!("  d/dx [{}] = {}", expr, result),
        Err(e) => println!("  Error: {}", e),
    }

    println!("\nExample: Product of custom functions");
    let expr = "f(x) * g(x) + h(x)^2";
    match diff(
        expr.to_string(),
        "x".to_string(),
        None,
        Some(&["f".to_string(), "g".to_string(), "h".to_string()]),
    ) {
        Ok(result) => println!("  d/dx [{}] = {}", expr, result),
        Err(e) => println!("  Error: {}", e),
    }

    println!("\n📌 ADVANCED MULTI-CHARACTER SYMBOLS\n");

    println!("Example: Complex Greek expressions");
    let expr = "(alpha^2 + beta^2)^gamma";
    match simplify(
        expr.to_string(),
        Some(&["alpha".to_string(), "beta".to_string(), "gamma".to_string()]),
        None,
    ) {
        Ok(result) => println!("  {} → {}", expr, result),
        Err(e) => println!("  Error: {}", e),
    }

    println!("\nExample: Mixed symbols and operations");
    let expr = "sigma * tau + omega^2";
    match simplify(
        expr.to_string(),
        Some(&["sigma".to_string(), "tau".to_string(), "omega".to_string()]),
        None,
    ) {
        Ok(result) => println!("  {} → {}", expr, result),
        Err(e) => println!("  Error: {}", e),
    }
}

fn demo_diff(expr: &str, var: &str) {
    match diff(expr.to_string(), var.to_string(), None, None) {
        Ok(result) => println!("  d/d{} [{}] = {}", var, expr, result),
        Err(e) => println!("  d/d{} [{}] = Error: {}", var, expr, e),
    }
}

fn demo_simplify(expr: &str) {
    match simplify(expr.to_string(), None, None) {
        Ok(result) => println!("  {} → {}", expr, result),
        Err(e) => println!("  {} → Error: {}", expr, e),
    }
}
