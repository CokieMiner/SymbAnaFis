use symb_anafis::diff;

fn main() {
    println!("Testing SymbAnaFis with Custom Variables, Functions, and Edge Cases...\n");

    // 1. Define Fixed Variables (Constants)
    // Note: 'ax' and 'a' both exist to test longest-match resolution.
    // 'alpha' is a variable, so 'alpha(x)' should be implicit multiplication 'alpha * (x)'.
    let fixed_vars = vec![
        "alpha", "beta", "theta", "my_var", "ax", "a", "y", "z", "phi",
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();

    // 2. Define Custom Functions
    // These will be treated as opaque functions.
    // Multi-argument functions like 'h(x,y,z)' will produce partial derivative notation.
    let custom_funcs = vec!["f", "g", "my_func", "h", "psi"]
        .into_iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    // 3. The Complex Expression
    // Features:
    // - f(x * alpha): Custom function with variable and constant (modified from multi-arg due to current AST limits)
    // - g(beta^x): Custom function with exponential argument
    // - my_var * my_func(x): Implicit multiplication between var and func call
    // - 2alpha(x+1): Implicit multiplication chain: 2 * alpha * (x+1)
    // - 3.14theta: Implicit multiplication number * var
    // - h(x * y * z): Custom function (modified from multi-arg)
    // - ax*a*x: Longest match test (ax should be one token, a another)
    // - psi(sin(x)): Custom function wrapping built-in
    // - phi * x: Simple constant * variable
    let expr = "f(x * alpha) * g(beta^x) + \
                my_var * my_func(x) + \
                2alpha(x+1) - \
                3.14theta * h(x * y * z) + \
                ax*a*x + \
                psi(sin(x)) / (phi + 1)";

    let var = "x";

    println!("Expression: d/d{} [ {} ]", var, expr);
    println!("Fixed Vars:   {:?}", fixed_vars);
    println!("Custom Funcs: {:?}", custom_funcs);
    println!("--------------------------------------------------");

    match diff(
        expr.to_string(),
        var.to_string(),
        Some(&fixed_vars),
        Some(&custom_funcs),
    ) {
        Ok(result) => {
            println!("Result: {}", result);
        }
        Err(e) => {
            println!("Error:  {}", e);
        }
    }
    println!("--------------------------------------------------");
}
