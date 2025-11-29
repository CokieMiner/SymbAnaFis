use symb_anafis::diff;

fn main() {
    println!("SymbAnaFis nth Derivative Example");
    println!("==================================");

    // Define custom functions for differentiation
    let custom_funcs = vec!["f".to_string()];

    // Start with a custom function
    let expr = "f(x)";

    println!("Original expression: {}", expr);

    // First derivative using diff API
    let first_deriv = diff(expr.to_string(), "x".to_string(), None, Some(&custom_funcs)).unwrap();
    println!("First derivative: {}", first_deriv);

    // Second derivative - now we can parse the derivative notation!
    let second_deriv = diff(
        first_deriv.clone(),
        "x".to_string(),
        None,
        Some(&custom_funcs),
    )
    .unwrap();
    println!("Second derivative: {}", second_deriv);

    // Third derivative
    let third_deriv = diff(
        second_deriv.clone(),
        "x".to_string(),
        None,
        Some(&custom_funcs),
    )
    .unwrap();
    println!("Third derivative: {}", third_deriv);

    // Fourth derivative
    let fourth_deriv = diff(
        third_deriv.clone(),
        "x".to_string(),
        None,
        Some(&custom_funcs),
    )
    .unwrap();
    println!("Fourth derivative: {}", fourth_deriv);
}
