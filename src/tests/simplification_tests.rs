#[cfg(test)]
mod tests {
    use crate::simplification::simplify_expr;
    use crate::{Expr, ExprKind};
    use std::collections::HashSet;

    #[test]
    fn test_fraction_difference_issue() {
        use crate::parser;

        // Test case: 1/(x^2 - 1) - 1/(x^2 + 1)
        // Should simplify properly, canceling -1 + 1 and removing * 1
        let expr = parser::parse(
            "1/(x^2 - 1) - 1/(x^2 + 1)",
            &HashSet::new(),
            &HashSet::new(),
            None,
        )
        .unwrap();
        let result = simplify_expr(expr.clone(), HashSet::new());

        println!("Original: {}", expr);
        println!("Simplified: {}", result);

        // Test simpler cases that should work
        let test1 =
            parser::parse("x^2 + 1 - x^2 + 1", &HashSet::new(), &HashSet::new(), None).unwrap();
        let result1 = simplify_expr(test1.clone(), HashSet::new());
        println!("\nTest x^2 + 1 - x^2 + 1:");
        println!("  Original: {}", test1);
        println!("  Simplified: {}", result1);
        assert_eq!(format!("{}", result1), "2");

        let test2 =
            parser::parse("(1 + x) * (1 - x)", &HashSet::new(), &HashSet::new(), None).unwrap();
        let result2 = simplify_expr(test2.clone(), HashSet::new());
        println!("\nTest (1 + x) * (1 - x):");
        println!("  Original: {}", test2);
        println!("  Simplified: {}", result2);

        let test3 = parser::parse(
            "x^2 + (1 + x) * (1 - x) + 1",
            &HashSet::new(),
            &HashSet::new(),
            None,
        )
        .unwrap();
        let result3 = simplify_expr(test3.clone(), HashSet::new());
        println!("\nTest x^2 + (1 + x) * (1 - x) + 1:");
        println!("  Original: {}", test3);
        println!("  Simplified: {}", result3);
        println!("  Expected: 2");

        // This is what we're seeing - let me check if it further simplifies
        let test4 =
            parser::parse("x^2 + 1 - x^2 + 1", &HashSet::new(), &HashSet::new(), None).unwrap();
        let result4 = simplify_expr(test4.clone(), HashSet::new());
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
        let expr = Expr::sum(vec![Expr::symbol("x"), Expr::number(0.0)]);
        let result = simplify_expr(expr, HashSet::new());
        assert_eq!(result, Expr::symbol("x"));
    }

    #[test]
    fn test_mul_zero() {
        let expr = Expr::product(vec![Expr::symbol("x"), Expr::number(0.0)]);
        let result = simplify_expr(expr, HashSet::new());
        assert_eq!(result, Expr::number(0.0));
    }

    #[test]
    fn test_mul_one() {
        let expr = Expr::product(vec![Expr::symbol("x"), Expr::number(1.0)]);
        let result = simplify_expr(expr, HashSet::new());
        assert_eq!(result, Expr::symbol("x"));
    }

    #[test]
    fn test_pow_zero() {
        let expr = Expr::pow(Expr::symbol("x"), Expr::number(0.0));
        let result = simplify_expr(expr, HashSet::new());
        assert_eq!(result, Expr::number(1.0));
    }

    #[test]
    fn test_pow_one() {
        let expr = Expr::pow(Expr::symbol("x"), Expr::number(1.0));
        let result = simplify_expr(expr, HashSet::new());
        assert_eq!(result, Expr::symbol("x"));
    }

    #[test]
    fn test_constant_folding() {
        let expr = Expr::sum(vec![Expr::number(2.0), Expr::number(3.0)]);
        let result = simplify_expr(expr, HashSet::new());
        assert_eq!(result, Expr::number(5.0));
    }

    #[test]
    fn test_nested_simplification() {
        // (x + 0) * 1 should simplify to x
        let expr = Expr::product(vec![
            Expr::sum(vec![Expr::symbol("x"), Expr::number(0.0)]),
            Expr::number(1.0),
        ]);
        let result = simplify_expr(expr, HashSet::new());
        assert_eq!(result, Expr::symbol("x"));
    }

