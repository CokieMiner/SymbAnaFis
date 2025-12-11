#[cfg(test)]
mod division_bug_tests {
    use crate::{Expr, ExprKind, simplification::simplify_expr};
    use std::collections::HashSet;
    use std::sync::Arc;
    #[test]
    fn verify_display_parens() {
        // Test 1: A / (C * R^2) - should have parentheses
        let expr = Expr::new(ExprKind::Div(
            Arc::new(Expr::symbol("A")),
            Arc::new(Expr::new(ExprKind::Mul(
                Arc::new(Expr::symbol("C")),
                Arc::new(Expr::new(ExprKind::Pow(
                    Arc::new(Expr::symbol("R")),
                    Arc::new(Expr::number(2.0)),
                ))),
            ))),
        ));
        let display = format!("{}", expr);
        println!("Display: {}", display);
        assert_eq!(
            display, "A/(C*R^2)",
            "Display should be 'A/(C*R^2)' not '{}'",
            display
        );
    }

    #[test]
    fn verify_simplification_cancellation() {
        // Test 2: (-C * R * V0) / (C * R^2) should simplify to -V0 / R
        let expr = Expr::new(ExprKind::Div(
            Arc::new(Expr::new(ExprKind::Mul(
                Arc::new(Expr::new(ExprKind::Mul(
                    Arc::new(Expr::new(ExprKind::Mul(
                        Arc::new(Expr::number(-1.0)),
                        Arc::new(Expr::symbol("C")),
                    ))),
                    Arc::new(Expr::symbol("R")),
                ))),
                Arc::new(Expr::symbol("V0")),
            ))),
            Arc::new(Expr::new(ExprKind::Mul(
                Arc::new(Expr::symbol("C")),
                Arc::new(Expr::new(ExprKind::Pow(
                    Arc::new(Expr::symbol("R")),
                    Arc::new(Expr::number(2.0)),
                ))),
            ))),
        ));

        println!("Original:   {}", expr);
        let simplified = simplify_expr(expr, HashSet::new());
        println!("Simplified: {}", simplified);

        // Expected: -V0/R
        let display = format!("{}", simplified);
        assert_eq!(
            display, "-V0/R",
            "Simplification should be '-V0/R' not '{}'",
            display
        );
    }
}
