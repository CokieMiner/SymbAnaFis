//! Example demonstrating domain safety in simplification
//!
//! Domain safety controls whether simplification rules that may alter the domain of validity
//! are applied. For example, exp(ln(x)) = x is only valid for x > 0, so it's a domain-altering rule.
//!
//! By setting SYMB_ANAFIS_DOMAIN_SAFETY=true, you can enable conservative simplification that
//! avoids such rules.

use symb_anafis::{diff, simplify};

fn main() {

    println!("=== Domain Safety Example ===");
    println!("Current mode: {}\n",
        std::env::var("SYMB_ANAFIS_DOMAIN_SAFETY")
            .unwrap_or_else(|_| "disabled (aggressive simplification)".to_string()));

    // Example 1: exp(ln(x)) - domain-altering rule
    println!("Example 1: exp(ln(x))");
    println!("-----------------------");

    let expr = "exp(ln(x))".to_string();

    // Default behavior (SYMB_ANAFIS_DOMAIN_SAFETY not set or false)
    match simplify(expr.clone(), None, None) {
        Ok(result) => {
            println!("  Without domain safety: {}", result);
            println!("  (Simplified to x because exp(ln(x)) = x for x > 0)");
        }
        Err(e) => println!("  Error: {:?}", e),
    }

    println!("\n  To enable domain-safe mode, set: export SYMB_ANAFIS_DOMAIN_SAFETY=true");
    println!("  With domain safety: exp(ln(x))");
    println!("  (Not simplified because ln(x) assumes x > 0)\n");

    // Example 2: sqrt(x^2) - domain-altering rule
    println!("Example 2: sqrt(x^2)");
    println!("--------------------");

    let expr = "sqrt(x^2)".to_string();

    match simplify(expr.clone(), None, None) {
        Ok(result) => {
            println!("  Without domain safety: {}", result);
            println!("  (Simplified to |x| or abs(x))");
        }
        Err(e) => println!("  Error: {:?}", e),
    }

    println!("\n  With domain safety enabled:");
    println!("  sqrt(x^2) would remain as sqrt(x^2)");
    println!("  (Because simplifying to x assumes x ≥ 0)\n");

    // Example 3: exp(a*ln(b)) - domain-altering rule
    println!("Example 3: exp(2*ln(x))");
    println!("------------------------");

    let expr = "exp(2*ln(x))".to_string();

    match simplify(expr.clone(), None, None) {
        Ok(result) => {
            println!("  Without domain safety: {}", result);
            println!("  (Simplified to x^2 because exp(2*ln(x)) = x^2 for x > 0)");
        }
        Err(e) => println!("  Error: {:?}", e),
    }

    println!("\n  With domain safety enabled:");
    println!("  exp(2*ln(x)) remains unsimplified");
    println!("  (Because ln(x) requires x > 0)\n");

    // Example 4: Safe simplification (not domain-altering)
    println!("Example 4: x + 0 (safe simplification)");
    println!("--------------------------------------");

    let expr = "x + 0".to_string();

    match simplify(expr.clone(), None, None) {
        Ok(result) => {
            println!("  Without domain safety: {}", result);
            println!("  With domain safety:    {}", result);
            println!("  (Same result - this rule is safe and always applied)");
        }
        Err(e) => println!("  Error: {:?}", e),
    }

    // Example 5: Differentiation with domain-altering simplification
    println!("\nExample 5: d/dx[ln(x^2)] with simplification");
    println!("--------------------------------------------");

    let expr = "ln(x^2)";
    match diff(expr.to_string(), "x".to_string(), None, None) {
        Ok(result) => {
            println!("  Derivative: d/dx[ln(x^2)] = {}", result);
            println!("  (May simplify differently based on domain safety)");
        }
        Err(e) => println!("  Error: {:?}", e),
    }

    // Example 6: Power and root simplification
    println!("\nExample 6: (x^3)^(1/3) - domain-altering");
    println!("----------------------------------------");

    let expr = "(x^3)^(1/3)".to_string();
    match simplify(expr.clone(), None, None) {
        Ok(result) => {
            println!("  Result: {}", result);
            println!("  (Aggressive: simplifies to x; Safe: keeps original)");
        }
        Err(e) => println!("  Error: {:?}", e),
    }

    // Example 7: Multiple operations
    println!("\nExample 7: d/dx[exp(2*ln(x))] with simplification");
    println!("------------------------------------------------");

    let expr = "exp(2*ln(x))";
    match diff(expr.to_string(), "x".to_string(), None, None) {
        Ok(result) => {
            println!("  Derivative: {}", result);
        }
        Err(e) => println!("  Error: {:?}", e),
    }

    // Example 8: Trigonometric with domain issues
    println!("\nExample 8: sqrt(sin(x)^2 + cos(x)^2)");
    println!("------------------------------------");

    let expr = "sqrt(sin(x)^2 + cos(x)^2)".to_string();
    match simplify(expr.clone(), None, None) {
        Ok(result) => {
            println!("  Result: {}", result);
            println!("  (Should simplify to 1 - this is safe!)");
        }
        Err(e) => println!("  Error: {:?}", e),
    }

    // Example 9: Logarithm differentiation
    println!("\nExample 9: d/dx[log(exp(x))]");
    println!("---------------------------");

    let expr = "log(exp(x))";
    match diff(expr.to_string(), "x".to_string(), None, None) {
        Ok(result) => {
            println!("  Derivative: {}", result);
        }
        Err(e) => println!("  Error: {:?}", e),
    }

    // Example 10: Complex expression
    println!("\nExample 10: Simplify sqrt(x^4)");
    println!("------------------------------");

    let expr = "sqrt(x^4)".to_string();
    match simplify(expr.clone(), None, None) {
        Ok(result) => {
            println!("  Result: {}", result);
            println!("  (Aggressive: x^2; Safe: sqrt(x^4))");
        }
        Err(e) => println!("  Error: {:?}", e),
    }

    println!("\n=== Running with Different Modes ===");
    println!("Default (aggressive simplification):");
    println!("  cargo run --example domain_safety\n");

    println!("Domain-safe mode (conservative simplification):");
    println!("  SYMB_ANAFIS_DOMAIN_SAFETY=true cargo run --example domain_safety\n");

    println!("Configuration environment variables:");
    println!("  SYMB_ANAFIS_DOMAIN_SAFETY - 'true', '1' to enable domain-safe mode (default: false)");
    println!("  SYMB_ANAFIS_MAX_DEPTH     - Maximum AST depth allowed (default: 100)");
    println!("  SYMB_ANAFIS_MAX_NODES     - Maximum AST nodes allowed (default: 10,000)");
}
