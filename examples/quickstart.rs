/// Quickstart: `SymbAnaFis` in 30 seconds
///
/// Run with: cargo run --example quickstart
use symb_anafis::{diff, simplify, symb};

fn main() {
    // 1. Differentiate a string formula
    let result = diff("x^3 + sin(x)", "x", &[], None).unwrap();
    println!("d/dx [x³ + sin(x)] = {result}");

    // 2. Simplify expressions
    let simplified = simplify("sin(x)^2 + cos(x)^2", &[], None).unwrap();
    println!("sin²(x) + cos²(x) = {simplified}");

    // 3. Build expressions with Copy symbols
    let x = symb("x");
    let expr = x.pow(2.0) + x.sin() + x; // No .clone() needed!
    println!("Expression: {expr}");

    // 4. Pretty output
    println!("LaTeX: {}", expr.to_latex());
    println!("Unicode: {}", expr.to_unicode());
}
