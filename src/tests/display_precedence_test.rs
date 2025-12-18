#[cfg(test)]
mod tests {
    use crate::Expr;
    use std::collections::HashSet;

    #[test]
    fn test_division_precedence_mul() {
        // a / (b * c) should be displayed as "a / (b * c)"
        // If displayed as "a / b * c", it means (a / b) * c which is wrong.
        let expr = Expr::div_expr(
            Expr::symbol("a"),
            Expr::product(vec![Expr::symbol("b"), Expr::symbol("c")]),
        );
        let display = format!("{}", expr);
        println!("Display: {}", display);
        assert_eq!(display, "a/(b*c)");
    }

    #[test]
    fn test_division_precedence_div() {
        // a / (b / c) should be displayed as "a / (b / c)"
        let expr = Expr::div_expr(
            Expr::symbol("a"),
            Expr::div_expr(Expr::symbol("b"), Expr::symbol("c")),
        );
        let display = format!("{}", expr);
        println!("Display: {}", display);
        assert_eq!(display, "a/(b/c)");
    }

    #[test]
    fn test_rc_circuit_derivative() {
        use crate::simplification::simplify_expr;
        // V0 * exp(-t / (R * C))
        // Derivative should be: -V0 * exp(-t / (R * C)) / (R * C)

        // Construct Div(Product([-1, C, R, V0]), Product([C, R^2]))
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

        println!("Original: {}", expr);
        let simplified = simplify_expr(expr.clone(), HashSet::new());
        println!("Simplified: {}", simplified);

        // With exp factor
        let exp_term = Expr::func("exp", Expr::symbol("t"));

        let expr_full = Expr::div_expr(
            Expr::product(vec![
                Expr::number(-1.0),
                Expr::symbol("C"),
                Expr::symbol("R"),
                Expr::symbol("V0"),
                exp_term.clone(),
            ]),
            Expr::product(vec![
                Expr::symbol("C"),
                Expr::pow(Expr::symbol("R"), Expr::number(2.0)),
            ]),
        );

        println!("Full Original: {}", expr_full);
        let simplified_full = simplify_expr(expr_full, HashSet::new());
        println!("Full Simplified: {}", simplified_full);

        // Expected display: -V0 * exp(t) / R
        // Bad display: -V0 * exp(t) / C * R^2 (or similar)
        let display = format!("{}", simplified_full);
        assert!(
            !display.contains("/ C *"),
            "Display contains unparenthesized denominator multiplication: {}",
            display
        );
    }

    #[test]
    fn test_display_rc_denominator() {
        // A / (C * R^2)
        let expr = Expr::div_expr(
            Expr::symbol("A"),
            Expr::product(vec![
                Expr::symbol("C"),
                Expr::pow(Expr::symbol("R"), Expr::number(2.0)),
            ]),
        );
        let display = format!("{}", expr);
        println!("Display: {}", display);
        assert_eq!(display, "A/(C*R^2)");
    }
}
