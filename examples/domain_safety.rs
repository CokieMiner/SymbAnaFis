//! Example demonstrating domain safety in simplification
//!
//! Domain safety controls whether simplification rules that may alter the domain of validity
//! are applied. For example, exp(ln(x)) = x is only valid for x > 0, so it's a domain-altering rule.
//!
//! Run with:
//!   cargo run --example domain_safety                          # Normal (aggressive) simplification
//!   cargo run --example domain_safety -- --safe                # Domain-safe simplification

use std::env;
use symb_anafis::{diff, simplify};

fn main() {
    // Check for --safe flag in command line arguments
    let args: Vec<String> = env::args().collect();
    let domain_safe = args.iter().any(|arg| arg == "--safe" || arg == "-s");

    // Set the environment variable so the library uses domain-safe mode
    // SAFETY: We're setting an environment variable before any parallel code runs
    if domain_safe {
        unsafe {
            env::set_var("SYMB_ANAFIS_DOMAIN_SAFETY", "true");
        }
    }

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║          DOMAIN SAFETY IN SYMBOLIC SIMPLIFICATION                ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    if domain_safe {
        println!("🔒 Mode: DOMAIN-SAFE (conservative simplification)");
        println!("   Rules that may alter domain of validity are skipped.\n");
    } else {
        println!("⚡ Mode: NORMAL (aggressive simplification)");
        println!("   All simplification rules are applied.\n");
        println!("   To enable domain safety, run with:");
        println!("   SYMB_ANAFIS_DOMAIN_SAFETY=true cargo run --example domain_safety\n");
    }

    println!("═══════════════════════════════════════════════════════════════════");
    println!("1. EXPONENTIAL-LOGARITHM IDENTITIES");
    println!("═══════════════════════════════════════════════════════════════════\n");

    println!("📌 exp(ln(x)) = x");
    println!("   Valid only for x > 0 (ln is undefined for x ≤ 0)\n");
    demo_simplify("exp(ln(x))");
    demo_simplify("exp(ln(a))");
    demo_simplify("exp(2 * ln(x))");

    println!("\n📌 ln(exp(x)) = x");
    println!("   This is safe for all real x (exp is always positive)\n");
    demo_simplify("ln(exp(x))");
    demo_simplify("ln(exp(2*x))");

    println!("\n═══════════════════════════════════════════════════════════════════");
    println!("2. POWER AND ROOT SIMPLIFICATIONS");
    println!("═══════════════════════════════════════════════════════════════════\n");

    println!("📌 sqrt(x^2) = |x| vs x");
    println!("   sqrt(x^2) = x only for x ≥ 0; for all real x, sqrt(x^2) = |x|\n");
    demo_simplify("sqrt(x^2)");
    demo_simplify("(x^2)^(1/2)");

    println!("\n📌 sqrt(x) * sqrt(y) = sqrt(x*y)");
    println!("   Only valid for x,y ≥ 0. The rule checks for known non-negative expressions.\n");
    demo_simplify("sqrt(x) * sqrt(y)"); // Unknown sign - depends on domain_safe mode
    demo_simplify("sqrt(x^2) * sqrt(y^2)"); // Both simplify to abs() first
    demo_simplify("sqrt(exp(x)) * sqrt(exp(y))"); // exp() is always positive

    println!("\n📌 (x^a)^b = x^(a*b)");
    println!("   Not always valid: (-1)^(1/2))^2 = i^2 = -1, but (-1)^1 = -1 ✓\n");
    println!("   However (-8)^(2/3) via ((-8)^2)^(1/3) = 64^(1/3) = 4");
    println!("   But (-8)^(1/3))^2 = (-2)^2 = 4 only if we allow complex roots\n");
    demo_simplify("(x^2)^3");
    demo_simplify("(x^3)^2");

    println!("═══════════════════════════════════════════════════════════════════");
    println!("3. LOGARITHM PROPERTIES");
    println!("═══════════════════════════════════════════════════════════════════\n");

    println!("📌 ln(a) + ln(b) → ln(a*b) [COMBINING - domain safe]");
    println!("   If ln(a) and ln(b) exist, then a,b > 0 so ln(a*b) is valid\n");
    demo_simplify("ln(x) + ln(y)");
    demo_simplify("ln(a) + ln(b) + ln(c)");

    println!("\n📌 ln(a) - ln(b) → ln(a/b) [COMBINING - domain safe]");
    println!("   If ln(a) and ln(b) exist, then a,b > 0 so ln(a/b) is valid\n");
    demo_simplify("ln(x) - ln(y)");

    println!("\n📌 ln(x*y) → ln(x) + ln(y) [SPLITTING - NOT implemented]");
    println!("   Would alter domain: ln((-2)*(-3)) = ln(6) exists,");
    println!("   but ln(-2) + ln(-3) is undefined in reals\n");
    demo_simplify("ln(x * y)");
    demo_simplify("ln(a * b * c)");

    println!("\n📌 ln(x^n) = n * ln(x)");
    println!("   Only valid when x > 0\n");
    demo_simplify("ln(x^2)");
    demo_simplify("ln(x^3)");
    demo_simplify("ln(x^n)");

    println!("\n═══════════════════════════════════════════════════════════════════");
    println!("4. INVERSE FUNCTION COMPOSITIONS");
    println!("═══════════════════════════════════════════════════════════════════\n");

    println!("📌 sin(asin(x)) = x");
    println!("   Only valid for -1 ≤ x ≤ 1 (domain of asin)\n");
    demo_simplify("sin(asin(x))");
    demo_simplify("cos(acos(x))");
    demo_simplify("tan(atan(x))");

    println!("\n📌 asin(sin(x)) = x");
    println!("   Only valid for -π/2 ≤ x ≤ π/2 (principal branch)\n");
    demo_simplify("asin(sin(x))");

    println!("\n═══════════════════════════════════════════════════════════════════");
    println!("5. HYPERBOLIC FUNCTION IDENTITIES");
    println!("═══════════════════════════════════════════════════════════════════\n");

    println!("📌 sinh(asinh(x)) = x");
    println!("   Safe for all real x\n");
    demo_simplify("sinh(asinh(x))");
    demo_simplify("cosh(acosh(x))");
    demo_simplify("tanh(atanh(x))");

    println!("\n═══════════════════════════════════════════════════════════════════");
    println!("6. DIFFERENTIATION WITH DOMAIN SAFETY");
    println!("═══════════════════════════════════════════════════════════════════\n");

    println!("📌 Derivatives are simplified according to domain safety setting\n");

    demo_diff("ln(x)", "x");
    demo_diff("exp(ln(x))", "x");
    demo_diff("sqrt(x^2)", "x");
    demo_diff("ln(x^2)", "x");

    println!("\n═══════════════════════════════════════════════════════════════════");
    println!("7. PRACTICAL EXAMPLES");
    println!("═══════════════════════════════════════════════════════════════════\n");

    println!("📌 Physics: Entropy change ΔS = R * ln(V2/V1)");
    println!("   V1, V2 must be positive (volumes)\n");
    println!("   Note: Multi-char symbols like V1, V2 need to be passed as fixed_vars\n");
    demo_simplify_fixed("R * ln(V2 / V1)", &["V1", "V2", "R"]);
    demo_simplify_fixed("R * ln(V2) - R * ln(V1)", &["V1", "V2", "R"]);

    println!("\n📌 Engineering: Decibel formula dB = 10 * log10(P/P0)");
    println!("   P, P0 must be positive (power levels)\n");
    demo_simplify_fixed("10 * ln(P) / ln(10) - 10 * ln(P0) / ln(10)", &["P", "P0"]);

    println!("\n📌 Finance: Continuous compounding A = P * exp(r*t)");
    println!("   Finding time: t = ln(A/P) / r\n");
    demo_simplify("ln(exp(r * t))");

    println!("\n═══════════════════════════════════════════════════════════════════");
    println!("8. SPECIAL CONSTANTS");
    println!("═══════════════════════════════════════════════════════════════════\n");

    println!("📌 ln(e) = 1 (when 'e' is Euler's number, not a variable)");
    println!("   The library recognizes 'e' as Euler's constant by default\n");
    demo_simplify("ln(e)");
    demo_simplify("ln(e^2)");
    demo_simplify("exp(1)");

    println!("\n📌 Using 'e' as a variable (fixed_vars)");
    println!("   When 'e' is passed as a fixed variable, it's treated as a symbol\n");
    demo_simplify_fixed("ln(e)", &["e"]);
    demo_simplify_fixed("e^x", &["e"]);

    // Differentiate with e as fixed variable
    println!("\n  Differentiation with 'e' as fixed variable:");
    match diff(
        "e^x".to_string(),
        "x".to_string(),
        Some(&["e".to_string()]),
        None,
    ) {
        Ok(result) => println!("    d/dx [e^x] (e as constant) = {}", result),
        Err(e) => println!("    Error: {}", e),
    }

    // Differentiate normally (e as Euler's number)
    println!("\n  Differentiation with 'e' as Euler's number:");
    match diff("e^x".to_string(), "x".to_string(), None, None) {
        Ok(result) => println!("    d/dx [e^x] (e = Euler's number) = {}", result),
        Err(e) => println!("    Error: {}", e),
    }

    println!("\n═══════════════════════════════════════════════════════════════════");
    println!("SUMMARY");
    println!("═══════════════════════════════════════════════════════════════════\n");

    if domain_safe {
        println!("🔒 Domain-safe mode is ENABLED");
        println!("   - exp(ln(x)) is NOT simplified to x");
        println!("   - sqrt(x^2) → abs(x) (mathematically correct)");
        println!("   - Inverse compositions are preserved");
        println!("   - Use this when domain correctness is critical");
    } else {
        println!("⚡ Normal mode is ENABLED (aggressive simplification)");
        println!("   - exp(ln(x)) → x");
        println!("   - sqrt(x^2) → abs(x) (always correct)");
        println!("   - Inverse compositions are simplified");
        println!("   - Results may have different domain than input");
    }

    println!("\n   Toggle with: SYMB_ANAFIS_DOMAIN_SAFETY=true");
}

