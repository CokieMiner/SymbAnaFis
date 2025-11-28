use symb_anafis::diff;

fn main() {
    println!("Testing SymbAnaFis with complex nested expressions...\n");

    let expressions = vec![
        // Nested trigonometric and exponential
        ("sin(cos(exp(x^2)))", "x"),
        // Logarithm of hyperbolic
        ("ln(tanh(x))", "x"),
        // Mixed inverse trig and hyperbolic
        ("asin(tanh(x))", "x"),
        // Deeply nested composition
        ("sin(sin(sin(sin(x))))", "x"),
        // Product of multiple functions
        ("x * exp(x) * sin(x) * ln(x)", "x"),
        // Quotient with nested terms
        ("sin(x^2) / (1 + exp(x))", "x"),
        // Power with variable exponent (x^x)
        ("x^x", "x"),
        // Complex chain rule with multiple variables (partial derivative)
        ("sin(x*y) + cos(x/y)", "x"),
        // Hyperbolic identity check (should simplify)
        ("cosh(x)^2 - sinh(x)^2", "x"),
        // THE KITCHEN SINK: All supported functions in one expression
        (
            "sin(cos(tan(x))) + csc(sec(cot(x))) + \
             sinh(cosh(tanh(x))) + csch(sech(coth(x))) + \
             asin(acos(atan(x))) + acsc(asec(acot(x))) + \
             asinh(acosh(atanh(x))) + acsch(asech(acoth(x))) + \
             sqrt(cbrt(x)) + ln(exp(x)) + log2(log10(x)) + \
             erf(erfc(x)) + gamma(sinc(x)) + LambertW(x)",
            "x",
        ),
    ];

    for (expr, var) in expressions {
        println!("Expression: d/d{} [ {} ]", var, expr);
        // We pass None for fixed_vars and custom_functions for simplicity,
        // relying on the library's default detection.
        match diff(expr.to_string(), var.to_string(), None, None) {
            Ok(result) => {
                println!("Result:     {}", result);
            }
            Err(e) => {
                println!("Error:      {}", e);
            }
        }
        println!("--------------------------------------------------");
    }
}