    #[test]
    fn test_trig_simplification() {
        // sin(0) = 0
        let expr = Expr::func("sin", Expr::number(0.0));
        assert_eq!(simplify_expr(expr, HashSet::new()), Expr::number(0.0));

        // cos(0) = 1
        let expr = Expr::func("cos", Expr::number(0.0));
        assert_eq!(simplify_expr(expr, HashSet::new()), Expr::number(1.0));

        // sin(-x) = -sin(x)
        let expr = Expr::func(
            "sin",
            Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
        );
        let simplified = simplify_expr(expr, HashSet::new());
        // Should be -1 * sin(x) or Product([-1, sin(x)])
        if let ExprKind::Product(factors) = &simplified.kind {
            assert!(factors.len() == 2);
            assert!(matches!(&factors[0].kind, ExprKind::Number(n) if *n == -1.0));
            if let ExprKind::FunctionCall { name, args } = &factors[1].kind {
                assert_eq!(name.as_str(), "sin");
                assert_eq!(*args[0], Expr::symbol("x"));
            } else {
                panic!("Expected function call");
            }
        } else {
            panic!("Expected Product, got {:?}", simplified);
        }
    }

    #[test]
    fn test_hyperbolic_simplification() {
        // sinh(0) = 0
        let expr = Expr::func("sinh", Expr::number(0.0));
        assert_eq!(simplify_expr(expr, HashSet::new()), Expr::number(0.0));

        // cosh(0) = 1
        let expr = Expr::func("cosh", Expr::number(0.0));
        assert_eq!(simplify_expr(expr, HashSet::new()), Expr::number(1.0));
    }

    #[test]
    fn test_log_exp_simplification() {
        // ln(1) = 0
        let expr = Expr::func("ln", Expr::number(1.0));
        assert_eq!(simplify_expr(expr, HashSet::new()), Expr::number(0.0));

        // exp(0) = 1
        let expr = Expr::func("exp", Expr::number(0.0));
        assert_eq!(simplify_expr(expr, HashSet::new()), Expr::number(1.0));

        // exp(ln(x)) = x
        let expr = Expr::func("exp", Expr::func("ln", Expr::symbol("x")));
        assert_eq!(simplify_expr(expr, HashSet::new()), Expr::symbol("x"));
    }

    #[test]
    fn test_fraction_preservation() {
        // 1/3 should stay 1/3
        let expr = Expr::div_expr(Expr::number(1.0), Expr::number(3.0));
        let simplified = simplify_expr(expr.clone(), HashSet::new());
        assert_eq!(simplified, expr);

        // 4/2 should become 2
        let expr = Expr::div_expr(Expr::number(4.0), Expr::number(2.0));
        let simplified = simplify_expr(expr, HashSet::new());
        assert_eq!(simplified, Expr::number(2.0));

        // 2/3 should stay 2/3
        let expr = Expr::div_expr(Expr::number(2.0), Expr::number(3.0));
        let simplified = simplify_expr(expr.clone(), HashSet::new());
        assert_eq!(simplified, expr);
    }

    #[test]
    fn test_distributive_property() {
        // x*y + x*z should become x*(y + z)
        let expr = Expr::sum(vec![
            Expr::product(vec![Expr::symbol("x"), Expr::symbol("y")]),
            Expr::product(vec![Expr::symbol("x"), Expr::symbol("z")]),
        ]);
        let simplified = simplify_expr(expr, HashSet::new());
        // Should be x*(y + z) (Product with x and Sum of y, z)
        if let ExprKind::Product(factors) = &simplified.kind {
            // Check if x is one of the factors
            let has_x = factors
                .iter()
                .any(|f| matches!(&f.kind, ExprKind::Symbol(s) if s.as_str() == "x"));
            assert!(has_x, "Expected x as a factor, got {:?}", simplified);

            // Check if (y + z) is one of the factors
            let has_sum = factors.iter().any(|f| {
                if let ExprKind::Sum(terms) = &f.kind {
                    let has_y = terms
                        .iter()
                        .any(|t| matches!(&t.kind, ExprKind::Symbol(s) if s.as_str() == "y"));
                    let has_z = terms
                        .iter()
                        .any(|t| matches!(&t.kind, ExprKind::Symbol(s) if s.as_str() == "z"));
                    has_y && has_z
                } else {
                    false
                }
            });
            assert!(
                has_sum,
                "Expected (y + z) as a factor, got {:?}",
                simplified
            );
        } else {
            panic!("Expected Product, got {:?}", simplified);
        }
    }

