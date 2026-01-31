#[cfg(test)]
mod actual_bug_test {
    use crate::{Expr, simplification::simplify_expr, Simplify, symb};
    use std::collections::{HashMap, HashSet};

    #[test]
    fn test_rc_derivative_actual_bug() {
        // The actual expression from the user:
        // -C * R * V0 * exp(-t / (C * R)) / (C * R^2)
        // Should simplify to: -V0 * exp(-t / (C * R)) / R

        let exp_term = Expr::func(
            "exp",
            Expr::div_expr(
                Expr::mul_expr(Expr::number(-1.0), Expr::symbol("t")),
                Expr::mul_expr(Expr::symbol("C"), Expr::symbol("R")),
            ),
        );

        // Numerator: -C * R * V0 * exp(...)
        // which is: -1 * C * R * V0 * exp(...)
        let numerator = Expr::mul_expr(
            Expr::mul_expr(
                Expr::mul_expr(
                    Expr::mul_expr(Expr::number(-1.0), Expr::symbol("C")),
                    Expr::symbol("R"),
                ),
                Expr::symbol("V0"),
            ),
            exp_term,
        );

        // Denominator: C * R^2
        let denominator = Expr::mul_expr(
            Expr::symbol("C"),
            Expr::pow(Expr::symbol("R"), Expr::number(2.0)),
        );

        let expr = Expr::div_expr(numerator, denominator);

        println!("\nOriginal expression:");
        println!("{}", expr);

        let simplified = simplify_expr(
            expr,
            HashSet::new(),
            HashMap::new(),
            None,
            None,
            None,
            false,
        );

        println!("\nSimplified expression:");
        println!("{}", simplified);

        println!("\nExpected:");
        println!("-V0*exp(-t/(C*R))/R");
        // The simplified form should have cancelled C and reduced R^2 to R
        // Check that the denominator is just R, not (C * R^2) or similar
        let display = format!("{}", simplified);
        assert!(
            display.ends_with("/R"),
            "Expression should end with '/R', got: {}",
            display
        );
    }

    #[test]
    fn test_simple_constant_division() {
        // Even simpler: (-1 * C * R) / (C * R^2)
        // Should simplify to: -1 / R

        let numerator = Expr::mul_expr(
            Expr::mul_expr(Expr::number(-1.0), Expr::symbol("C")),
            Expr::symbol("R"),
        );

        let denominator = Expr::mul_expr(
            Expr::symbol("C"),
            Expr::pow(Expr::symbol("R"), Expr::number(2.0)),
        );

        let expr = Expr::div_expr(numerator, denominator);

        println!("\nSimple test:");
        println!("Original:   {}", expr);
        let simplified = simplify_expr(
            expr,
            HashSet::new(),
            HashMap::new(),
            None,
            None,
            None,
            false,
        );
        println!("Simplified: {}", simplified);
        println!("Expected:   -1/R");

        let display = format!("{}", simplified);
        assert_eq!(display, "-1/R", "Expected '-1/R', got '{}'", display);
    }

    #[test]
    fn test_abs() {
        let y = symb("y");
        let expr = y.abs() + (y/y.abs());
        let simplified = Simplify::new().simplify(
            &expr
        ).unwrap();
        let mut vars = HashMap::new();
        vars.insert("y", 2.0);
        let val1 = simplified.evaluate(&vars, &HashMap::new());
        let val2 = expr.evaluate(&vars, &HashMap::new());
        println!("\nAbs test:");
        println!("Original:   {}", expr);
        println!("Simplified: {}", simplified);
        println!("Value check with y = -3: original = {}, simplified = {}", val2, val1);
        assert_eq!(val1, val2, "Values do not match");
    }   
}
