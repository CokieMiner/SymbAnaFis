#[cfg(test)]
mod rc_circuit_differentiation_bug {
    use crate::{Expr, simplification::simplify_expr};
    use std::collections::HashSet;

    #[test]
    fn test_rc_circuit_derivative_simplification() {
        // RC Circuit: V(t) = V0 * exp(-t / (R * C))
        // Derivative should simplify cleanly

        let rc_expr = Expr::product(vec![
            Expr::symbol("V0"),
            Expr::func(
                "exp",
                Expr::div_expr(
                    Expr::product(vec![Expr::number(-1.0), Expr::symbol("t")]),
                    Expr::product(vec![Expr::symbol("R"), Expr::symbol("C")]),
                ),
            ),
        ]);

        println!("\nOriginal: V(t) = {}", rc_expr);

        // Take derivative with respect to t, treating R, C, V0 as constants
        let mut fixed_vars = HashSet::new();
        fixed_vars.insert("R".to_string());
        fixed_vars.insert("C".to_string());
        fixed_vars.insert("V0".to_string());

        let deriv = rc_expr.derive(
            "t",
            &fixed_vars,
            &std::collections::HashMap::new(),
            &std::collections::HashMap::new(),
        );
        println!("Derivative (raw): {}", deriv);

        // Simplify
        let simplified = simplify_expr(deriv, fixed_vars);
        println!("Derivative (simplified): {}", simplified);

        // Expected: -V0 * exp(-t / (R * C)) / (R * C)
        // Or even better: -V0 / (R * C) * exp(-t / (R * C))

        let display = format!("{}", simplified);

        // Check that it doesn't have the bug pattern "/ C * R"
        assert!(
            !display.contains("/ C * R"),
            "Derivative contains unparenthesized denominator: {}",
            display
        );

        // The simplified form should have (R * C) in denominator, not separate R and C
        // OR it should have cancelled properly if there are common factors
        println!("Expected pattern: -V0 * exp(...) / (R * C) or similar");
    }
}
