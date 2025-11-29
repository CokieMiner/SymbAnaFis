#[cfg(test)]
mod division_bug_tests {
    use crate::{Expr, simplification::simplify};

    #[test]
    fn verify_display_parens() {
        // Test 1: A / (C * R^2) - should have parentheses
        let expr = Expr::Div(
            Box::new(Expr::Symbol("A".to_string())),
            Box::new(Expr::Mul(
                Box::new(Expr::Symbol("C".to_string())),
                Box::new(Expr::Pow(
                    Box::new(Expr::Symbol("R".to_string())),
                    Box::new(Expr::Number(2.0)),
                )),
            )),
        );
        let display = format!("{}", expr);
        println!("Display: {}", display);
        assert_eq!(
            display, "A / (C * R^2)",
            "Display should be 'A / (C * R^2)' not '{}'",
            display
        );
    }

    #[test]
    fn verify_simplification_cancellation() {
        // Test 2: (-C * R * V0) / (C * R^2) should simplify to -V0 / R
        let expr = Expr::Div(
            Box::new(Expr::Mul(
                Box::new(Expr::Mul(
                    Box::new(Expr::Mul(
                        Box::new(Expr::Number(-1.0)),
                        Box::new(Expr::Symbol("C".to_string())),
                    )),
                    Box::new(Expr::Symbol("R".to_string())),
                )),
                Box::new(Expr::Symbol("V0".to_string())),
            )),
            Box::new(Expr::Mul(
                Box::new(Expr::Symbol("C".to_string())),
                Box::new(Expr::Pow(
                    Box::new(Expr::Symbol("R".to_string())),
                    Box::new(Expr::Number(2.0)),
                )),
            )),
        );

        println!("Original:   {}", expr);
        let simplified = simplify(expr);
        println!("Simplified: {}", simplified);

        // Expected: -V0 / R
        let display = format!("{}", simplified);
        assert_eq!(
            display, "-V0 / R",
            "Simplification should be '-V0 / R' not '{}'",
            display
        );
    }
}
