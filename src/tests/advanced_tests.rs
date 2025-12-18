// Advanced simplification and expression tests
mod simplification_advanced {
    use crate::simplification::simplify_expr;
    use crate::{Expr, ExprKind};
    use std::collections::HashSet;

    #[test]
    fn test_nested_add_zero() {
        // (x + 0) + (0 + y) should simplify to (x + y)
        let expr = Expr::sum(vec![
            Expr::sum(vec![Expr::symbol("x"), Expr::number(0.0)]),
            Expr::sum(vec![Expr::number(0.0), Expr::symbol("y")]),
        ]);
        let result = simplify_expr(expr, HashSet::new());
        // Should have no zeros
        assert!(!format!("{:?}", result).contains("0.0"));
    }

    #[test]
    fn test_mul_by_zero_nested() {
        // (x + 1) * 0 should become 0
        let expr = Expr::product(vec![
            Expr::sum(vec![Expr::symbol("x"), Expr::number(1.0)]),
            Expr::number(0.0),
        ]);
        let result = simplify_expr(expr, HashSet::new());
        assert_eq!(result, Expr::number(0.0));
    }

    #[test]
    fn test_constant_folding_chain() {
        // (2 + 3) * (4 + 1) should become 25
        let expr = Expr::product(vec![
            Expr::sum(vec![Expr::number(2.0), Expr::number(3.0)]),
            Expr::sum(vec![Expr::number(4.0), Expr::number(1.0)]),
        ]);
        let result = simplify_expr(expr, HashSet::new());
        assert_eq!(result, Expr::number(25.0));
    }

    #[test]
    fn test_power_simplification() {
        // x^0 * y^1 * z^2 should simplify to y * z^2
        let expr = Expr::product(vec![
            Expr::pow(Expr::symbol("x"), Expr::number(0.0)),
            Expr::pow(Expr::symbol("y"), Expr::number(1.0)),
            Expr::pow(Expr::symbol("z"), Expr::number(2.0)),
        ]);
        let result = simplify_expr(expr, HashSet::new());
        // x^0 = 1, y^1 = y, so should have y and z
        assert!(format!("{:?}", result).contains("z"));
    }

    #[test]
    fn test_power_division_simplification() {
        // x^2 / x^2 should simplify to 1
        let expr = Expr::div_expr(
            Expr::pow(Expr::symbol("x"), Expr::number(2.0)),
            Expr::pow(Expr::symbol("x"), Expr::number(2.0)),
        );
        let result = simplify_expr(expr, HashSet::new());
        assert_eq!(result, Expr::number(1.0));
    }

