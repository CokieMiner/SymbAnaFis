use symb_anafis::diff;

fn main() {
    println!("SymbAnaFis Custom Functions Example");
    println!("-----------------------------------");

    // Define fixed variables (constants) and custom functions
    let fixed_vars = vec!["alpha".to_string(), "beta".to_string()];
    let custom_funcs = vec!["psi".to_string(), "phi".to_string()];

    // Expression involving constants and custom functions
    let expr = "alpha * psi(x) + beta * phi(x^2)";

    let result = diff(
        expr.to_string(),
        "x".to_string(),
        Some(&fixed_vars),
        Some(&custom_funcs),
    )
    .unwrap();

    println!("Expression: {}", expr);
    println!("Fixed Vars: {:?}", fixed_vars);
    println!("Custom Funcs: {:?}", custom_funcs);
    println!("Derivative: {}", result);
    // Expected: alpha * ∂_psi(x)/∂_x + beta * ∂_phi(x^2)/∂_x * 2 * x
}
