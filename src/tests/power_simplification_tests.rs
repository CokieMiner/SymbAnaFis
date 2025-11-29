#[cfg(test)]
mod tests {
    use crate::Expr;
    use crate::simplification::simplify;

    #[test]
    fn test_power_of_power() {
        // (x^2)^2 -> x^4
        let expr = Expr::Pow(
            Box::new(Expr::Pow(
                Box::new(Expr::Symbol("x".to_string())),
                Box::new(Expr::Number(2.0)),
            )),
            Box::new(Expr::Number(2.0)),
        );
        let simplified = simplify(expr);

        // Expected: x^4
        if let Expr::Pow(base, exp) = simplified {
            if let Expr::Symbol(s) = *base {
                assert_eq!(s, "x");
            } else {
                panic!("Expected base x");
            }
            if let Expr::Number(n) = *exp {
                assert_eq!(n, 4.0);
            } else {
                panic!("Expected exponent 4.0");
            }
        } else {
            panic!("Expected power expression, got {:?}", simplified);
        }
    }

    #[test]
    fn test_power_of_power_symbolic() {
        // (x^a)^b -> x^(a*b)
        let expr = Expr::Pow(
            Box::new(Expr::Pow(
                Box::new(Expr::Symbol("x".to_string())),
                Box::new(Expr::Symbol("a".to_string())),
            )),
            Box::new(Expr::Symbol("b".to_string())),
        );
        let simplified = simplify(expr);

        // Expected: x^(a*b)
        if let Expr::Pow(base, exp) = simplified {
            if let Expr::Symbol(s) = *base {
                assert_eq!(s, "x");
            } else {
                panic!("Expected base x");
            }
            // Exponent should be a * b (or b * a depending on sorting)
            if let Expr::Mul(lhs, rhs) = *exp {
                // Check for a*b or b*a
                let s1 = format!("{}", lhs);
                let s2 = format!("{}", rhs);
                assert!((s1 == "a" && s2 == "b") || (s1 == "b" && s2 == "a"));
            } else {
                panic!("Expected multiplication in exponent");
            }
        } else {
            panic!("Expected power expression");
        }
    }

    #[test]
    fn test_sigma_power_of_power() {
        // (sigma^2)^2 -> sigma^4
        let expr = Expr::Pow(
            Box::new(Expr::Pow(
                Box::new(Expr::Symbol("sigma".to_string())),
                Box::new(Expr::Number(2.0)),
            )),
            Box::new(Expr::Number(2.0)),
        );
        let simplified = simplify(expr);

        // Expected: sigma^4
        if let Expr::Pow(base, exp) = simplified {
            if let Expr::Symbol(s) = *base {
                assert_eq!(s, "sigma");
            } else {
                panic!("Expected base sigma");
            }
            if let Expr::Number(n) = *exp {
                assert_eq!(n, 4.0);
            } else {
                panic!("Expected exponent 4.0");
            }
        } else {
            panic!("Expected power expression");
        }
    }
}
