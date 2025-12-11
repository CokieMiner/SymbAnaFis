#[test]
fn test_power_of_product() {
    use crate::{Expr, ExprKind, simplification::simplify_expr};
    use std::collections::HashSet;
    use std::sync::Arc;
    // Test: (R * C)^2
    let product = Expr::new(ExprKind::Mul(
        Arc::new(Expr::symbol("R")),
        Arc::new(Expr::symbol("C")),
    ));
    let squared = Expr::new(ExprKind::Pow(
        Arc::new(product),
        Arc::new(Expr::number(2.0)),
    ));

    eprintln!("(R * C)^2 displays as: {}", squared);
    eprintln!("Simplified: {}", simplify_expr(squared, HashSet::new()));
    eprintln!("Expected: R^2 * C^2 or (R * C)^2");

    // Test: Something / (R * C)^2
    let div = Expr::new(ExprKind::Div(
        Arc::new(Expr::symbol("X")),
        Arc::new(Expr::new(ExprKind::Pow(
            Arc::new(Expr::new(ExprKind::Mul(
                Arc::new(Expr::symbol("R")),
                Arc::new(Expr::symbol("C")),
            ))),
            Arc::new(Expr::number(2.0)),
        ))),
    ));

    eprintln!("\nX / (R * C)^2 displays as: {}", div);
    eprintln!("Simplified: {}", simplify_expr(div, HashSet::new()));
    eprintln!("Expected: X / (R^2 * C^2) or X / (R * C)^2");
}
