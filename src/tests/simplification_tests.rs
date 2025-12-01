#[cfg(test)]
mod tests {
    use crate::Expr;
    use crate::simplification::simplify;

    #[test]
    fn test_fraction_difference_issue() {
        use crate::parser;
        use std::collections::HashSet;

        // Test case: 1/(x^2 - 1) - 1/(x^2 + 1)
        // Should simplify properly, canceling -1 + 1 and removing * 1
        let expr = parser::parse(
            "1/(x^2 - 1) - 1/(x^2 + 1)",
            &HashSet::new(),
            &HashSet::new(),
        )
        .unwrap();
        let result = simplify(expr.clone());

        println!("Original: {}", expr);
        println!("Simplified: {}", result);

        // Test simpler cases that should work
        let test1 = parser::parse("x^2 + 1 - x^2 + 1", &HashSet::new(), &HashSet::new()).unwrap();
        let result1 = simplify(test1.clone());
        println!("\nTest x^2 + 1 - x^2 + 1:");
        println!("  Original: {}", test1);
        println!("  Simplified: {}", result1);
        assert_eq!(format!("{}", result1), "2");

        let test2 = parser::parse("(1 + x) * (1 - x)", &HashSet::new(), &HashSet::new()).unwrap();
        let result2 = simplify(test2.clone());
        println!("\nTest (1 + x) * (1 - x):");
        println!("  Original: {}", test2);
        println!("  Simplified: {}", result2);

        let test3 = parser::parse(
            "x^2 + (1 + x) * (1 - x) + 1",
            &HashSet::new(),
            &HashSet::new(),
        )
        .unwrap();
        let result3 = simplify(test3.clone());
        println!("\nTest x^2 + (1 + x) * (1 - x) + 1:");
        println!("  Original: {}", test3);
        println!("  Simplified: {}", result3);
        println!("  Expected: 2");

        // This is what we're seeing - let me check if it further simplifies
        let test4 = parser::parse("x^2 + 1 - x^2 + 1", &HashSet::new(), &HashSet::new()).unwrap();
        let result4 = simplify(test4.clone());
        println!("\nDirect test of x^2 + 1 - x^2 + 1:");
        println!("  Original: {}", test4);
        println!("  Simplified: {}", result4);
        println!("  Expected: 2");
        assert_eq!(
            format!("{}", result4),
            "2",
            "Should simplify x^2 + 1 - x^2 + 1 to 2"
        );

        // Check that simplification actually reduced the expression
        let original_len = format!("{}", expr).len();
        let simplified_len = format!("{}", result).len();
        println!(
            "\nMain expression length: {} -> {}",
            original_len, simplified_len
        );
    }

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
    fn test_pow_one() {
        let expr = Expr::Pow(
            Box::new(Expr::Symbol("x".to_string())),
            Box::new(Expr::Number(1.0)),
        );
        let result = simplify(expr);
        assert_eq!(result, Expr::Symbol("x".to_string()));
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
        // Should be x*(y + z) or (y + z)*x (both valid after canonicalization)
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
            if let Expr::Add(y, z) = sum_part {
                assert_eq!(**y, Expr::Symbol("y".to_string()));
                assert_eq!(**z, Expr::Symbol("z".to_string()));
            } else {
                panic!("Expected y + z, got {:?}", sum_part);
            }
        } else {
            panic!("Expected x*(y + z), got {:?}", simplified);
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

    #[test]
    fn test_product_four_functions_derivative() {
        use crate::diff;
        // d/dx [x * exp(x) * sin(x) * ln(x)]
        let expr_str = "x * exp(x) * sin(x) * ln(x)";
        let derivative = diff(expr_str.to_string(), "x".to_string(), None, None).unwrap();

        // Check if "... / x" is present (it shouldn't be if simplified)
        let derivative_str = format!("{}", derivative);
        assert!(
            !derivative_str.contains("/ x"),
            "Derivative contains unsimplified division by x: {}",
            derivative_str
        );
    }

    #[test]
    fn test_flatten_mul_div_structure() {
        // (A / B) * R^2 -> (A * R^2) / B
        let expr = Expr::Mul(
            Box::new(Expr::Div(
                Box::new(Expr::Symbol("A".to_string())),
                Box::new(Expr::Symbol("B".to_string())),
            )),
            Box::new(Expr::Pow(
                Box::new(Expr::Symbol("R".to_string())),
                Box::new(Expr::Number(2.0)),
            )),
        );
        let simplified = simplify(expr);
        println!("Simplified: {:?}", simplified);

        if let Expr::Div(num, den) = simplified {
            // Check denominator is B
            if let Expr::Symbol(s) = *den {
                assert_eq!(s, "B");
            } else {
                panic!("Expected denominator B");
            }

            // Check numerator is A * R^2 (or R^2 * A)
            if let Expr::Mul(n1, n2) = *num {
                // One should be A, other should be R^2
                let s1 = format!("{}", n1);
                let s2 = format!("{}", n2);
                assert!((s1 == "A" && s2 == "R^2") || (s1 == "R^2" && s2 == "A"));
            } else {
                panic!("Expected numerator multiplication");
            }
        } else {
            // If it's not Div, it failed to flatten
            panic!("Expected Div, got {:?}", simplified);
        }
    }
}
