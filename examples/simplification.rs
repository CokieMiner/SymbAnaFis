/// 03 - Comprehensive Simplification Guide
///
/// Complete showcase of all simplification rules including:
/// - Algebraic (factoring, fractions, powers, signs)
/// - Trigonometric identities
/// - Hyperbolic identities  
/// - Logarithmic/Exponential properties
/// - Root simplifications
///
/// Run with: cargo run --example 03_simplification
use symb_anafis::simplify;

fn main() {
    println!("=== COMPREHENSIVE SIMPLIFICATION GUIDE ===\n");

    // ===================================
    // ALGEBRAIC SIMPLIFICATIONS
    // ===================================
    println!("📌 SECTION 1: ALGEBRAIC SIMPLIFICATIONS\n");

    println!("Factoring - Common Factors:");
    demo("x*y + x*z"); // → x*(y+z)
    demo("2*x + 2*y"); // → 2*(x+y)
    demo("e^x + e^x*sin(x)"); // → exp(x)*(1+sin(x))
    demo("x^2*y + x^2*z"); // → x^2*(y+z)

    println!("\nFactoring - Perfect Squares:");
    demo("x^2 + 2*x + 1"); // → (x+1)^2
    demo("x^2 + 2*x*y + y^2"); // → (x+y)^2
    demo("4*x^2 + 4*x + 1"); // → (2*x+1)^2

    println!("\nAdvanced Factoring:");
    demo("x^3 + 3*x^2 + 3*x + 1"); // → (x+1)^3
    demo("x^4 - 1"); // → (x^2+1)*(x+1)*(x-1)
    demo("x^6 + x^3 + 1"); // Complex factoring

    println!("\nPolynomial Expansion (for cancellation):");
    demo("(x + 1)^2 / (x + 1)"); // Should expand and cancel
    demo("(x + 1)^3 / (x^2 + 2*x + 1)"); // Should simplify
    demo("(x - 1)^2 * (x + 1) / (x^2 - 1)"); // Should cancel

    println!("\nComplex Fraction Operations:");
    demo("(x^2 + 1)/(x^2 - 1) + 1/(x + 1)");
    demo("1/(x^2 - 1) - 1/(x^2 + 1)");
    demo("(x^3 + 1)/(x^2 + x + 1)");

    println!("\nSign Cleanup:");
    demo("-(x - y)"); // → y - x
    demo("-(A - B)"); // → B - A
    demo("-1 * (x - 2)"); // → 2 - x

    println!("\nExponential Canonical Form:");
    demo("e^x"); // → exp(x)
    demo("e^(2*x)"); // → exp(2*x)
    demo("exp(x)^2"); // → exp(2*x)
    demo("exp(x)^3"); // → exp(3*x)

    println!("\nExponential Combination:");
    demo("e^x * e^y"); // → exp(x+y)
    demo("exp(a) * exp(b)"); // → exp(a+b)
    demo("exp(x) * exp(2*x)"); // → exp(3*x)

    println!("\nPower Simplification:");
    demo("x * x"); // → x^2
    demo("x^2 * x^3"); // → x^5
    demo("(x^2)^3"); // → x^6
    demo("2 * 2^x"); // → 2^(x+1)
    demo("x^a * x^b"); // → x^(a+b)

    println!("\nLike Terms:");
    demo("x + x"); // → 2*x
    demo("2*x + 3*x"); // → 5*x
    demo("sin(x) + sin(x)"); // → 2*sin(x)
    demo("x*y + 2*x*y"); // → 3*x*y

    // ===================================
    // TRIGONOMETRIC IDENTITIES
    // ===================================
    println!("\n📌 SECTION 2: TRIGONOMETRIC IDENTITIES\n");

    println!("Pythagorean Identities:");
    demo("sin(x)^2 + cos(x)^2"); // → 1
    demo("1 - cos(x)^2"); // → sin(x)^2
    demo("1 + tan(x)^2"); // → sec(x)^2
    demo("1 + cot(x)^2"); // → csc(x)^2

    println!("\nExact Values:");
    demo("sin(0)"); // → 0
    demo("cos(0)"); // → 1
    demo("tan(0)"); // → 0
    demo("sin(3.14159265359/2)"); // → 1 (approximately π/2)

    println!("\nParity (Odd/Even Functions):");
    demo("sin(-x)"); // → -sin(x)
    demo("cos(-x)"); // → cos(x)
    demo("tan(-x)"); // → -tan(x)

    println!("\nInverse Composition:");
    demo("sin(asin(x))"); // → x
    demo("cos(acos(x))"); // → x
    demo("tan(atan(x))"); // → x
    demo("asin(sin(x))"); // → x

    println!("\nAdvanced Trigonometric Identities:");
    demo("sin(2*x)"); // → 2*sin(x)*cos(x)
    demo("cos(2*x)"); // → cos(x)^2 - sin(x)^2
    demo("tan(x + y)"); // → (tan(x) + tan(y))/(1 - tan(x)*tan(y))
    demo("sin(x)^4 + cos(x)^4"); // → 1 - 2*sin(x)^2*cos(x)^2

    println!("\nComplex Angle Identities:");
    demo("sin(3*x)"); // → 3*sin(x) - 4*sin(x)^3
    demo("cos(3*x)"); // → 4*cos(x)^3 - 3*cos(x)
    demo("tan(2*x)"); // → 2*tan(x)/(1 - tan(x)^2)

    // ===================================
    // HYPERBOLIC IDENTITIES
    // ===================================
    println!("\n📌 SECTION 3: HYPERBOLIC IDENTITIES\n");

    println!("Hyperbolic Pythagorean:");
    demo("cosh(x)^2 - sinh(x)^2"); // → 1
    demo("1 - tanh(x)^2"); // → sech(x)^2
    demo("coth(x)^2 - 1"); // → csch(x)^2

    println!("\nExponential Form Recognition:");
    demo("(e^x - e^(-x))/2"); // → sinh(x)
    demo("(e^x + e^(-x))/2"); // → cosh(x)
    demo("(e^x - e^(-x))/(e^x + e^(-x))"); // → tanh(x)

    println!("\nRatio Identities:");
    demo("sinh(x)/cosh(x)"); // → tanh(x)
    demo("cosh(x)/sinh(x)"); // → coth(x)
    demo("1/cosh(x)"); // → sech(x)
    demo("1/sinh(x)"); // → csch(x)

    println!("\nParity:");
    demo("sinh(-x)"); // → -sinh(x)
    demo("cosh(-x)"); // → cosh(x)
    demo("tanh(-x)"); // → -tanh(x)

    // ===================================
    // LOGARITHMIC/EXPONENTIAL PROPERTIES
    // ===================================
    println!("\n📌 SECTION 4: LOGARITHMIC/EXPONENTIAL\n");

    println!("Inverse Functions:");
    demo("ln(e^x)"); // → x
    demo("e^(ln(x))"); // → x (as exp(ln(x))→x)
    demo("log10(10^x)"); // → x

    println!("\nLogarithm Properties:");
    demo("ln(x^2)"); // → 2*ln(x)
    demo("ln(x^n)"); // → n*ln(x)
    demo("ln(1)"); // → 0
    demo("log10(1)"); // → 0
    demo("log10(10)"); // → 1

    println!("\nAdvanced Logarithm Properties:");
    demo("ln(x^y * z^w)"); // → y*ln(x) + w*ln(z)
    demo("ln(exp(x) * y)"); // → x + ln(y)
    demo("ln(sqrt(x))"); // → (1/2)*ln(x)

    println!("\nComplex Exponential Combinations:");
    demo("exp(x + y + z)"); // stays as is
    demo("exp(2*x + 3*y)"); // stays as is
    demo("exp(ln(x) + ln(y))"); // → x*y

    // ===================================
    // ROOT SIMPLIFICATIONS
    // ===================================
    println!("\n📌 SECTION 5: ROOT SIMPLIFICATIONS\n");

    println!("Basic Roots:");
    demo("sqrt(0)"); // → 0
    demo("sqrt(1)"); // → 1
    demo("sqrt(4)"); // → 2
    demo("cbrt(0)"); // → 0
    demo("cbrt(1)"); // → 1
    demo("cbrt(8)"); // → 2

    println!("\nRoot of Powers:");
    demo("sqrt(x^2)"); // → x (assuming x≥0)
    demo("cbrt(x^3)"); // → x
    demo("sqrt(x^4)"); // → x^2

    println!("\nNested Roots:");
    demo("sqrt(sqrt(x))"); // → x^(1/4)
    demo("sqrt(cbrt(x))"); // → x^(1/6)

    println!("\nPower to Root Conversion:");
    demo("x^0.5"); // → sqrt(x)
    demo("x^(1/2)"); // → sqrt(x)
    demo("x^(1/3)"); // → cbrt(x)

    // ===================================
    // DIVISION SIMPLIFICATIONS
    // ===================================
    println!("\n📌 SECTION 6: DIVISION SIMPLIFICATIONS\n");

    println!("Cancellation:");
    demo("x / x"); // → 1
    demo("(x * y) / (x * z)"); // → y/z
    demo("x^3 / x^2"); // → x
    demo("x^2 / x^3"); // → 1/x

    println!("\nNested Divisions:");
    demo("(x/y) / z"); // → x/(y*z)
    demo("x / (y/z)"); // → (x*z)/y
    demo("(a/b) / (c/d)"); // → (a*d)/(b*c)

    // ===================================
    // NUMERIC SIMPLIFICATIONS
    // ===================================
    println!("\n📌 SECTION 7: NUMERIC SIMPLIFICATIONS\n");

    println!("Constant Folding:");
    demo("2 + 3"); // → 5
    demo("2 * 3"); // → 6
    demo("2^3"); // → 8
    demo("10 / 2"); // → 5

    println!("\nIdentity Elements:");
    demo("x + 0"); // → x
    demo("x * 1"); // → x
    demo("x * 0"); // → 0
    demo("x^1"); // → x
    demo("x^0"); // → 1
    demo("1^x"); // → 1
}

fn demo(expr: &str) {
    match simplify(expr.to_string(), None, None) {
        Ok(result) => println!("  {:<35} → {}", expr, result),
        Err(e) => println!("  {:<35} → Error: {}", expr, e),
    }
}
