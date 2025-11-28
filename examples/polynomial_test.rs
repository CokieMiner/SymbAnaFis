use std::collections::HashSet;
use symb_anafis::{parser, simplification};

fn main() {
    println!("Testing Polynomial Simplifications (Direct Parse & Simplify)...");
    println!("--------------------------------------------------");

    let test_cases = vec![
        ("x + x", "2 * x"),
        ("x * x", "(x)^2"),
        ("x * x * x", "(x)^3"),
        ("x^2 * x^3", "(x)^5"),
        ("(x^2)^3", "(x)^6"),
        ("x / x", "1"),
        ("x^5 / x^2", "(x)^3"),
        ("x^2 / x^5", "(x)^-3"),
        ("x / x^3", "(x)^-2"),
        ("x^3 / x", "(x)^2"),
        ("2*x + 3*x", "5 * x"),
        ("x + 2*x", "3 * x"),
        ("x + x + x", "3 * x"),
    ];

    let fixed_vars = HashSet::new();
    let custom_funcs = HashSet::new();

    for (input, expected) in test_cases {
        match parser::parse(input, &fixed_vars, &custom_funcs) {
            Ok(ast) => {
                let simplified = simplification::simplify(ast);
                println!("Input:    {}", input);
                println!("Result:   {}", simplified);
                println!("Expected: {}", expected);

                // Simple string check (ignoring whitespace/parens for now)
                let result_str = format!("{}", simplified);
                if result_str
                    .replace(" ", "")
                    .replace("(", "")
                    .replace(")", "")
                    == expected.replace(" ", "").replace("(", "").replace(")", "")
                {
                    println!("Status:   PASS");
                } else {
                    println!("Status:   FAIL");
                }
                println!("--------------------------------------------------");
            }
            Err(e) => println!("Error parsing {}: {:?}", input, e),
        }
    }
}
