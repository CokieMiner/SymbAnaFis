#[cfg(test)]
mod tests {
    use crate::{Expr, simplification::simplify};

    #[test]
    fn test_trig_sum_identity() {
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
        let simplified = simplify(expr);
        let expected = Expr::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::Add(
                Box::new(Expr::Symbol("x".to_string())),
                Box::new(Expr::Symbol("y".to_string())),
            )],
        };
        assert_eq!(simplified, expected);
    }

    #[test]
    fn test_trig_combination() {
        let simplified = simplify(Expr::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::Add(
                Box::new(Expr::Symbol("x".to_string())),
                Box::new(Expr::Symbol("y".to_string())),
            )],
        });
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

    #[test]
    fn test_roots_numeric_integer() {
        // sqrt(4) -> 2, sqrt(2) stays symbolic
        let expr = Expr::FunctionCall {
            name: "sqrt".to_string(),
            args: vec![Expr::Number(4.0)],
        };
        let simplified = simplify(expr.clone());
        assert_eq!(simplified, Expr::Number(2.0));

        let expr2 = Expr::FunctionCall {
            name: "sqrt".to_string(),
            args: vec![Expr::Number(2.0)],
        };
        let simplified2 = simplify(expr2.clone());
        assert_eq!(simplified2, expr2);

        // cbrt(27) -> 3, cbrt(2) remains symbolic
        let expr = Expr::FunctionCall {
            name: "cbrt".to_string(),
            args: vec![Expr::Number(27.0)],
        };
        let simplified = simplify(expr.clone());
        assert_eq!(simplified, Expr::Number(3.0));

        let expr2 = Expr::FunctionCall {
            name: "cbrt".to_string(),
            args: vec![Expr::Number(2.0)],
        };
        let simplified2 = simplify(expr2.clone());
        assert_eq!(simplified2, expr2);
    }

    #[test]
    fn test_fraction_integer_collapsing() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        let ast = parser::parse("12 / 3", &fixed, &funcs).unwrap();
        let simplified = simplify(ast);
        assert_eq!(format!("{}", simplified), "4");
    }

    #[test]
    fn test_trig_triple_angle() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        let ast = parser::parse("3 * sin(x) - 4 * sin(x)^3", &fixed, &funcs).unwrap();
        let simplified = simplify(ast);
        // 3*sin(x) - 4*sin(x)^3 (or equivalent canonical Form)
        // Build expected expression structurally for exact matching
        // Expect sin(3 * x)
        let expected = Expr::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::Mul(
                Box::new(Expr::Number(3.0)),
                Box::new(Expr::Symbol("x".to_string())),
            )],
        };
        assert_eq!(simplified, expected);
    }

    #[test]
    fn test_hyperbolic_triple_angle() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        let ast = parser::parse("4 * sinh(x)^3 + 3 * sinh(x)", &fixed, &funcs).unwrap();
        let simplified = simplify(ast);
        let expected = Expr::FunctionCall {
            name: "sinh".to_string(),
            args: vec![Expr::Mul(
                Box::new(Expr::Number(3.0)),
                Box::new(Expr::Symbol("x".to_string())),
            )],
        };
        assert_eq!(simplified, expected);
    }

    #[test]
    fn test_trig_triple_angle_permutations() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        // Test the canonical form
        let s = "3 * sin(x) - 4 * sin(x)^3";
        let ast = parser::parse(s, &fixed, &funcs).unwrap();
        let simplified = simplify(ast);
        let expected = Expr::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::Mul(
                Box::new(Expr::Number(3.0)),
                Box::new(Expr::Symbol("x".to_string())),
            )],
        };
        assert_eq!(simplified, expected);
    }

    #[test]
    fn test_trig_triple_angle_edge_cases() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        // Floating coefficient should not fold
        let ast = parser::parse("3.000000001 * sin(x) - 4 * sin(x)^3", &fixed, &funcs).unwrap();
        let simplified = simplify(ast);
        // Expect no simplification to triple-angle
        assert!(!matches!(simplified, Expr::FunctionCall { name, .. } if name == "sin"));

        // Symbolic coefficient should not fold
        let ast = parser::parse("a * sin(x) - 4 * sin(x)^3", &fixed, &funcs).unwrap();
        let simplified = simplify(ast);
        assert!(!matches!(simplified, Expr::FunctionCall { name, .. } if name == "sin"));
    }

    #[test]
    fn test_trig_triple_angle_float_tolerance() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        // small floating difference within tolerance should fold
        let ast = parser::parse("3.00000000001 * sin(x) - 4.0 * sin(x)^3", &fixed, &funcs).unwrap();
        let simplified = simplify(ast);
        let expected = Expr::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::Mul(
                Box::new(Expr::Number(3.0)),
                Box::new(Expr::Symbol("x".to_string())),
            )],
        };
        assert_eq!(simplified, expected);

        // same for cos triple angle
        let ast = parser::parse("4.00000000001 * cos(x)^3 - 3.0 * cos(x)", &fixed, &funcs).unwrap();
        let simplified2 = simplify(ast);
        let expected2 = Expr::FunctionCall {
            name: "cos".to_string(),
            args: vec![Expr::Mul(
                Box::new(Expr::Number(3.0)),
                Box::new(Expr::Symbol("x".to_string())),
            )],
        };
        assert_eq!(simplified2, expected2);
    }

    #[test]
    fn test_trig_triple_angle_float_exact() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        let ast = parser::parse("3.0 * sin(x) - 4.0 * sin(x)^3", &fixed, &funcs).unwrap();
        let simplified = simplify(ast);
        let expected = Expr::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::Mul(
                Box::new(Expr::Number(3.0)),
                Box::new(Expr::Symbol("x".to_string())),
            )],
        };
        assert_eq!(simplified, expected);
    }

    #[test]
    fn test_hyperbolic_triple_angle_float_tolerance() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        // sinh triple angle small difference
        let ast =
            parser::parse("4.00000000001 * sinh(x)^3 + 3.0 * sinh(x)", &fixed, &funcs).unwrap();
        let simplified = simplify(ast);
        let expected = Expr::FunctionCall {
            name: "sinh".to_string(),
            args: vec![Expr::Mul(
                Box::new(Expr::Number(3.0)),
                Box::new(Expr::Symbol("x".to_string())),
            )],
        };
        assert_eq!(simplified, expected);
    }

    #[test]
    fn test_roots_numeric_more_examples() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        // sqrt(9) -> 3; sqrt(8) should remain symbolic
        let ast = parser::parse("sqrt(9)", &fixed, &funcs).unwrap();
        assert_eq!(simplify(ast), Expr::Number(3.0));
        let ast = parser::parse("sqrt(8)", &fixed, &funcs).unwrap();
        assert_eq!(simplify(ast.clone()), ast);

        // cbrt(27) -> 3; cbrt(9) stays symbolic
        let ast = parser::parse("cbrt(27)", &fixed, &funcs).unwrap();
        assert_eq!(simplify(ast), Expr::Number(3.0));
        let ast = parser::parse("cbrt(9)", &fixed, &funcs).unwrap();
        assert_eq!(simplify(ast.clone()), ast);
    }

    #[test]
    fn test_fraction_more_examples() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        // 18 / 3 -> 6
        let ast = parser::parse("18 / 3", &fixed, &funcs).unwrap();
        assert_eq!(format!("{}", simplify(ast)), "6");
        // 7 / 2 remains 7/2
        let ast = parser::parse("7 / 2", &fixed, &funcs).unwrap();
        let simplified = simplify(ast.clone());
        assert_eq!(simplified, ast);
    }

    #[test]
    fn test_simplification_reduces_display_length() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        // triple-angle reduced
        let s = "3 * sin(x) - 4 * sin(x)^3";
        let ast = parser::parse(s, &fixed, &funcs).unwrap();
        let orig = format!("{}", ast);
        let simplified = format!("{}", simplify(ast));
        assert!(simplified.len() < orig.len());

        // numeric fraction reduced
        let s = "12 / 3";
        let ast = parser::parse(s, &fixed, &funcs).unwrap();
        let orig = format!("{}", ast);
        let simplified = format!("{}", simplify(ast));
        assert!(simplified.len() < orig.len());

        // sqrt numeric example
        let s = "sqrt(9)";
        let ast = parser::parse(s, &fixed, &funcs).unwrap();
        let orig = format!("{}", ast);
        let simplified = format!("{}", simplify(ast));
        assert!(simplified.len() < orig.len());
    }
}
