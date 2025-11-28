// Advanced simplification and expression tests

mod simplification_advanced {

    use crate::simplification::simplify;
    use crate::Expr;

    #[test]
    fn test_nested_add_zero() {
        // (x + 0) + (0 + y) should simplify to (x + y)
        let expr = Expr::Add(
            Box::new(Expr::Add(
                Box::new(Expr::Symbol("x".to_string())),
                Box::new(Expr::Number(0.0)),
            )),
            Box::new(Expr::Add(
                Box::new(Expr::Number(0.0)),
                Box::new(Expr::Symbol("y".to_string())),
            )),
        );
        let result = simplify(expr);
        // Should have no zeros
        assert!(!format!("{:?}", result).contains("0.0"));
    }

    #[test]
    fn test_mul_by_zero_nested() {
        // (x + 1) * 0 should become 0
        let expr = Expr::Mul(
            Box::new(Expr::Add(
                Box::new(Expr::Symbol("x".to_string())),
                Box::new(Expr::Number(1.0)),
            )),
            Box::new(Expr::Number(0.0)),
        );
        let result = simplify(expr);
        assert_eq!(result, Expr::Number(0.0));
    }

    #[test]
    fn test_constant_folding_chain() {
        // (2 + 3) * (4 + 1) should become 25
        let expr = Expr::Mul(
            Box::new(Expr::Add(
                Box::new(Expr::Number(2.0)),
                Box::new(Expr::Number(3.0)),
            )),
            Box::new(Expr::Add(
                Box::new(Expr::Number(4.0)),
                Box::new(Expr::Number(1.0)),
            )),
        );
        let result = simplify(expr);
        assert_eq!(result, Expr::Number(25.0));
    }

    #[test]
    fn test_power_simplification() {
        // x^0 * y^1 * z^2 should simplify to z^2
        let expr = Expr::Mul(
            Box::new(Expr::Mul(
                Box::new(Expr::Pow(
                    Box::new(Expr::Symbol("x".to_string())),
                    Box::new(Expr::Number(0.0)),
                )),
                Box::new(Expr::Pow(
                    Box::new(Expr::Symbol("y".to_string())),
                    Box::new(Expr::Number(1.0)),
                )),
            )),
            Box::new(Expr::Pow(
                Box::new(Expr::Symbol("z".to_string())),
                Box::new(Expr::Number(2.0)),
            )),
        );
        let result = simplify(expr);
        // x^0 = 1, y^1 = y, so should have y and z
        assert!(format!("{:?}", result).contains("z"));
    }

    #[test]
    fn test_div_by_one() {
        // (x + y) / 1 should become (x + y)
        let expr = Expr::Div(
            Box::new(Expr::Add(
                Box::new(Expr::Symbol("x".to_string())),
                Box::new(Expr::Symbol("y".to_string())),
            )),
            Box::new(Expr::Number(1.0)),
        );
        let result = simplify(expr);
        // Should not contain Div anymore
        assert!(!matches!(result, Expr::Div(_, _)));
    }

    #[test]
    fn test_div_zero() {
        // 0 / x should become 0
        let expr = Expr::Div(
            Box::new(Expr::Number(0.0)),
            Box::new(Expr::Symbol("x".to_string())),
        );
        let result = simplify(expr);
        assert_eq!(result, Expr::Number(0.0));
    }

    #[test]
    fn test_sub_zero() {
        // x - 0 should become x
        let expr = Expr::Sub(
            Box::new(Expr::Symbol("x".to_string())),
            Box::new(Expr::Number(0.0)),
        );
        let result = simplify(expr);
        assert_eq!(result, Expr::Symbol("x".to_string()));
    }

    #[test]
    fn test_constant_div() {
        // 10 / 2 should become 5
        let expr = Expr::Div(Box::new(Expr::Number(10.0)), Box::new(Expr::Number(2.0)));
        let result = simplify(expr);
        assert_eq!(result, Expr::Number(5.0));
    }

