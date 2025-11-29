#[cfg(test)]
mod tests {
    use crate::Expr;
    use crate::simplification::simplify;

    #[test]
    fn test_nested_fraction_div_div() {
        // (x/y) / (z/a) -> (x*a) / (y*z)
        let expr = Expr::Div(
            Box::new(Expr::Div(
                Box::new(Expr::Symbol("x".to_string())),
                Box::new(Expr::Symbol("y".to_string())),
            )),
            Box::new(Expr::Div(
                Box::new(Expr::Symbol("z".to_string())),
                Box::new(Expr::Symbol("a".to_string())),
            )),
        );
        let simplified = simplify(expr);

        // Expected: (x*a) / (y*z)
        // Note: ordering of multiplication might vary, so we check structure
        if let Expr::Div(num, den) = simplified {
            // Check numerator: x*a or a*x
            if let Expr::Mul(n1, n2) = *num {
                let s1 = format!("{}", n1);
                let s2 = format!("{}", n2);
                assert!((s1 == "x" && s2 == "a") || (s1 == "a" && s2 == "x"));
            } else {
                panic!("Expected numerator to be multiplication");
            }

            // Check denominator: y*z or z*y
            if let Expr::Mul(d1, d2) = *den {
                let s1 = format!("{}", d1);
                let s2 = format!("{}", d2);
                assert!((s1 == "y" && s2 == "z") || (s1 == "z" && s2 == "y"));
            } else {
                panic!("Expected denominator to be multiplication");
            }
        } else {
            panic!("Expected division");
        }
    }

    #[test]
    fn test_nested_fraction_val_div() {
        // x / (y/z) -> (x*z) / y
        let expr = Expr::Div(
            Box::new(Expr::Symbol("x".to_string())),
            Box::new(Expr::Div(
                Box::new(Expr::Symbol("y".to_string())),
                Box::new(Expr::Symbol("z".to_string())),
            )),
        );
        let simplified = simplify(expr);

        // Expected: (x*z) / y
        if let Expr::Div(num, den) = simplified {
            if let Expr::Mul(n1, n2) = *num {
                let s1 = format!("{}", n1);
                let s2 = format!("{}", n2);
                assert!((s1 == "x" && s2 == "z") || (s1 == "z" && s2 == "x"));
            } else {
                panic!("Expected numerator to be multiplication");
            }

            if let Expr::Symbol(s) = *den {
                assert_eq!(s, "y");
            } else {
                panic!("Expected denominator to be y");
            }
        } else {
            panic!("Expected division");
        }
    }

    #[test]
    fn test_nested_fraction_div_val() {
        // (x/y) / z -> x / (y*z)
        let expr = Expr::Div(
            Box::new(Expr::Div(
                Box::new(Expr::Symbol("x".to_string())),
                Box::new(Expr::Symbol("y".to_string())),
            )),
            Box::new(Expr::Symbol("z".to_string())),
        );
        let simplified = simplify(expr);

        // Expected: x / (y*z)
        if let Expr::Div(num, den) = simplified {
            if let Expr::Symbol(s) = *num {
                assert_eq!(s, "x");
            } else {
                panic!("Expected numerator to be x");
            }

            if let Expr::Mul(d1, d2) = *den {
                let s1 = format!("{}", d1);
                let s2 = format!("{}", d2);
                assert!((s1 == "y" && s2 == "z") || (s1 == "z" && s2 == "y"));
            } else {
                panic!("Expected denominator to be multiplication");
            }
        } else {
            panic!("Expected division");
        }
    }

    #[test]
    fn test_nested_fraction_numbers() {
        // (1/2) / (1/3) -> (1*3) / (2*1) -> 3/2 -> 1.5
        let expr = Expr::Div(
            Box::new(Expr::Div(
                Box::new(Expr::Number(1.0)),
                Box::new(Expr::Number(2.0)),
            )),
            Box::new(Expr::Div(
                Box::new(Expr::Number(1.0)),
                Box::new(Expr::Number(3.0)),
            )),
        );
        let simplified = simplify(expr);

        if let Expr::Div(num, den) = simplified {
            if let (Expr::Number(n), Expr::Number(d)) = (*num, *den) {
                assert_eq!(n, 3.0);
                assert_eq!(d, 2.0);
            } else {
                panic!("Expected numerator and denominator to be numbers");
            }
        } else {
            panic!("Expected division 3/2, got {:?}", simplified);
        }
    }

    #[test]
    fn test_fraction_cancellation_products() {
        // (C * R) / (C * R^2) -> 1 / R
        let expr = Expr::Div(
            Box::new(Expr::Mul(
                Box::new(Expr::Symbol("C".to_string())),
                Box::new(Expr::Symbol("R".to_string())),
            )),
            Box::new(Expr::Mul(
                Box::new(Expr::Symbol("C".to_string())),
                Box::new(Expr::Pow(
                    Box::new(Expr::Symbol("R".to_string())),
                    Box::new(Expr::Number(2.0)),
                )),
            )),
        );
        let simplified = simplify(expr);

        // Expected: 1 / R
        if let Expr::Div(num, den) = simplified {
            assert_eq!(*num, Expr::Number(1.0));
            if let Expr::Symbol(s) = *den {
                assert_eq!(s, "R");
            } else {
                panic!("Expected denominator R, got {:?}", den);
            }
        } else if let Expr::Pow(base, exp) = simplified {
            // R^-1
            if let Expr::Symbol(s) = *base {
                assert_eq!(s, "R");
                assert_eq!(*exp, Expr::Number(-1.0));
            } else {
                panic!("Expected R^-1");
            }
        } else {
            panic!("Expected 1/R or R^-1, got {:?}", simplified);
        }
    }
}
