use symb_anafis::diff;

fn main() {
    println!("Testing Polynomial Differentiation and Simplifications...");
    println!("--------------------------------------------------");

    let test_cases = vec![
        ("x^2", "2 * x"),
        ("x^3", "3 * (x)^2"),
        ("x^4", "4 * (x)^3"),
        ("2*x^2 + 3*x + 5", "3 + 4 * x"),
        ("x^2 + 2*x + 1", "2 + 2 * x"),
        ("(x + 1)^2", "2 * (1 + x)"),
        ("x^5 / x^2", "3 * (x)^2"),  // d/dx[x^5/x^2] = d/dx[x^3] = 3*x^2
        ("x^2 / x^5", "-3 / (x^4)"), // d/dx[x^2/x^5] = d/dx[x^(-3)] = -3*x^(-4)
    ];

    for (input, expected) in test_cases {
        match diff(input.to_string(), "x".to_string(), None, None) {
            Ok(result) => {
                println!("Input:    d/dx [{}]", input);
                println!("Result:   {}", result);
                println!("Expected: {}", expected);

                // Simple string check (ignoring whitespace/parens for now)
                let result_str = format!("{}", result);
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
            Err(e) => println!("Error differentiating {}: {:?}", input, e),
        }
    }
}