    #[test]
    fn test_binomial_expansion() {
        // x^2 + 2*x*y + y^2 should become (x + y)^2
        let expr = Expr::sum(vec![
            Expr::pow(Expr::symbol("x"), Expr::number(2.0)),
            Expr::product(vec![
                Expr::number(2.0),
                Expr::symbol("x"),
                Expr::symbol("y"),
            ]),
            Expr::pow(Expr::symbol("y"), Expr::number(2.0)),
        ]);
        let simplified = simplify_expr(expr, HashSet::new());
        println!("Binomial expansion test - Original: x^2 + 2*x*y + y^2");
        println!("Binomial expansion test - Simplified: {:?}", simplified);

        // Must be factored to (x + y)^2
        if let ExprKind::Pow(base, exp) = &simplified.kind {
            assert_eq!(**exp, Expr::number(2.0), "Expected exponent 2");
            // Check base contains x and y
            let base_str = base.to_string();
            assert!(
                base_str.contains("x") && base_str.contains("y"),
                "Expected base (x + y), got: {}",
                base_str
            );
        } else {
            panic!("Expected factored form (x+y)^2, but got: {:?}", simplified);
        }
    }

    #[test]
    fn test_product_four_functions_derivative() {
        use crate::diff;
        // d/dx [x * exp(x) * sin(x) * ln(x)]
        let expr_str = "x * exp(x) * sin(x) * ln(x)";
        let derivative = diff(expr_str, "x", None, None).unwrap();

        // Check if "... / x" is present (it shouldn't be if simplified)
        let derivative_str = derivative.to_string();
        assert!(
            !derivative_str.contains("/ x"),
            "Derivative contains unsimplified division by x: {}",
            derivative_str
        );
    }

    #[test]
    fn test_flatten_mul_div_structure() {
        // (A / B) * R^2 -> (A * R^2) / B
        let expr = Expr::product(vec![
            Expr::div_expr(Expr::symbol("A"), Expr::symbol("B")),
            Expr::pow(Expr::symbol("R"), Expr::number(2.0)),
        ]);
        let simplified = simplify_expr(expr, HashSet::new());
        println!("Simplified: {:?}", simplified);

        if let ExprKind::Div(num, den) = &simplified.kind {
            // Check denominator is B
            if let ExprKind::Symbol(s) = &den.kind {
                assert_eq!(s.as_str(), "B");
            } else {
                panic!("Expected denominator B");
            }

            // Check numerator contains A and R^2
            let num_str = format!("{}", num);
            assert!(
                num_str.contains("A") && num_str.contains("R"),
                "Expected numerator with A and R, got {}",
                num_str
            );
        } else {
            // If it's not Div, it failed to flatten
            panic!("Expected Div, got {:?}", simplified);
        }
    }

    #[test]
    fn test_polynomial_gcd_simplification() {
        use crate::parser;

        // (x^2 - 1) / (x - 1) should simplify to (x + 1)
        // Since x^2 - 1 = (x-1)(x+1), GCD is (x-1)
        let expr = parser::parse(
            "(x^2 - 1) / (x - 1)",
            &HashSet::new(),
            &HashSet::new(),
            None,
        )
        .unwrap();
        let simplified = simplify_expr(expr.clone(), HashSet::new());

        println!("PolyGCD test: {} -> {}", expr, simplified);

        // Should become x + 1
        let expected = parser::parse("x + 1", &HashSet::new(), &HashSet::new(), None).unwrap();
        let expected_simplified = simplify_expr(expected, HashSet::new());

        assert_eq!(
            format!("{}", simplified),
            format!("{}", expected_simplified),
            "Expected (x^2-1)/(x-1) to simplify to x+1"
        );
    }
}
