use symb_anafis::{diff, parse, simplify};

fn main() {
    // Test fraction addition using the new simplify API
    let simplified = simplify(
        "1/2 + 1/3".to_string(),
        None, // No fixed variables
        None, // No custom functions
    )
    .unwrap();
    println!("1/2 + 1/3 simplified: {}", simplified);

    // Original test
    let ast = parse(
        "x^(1/3)",
        &std::collections::HashSet::new(),
        &std::collections::HashSet::new(),
    )
    .unwrap();
    println!("Parsed AST: {:?}", ast);
    println!("Display: {}", ast);

    // Get derivative before simplification
    let derivative = ast.derive("x", &std::collections::HashSet::new());
    println!("Derivative AST: {:?}", derivative);
    println!("Derivative display: {}", derivative);

    let result = diff("x^(1/3)".to_string(), "x".to_string(), None, None).unwrap();
    println!("x^(1/3) derivative: {}", result);
    println!("Debug: {:?}", result);
}
