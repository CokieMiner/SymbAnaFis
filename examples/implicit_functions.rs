use symb_anafis::diff;

fn main() {
    println!("SymbAnaFis Implicit Functions Example");
    println!("-------------------------------------");

    // Example 1: Implicit function y(x)
    // We declare 'y' as a custom function
    let expr1 = "x * y(x)";
    let custom_funcs = vec!["y".to_string()];
    let result1 = diff(
        expr1.to_string(),
        "x".to_string(),
        None,
        Some(&custom_funcs),
    )
    .unwrap();
    println!("d/dx [{}] = {}", expr1, result1);
    // Expected: y(x) + x * ∂_y(x)/∂_x

    // Example 2: Implicit function with multiple arguments f(x, a)
    // 'a' is a fixed variable (constant)
    let expr2 = "f(x, a)";
    let fixed_vars = vec!["a".to_string()];
    let custom_funcs = vec!["f".to_string()];
    let result2 = diff(
        expr2.to_string(),
        "x".to_string(),
        Some(&fixed_vars),
        Some(&custom_funcs),
    )
    .unwrap();
    println!("d/dx [{}] = {}", expr2, result2);
    // Expected: ∂_f(x, a)/∂_x
}