    #[test]
    fn test_constant_sub() {
        // 10 - 3 should become 7
        let expr = Expr::Sub(Box::new(Expr::Number(10.0)), Box::new(Expr::Number(3.0)));
        let result = simplify(expr);
        assert_eq!(result, Expr::Number(7.0));
    }
}

mod real_world_expressions {

    use crate::diff;

    #[test]
    fn test_quadratic_derivative() {
        // d/dx[ax^2 + bx + c] = 2ax + b
        let result = diff(
            "a*x^2 + b*x + c".to_string(),
            "x".to_string(),
            Some(&["a".to_string(), "b".to_string(), "c".to_string()]),
            None,
        );
        assert!(result.is_ok());
        let deriv = result.unwrap();
        assert!(deriv.contains("a") && deriv.contains("x") && deriv.contains("b"));
    }

    #[test]
    fn test_rational_function() {
        // d/dx[(x^2 + 1)/(x - 1)]
        let result = diff("(x^2 + 1)/(x - 1)".to_string(), "x".to_string(), None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_exponential_growth() {
        // d/dx[a * exp(k*x)]
        let result = diff(
            "a*exp(k*x)".to_string(),
            "x".to_string(),
            Some(&["a".to_string(), "k".to_string()]),
            None,
        );
        assert!(result.is_ok());
        let deriv = result.unwrap();
        assert!(deriv.contains("exp") && deriv.contains("k"));
    }

    #[test]
    fn test_trig_identity_input() {
        // d/dx[sin(x)^2 + cos(x)^2] should work (even if not simplified to 0)
        let result = diff(
            "sin(x)^2 + cos(x)^2".to_string(),
            "x".to_string(),
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_product_of_three() {
        // d/dx[x * sin(x) * exp(x)]
        let result = diff("x*sin(x)*exp(x)".to_string(), "x".to_string(), None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_chain_rule_triple() {
        // d/dx[sin(exp(ln(x)))]
        let result = diff("sin(exp(ln(x)))".to_string(), "x".to_string(), None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hyperbolic_combo() {
        // d/dx[sinh(x) * cosh(x)]
        let result = diff("sinh(x)*cosh(x)".to_string(), "x".to_string(), None, None);
        assert!(result.is_ok());
        let deriv = result.unwrap();
        assert!(deriv.contains("sinh") || deriv.contains("cosh"));
    }

    #[test]
    fn test_implicit_with_division() {
        // d/dx[y(x)/x] where y is custom function
        let result = diff(
            "y(x)/x".to_string(),
            "x".to_string(),
            None,
            Some(&["y".to_string()]),
        );
        assert!(result.is_ok());
    }
}

mod boundary_conditions {

    use crate::diff;

    #[test]
    fn test_single_char_fixed_var() {
        let result = diff(
            "a".to_string(),
            "x".to_string(),
            Some(&["a".to_string()]),
            None,
        )
        .unwrap();
        assert_eq!(result, "0");
    }

    #[test]
    fn test_unicode_identifier() {
        // Greek letters should work
        let result = diff(
            "α + β".to_string(),
            "x".to_string(),
            Some(&["α".to_string(), "β".to_string()]),
            None,
        )
        .unwrap();
        assert_eq!(result, "0");
    }

    #[test]
    fn test_underscore_in_name() {
        let result = diff(
            "var_1*x".to_string(),
            "x".to_string(),
            Some(&["var_1".to_string()]),
            None,
        )
        .unwrap();
        assert!(result.contains("var_1"));
    }

    #[test]
    fn test_very_deep_nesting() {
        // Test recursion limit isn't hit
        let mut formula = "x".to_string();
        for _ in 0..50 {
            formula = format!("({})", formula);
        }
        let result = diff(formula, "x".to_string(), None, None);
        assert!(result.is_ok());
    }
}
