#[cfg(test)]
mod tests {
    use crate::Expr;
    use crate::simplification::simplify;

    #[test]
    fn test_ln_power() {
        // ln(x^2) -> 2 * ln(x)
        let expr = Expr::FunctionCall {
            name: "ln".to_string(),
            args: vec![Expr::Pow(
                Box::new(Expr::Symbol("x".to_string())),
                Box::new(Expr::Number(2.0)),
            )],
        };
        let simplified = simplify(expr);

        // Expected: 2 * ln(x)
        if let Expr::Mul(coeff, func) = simplified {
            assert_eq!(*coeff, Expr::Number(2.0));
            if let Expr::FunctionCall { name, args } = *func {
                assert_eq!(name, "ln");
                assert_eq!(args[0], Expr::Symbol("x".to_string()));
            } else {
                panic!("Expected ln function call");
            }
        } else {
            panic!("Expected multiplication");
        }
    }

    #[test]
    fn test_log10_power() {
        // log10(x^3) -> 3 * log10(x)
        let expr = Expr::FunctionCall {
            name: "log10".to_string(),
            args: vec![Expr::Pow(
                Box::new(Expr::Symbol("x".to_string())),
                Box::new(Expr::Number(3.0)),
            )],
        };
        let simplified = simplify(expr);

        // Expected: 3 * log10(x)
        if let Expr::Mul(coeff, func) = simplified {
            assert_eq!(*coeff, Expr::Number(3.0));
            if let Expr::FunctionCall { name, args } = *func {
                assert_eq!(name, "log10");
                assert_eq!(args[0], Expr::Symbol("x".to_string()));
            } else {
                panic!("Expected log10 function call");
            }
        } else {
            panic!("Expected multiplication");
        }
    }

    #[test]
    fn test_log2_power() {
        // log2(x^0.5) -> 0.5 * log2(x)
        let expr = Expr::FunctionCall {
            name: "log2".to_string(),
            args: vec![Expr::Pow(
                Box::new(Expr::Symbol("x".to_string())),
                Box::new(Expr::Number(0.5)),
            )],
        };
        let simplified = simplify(expr);

        // Expected: log2(x) / 2
        if let Expr::Div(num, den) = simplified {
            if let Expr::FunctionCall { name, args } = *num {
                assert_eq!(name, "log2");
                assert_eq!(args[0], Expr::Symbol("x".to_string()));
            } else {
                panic!("Expected numerator to be log2(x)");
            }

            if let Expr::Number(n) = *den {
                assert_eq!(n, 2.0);
            } else {
                panic!("Expected denominator to be 2");
            }
        } else {
            // It might also be (1/2) * log2(x) if flattening didn't happen,
            // but based on analysis it should be Div.
            // Let's print what we got if it fails
            panic!("Expected division log2(x)/2, got {:?}", simplified);
        }
    }
}
