use symb_anafis::diff;

fn main() {
    println!("SymbAnaFis Chain Rule Example");
    println!("-----------------------------");

    // Example 1: Nested functions
    let expr1 = "sin(cos(x))";
    let result1 = diff(expr1.to_string(), "x".to_string(), None, None).unwrap();
    println!("d/dx [{}] = {}", expr1, result1);
    // Expected: -1 * cos(cos(x)) * sin(x)

    // Example 2: Deep nesting
    let expr2 = "exp(sin(x^2))";
    let result2 = diff(expr2.to_string(), "x".to_string(), None, None).unwrap();
    println!("d/dx [{}] = {}", expr2, result2);
    // Expected: exp(sin(x^2)) * cos(x^2) * 2 * x

    // Example 3: Chain rule with custom functions
    let expr3 = "f(g(x))";
    let custom_funcs = vec!["f".to_string(), "g".to_string()];
    let result3 = diff(
        expr3.to_string(),
        "x".to_string(),
        None,
        Some(&custom_funcs),
    )
    .unwrap();
    println!("d/dx [{}] = {}", expr3, result3);
    // Expected: ∂^1_f(g(x))/∂_x^1 * ∂^1_g(x)/∂_x^1 (or similar notation)
}
