#[test]
fn test_power_of_product() {
    use crate::{Expr, simplification::simplify};

    // Test: (R * C)^2
    let product = Expr::Mul(
        Box::new(Expr::Symbol("R".to_string())),
        Box::new(Expr::Symbol("C".to_string())),
    );
    let squared = Expr::Pow(Box::new(product), Box::new(Expr::Number(2.0)));

    eprintln!("(R * C)^2 displays as: {}", squared);
    eprintln!("Simplified: {}", simplify(squared));
    eprintln!("Expected: R^2 * C^2 or (R * C)^2");

    // Test: Something / (R * C)^2
    let div = Expr::Div(
        Box::new(Expr::Symbol("X".to_string())),
        Box::new(Expr::Pow(
            Box::new(Expr::Mul(
                Box::new(Expr::Symbol("R".to_string())),
                Box::new(Expr::Symbol("C".to_string())),
            )),
            Box::new(Expr::Number(2.0)),
        )),
    );

    eprintln!("\nX / (R * C)^2 displays as: {}", div);
    eprintln!("Simplified: {}", simplify(div));
    eprintln!("Expected: X / (R^2 * C^2) or X / (R * C)^2");
}
