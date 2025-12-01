#[cfg(test)]
mod tests {
    use crate::Expr;
    use crate::simplify;

    #[test]
    fn test_fraction_cancellation_product_base() {
        // (C * R) / (C * R)^2 -> 1 / (C * R)
        let expr = "C * R / (C * R)^2";
        let simplified = simplify(expr.to_string(), None, None).unwrap();
        println!("{} -> {}", expr, simplified);
        assert_eq!(simplified.to_string(), "1 / (C * R)");
    }

    #[test]
    fn test_sqrt_div_self() {
        // sqrt(x) / x -> 1 / sqrt(x)
        let expr = "sqrt(x) / x";
        let simplified = simplify(expr.to_string(), None, None).unwrap();
        println!("{} -> {}", expr, simplified);
        assert_eq!(simplified.to_string(), "1 / sqrt(x)");
    }

    #[test]
    fn test_numeric_fraction_simplification() {
        // 2 * x / 4 -> x / 2
        let expr = "2 * x / 4";
        let simplified = simplify(expr.to_string(), None, None).unwrap();
        println!("{} -> {}", expr, simplified);
        assert_eq!(simplified.to_string(), "x / 2");
    }


    #[test]
    fn test_sqrt_product_div_product() {
        let a = Expr::Symbol("a".to_string());
        let b = Expr::Symbol("b".to_string());
        let expr = Expr::Div(
            Box::new(Expr::FunctionCall {
                name: "sqrt".to_string(),
                args: vec![Expr::Mul(Box::new(a.clone()), Box::new(b.clone()))],
            }),
            Box::new(Expr::Mul(Box::new(a.clone()), Box::new(b.clone()))),
        );

        let simplified = crate::simplification::simplify(expr);
        let result = format!("{}", simplified);
        println!("sqrt(a * b) / (a * b) -> {}", result);

        assert!(
            result.contains("1 / sqrt(a * b)") || result.contains("1 / (sqrt(a) * sqrt(b))"),
            "Expected 1 / sqrt(a * b) or equivalent, got {}",
            result
        );
    }

    #[test]
    fn test_heat_flux_simplification() {
        // sqrt(alpha) * sqrt(t) / (alpha * t * sqrt(pi))
        let alpha = Expr::Symbol("alpha".to_string());
        let t = Expr::Symbol("t".to_string());
        let pi = Expr::Symbol("pi".to_string());

        let num = Expr::Mul(
            Box::new(Expr::FunctionCall {
                name: "sqrt".to_string(),
                args: vec![alpha.clone()],
            }),
            Box::new(Expr::FunctionCall {
                name: "sqrt".to_string(),
                args: vec![t.clone()],
            }),
        );
        let den = Expr::Mul(
            Box::new(alpha.clone()),
            Box::new(Expr::Mul(
                Box::new(t.clone()),
                Box::new(Expr::FunctionCall {
                    name: "sqrt".to_string(),
                    args: vec![pi.clone()],
                }),
            )),
        );
        let expr = Expr::Div(Box::new(num), Box::new(den));

        let simplified = crate::simplification::simplify(expr);
        let result = format!("{}", simplified);
        println!("Heat flux term -> {}", result);

        // Should simplify to 1 / (sqrt(alpha) * sqrt(t) * sqrt(pi))
        // or 1 / sqrt(alpha * t * pi)
        assert!(
            !result.contains("sqrt(alpha) /"),
            "Should not contain sqrt(alpha) in numerator, got {}",
            result
        );
        assert!(
            !result.contains("sqrt(t) /"),
            "Should not contain sqrt(t) in numerator, got {}",
            result
        );
    }
}
