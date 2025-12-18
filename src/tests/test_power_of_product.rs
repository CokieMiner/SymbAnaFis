#[test]
fn test_power_of_product() {
    use crate::{Expr, simplification::simplify_expr};
    use std::collections::HashSet;

    // Test: (R * C)^2
    let product = Expr::product(vec![Expr::symbol("R"), Expr::symbol("C")]);
    let squared = Expr::pow(product, Expr::number(2.0));

    eprintln!("(R * C)^2 displays as: {}", squared);
    eprintln!("Simplified: {}", simplify_expr(squared, HashSet::new()));
    eprintln!("Expected: R^2 * C^2 or (R * C)^2");

    // Test: Something / (R * C)^2
    let div = Expr::div_expr(
        Expr::symbol("X"),
        Expr::pow(
            Expr::product(vec![Expr::symbol("R"), Expr::symbol("C")]),
            Expr::number(2.0),
        ),
    );

    eprintln!("\nX / (R * C)^2 displays as: {}", div);
    eprintln!("Simplified: {}", simplify_expr(div, HashSet::new()));
    eprintln!("Expected: X / (R^2 * C^2) or X / (R * C)^2");
}
