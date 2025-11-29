use symb_anafis::Expr;

fn main() {
    println!("=== Test 1: A / R^2 (Pow alone in denominator) ===");
    let expr1 = Expr::Div(
        Box::new(Expr::Symbol("A".to_string())),
        Box::new(Expr::Pow(
            Box::new(Expr::Symbol("R".to_string())),
            Box::new(Expr::Number(2.0)),
        )),
    );
    println!("Got:      {}", expr1);
    println!("Expected: A / R^2");
    println!();

    println!("=== Test 2: A / (C * R^2) (Pow in Mul) ===");
    let expr2 = Expr::Div(
        Box::new(Expr::Symbol("A".to_string())),
        Box::new(Expr::Mul(
            Box::new(Expr::Symbol("C".to_string())),
            Box::new(Expr::Pow(
                Box::new(Expr::Symbol("R".to_string())),
                Box::new(Expr::Number(2.0)),
            )),
        )),
    );
    println!("Got:      {}", expr2);
    println!("Expected: A / (C * R^2)");
    println!();

    println!("=== Test 3: A / (b * c) (simple Mul) ===");
    let expr3 = Expr::Div(
        Box::new(Expr::Symbol("A".to_string())),
        Box::new(Expr::Mul(
            Box::new(Expr::Symbol("b".to_string())),
            Box::new(Expr::Symbol("c".to_string())),
        )),
    );
    println!("Got:      {}", expr3);
    println!("Expected: A / (b * c)");
}
