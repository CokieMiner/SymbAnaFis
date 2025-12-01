#[cfg(test)]
mod tests {
    use crate::{Expr, simplification::simplify};

    // Rule 1: Pull Out Common Factors
    #[test]
    fn test_factor_common_terms() {
        // x*y + x*z -> x*(y+z)
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
        // Expected: x * (y + z) or (y + z) * x (both are valid after canonicalization)
        // Note: The order of factors and terms might depend on canonicalization
        if let Expr::Mul(a, b) = simplified {
            // Check if it's x * (y+z) or (y+z) * x
            let (x_part, sum_part) = if matches!(&*a, Expr::Symbol(s) if s == "x") {
                (&*a, &*b)
            } else if matches!(&*b, Expr::Symbol(s) if s == "x") {
                (&*b, &*a)
            } else {
                panic!("Expected one factor to be x, got {:?}", (a, b));
            };

            assert_eq!(x_part, &Expr::Symbol("x".to_string()));
            if let Expr::Add(c, d) = sum_part {
                let is_yz =
                    **c == Expr::Symbol("y".to_string()) && **d == Expr::Symbol("z".to_string());
                let is_zy =
                    **c == Expr::Symbol("z".to_string()) && **d == Expr::Symbol("y".to_string());
                assert!(is_yz || is_zy, "Expected (y+z) or (z+y), got {:?}", sum_part);
            } else {
                panic!("Expected Add inside Mul, got {:?}", sum_part);
            }
        } else {
            panic!("Expected Mul, got {:?}", simplified);
        }
    }