fn demo_simplify(expr: &str) {
    match simplify(expr.to_string(), None, None) {
        Ok(simplified) => {
            if simplified == expr {
                println!("  {} → {} (unchanged)", expr, simplified);
            } else {
                println!("  {} → {}", expr, simplified);
            }
        }
        Err(e) => println!("  {} → Error: {}", expr, e),
    }
}

fn demo_simplify_fixed(expr: &str, fixed: &[&str]) {
    let fixed_vars: Vec<String> = fixed.iter().map(|s| s.to_string()).collect();
    match simplify(expr.to_string(), Some(&fixed_vars), None) {
        Ok(simplified) => {
            if simplified == expr {
                println!(
                    "  {} → {} (unchanged, fixed: {:?})",
                    expr, simplified, fixed
                );
            } else {
                println!("  {} → {} (fixed: {:?})", expr, simplified, fixed);
            }
        }
        Err(e) => println!("  {} → Error: {} (fixed: {:?})", expr, e, fixed),
    }
}

fn demo_diff(expr: &str, var: &str) {
    match diff(expr.to_string(), var.to_string(), None, None) {
        Ok(derivative) => println!("  d/d{} [{}] = {}", var, expr, derivative),
        Err(e) => println!("  d/d{} [{}] = Error: {}", var, expr, e),
    }
}
