use symb_anafis::{Expr, simplify};

fn main() {
    println!("--- Log Power Rule ---");
    // ln(x^2) -> 2 * ln(x)
    let expr = Expr::FunctionCall {
        name: "ln".to_string(),
        args: vec![Expr::Pow(
            Box::new(Expr::Symbol("x".to_string())),
            Box::new(Expr::Number(2.0)),
        )],
    };
    println!("Original: {}", expr);
    println!("Simplified: {}", simplify(expr));

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
    let expr = Expr::Pow(
        Box::new(Expr::Pow(
            Box::new(Expr::Symbol("x".to_string())),
            Box::new(Expr::Number(2.0)),
        )),
        Box::new(Expr::Number(2.0)),
    );
    println!("Original: {}", expr);
    println!("Simplified: {}", simplify(expr));

    println!("\n--- Sigma Power ---");
    // (sigma^2)^2 -> sigma^4
    let expr = Expr::Pow(
        Box::new(Expr::Pow(
            Box::new(Expr::Symbol("sigma".to_string())),
            Box::new(Expr::Number(2.0)),
        )),
        Box::new(Expr::Number(2.0)),
    );
    println!("Original: {}", expr);
    println!("Simplified: {}", simplify(expr));
}
