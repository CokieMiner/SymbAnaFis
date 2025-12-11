#[cfg(test)]
mod tests {
    use crate::{Expr, ExprKind, simplification::simplify_expr};
    use std::collections::HashSet;
    use std::sync::Arc;

    #[test]
    fn test_trig_sum_identity() {
        // sin(x)cos(y) + cos(x)sin(y) -> sin(x+y)
        let expr = Expr::new(ExprKind::Add(
            Arc::new(Expr::new(ExprKind::Mul(
                Arc::new(Expr::new(ExprKind::FunctionCall {
                    name: "sin".to_string(),
                    args: vec![Expr::symbol("x")],
                })),
                Arc::new(Expr::new(ExprKind::FunctionCall {
                    name: "cos".to_string(),
                    args: vec![Expr::symbol("y")],
                })),
            ))),
            Arc::new(Expr::new(ExprKind::Mul(
                Arc::new(Expr::new(ExprKind::FunctionCall {
                    name: "cos".to_string(),
                    args: vec![Expr::symbol("x")],
                })),
                Arc::new(Expr::new(ExprKind::FunctionCall {
                    name: "sin".to_string(),
                    args: vec![Expr::symbol("y")],
                })),
            ))),
        ));
        let simplified = simplify_expr(expr, HashSet::new());
        let expected = Expr::new(ExprKind::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::new(ExprKind::Add(
                Arc::new(Expr::symbol("x")),
                Arc::new(Expr::symbol("y")),
            ))],
        });
        assert_eq!(simplified, expected);
    }

    #[test]
    fn test_trig_combination() {
        let simplified = simplify_expr(
            Expr::new(ExprKind::FunctionCall {
                name: "sin".to_string(),
                args: vec![Expr::new(ExprKind::Add(
                    Arc::new(Expr::symbol("x")),
                    Arc::new(Expr::symbol("y")),
                ))],
            }),
            HashSet::new(),
        );
        if let ExprKind::FunctionCall { name, args } = &simplified.kind {
            assert_eq!(name, "sin");
            assert_eq!(args.len(), 1);
            if let ExprKind::Add(u, v) = &args[0].kind {
                let is_xy = **u == Expr::symbol("x") && **v == Expr::symbol("y");
                let is_yx = **u == Expr::symbol("y") && **v == Expr::symbol("x");
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
        let expr = Expr::new(ExprKind::FunctionCall {
            name: "sqrt".to_string(),
            args: vec![Expr::number(4.0)],
        });
        let simplified = simplify_expr(expr.clone(), HashSet::new());
        assert_eq!(simplified, Expr::number(2.0));

        let expr2 = Expr::new(ExprKind::FunctionCall {
            name: "sqrt".to_string(),
            args: vec![Expr::number(2.0)],
        });
        let simplified2 = simplify_expr(expr2.clone(), HashSet::new());
        assert_eq!(simplified2, expr2);

        // cbrt(27) -> 3, cbrt(2) remains symbolic
        let expr = Expr::new(ExprKind::FunctionCall {
            name: "cbrt".to_string(),
            args: vec![Expr::number(27.0)],
        });
        let simplified = simplify_expr(expr.clone(), HashSet::new());
        assert_eq!(simplified, Expr::number(3.0));

        let expr2 = Expr::new(ExprKind::FunctionCall {
            name: "cbrt".to_string(),
            args: vec![Expr::number(2.0)],
        });
        let simplified2 = simplify_expr(expr2.clone(), HashSet::new());
        assert_eq!(simplified2, expr2);
    }

    #[test]
    fn test_fraction_integer_collapsing() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        let ast = parser::parse("12 / 3", &fixed, &funcs).unwrap();
        let simplified = simplify_expr(ast, HashSet::new());
        assert_eq!(format!("{}", simplified), "4");
    }

    #[test]
    fn test_trig_triple_angle() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        let ast = parser::parse("3 * sin(x) - 4 * sin(x)^3", &fixed, &funcs).unwrap();
        let simplified = simplify_expr(ast, HashSet::new());
        // 3*sin(x) - 4*sin(x)^3 (or equivalent canonical Form)
        // Build expected expression structurally for exact matching
        // Expect sin(3 * x)
        let expected = Expr::new(ExprKind::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::new(ExprKind::Mul(
                Arc::new(Expr::number(3.0)),
                Arc::new(Expr::symbol("x")),
            ))],
        });
        assert_eq!(simplified, expected);
    }

    #[test]
    fn test_hyperbolic_triple_angle() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        let ast = parser::parse("4 * sinh(x)^3 + 3 * sinh(x)", &fixed, &funcs).unwrap();
        let simplified = simplify_expr(ast, HashSet::new());
        let expected = Expr::new(ExprKind::FunctionCall {
            name: "sinh".to_string(),
            args: vec![Expr::new(ExprKind::Mul(
                Arc::new(Expr::number(3.0)),
                Arc::new(Expr::symbol("x")),
            ))],
        });
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
        let simplified = simplify_expr(ast, HashSet::new());
        let expected = Expr::new(ExprKind::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::new(ExprKind::Mul(
                Arc::new(Expr::number(3.0)),
                Arc::new(Expr::symbol("x")),
            ))],
        });
        assert_eq!(simplified, expected);
    }

    #[test]
    fn test_trig_triple_angle_edge_cases() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        // Floating coefficient should not fold
        let ast = parser::parse("3.000000001 * sin(x) - 4 * sin(x)^3", &fixed, &funcs).unwrap();
        let simplified = simplify_expr(ast, HashSet::new());
        // Expect no simplification to triple-angle
        assert!(!matches!(simplified.kind, ExprKind::FunctionCall { name, .. } if name == "sin"));

        // Symbolic coefficient should not fold
        let ast = parser::parse("a * sin(x) - 4 * sin(x)^3", &fixed, &funcs).unwrap();
        let simplified = simplify_expr(ast, HashSet::new());
        assert!(!matches!(simplified.kind, ExprKind::FunctionCall { name, .. } if name == "sin"));
    }

    #[test]
    fn test_trig_triple_angle_float_tolerance() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        // small floating difference within tolerance should fold
        let ast = parser::parse("3.00000000001 * sin(x) - 4.0 * sin(x)^3", &fixed, &funcs).unwrap();
        let simplified = simplify_expr(ast, HashSet::new());
        let expected = Expr::new(ExprKind::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::new(ExprKind::Mul(
                Arc::new(Expr::number(3.0)),
                Arc::new(Expr::symbol("x")),
            ))],
        });
        assert_eq!(simplified, expected);

        // same for cos triple angle
        let ast = parser::parse("4.00000000001 * cos(x)^3 - 3.0 * cos(x)", &fixed, &funcs).unwrap();
        let simplified2 = simplify_expr(ast, HashSet::new());
        let expected2 = Expr::new(ExprKind::FunctionCall {
            name: "cos".to_string(),
            args: vec![Expr::new(ExprKind::Mul(
                Arc::new(Expr::number(3.0)),
                Arc::new(Expr::symbol("x")),
            ))],
        });
        assert_eq!(simplified2, expected2);
    }

    #[test]
    fn test_trig_triple_angle_float_exact() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        let ast = parser::parse("3.0 * sin(x) - 4.0 * sin(x)^3", &fixed, &funcs).unwrap();
        let simplified = simplify_expr(ast, HashSet::new());
        let expected = Expr::new(ExprKind::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::new(ExprKind::Mul(
                Arc::new(Expr::number(3.0)),
                Arc::new(Expr::symbol("x")),
            ))],
        });
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
        let simplified = simplify_expr(ast, HashSet::new());
        let expected = Expr::new(ExprKind::FunctionCall {
            name: "sinh".to_string(),
            args: vec![Expr::new(ExprKind::Mul(
                Arc::new(Expr::number(3.0)),
                Arc::new(Expr::symbol("x")),
            ))],
        });
        assert_eq!(simplified, expected);
    }

    #[test]
    fn test_roots_numeric_more_examples() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        // sqrt(9) -> 3; sqrt(8) should remain symbolic
        let ast = parser::parse("sqrt(9)", &fixed, &funcs).unwrap();
        assert_eq!(simplify_expr(ast, HashSet::new()), Expr::number(3.0));
        let ast = parser::parse("sqrt(8)", &fixed, &funcs).unwrap();
        assert_eq!(simplify_expr(ast.clone(), HashSet::new()), ast);

        // cbrt(27) -> 3; cbrt(9) stays symbolic
        let ast = parser::parse("cbrt(27)", &fixed, &funcs).unwrap();
        assert_eq!(simplify_expr(ast, HashSet::new()), Expr::number(3.0));
        let ast = parser::parse("cbrt(9)", &fixed, &funcs).unwrap();
        assert_eq!(simplify_expr(ast.clone(), HashSet::new()), ast);
    }

    #[test]
    fn test_fraction_more_examples() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        // 18 / 3 -> 6
        let ast = parser::parse("18 / 3", &fixed, &funcs).unwrap();
        assert_eq!(format!("{}", simplify_expr(ast, HashSet::new())), "6");
        // 7 / 2 remains 7/2
        let ast = parser::parse("7 / 2", &fixed, &funcs).unwrap();
        let simplified = simplify_expr(ast.clone(), HashSet::new());
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
        let simplified = format!("{}", simplify_expr(ast, HashSet::new()));
        assert!(simplified.len() < orig.len());

        // numeric fraction reduced
        let s = "12 / 3";
        let ast = parser::parse(s, &fixed, &funcs).unwrap();
        let orig = format!("{}", ast);
        let simplified = format!("{}", simplify_expr(ast, HashSet::new()));
        assert!(simplified.len() < orig.len());

        // sqrt numeric example
        let s = "sqrt(9)";
        let ast = parser::parse(s, &fixed, &funcs).unwrap();
        let orig = format!("{}", ast);
        let simplified = format!("{}", simplify_expr(ast, HashSet::new()));
        assert!(simplified.len() < orig.len());
    }
}
