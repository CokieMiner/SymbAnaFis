#[cfg(test)]
mod rc_circuit_differentiation_bug {
    use crate::{Expr, simplification::simplify};
    use std::collections::HashSet;

    #[test]
    fn test_rc_circuit_derivative_simplification() {
        // RC Circuit: V(t) = V0 * exp(-t / (R * C))
        // Derivative should simplify cleanly

        let rc_expr = Expr::Mul(
            Box::new(Expr::Symbol("V0".to_string())),
            Box::new(Expr::FunctionCall {
                name: "exp".to_string(),
                args: vec![Expr::Div(
                    Box::new(Expr::Mul(
                        Box::new(Expr::Number(-1.0)),
                        Box::new(Expr::Symbol("t".to_string())),
                    )),
                    Box::new(Expr::Mul(
                        Box::new(Expr::Symbol("R".to_string())),
                        Box::new(Expr::Symbol("C".to_string())),
                    )),
                )],
            }),
        );

        println!("\nOriginal: V(t) = {}", rc_expr);

        // Take derivative with respect to t, treating R, C, V0 as constants
        let mut fixed_vars = HashSet::new();
        fixed_vars.insert("R".to_string());
        fixed_vars.insert("C".to_string());
        fixed_vars.insert("V0".to_string());

        let deriv = rc_expr.derive("t", &fixed_vars);
        println!("Derivative (raw): {}", deriv);

        // Simplify
        let simplified = simplify(deriv);
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
