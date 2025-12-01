/// 02 - Comprehensive Differentiation Guide
///
/// Complete showcase of all differentiation capabilities including:
/// - Basic rules (power, product, quotient, chain)
/// - All trigonometric functions (standard + inverse)
/// - Hyperbolic functions
/// - Exponential and logarithmic
/// - Special functions
/// - Complex nested expressions
///
/// Run with: cargo run --example 02_differentiation
use symb_anafis::diff;

fn main() {
    println!("=== COMPREHENSIVE DIFFERENTIATION GUIDE ===\n");

    // ===================================
    // BASIC DIFFERENTIATION RULES
    // ===================================
    println!("📌 SECTION 1: BASIC RULES\n");

    println!("Power Rule:");
    demo("x", "x"); // → 1
    demo("x^2", "x"); // → 2*x
    demo("x^3", "x"); // → 3*x^2
    demo("x^(-1)", "x"); // → -x^(-2)
    demo("sqrt(x)", "x"); // → 1/(2*sqrt(x))

    println!("\nConstant Rule:");
    demo("5", "x"); // → 0
    demo("2*x", "x"); // → 2

    println!("\nProduct Rule:");
    demo("x * sin(x)", "x"); // → sin(x) + x*cos(x)
    demo("x^2 * e^x", "x"); // → 2*x*exp(x) + x^2*exp(x)

    println!("\nQuotient Rule:");
    demo("x / (x + 1)", "x"); // → 1/(x+1) - x/(x+1)^2
    demo("sin(x) / cos(x)", "x"); // → (simplified to sec^2(x))
    demo("(x^2 + 1) / (x - 1)", "x"); // applying quotient rule

    // ===================================
    // CHAIN RULE
    // ===================================
    println!("\n📌 SECTION 2: CHAIN RULE\n");

    println!("Simple Composition:");
    demo("sin(2*x)", "x"); // → 2*cos(2*x)
    demo("cos(x^2)", "x"); // → -2*x*sin(x^2)
    demo("exp(3*x)", "x"); // → 3*exp(3*x)

    println!("\nNested Composition:");
    demo("sin(cos(x))", "x"); // → -sin(x)*cos(cos(x))
    demo("exp(sin(x^2))", "x"); // → 2*x*cos(x^2)*exp(sin(x^2))
    demo("ln(sin(x))", "x"); // → cos(x)/sin(x) = cot(x)

    println!("\nPower Chain Rule:");
    demo("(x^2 + 1)^3", "x"); // → 3*(x^2+1)^2 * 2*x
    demo("(sin(x))^2", "x"); // → 2*sin(x)*cos(x)
    demo("(x + 1)^(-1)", "x"); // → -(x+1)^(-2)

    // ===================================
    // TRIGONOMETRIC DERIVATIVES
    // ===================================
    println!("\n📌 SECTION 3: TRIGONOMETRIC FUNCTIONS\n");

    println!("Standard Trig:");
    demo("sin(x)", "x"); // → cos(x)
    demo("cos(x)", "x"); // → -sin(x)
    demo("tan(x)", "x"); // → sec^2(x) or 1/cos^2(x)
    demo("cot(x)", "x"); // → -csc^2(x)
    demo("sec(x)", "x"); // → sec(x)*tan(x)
    demo("csc(x)", "x"); // → -csc(x)*cot(x)

    println!("\nInverse Trig:");
    demo("asin(x)", "x"); // → 1/sqrt(1-x^2)
    demo("acos(x)", "x"); // → -1/sqrt(1-x^2)
    demo("atan(x)", "x"); // → 1/(1+x^2)
    demo("acot(x)", "x"); // → -1/(1+x^2)
    demo("asec(x)", "x"); // → 1/(|x|*sqrt(x^2-1))
    demo("acsc(x)", "x"); // → -1/(|x|*sqrt(x^2-1))

    // ===================================
    // HYPERBOLIC DERIVATIVES
    // ===================================
    println!("\n📌 SECTION 4: HYPERBOLIC FUNCTIONS\n");

    println!("Standard Hyperbolic:");
    demo("sinh(x)", "x"); // → cosh(x)
    demo("cosh(x)", "x"); // → sinh(x)
    demo("tanh(x)", "x"); // → sech^2(x)
    demo("coth(x)", "x"); // → -csch^2(x)
    demo("sech(x)", "x"); // → -sech(x)*tanh(x)
    demo("csch(x)", "x"); // → -csch(x)*coth(x)

    println!("\nInverse Hyperbolic:");
    demo("asinh(x)", "x"); // → 1/sqrt(x^2+1)
    demo("acosh(x)", "x"); // → 1/sqrt(x^2-1)
    demo("atanh(x)", "x"); // → 1/(1-x^2)

    // ===================================
    // EXPONENTIAL & LOGARITHMIC
    // ===================================
    println!("\n📌 SECTION 5: EXPONENTIAL & LOGARITHMIC\n");

    println!("Exponential:");
    demo("e^x", "x"); // → exp(x)
    demo("exp(x)", "x"); // → exp(x)
    demo("2^x", "x"); // → 2^x * ln(2)
    demo("x^x", "x"); // → x^x * (ln(x) + 1) (logarithmic differentiation)

    println!("\nComplex Power Rules:");
    demo("x^(2*x)", "x"); // → x^(2*x) * (ln(x) + 1) * 2
    demo("(x^2 + 1)^(x)", "x"); // Very complex
    demo("x^(x^2)", "x"); // Extremely complex

    println!("\nWith Chain Rule:");
    demo("e^(x^2)", "x"); // → 2*x*exp(x^2)
    demo("ln(x^2)", "x"); // → 2/x (simplified)
    demo("ln(sin(x))", "x"); // → cot(x)
    demo("exp(sin(x))", "x"); // → cos(x)*exp(sin(x))

    // ===================================
    // SPECIAL FUNCTIONS
    // ===================================
    println!("\n📌 SECTION 6: SPECIAL FUNCTIONS\n");

    println!("Root Functions:");
    demo("sqrt(x)", "x"); // → 1/(2*sqrt(x))
    demo("cbrt(x)", "x"); // → 1/(3*cbrt(x)^2)
    demo("x^(1/2)", "x"); // → (1/2)*x^(-1/2)
    demo("x^(1/3)", "x"); // → (1/3)*x^(-2/3)

    println!("\nError Functions:");
    demo("erf(x)", "x"); // → (2/sqrt(pi))*exp(-x^2)
    demo("erfc(x)", "x"); // → -(2/sqrt(pi))*exp(-x^2)

    // ===================================
    // COMPLEX EXPRESSIONS
    // ===================================
    println!("\n📌 SECTION 7: COMPLEX EXPRESSIONS\n");

    println!("Multiple Rules Combined:");
    demo("x^2 * sin(x) * e^x", "x");
    demo("(x^2 + 1) * exp(x) / x", "x");
    demo("sin(x) * cos(x) * tan(x)", "x");

    println!("\nPolynomial Expressions:");
    demo("(x + 1)^2", "x"); // Should expand and simplify
    demo("(x + 1)^3", "x"); // Cubic expansion
    demo("(x^2 + 2*x + 1) / (x + 1)", "x"); // Rational with cancellation

    println!("\nExtremely Complex Expressions:");
    demo("sin(x^2) * cos(x^3) + exp(x^4) * ln(x^5)", "x");
    demo("(x^2 + sin(x))^3 / exp(x)", "x");
    demo("sqrt(x^4 + 1) * tanh(x^2)", "x");

    println!("\nRational Functions:");
    demo("(x^2 - 1) / (x^2 + 1)", "x");
    demo("(sin(x) + cos(x)) / (sin(x) - cos(x))", "x");

    // ===================================
    // CUSTOM FUNCTIONS
    // ===================================
    println!("\n📌 SECTION 8: CUSTOM FUNCTIONS\n");

    println!("Differentiating expressions with user-defined functions:");
    let expr = "f(g(x))";
    let custom_funcs = vec!["f".to_string(), "g".to_string()];
    match diff(expr.to_string(), "x".to_string(), None, Some(&custom_funcs)) {
        Ok(result) => println!("  d/dx [{}] = {}", expr, result),
        Err(e) => println!("  Error: {}", e),
    }

    // ===================================
    // HIGHER-ORDER DERIVATIVES
    // ===================================
    println!("\n📌 SECTION 9: HIGHER-ORDER DERIVATIVES\n");

    println!("Second Derivatives (computed by differentiating twice):");
    // First derivative
    let expr = "sin(x)";
    match diff(expr.to_string(), "x".to_string(), None, None) {
        Ok(first) => {
            println!("  d/dx [{}] = {}", expr, first);
            // Second derivative
            match diff(first.clone(), "x".to_string(), None, None) {
                Ok(second) => println!("  d²/dx² [{}] = {}", expr, second),
                Err(e) => println!("  Error in second derivative: {}", e),
            }
        }
        Err(e) => println!("  Error: {}", e),
    }

    println!("\nThird and Higher Derivatives:");
    let expr = "f(x)";
    let custom_funcs = vec!["f".to_string()];
    match diff(expr.to_string(), "x".to_string(), None, Some(&custom_funcs)) {
        Ok(first) => {
            println!("  1st derivative of {}: {}", expr, first);
            match diff(first.clone(), "x".to_string(), None, Some(&custom_funcs)) {
                Ok(second) => {
                    println!("  2nd derivative: {}", second);
                    match diff(second.clone(), "x".to_string(), None, Some(&custom_funcs)) {
                        Ok(third) => {
                            println!("  3rd derivative: {}", third);
                            match diff(third.clone(), "x".to_string(), None, Some(&custom_funcs)) {
                                Ok(fourth) => println!("  4th derivative: {}", fourth),
                                Err(e) => println!("  Error in 4th derivative: {}", e),
                            }
                        }
                        Err(e) => println!("  Error in 3rd derivative: {}", e),
                    }
                }
                Err(e) => println!("  Error in 2nd derivative: {}", e),
            }
        }
        Err(e) => println!("  Error: {}", e),
    }
}

fn demo(expr: &str, var: &str) {
    match diff(expr.to_string(), var.to_string(), None, None) {
        Ok(result) => println!("  d/d{} [{}] = {}", var, expr, result),
        Err(e) => println!("  d/d{} [{}] = Error: {}", var, expr, e),
    }
}
