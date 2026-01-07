//! Dual Numbers for Automatic Differentiation
//!
//! This example demonstrates using `SymbAnaFis`'s Dual number implementation
//! for automatic differentiation. Dual numbers enable exact derivative computation
//! through algebraic manipulation.
//!
//! This is not the focus of the crate but could be useful
//!
//! Dual numbers extend real numbers with an infinitesimal ε where ε² = 0.
//! A dual number a + bε represents both a value (a) and its derivative (b).

use num_traits::Float;
use symb_anafis::Dual;

/// Evaluate f(x) = x² + 3x + 1 and its derivative at x = 2
fn quadratic_example() {
    println!("=== Quadratic Function: f(x) = x² + 3x + 1 ===");

    // Create dual number: x = 2 + 1ε (value 2, derivative 1)
    let x = Dual::new(2.0, 1.0);

    // Compute f(x) = x² + 3x + 1
    let fx = x * x + Dual::new(3.0, 0.0) * x + Dual::new(1.0, 0.0);

    println!("f(2) = {}", fx.val); // Should be 2² + 3*2 + 1 = 11
    println!("f'(2) = {}", fx.eps); // Should be 2*2 + 3 = 7
    println!("Expected: f(2) = 11, f'(2) = 7");
    println!();
}

/// Evaluate f(x) = sin(x) * exp(x) and its derivative
fn transcendental_example() {
    println!("=== Transcendental Function: f(x) = sin(x) * exp(x) ===");

    let x = Dual::new(1.0, 1.0); // x = 1 + 1ε

    // f(x) = sin(x) * exp(x)
    let sin_x = x.sin();
    let exp_x = x.exp();
    let fx = sin_x * exp_x;

    // f'(x) = cos(x) * exp(x) + sin(x) * exp(x) = exp(x) * (cos(x) + sin(x))
    let expected_deriv = x.val.exp() * (x.val.cos() + x.val.sin());

    println!("f(1) = {:.6}", fx.val);
    println!("f'(1) = {:.6}", fx.eps);
    println!("Expected f'(1) = {expected_deriv:.6}");
    println!("Error: {:.2e}", (fx.eps - expected_deriv).abs());
    println!();
}

/// Higher-order derivatives using dual numbers
fn higher_order_example() {
    println!("=== Higher-Order Derivatives ===");
    println!();

    // For f(x) = x³, f'(x) = 3x², f''(x) = 6x, f'''(x) = 6

    // First derivative at x = 2
    let x1 = Dual::new(2.0, 1.0);
    let fx1 = x1 * x1 * x1; // x³

    println!("f(x) = x³ at x = 2:");
    println!("f(2) = {}", fx1.val); // 8
    println!("f'(2) = {}", fx1.eps); // 3*2² = 12

    // For second derivative, evaluate f'(x) = 3x² with dual numbers
    let x2 = Dual::new(2.0, 1.0);
    let f_prime = Dual::new(3.0, 0.0) * x2 * x2; // 3x²
    println!("f''(2) = {}", f_prime.eps); // d/dx(3x²) at x=2 = 6x = 12

    // For third derivative, evaluate f''(x) = 6x with dual numbers
    let x3 = Dual::new(2.0, 1.0);
    let f_double_prime = Dual::new(6.0, 0.0) * x3; // 6x
    println!("f'''(2) = {}", f_double_prime.eps); // d/dx(6x) = 6
    println!();
}

/// Chain rule example: f(x) = sin(x² + 1)
fn chain_rule_example() {
    println!("=== Chain Rule: f(x) = sin(x² + 1) ===");

    let x = Dual::new(1.5, 1.0);

    // f(x) = sin(x² + 1)
    let inner = x * x + Dual::new(1.0, 0.0); // x² + 1
    let fx = inner.sin(); // sin(x² + 1)

    // f'(x) = cos(x² + 1) * (2x)
    let expected_deriv = inner.val.cos() * (2.0 * x.val);

    println!("f(1.5) = {:.6}", fx.val);
    println!("f'(1.5) = {:.6}", fx.eps);
    println!("Expected f'(1.5) = {expected_deriv:.6}");
    println!("Error: {:.2e}", (fx.eps - expected_deriv).abs());
    println!();
}

/// Compare dual number derivatives with symbolic differentiation
fn compare_with_symbolic() {
    println!("=== Comparison with Symbolic Differentiation ===");

    use symb_anafis::{Diff, symb};

    let x = symb("x");

    // f(x) = x³ + 2x² + x + 1
    let expr = x.pow(3.0) + 2.0 * x.pow(2.0) + x + 1.0;

    // Symbolic derivative
    let symbolic_deriv = Diff::new().differentiate(&expr, &x).unwrap();
    println!("Symbolic: d/dx(x³ + 2x² + x + 1) = {symbolic_deriv}");
    println!(
        "         (Note: {symbolic_deriv} is mathematically equivalent to 3x² + 4x + 1)"
    );

    // Evaluate symbolic derivative at x=2
    let vars = std::collections::HashMap::from([("x", 2.0)]);
    let symbolic_val = symbolic_deriv.evaluate(&vars, &std::collections::HashMap::new());
    println!("Symbolic derivative at x=2: {symbolic_val}");

    // Dual number derivative at x = 2
    let x_dual = Dual::new(2.0, 1.0);
    let two = Dual::new(2.0, 0.0);
    let one = Dual::new(1.0, 0.0);
    let fx = x_dual * x_dual * x_dual + two * x_dual * x_dual + x_dual + one;

    println!("Dual numbers at x = 2:");
    println!("f(2) = {}", fx.val); // Should be 8 + 8 + 2 + 1 = 19
    println!("f'(2) = {}", fx.eps); // Should be 3*4 + 4*2 + 1 = 20
    println!();
}

fn main() {
    println!("SymbAnaFis Dual Number Automatic Differentiation Examples");
    println!("========================================================\n");

    quadratic_example();
    transcendental_example();
    higher_order_example();
    chain_rule_example();
    compare_with_symbolic();

    println!("Key Benefits of Dual Numbers:");
    println!("• Exact derivatives (no numerical approximation)");
    println!("• Handles complex expressions automatically");
    println!("• Chain rule applied algebraically");
    println!("• Higher-order derivatives possible");
    println!("• Composable with other numeric operations");
}
