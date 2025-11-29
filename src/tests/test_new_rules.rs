#[cfg(test)]
mod tests {
    use crate::{Expr, simplify};

    #[test]
    fn test_roots_simplification() {
        // sqrt(x^4) -> x^2
        let expr = Expr::FunctionCall {
            name: "sqrt".to_string(),
            args: vec![Expr::Pow(
                Box::new(Expr::Symbol("x".to_string())),
                Box::new(Expr::Number(4.0)),
            )],
        };
        let simplified = simplify(expr);
        assert_eq!(
            simplified,
            Expr::Pow(
                Box::new(Expr::Symbol("x".to_string())),
                Box::new(Expr::Number(2.0))
            )
        );
    }

    #[test]
    fn test_trig_combination() {
        use crate::simplification::trig::apply_trig_rules;

        // sin(x)cos(y) + cos(x)sin(y) -> sin(x+y)
        let expr = Expr::Add(
            Box::new(Expr::Mul(
                Box::new(Expr::FunctionCall {
                    name: "sin".to_string(),
                    args: vec![Expr::Symbol("x".to_string())],
                }),
                Box::new(Expr::FunctionCall {
                    name: "cos".to_string(),
                    args: vec![Expr::Symbol("y".to_string())],
                }),
            )),
            Box::new(Expr::Mul(
                Box::new(Expr::FunctionCall {
                    name: "cos".to_string(),
                    args: vec![Expr::Symbol("x".to_string())],
                }),
                Box::new(Expr::FunctionCall {
                    name: "sin".to_string(),
                    args: vec![Expr::Symbol("y".to_string())],
                }),
            )),
        );
        let simplified = apply_trig_rules(expr);
        if let Expr::FunctionCall { name, args } = simplified {
            assert_eq!(name, "sin");
            assert_eq!(args.len(), 1);
            if let Expr::Add(u, v) = &args[0] {
                let is_xy =
                    **u == Expr::Symbol("x".to_string()) && **v == Expr::Symbol("y".to_string());
                let is_yx =
                    **u == Expr::Symbol("y".to_string()) && **v == Expr::Symbol("x".to_string());
                assert!(is_xy || is_yx);
            } else {
                panic!("Expected Add inside sin");
            }
        } else {
            panic!("Expected FunctionCall sin");
        }
    }
}
