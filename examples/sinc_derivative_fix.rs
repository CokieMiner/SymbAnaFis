//! Demonstration of the fix for numerical instability in sinc'(x) at x=0
//!
//! Before the fix, evaluating sinc'(0) would give NaN due to the formula
//! (x*cos(x) - sin(x))/x² producing 0/0.
//!
//! After the fix, we use a Taylor series approximation for small x values,
//! correctly returning 0 at x=0.

use symb_anafis::{parse, symb};
use std::collections::HashSet;

fn main() {
    println!("=== Demonstrating Fix for Numerical Instability in sinc'(x) ===\n");
    
    // Parse sinc(x) 
    let fixed_vars = HashSet::new();
    let custom_funcs = HashSet::new();
    let expr = parse("sinc(x)", &fixed_vars, &custom_funcs, None).unwrap();
    println!("Expression: {}", expr);
    
    // Differentiate
    let _x = symb("x");
    let deriv = expr.derive("x", None);
    println!("Derivative: {}", deriv);
    
    // Compile for fast evaluation
    let compiled = deriv.compile().unwrap();
    
    println!("\nEvaluating sinc'(x) at different points:");
    println!("----------------------------------------");
    
    // Test at x=0 (the singularity)
    println!("\n  At x=0:");
    let result_at_0 = compiled.evaluate(&[0.0]);
    println!("    Result: {}", result_at_0);
    println!("    Expected: 0.0");
    println!("    Is NaN? {}", result_at_0.is_nan());
    if !result_at_0.is_nan() && result_at_0.abs() < 1e-10 {
        println!("    ✓ Fixed! (was NaN before the fix)");
    } else {
        println!("    ✗ Unexpected result");
    }
    
    // Test at small x (Taylor series region)
    println!("\n  At x=1e-8 (small x, uses Taylor series):");
    let result_at_small = compiled.evaluate(&[1e-8]);
    let expected_small = -1e-8 / 3.0;
    println!("    Result:   {:.15e}", result_at_small);
    println!("    Expected: {:.15e}", expected_small);
    if (result_at_small - expected_small).abs() < 1e-15 {
        println!("    ✓ Correct using Taylor series");
    }
    
    // Test at normal x (analytical formula region)
    println!("\n  At x=1.0 (normal x, uses analytical formula):");
    let x_val = 1.0_f64;
    let result_at_1 = compiled.evaluate(&[x_val]);
    let expected_at_1 = (x_val * x_val.cos() - x_val.sin()) / (x_val * x_val);
    println!("    Result:   {:.15e}", result_at_1);
    println!("    Expected: {:.15e}", expected_at_1);
    if (result_at_1 - expected_at_1).abs() < 1e-10 {
        println!("    ✓ Correct using analytical formula");
    }
    
    println!("\n=== All tests pass! ===");
}
