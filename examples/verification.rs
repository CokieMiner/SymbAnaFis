use symb_anafis::{Expr, simplify};

fn main() {
    println!("--- Log Power Rule ---");
    // ln(x^2) -> 2 * ln(x)
    let result = simplify("ln(x^2)".to_string(), None, None).unwrap();
    println!("ln(x^2) simplified: {}", result);

    println!("\n--- Fraction Display ---");
    // 1 / x^2 -> 1 / x^2 (no parens around denominator)
    let expr = Expr::Div(
        Box::new(Expr::Number(1.0)),
        Box::new(Expr::Pow(
            Box::new(Expr::Symbol("x".to_string())),
            Box::new(Expr::Number(2.0)),
        )),
    );
    println!("Display: {}", expr);

    println!("\n--- Power of Power ---");
    // (x^2)^2 -> x^4
    let result = simplify("(x^2)^2".to_string(), None, None).unwrap();
    println!("(x^2)^2 simplified: {}", result);

    println!("\n--- Sigma Power ---");
    // (sigma^2)^2 -> sigma^4
    let result = simplify(
        "(sigma^2)^2".to_string(),
        Some(&["sigma".to_string()]),
        None,
    )
    .unwrap();
    println!("(sigma^2)^2 simplified: {}", result);
}
