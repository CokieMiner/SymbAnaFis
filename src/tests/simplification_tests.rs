#[cfg(test)]
mod tests {
    use crate::Expr;
    use crate::simplification::simplify;

    #[test]
    fn test_add_zero() {
        let expr = Expr::Add(
            Box::new(Expr::Symbol("x".to_string())),
            Box::new(Expr::Number(0.0)),
        );
        let result = simplify(expr);
        assert_eq!(result, Expr::Symbol("x".to_string()));
    }

    #[test]
    fn test_mul_zero() {
        let expr = Expr::Mul(
            Box::new(Expr::Symbol("x".to_string())),
            Box::new(Expr::Number(0.0)),
        );
        let result = simplify(expr);
        assert_eq!(result, Expr::Number(0.0));
    }

    #[test]
    fn test_mul_one() {
        let expr = Expr::Mul(
            Box::new(Expr::Symbol("x".to_string())),
            Box::new(Expr::Number(1.0)),
        );
        let result = simplify(expr);
        assert_eq!(result, Expr::Symbol("x".to_string()));
    }

    #[test]
    fn test_pow_zero() {
        let expr = Expr::Pow(
            Box::new(Expr::Symbol("x".to_string())),
            Box::new(Expr::Number(0.0)),
        );
        let result = simplify(expr);
        assert_eq!(result, Expr::Number(1.0));
    }

    #[test]
    fn test_constant_folding() {
        let expr = Expr::Add(Box::new(Expr::Number(2.0)), Box::new(Expr::Number(3.0)));
        let result = simplify(expr);
        assert_eq!(result, Expr::Number(5.0));
    }

    #[test]
    fn test_nested_simplification() {
        // (x + 0) * 1 should simplify to x
        let expr = Expr::Mul(
            Box::new(Expr::Add(
                Box::new(Expr::Symbol("x".to_string())),
                Box::new(Expr::Number(0.0)),
            )),
            Box::new(Expr::Number(1.0)),
        );
        let result = simplify(expr);
        assert_eq!(result, Expr::Symbol("x".to_string()));
    }

    #[test]
    fn test_trig_simplification() {
        // sin(0) = 0
        let expr = Expr::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::Number(0.0)],
        };
        assert_eq!(simplify(expr), Expr::Number(0.0));

        // cos(0) = 1
        let expr = Expr::FunctionCall {
            name: "cos".to_string(),
            args: vec![Expr::Number(0.0)],
        };
        assert_eq!(simplify(expr), Expr::Number(1.0));

        // sin(-x) = -sin(x)
        let expr = Expr::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::Mul(
                Box::new(Expr::Number(-1.0)),
                Box::new(Expr::Symbol("x".to_string())),
            )],
        };
        let simplified = simplify(expr);
        // Should be -1 * sin(x)
        if let Expr::Mul(a, b) = simplified {
            assert_eq!(*a, Expr::Number(-1.0));
            if let Expr::FunctionCall { name, args } = *b {
                assert_eq!(name, "sin");
                assert_eq!(args[0], Expr::Symbol("x".to_string()));
            } else {
                panic!("Expected function call");
            }
        } else {
            panic!("Expected multiplication");
        }
    }

    #[test]
    fn test_hyperbolic_simplification() {
        // sinh(0) = 0
        let expr = Expr::FunctionCall {
            name: "sinh".to_string(),
            args: vec![Expr::Number(0.0)],
        };
        assert_eq!(simplify(expr), Expr::Number(0.0));

        // cosh(0) = 1
        let expr = Expr::FunctionCall {
            name: "cosh".to_string(),
            args: vec![Expr::Number(0.0)],
        };
        assert_eq!(simplify(expr), Expr::Number(1.0));
    }

    #[test]
    fn test_log_exp_simplification() {
        // ln(1) = 0
        let expr = Expr::FunctionCall {
            name: "ln".to_string(),
            args: vec![Expr::Number(1.0)],
        };
        assert_eq!(simplify(expr), Expr::Number(0.0));

        // exp(0) = 1
        let expr = Expr::FunctionCall {
            name: "exp".to_string(),
            args: vec![Expr::Number(0.0)],
        };
        assert_eq!(simplify(expr), Expr::Number(1.0));

        // exp(ln(x)) = x
        let expr = Expr::FunctionCall {
            name: "exp".to_string(),
            args: vec![Expr::FunctionCall {
                name: "ln".to_string(),
                args: vec![Expr::Symbol("x".to_string())],
            }],
        };
        assert_eq!(simplify(expr), Expr::Symbol("x".to_string()));
    }

    #[test]
    fn test_fraction_preservation() {
        // 1/3 should stay 1/3
        let expr = Expr::Div(Box::new(Expr::Number(1.0)), Box::new(Expr::Number(3.0)));
        let simplified = simplify(expr.clone());
        assert_eq!(simplified, expr);

        // 4/2 should become 2
        let expr = Expr::Div(Box::new(Expr::Number(4.0)), Box::new(Expr::Number(2.0)));
        let simplified = simplify(expr);
        assert_eq!(simplified, Expr::Number(2.0));

        // 2/3 should stay 2/3
        let expr = Expr::Div(Box::new(Expr::Number(2.0)), Box::new(Expr::Number(3.0)));
        let simplified = simplify(expr.clone());
        assert_eq!(simplified, expr);
    }

    #[test]
    fn test_distributive_property() {
        // x*y + x*z should become x*(y + z)
        let expr = Expr::Add(
            Box::new(Expr::Mul(
                Box::new(Expr::Symbol("x".to_string())),
                Box::new(Expr::Symbol("y".to_string())),
            )),
            Box::new(Expr::Mul(
                Box::new(Expr::Symbol("x".to_string())),
                Box::new(Expr::Symbol("z".to_string())),
            )),
        );
        let simplified = simplify(expr);
        // Should be x*(y + z)
        if let Expr::Mul(x, sum) = simplified {
            assert_eq!(*x, Expr::Symbol("x".to_string()));
            if let Expr::Add(y, z) = *sum {
                assert_eq!(*y, Expr::Symbol("y".to_string()));
                assert_eq!(*z, Expr::Symbol("z".to_string()));
            } else {
                panic!("Expected y + z");
            }
        } else {
            panic!("Expected x*(y + z)");
        }
    }

    #[test]
    fn test_binomial_expansion() {
        // x^2 + 2*x*y + y^2 should become (x + y)^2
        let expr = Expr::Add(
            Box::new(Expr::Add(
                Box::new(Expr::Pow(
                    Box::new(Expr::Symbol("x".to_string())),
                    Box::new(Expr::Number(2.0)),
                )),
                Box::new(Expr::Mul(
                    Box::new(Expr::Number(2.0)),
                    Box::new(Expr::Mul(
                        Box::new(Expr::Symbol("x".to_string())),
                        Box::new(Expr::Symbol("y".to_string())),
                    )),
                )),
            )),
            Box::new(Expr::Pow(
                Box::new(Expr::Symbol("y".to_string())),
                Box::new(Expr::Number(2.0)),
            )),
        );
        let simplified = simplify(expr);
        // Should be (x + y)^2
        if let Expr::Pow(sum, exp) = simplified {
            assert_eq!(*exp, Expr::Number(2.0));
            if let Expr::Add(x, y) = *sum {
                assert_eq!(*x, Expr::Symbol("x".to_string()));
                assert_eq!(*y, Expr::Symbol("y".to_string()));
            } else {
                panic!("Expected x + y");
            }
        } else {
            panic!("Expected (x + y)^2");
        }
    }
}
