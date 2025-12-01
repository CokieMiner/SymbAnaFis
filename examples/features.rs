/// Comprehensive demonstration of advanced features
///
/// Run with: cargo run --example features
use symb_anafis::diff;

fn main() {
    println!("=== ADVANCED FEATURES EXAMPLES ===\n");

    // ===================================
    // CUSTOM FUNCTIONS
    // ===================================
    println!("📌 CUSTOM FUNCTIONS\n");

    let fixed_vars = vec!["alpha".to_string(), "beta".to_string()];
    let custom_funcs = vec!["psi".to_string(), "phi".to_string()];
    let expr = "alpha * psi(x) + beta * phi(x^2)";
    match diff(
        expr.to_string(),
        "x".to_string(),
        Some(&fixed_vars),
        Some(&custom_funcs),
    ) {
        Ok(result) => println!("  d/dx [{}] = {}", expr, result),
        Err(e) => println!("  Error: {}", e),
    }

    // ===================================
    // IMPLICIT FUNCTIONS
    // ===================================
    println!("\n📌 IMPLICIT FUNCTIONS\n");

    let expr = "x * y(x)";
    let custom_funcs = vec!["y".to_string()];
    match diff(expr.to_string(), "x".to_string(), None, Some(&custom_funcs)) {
        Ok(result) => println!("  d/dx [{}] = {}", expr, result),
        Err(e) => println!("  Error: {}", e),
    }

    // ===================================
    // COMPLEX NESTED EXPRESSIONS
    // ===================================
    println!("\n📌 COMPLEX NESTED EXPRESSIONS\n");

    let expressions = vec![
        "sin(cos(exp(x^2)))",
        "ln(tanh(x))",
        "sin(sin(sin(sin(x))))",
        "x * exp(x) * sin(x) * ln(x)",
    ];

    for expr in expressions {
        match diff(expr.to_string(), "x".to_string(), None, None) {
            Ok(result) => println!("  d/dx [{}] = {}", expr, result),
            Err(e) => println!("  Error: {}", e),
        }
    }

    // ===================================
    // POLYNOMIALS
    // ===================================
    println!("\n📌 POLYNOMIALS\n");

    println!("Example: Quadratic expansion");
    demo_simplify("(x + y)^2");

    println!("\nExample: Cubic expansion");
    demo_simplify("(a + b)^3");

    println!("\nExample: Polynomial with constants");
    let expr = "(x + alpha)^2";
    match symb_anafis::simplify(expr.to_string(), Some(&["alpha".to_string()]), None) {
        Ok(result) => println!("  {} → {}", expr, result),
        Err(e) => println!("  Error: {}", e),
    }

    println!("\nExample: Polynomial derivative and simplification");
    let expr = "(x + 1)^3";
    match diff(expr.to_string(), "x".to_string(), None, None) {
        Ok(result) => {
            println!("  d/dx [{}] = {}", expr, result);
            match symb_anafis::simplify(result, None, None) {
                Ok(simplified) => println!("  Simplified: {}", simplified),
                Err(e) => println!("  Simplification error: {}", e),
            }
        }
        Err(e) => println!("  Error: {}", e),
    }

    // ===================================
    // PARTIAL DERIVATIVES
    // ===================================
    println!("\n📌 PARTIAL DERIVATIVES\n");

    println!("Example: Mixed partials");
    let expr = "x^2 * y + sin(x*y) + exp(x + y)";
    // ∂/∂x
    match diff(
        expr.to_string(),
        "x".to_string(),
        Some(&["y".to_string()]),
        None,
    ) {
        Ok(result) => println!("  ∂/∂x [{}] = {}", expr, result),
        Err(e) => println!("  Error: {}", e),
    }
    // ∂/∂y
    match diff(
        expr.to_string(),
        "y".to_string(),
        Some(&["x".to_string()]),
        None,
    ) {
        Ok(result) => println!("  ∂/∂y [{}] = {}", expr, result),
        Err(e) => println!("  Error: {}", e),
    }

    // ===================================
    // MULTIVARIABLE FUNCTIONS
    // ===================================
    println!("\n📌 MULTIVARIABLE FUNCTIONS\n");

    println!("Example: Gradient components");
    let f = "x^2 + y^2 + z^2";
    let grad_x = diff(
        f.to_string(),
        "x".to_string(),
        Some(&["y".to_string(), "z".to_string()]),
        None,
    )
    .unwrap();
    let grad_y = diff(
        f.to_string(),
        "y".to_string(),
        Some(&["x".to_string(), "z".to_string()]),
        None,
    )
    .unwrap();
    let grad_z = diff(
        f.to_string(),
        "z".to_string(),
        Some(&["x".to_string(), "y".to_string()]),
        None,
    )
    .unwrap();
    println!("  ∇f = ({}, {}, {}) for f = {}", grad_x, grad_y, grad_z, f);

    // ===================================
    // VECTOR CALCULUS
    // ===================================
    println!("\n📌 VECTOR CALCULUS\n");

    println!("Example: Divergence");
    let vector_field = "x*i + y*j + z*k"; // Simple radial field
    let div_x = diff(
        "x".to_string(),
        "x".to_string(),
        Some(&["y".to_string(), "z".to_string()]),
        None,
    )
    .unwrap();
    let div_y = diff(
        "y".to_string(),
        "y".to_string(),
        Some(&["x".to_string(), "z".to_string()]),
        None,
    )
    .unwrap();
    let div_z = diff(
        "z".to_string(),
        "z".to_string(),
        Some(&["x".to_string(), "y".to_string()]),
        None,
    )
    .unwrap();
    println!(
        "  ∇·F = {} + {} + {} for F = {}",
        div_x, div_y, div_z, vector_field
    );
}

fn demo_simplify(expr: &str) {
    match symb_anafis::simplify(expr.to_string(), None, None) {
        Ok(result) => println!("  {} → {}", expr, result),
        Err(e) => println!("  {} → Error: {}", expr, e),
    }
}
