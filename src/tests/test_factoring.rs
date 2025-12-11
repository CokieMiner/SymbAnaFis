#[cfg(test)]
mod tests {
    use crate::{Expr, ExprKind, simplification::simplify_expr};
    use std::collections::HashSet;
    use std::sync::Arc;

    #[test]
    fn test_perfect_square_factoring() {
        // x^2 + 2x + 1 -> (x + 1)^2
        let expr = Expr::new(ExprKind::Add(
            Arc::new(Expr::new(ExprKind::Add(
                Arc::new(Expr::new(ExprKind::Pow(
                    Arc::new(Expr::symbol("x")),
                    Arc::new(Expr::number(2.0)),
                ))),
                Arc::new(Expr::new(ExprKind::Mul(
                    Arc::new(Expr::number(2.0)),
                    Arc::new(Expr::symbol("x")),
                ))),
            ))),
            Arc::new(Expr::number(1.0)),
        ));
        let simplified = simplify_expr(expr, HashSet::new());
        println!("Simplified: {:?}", simplified);

        // Expected: (x + 1)^2
        let expected = Expr::new(ExprKind::Pow(
            Arc::new(Expr::new(ExprKind::Add(
                Arc::new(Expr::symbol("x")),
                Arc::new(Expr::number(1.0)),
            ))),
            Arc::new(Expr::number(2.0)),
        ));

        assert_eq!(simplified, expected);
    }
}