    #[test]
    fn test_complex_power_division() {
        // (x+1)^3 / (x+1)^2 should simplify to (x+1)
        let base = Expr::sum(vec![Expr::symbol("x"), Expr::number(1.0)]);
        let expr = Expr::div_expr(
            Expr::pow(base.clone(), Expr::number(3.0)),
            Expr::pow(base, Expr::number(2.0)),
        );
        let result = simplify_expr(expr, HashSet::new());
        // Should simplify to (x+1)^1, which should further simplify to (x+1)
        // Due to canonical ordering by degree, it becomes x + 1
        let expected = Expr::sum(vec![Expr::symbol("x"), Expr::number(1.0)]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_general_power_division() {
        // Test with different expressions inside parentheses
        // (x^2 + y)^4 / (x^2 + y)^2 should simplify to (x^2 + y)^2
        let base = Expr::sum(vec![
            Expr::pow(Expr::symbol("x"), Expr::number(2.0)),
            Expr::symbol("y"),
        ]);
        let expr = Expr::div_expr(
            Expr::pow(base.clone(), Expr::number(4.0)),
            Expr::pow(base, Expr::number(2.0)),
        );
        let result = simplify_expr(expr, HashSet::new());
        // Should simplify to (x^2 + y)^2
        // Due to canonical ordering by degree, it becomes x^2 + y
        let expected_base = Expr::sum(vec![
            Expr::pow(Expr::symbol("x"), Expr::number(2.0)),
            Expr::symbol("y"),
        ]);
        let expected = Expr::pow(expected_base, Expr::number(2.0));
        assert_eq!(result, expected);
    }

    #[test]
    fn test_function_power_division() {
        // Test with function expressions: sin(x)^3 / sin(x)^2 should simplify to sin(x)
        let base = Expr::func("sin", Expr::symbol("x"));
        let expr = Expr::div_expr(
            Expr::pow(base.clone(), Expr::number(3.0)),
            Expr::pow(base, Expr::number(2.0)),
        );
        let result = simplify_expr(expr, HashSet::new());
        // Should simplify to sin(x)^1, which should further simplify to sin(x)
        let expected = Expr::func("sin", Expr::symbol("x"));
        assert_eq!(result, expected);
    }

    #[test]
    fn test_div_by_one() {
        // (x + y) / 1 should become (x + y)
        let expr = Expr::div_expr(
            Expr::sum(vec![Expr::symbol("x"), Expr::symbol("y")]),
            Expr::number(1.0),
        );
        let result = simplify_expr(expr, HashSet::new());
        // Should not contain Div anymore
        assert!(!matches!(result.kind, ExprKind::Div(_, _)));
    }

    #[test]
    fn test_div_zero() {
        // 0 / x should become 0
        let expr = Expr::div_expr(Expr::number(0.0), Expr::symbol("x"));
        let result = simplify_expr(expr, HashSet::new());
        assert_eq!(result, Expr::number(0.0));
    }

    #[test]
    fn test_sub_zero() {
        // x - 0 should become x (subtraction represented as x + (-0))
        let expr = Expr::sum(vec![
            Expr::symbol("x"),
            Expr::product(vec![Expr::number(-1.0), Expr::number(0.0)]),
        ]);
        let result = simplify_expr(expr, HashSet::new());
        assert_eq!(result, Expr::symbol("x"));
    }

    #[test]
    fn test_constant_div() {
        // 10 / 2 should become 5
        let expr = Expr::div_expr(Expr::number(10.0), Expr::number(2.0));
        let result = simplify_expr(expr, HashSet::new());
        assert_eq!(result, Expr::number(5.0));
    }

    #[test]
    fn test_constant_sub() {
        // 10 - 3 should become 7 (as Sum([10, Product([-1, 3])]))
        let expr = Expr::sum(vec![
            Expr::number(10.0),
            Expr::product(vec![Expr::number(-1.0), Expr::number(3.0)]),
        ]);
        let result = simplify_expr(expr, HashSet::new());
        assert_eq!(result, Expr::number(7.0));
    }
}

mod real_world_expressions {

    use crate::diff;

    #[test]
    fn test_quadratic_derivative() {
        // d/dx[ax^2 + bx + c] = 2ax + b
        let result = diff("a*x^2 + b*x + c", "x", Some(&["a", "b", "c"]), None);
        assert!(result.is_ok());
        let deriv = result.unwrap();
        assert!(deriv.contains("a") && deriv.contains("x") && deriv.contains("b"));
    }

    #[test]
    fn test_rational_function() {
        // d/dx[(x^2 + 1)/(x - 1)]
        let result = diff("(x^2 + 1)/(x - 1)", "x", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_exponential_growth() {
        // d/dx[a * exp(k*x)]
        let result = diff("a*exp(k*x)", "x", Some(&["a", "k"]), None);
        assert!(result.is_ok());
        let deriv = result.unwrap();
        assert!(deriv.contains("exp") && deriv.contains("k"));
    }

    #[test]
    fn test_trig_identity_input() {
        // d/dx[sin(x)^2 + cos(x)^2] should work (even if not simplified to 0)
        let result = diff("sin(x)^2 + cos(x)^2", "x", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_product_of_three() {
        // d/dx[x * sin(x) * exp(x)]
        let result = diff("x*sin(x)*exp(x)", "x", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_chain_rule_triple() {
        // d/dx[sin(exp(ln(x)))]
        let result = diff("sin(exp(ln(x)))", "x", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hyperbolic_combo() {
        // d/dx[sinh(x) * cosh(x)]
        let result = diff("sinh(x)*cosh(x)", "x", None, None);
        assert!(result.is_ok());
        let deriv = result.unwrap();
        assert!(deriv.contains("sinh") || deriv.contains("cosh"));
    }

    #[test]
    fn test_implicit_with_division() {
        // d/dx[y(x)/x] where y is custom function
        let result = diff("y(x)/x", "x", None, Some(&["y"]));
        assert!(result.is_ok());
    }
}

mod boundary_conditions {

    use crate::diff;

    #[test]
    fn test_single_char_fixed_var() {
        let result = diff("a", "x", Some(&["a"]), None).unwrap();
        assert_eq!(result, "0");
    }

    #[test]
    fn test_unicode_identifier() {
        // Greek letters should work
        let result = diff("α + β", "x", Some(&["α", "β"]), None).unwrap();
        assert_eq!(result, "0");
    }

    #[test]
    fn test_underscore_in_name() {
        let result = diff("var_1*x", "x", Some(&["var_1"]), None).unwrap();
        assert!(result.contains("var_1"));
    }

    #[test]
    fn test_very_deep_nesting() {
        // Test recursion limit isn't hit
        let mut formula = "x".to_string();
        for _ in 0..50 {
            formula = format!("({})", formula);
        }
        let result = diff(&formula, "x", None, None);
        assert!(result.is_ok());
    }
}
