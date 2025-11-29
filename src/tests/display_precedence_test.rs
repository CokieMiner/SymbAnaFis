#[cfg(test)]
mod tests {
    use crate::Expr;

    #[test]
    fn test_division_precedence_mul() {
        // a / (b * c) should be displayed as "a / (b * c)"
        // If displayed as "a / b * c", it means (a / b) * c which is wrong.
        let expr = Expr::Div(
            Box::new(Expr::Symbol("a".to_string())),
            Box::new(Expr::Mul(
                Box::new(Expr::Symbol("b".to_string())),
                Box::new(Expr::Symbol("c".to_string())),
            )),
        );
        let display = format!("{}", expr);
        println!("Display: {}", display);
        assert_eq!(display, "a / (b * c)");
    }

    #[test]
    fn test_division_precedence_div() {
        // a / (b / c) should be displayed as "a / (b / c)"
        let expr = Expr::Div(
            Box::new(Expr::Symbol("a".to_string())),
            Box::new(Expr::Div(
                Box::new(Expr::Symbol("b".to_string())),
                Box::new(Expr::Symbol("c".to_string())),
            )),
        );
        let display = format!("{}", expr);
        println!("Display: {}", display);
        assert_eq!(display, "a / (b / c)");
    }

    #[test]
    fn test_rc_circuit_derivative() {
        use crate::simplification::simplify;
        // V0 * exp(-t / (R * C))
        // Derivative should be: -V0 * exp(-t / (R * C)) / (R * C)
        // Or with the bug: ... / C * R^2 ??

        // Let's construct the expression that might be causing issues
        // Div(Mul(-C*R*V0, exp), Mul(C, R^2))
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

        println!("Original: {}", expr);
        let simplified = simplify(expr.clone());
        println!("Simplified: {}", simplified);

        // Check if simplified result has correct structure
        // Should be -V0 / R (ignoring exp for now as it's just a factor)
        // Actually the test case above doesn't include exp, let's add it to be precise

        let exp_term = Expr::FunctionCall {
            name: "exp".to_string(),
            args: vec![Expr::Symbol("t".to_string())], // Simplified arg
        };

        let expr_full = Expr::Div(
            Box::new(Expr::Mul(
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
                Box::new(exp_term.clone()),
            )),
            Box::new(Expr::Mul(
                Box::new(Expr::Symbol("C".to_string())),
                Box::new(Expr::Pow(
                    Box::new(Expr::Symbol("R".to_string())),
                    Box::new(Expr::Number(2.0)),
                )),
            )),
        );

        println!("Full Original: {}", expr_full);
        let simplified_full = simplify(expr_full);
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
        assert_eq!(display, "A / (C * R^2)");
    }
}
