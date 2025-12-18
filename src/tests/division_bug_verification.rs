#[cfg(test)]
mod division_bug_tests {
    use crate::{Expr, simplification::simplify_expr};
    use std::collections::HashSet;

    #[test]
    fn verify_display_parens() {
        // Test 1: A / (C * R^2) - should have parentheses
        let expr = Expr::div_expr(
            Expr::symbol("A"),
            Expr::product(vec![
                Expr::symbol("C"),
                Expr::pow(Expr::symbol("R"), Expr::number(2.0)),
            ]),
        );
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
        let expr = Expr::div_expr(
            Expr::product(vec![
                Expr::number(-1.0),
                Expr::symbol("C"),
                Expr::symbol("R"),
                Expr::symbol("V0"),
            ]),
            Expr::product(vec![
                Expr::symbol("C"),
                Expr::pow(Expr::symbol("R"), Expr::number(2.0)),
            ]),
        );

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