    #[test]
    fn test_factor_exponentials() {
        // e^x * sin(x) + e^x * cos(x) -> e^x * (sin(x) + cos(x))
        // Note: exp(x) gets converted to e^x during simplification
        let ex = Expr::Pow(
            Box::new(Expr::Symbol("e".to_string())),
            Box::new(Expr::Symbol("x".to_string())),
        );
        let sinx = Expr::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::Symbol("x".to_string())],
        };
        let cosx = Expr::FunctionCall {
            name: "cos".to_string(),
            args: vec![Expr::Symbol("x".to_string())],
        };

        let expr = Expr::Add(
            Box::new(Expr::Mul(Box::new(ex.clone()), Box::new(sinx.clone()))),
            Box::new(Expr::Mul(Box::new(ex.clone()), Box::new(cosx.clone()))),
        );

        let simplified = simplify(expr);
        // Expected: e^x * (sin(x) + cos(x)) or (sin(x) + cos(x)) * e^x
        if let Expr::Mul(a, b) = simplified {
            let (exp_part, sum_part) = if *a == ex {
                (a, b)
            } else if *b == ex {
                (b, a)
            } else {
                panic!("Expected e^x as a factor, got Mul({:?}, {:?})", a, b);
            };

            assert_eq!(*exp_part, ex);
            if let Expr::Add(c, d) = *sum_part {
                // Check for sin(x) + cos(x)
                let has_sin = *c == sinx || *d == sinx;
                let has_cos = *c == cosx || *d == cosx;
                assert!(has_sin && has_cos, "Expected sin(x) + cos(x)");
            } else {
                panic!("Expected Add inside Mul");
            }
        } else {
            panic!("Expected Mul, got {:?}", simplified);
        }
    }

    // Rule 2: Combine Fractions
    #[test]
    fn test_combine_common_denominator() {
        // A/C + B/C -> (A+B)/C
        let expr = Expr::Add(
            Box::new(Expr::Div(
                Box::new(Expr::Symbol("A".to_string())),
                Box::new(Expr::Symbol("C".to_string())),
            )),
            Box::new(Expr::Div(
                Box::new(Expr::Symbol("B".to_string())),
                Box::new(Expr::Symbol("C".to_string())),
            )),
        );
        let simplified = simplify(expr);
        // Expected: (A+B)/C
        if let Expr::Div(num, den) = simplified {
            assert_eq!(*den, Expr::Symbol("C".to_string()));
            if let Expr::Add(a, b) = *num {
                let is_ab =
                    *a == Expr::Symbol("A".to_string()) && *b == Expr::Symbol("B".to_string());
                let is_ba =
                    *a == Expr::Symbol("B".to_string()) && *b == Expr::Symbol("A".to_string());
                assert!(is_ab || is_ba, "Expected A+B");
            } else {
                panic!("Expected Add in numerator");
            }
        } else {
            panic!("Expected Div");
        }
    }

    // Rule 3: Sign Cleanup
    #[test]
    fn test_distribute_negation_sub() {
        // -(A - B) -> B - A
        // Represented as -1 * (A - B)
        let expr = Expr::Mul(
            Box::new(Expr::Number(-1.0)),
            Box::new(Expr::Sub(
                Box::new(Expr::Symbol("A".to_string())),
                Box::new(Expr::Symbol("B".to_string())),
            )),
        );
        let simplified = simplify(expr);
        // Expected: B - A or B + (-A), which should display as B - A
        let display = format!("{}", simplified);
        assert!(display == "B - A" || display == "B + (-1) * A", "Got display: {}", display);
    }

    #[test]
    fn test_distribute_negation_add() {
        // -1 * (A + (-1)*B) -> B - A
        let expr = Expr::Mul(
            Box::new(Expr::Number(-1.0)),
            Box::new(Expr::Add(
                Box::new(Expr::Symbol("A".to_string())),
                Box::new(Expr::Mul(
                    Box::new(Expr::Number(-1.0)),
                    Box::new(Expr::Symbol("B".to_string())),
                )),
            )),
        );
        let simplified = simplify(expr);
        let display = format!("{}", simplified);
        assert!(display == "B - A" || display == "B + (-1) * A", "Got display: {}", display);
    }

    #[test]
    fn test_neg_div_neg() {
        // -A / -B -> A / B
        let expr = Expr::Div(
            Box::new(Expr::Mul(
                Box::new(Expr::Number(-1.0)),
                Box::new(Expr::Symbol("A".to_string())),
            )),
            Box::new(Expr::Mul(
                Box::new(Expr::Number(-1.0)),
                Box::new(Expr::Symbol("B".to_string())),
            )),
        );
        let simplified = simplify(expr);
        if let Expr::Div(num, den) = simplified {
            assert_eq!(*num, Expr::Symbol("A".to_string()));
            assert_eq!(*den, Expr::Symbol("B".to_string()));
        } else {
            panic!("Expected Div(A, B)");
        }
    }

    // Rule 4: Absorb Constants in Powers
    #[test]
    fn test_absorb_constant_pow() {
        // 2 * 2^x -> 2^(x+1)
        let expr = Expr::Mul(
            Box::new(Expr::Number(2.0)),
            Box::new(Expr::Pow(
                Box::new(Expr::Number(2.0)),
                Box::new(Expr::Symbol("x".to_string())),
            )),
        );
        let simplified = simplify(expr);
        if let Expr::Pow(base, exp) = simplified {
            assert_eq!(*base, Expr::Number(2.0));
            if let Expr::Add(a, b) = *exp {
                // x + 1
                let has_x =
                    *a == Expr::Symbol("x".to_string()) || *b == Expr::Symbol("x".to_string());
                let has_1 = *a == Expr::Number(1.0) || *b == Expr::Number(1.0);
                assert!(has_x && has_1, "Expected x+1");
            } else {
                panic!("Expected Add in exponent");
            }
        } else {
            panic!("Expected Pow");
        }
    }
    #[test]
    fn test_factor_mixed_terms() {
        // e^x + sin(x)*e^x -> e^x * (1 + sin(x))
        // Note: exp(x) gets converted to e^x during simplification
        let ex = Expr::Pow(
            Box::new(Expr::Symbol("e".to_string())),
            Box::new(Expr::Symbol("x".to_string())),
        );
        let sinx = Expr::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::Symbol("x".to_string())],
        };

        // e^x + sin(x) * e^x
        let expr = Expr::Add(
            Box::new(ex.clone()),
            Box::new(Expr::Mul(Box::new(sinx.clone()), Box::new(ex.clone()))),
        );

        let simplified = simplify(expr);

        // Expected: e^x * (1 + sin(x)) or (1 + sin(x)) * e^x
        if let Expr::Mul(a, b) = simplified {
            // One part should be e^x
            let (factor, other) = if *a == ex {
                (a, b)
            } else if *b == ex {
                (b, a)
            } else {
                panic!("Expected e^x as a factor");
            };

            assert_eq!(*factor, ex);

            // The other part should be 1 + sin(x)
            if let Expr::Add(u, v) = *other {
                let has_1 = *u == Expr::Number(1.0) || *v == Expr::Number(1.0);
                let has_sin = *u == sinx || *v == sinx;
                assert!(has_1 && has_sin, "Expected 1 + sin(x)");
            } else {
                panic!("Expected Add(1, sin(x)) in other factor");
            }
        } else {
            panic!("Expected Mul, got {:?}", simplified);
        }
    }
}
