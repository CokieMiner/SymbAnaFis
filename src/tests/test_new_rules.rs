#[cfg(test)]
mod tests {
    use crate::core::known_symbols::KS;
    use crate::{Expr, ExprKind, simplification::simplify_expr};
    use std::collections::{HashMap, HashSet};

    #[test]
    fn test_trig_sum_identity() {
        // sin(x)cos(y) + cos(x)sin(y) -> sin(x+y)
        let expr = Expr::sum(vec![
            Expr::product(vec![
                Expr::func("sin", Expr::symbol("x")),
                Expr::func("cos", Expr::symbol("y")),
            ]),
            Expr::product(vec![
                Expr::func("cos", Expr::symbol("x")),
                Expr::func("sin", Expr::symbol("y")),
            ]),
        ]);
        let simplified = simplify_expr(
            expr,
            HashSet::new(),
            HashMap::new(),
            None,
            None,
            None,
            false,
        );
        let expected = Expr::func("sin", Expr::sum(vec![Expr::symbol("x"), Expr::symbol("y")]));
        assert_eq!(simplified.hash, expected.hash);
    }

    #[test]
    fn test_trig_combination() {
        let simplified = simplify_expr(
            Expr::func("sin", Expr::sum(vec![Expr::symbol("x"), Expr::symbol("y")])),
            HashSet::new(),
            HashMap::new(),
            None,
            None,
            None,
            false,
        );
        if let ExprKind::FunctionCall { name, args } = &simplified.kind {
            assert_eq!(name.as_str(), "sin");
            assert_eq!(args.len(), 1);
            if let ExprKind::Sum(terms) = &args[0].kind {
                let has_x = terms.iter().any(|t| **t == Expr::symbol("x"));
                let has_y = terms.iter().any(|t| **t == Expr::symbol("y"));
                assert!(has_x && has_y);
            } else {
                panic!("Expected Sum inside sin");
            }
        } else {
            panic!("Expected FunctionCall sin");
        }
    }

    #[test]
    fn test_roots_numeric_integer() {
        // sqrt(4) -> 2, sqrt(2) stays symbolic
        let expr = Expr::func("sqrt", Expr::number(4.0));
        let simplified = simplify_expr(
            expr.clone(),
            HashSet::new(),
            HashMap::new(),
            None,
            None,
            None,
            false,
        );
        assert_eq!(simplified, Expr::number(2.0));

        let expr2 = Expr::func("sqrt", Expr::number(2.0));
        let simplified2 = simplify_expr(
            expr2.clone(),
            HashSet::new(),
            HashMap::new(),
            None,
            None,
            None,
            false,
        );
        assert_eq!(simplified2, expr2);

        // cbrt(27) -> 3, cbrt(2) remains symbolic
        let expr = Expr::func("cbrt", Expr::number(27.0));
        let simplified = simplify_expr(
            expr.clone(),
            HashSet::new(),
            HashMap::new(),
            None,
            None,
            None,
            false,
        );
        assert_eq!(simplified, Expr::number(3.0));

        let expr2 = Expr::func("cbrt", Expr::number(2.0));
        let simplified2 = simplify_expr(
            expr2.clone(),
            HashSet::new(),
            HashMap::new(),
            None,
            None,
            None,
            false,
        );
        assert_eq!(simplified2, expr2);
    }

    #[test]
    fn test_fraction_integer_collapsing() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        let ast = parser::parse("12 / 3", &fixed, &funcs, None).unwrap();
        let simplified =
            simplify_expr(ast, HashSet::new(), HashMap::new(), None, None, None, false);
        assert_eq!(format!("{}", simplified), "4");
    }

    #[test]
    fn test_trig_triple_angle() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        let ast = parser::parse("3 * sin(x) - 4 * sin(x)^3", &fixed, &funcs, None).unwrap();
        let simplified =
            simplify_expr(ast, HashSet::new(), HashMap::new(), None, None, None, false);
        // Expect sin(3 * x) or sin(Product([3, x]))
        let expected = Expr::func(
            "sin",
            Expr::product(vec![Expr::number(3.0), Expr::symbol("x")]),
        );
        assert_eq!(simplified, expected);
    }

    #[test]
    fn test_hyperbolic_triple_angle() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        let ast = parser::parse("4 * sinh(x)^3 + 3 * sinh(x)", &fixed, &funcs, None).unwrap();
        let simplified =
            simplify_expr(ast, HashSet::new(), HashMap::new(), None, None, None, false);
        let expected = Expr::func(
            "sinh",
            Expr::product(vec![Expr::number(3.0), Expr::symbol("x")]),
        );
        assert_eq!(simplified, expected);
    }

    #[test]
    fn test_trig_triple_angle_permutations() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        // Test the canonical form
        let s = "3 * sin(x) - 4 * sin(x)^3";
        let ast = parser::parse(s, &fixed, &funcs, None).unwrap();
        let simplified =
            simplify_expr(ast, HashSet::new(), HashMap::new(), None, None, None, false);
        let expected = Expr::func(
            "sin",
            Expr::product(vec![Expr::number(3.0), Expr::symbol("x")]),
        );
        assert_eq!(simplified, expected);
    }

    #[test]
    fn test_trig_triple_angle_edge_cases() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        // Floating coefficient should not fold
        let ast =
            parser::parse("3.000000001 * sin(x) - 4 * sin(x)^3", &fixed, &funcs, None).unwrap();
        let simplified =
            simplify_expr(ast, HashSet::new(), HashMap::new(), None, None, None, false);
        // Expect no simplification to triple-angle
        assert!(
            !matches!(&simplified.kind, ExprKind::FunctionCall { name, .. } if name.id() == KS.sin)
        );

        // Symbolic coefficient should not fold
        let ast = parser::parse("a * sin(x) - 4 * sin(x)^3", &fixed, &funcs, None).unwrap();
        let simplified =
            simplify_expr(ast, HashSet::new(), HashMap::new(), None, None, None, false);
        assert!(
            !matches!(&simplified.kind, ExprKind::FunctionCall { name, .. } if name.id() == KS.sin)
        );
    }

    #[test]
    fn test_trig_triple_angle_float_tolerance() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        // small floating difference within tolerance should fold
        let ast = parser::parse(
            "3.0000000000000001 * sin(x) - 4.0 * sin(x)^3",
            &fixed,
            &funcs,
            None,
        )
        .unwrap();
        let simplified =
            simplify_expr(ast, HashSet::new(), HashMap::new(), None, None, None, false);
        let expected = Expr::func(
            "sin",
            Expr::product(vec![Expr::number(3.0), Expr::symbol("x")]),
        );
        assert_eq!(simplified, expected);

        // same for cos triple angle
        let ast = parser::parse(
            "4.0000000000000001 * cos(x)^3 - 3.0 * cos(x)",
            &fixed,
            &funcs,
            None,
        )
        .unwrap();
        let simplified2 =
            simplify_expr(ast, HashSet::new(), HashMap::new(), None, None, None, false);
        let expected2 = Expr::func(
            "cos",
            Expr::product(vec![Expr::number(3.0), Expr::symbol("x")]),
        );
        assert_eq!(simplified2, expected2);
    }

    #[test]
    fn test_trig_triple_angle_float_exact() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        let ast = parser::parse("3.0 * sin(x) - 4.0 * sin(x)^3", &fixed, &funcs, None).unwrap();
        let simplified =
            simplify_expr(ast, HashSet::new(), HashMap::new(), None, None, None, false);
        let expected = Expr::func(
            "sin",
            Expr::product(vec![Expr::number(3.0), Expr::symbol("x")]),
        );
        assert_eq!(simplified, expected);
    }

    #[test]
    fn test_hyperbolic_triple_angle_float_tolerance() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        // sinh triple angle small difference
        let ast = parser::parse(
            "4.0000000000000001 * sinh(x)^3 + 3.0 * sinh(x)",
            &fixed,
            &funcs,
            None,
        )
        .unwrap();
        let simplified =
            simplify_expr(ast, HashSet::new(), HashMap::new(), None, None, None, false);
        let expected = Expr::func(
            "sinh",
            Expr::product(vec![Expr::number(3.0), Expr::symbol("x")]),
        );
        assert_eq!(simplified, expected);
    }

    #[test]
    fn test_roots_numeric_more_examples() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        // sqrt(9) -> 3; sqrt(8) should remain symbolic
        let ast = parser::parse("sqrt(9)", &fixed, &funcs, None).unwrap();
        assert_eq!(
            simplify_expr(ast, HashSet::new(), HashMap::new(), None, None, None, false),
            Expr::number(3.0)
        );
        let ast = parser::parse("sqrt(8)", &fixed, &funcs, None).unwrap();
        assert_eq!(
            simplify_expr(
                ast.clone(),
                HashSet::new(),
                HashMap::new(),
                None,
                None,
                None,
                false,
            ),
            ast
        );

        // cbrt(27) -> 3; cbrt(9) stays symbolic
        let ast = parser::parse("cbrt(27)", &fixed, &funcs, None).unwrap();
        assert_eq!(
            simplify_expr(ast, HashSet::new(), HashMap::new(), None, None, None, false),
            Expr::number(3.0)
        );
        let ast = parser::parse("cbrt(9)", &fixed, &funcs, None).unwrap();
        assert_eq!(
            simplify_expr(
                ast.clone(),
                HashSet::new(),
                HashMap::new(),
                None,
                None,
                None,
                false,
            ),
            ast
        );
    }

    #[test]
    fn test_fraction_more_examples() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        // 18 / 3 -> 6
        let ast = parser::parse("18 / 3", &fixed, &funcs, None).unwrap();
        assert_eq!(
            format!(
                "{}",
                simplify_expr(ast, HashSet::new(), HashMap::new(), None, None, None, false)
            ),
            "6"
        );
        // 7 / 2 remains 7/2
        let ast = parser::parse("7 / 2", &fixed, &funcs, None).unwrap();
        let simplified = simplify_expr(
            ast.clone(),
            HashSet::new(),
            HashMap::new(),
            None,
            None,
            None,
            false,
        );
        assert_eq!(simplified, ast);
    }

    #[test]
    fn test_simplification_reduces_display_length() {
        use crate::parser;
        let fixed = std::collections::HashSet::new();
        let funcs = std::collections::HashSet::new();
        // triple-angle reduced
        let s = "3 * sin(x) - 4 * sin(x)^3";
        let ast = parser::parse(s, &fixed, &funcs, None).unwrap();
        let orig = format!("{}", ast);
        let simplified = format!(
            "{}",
            simplify_expr(ast, HashSet::new(), HashMap::new(), None, None, None, false)
        );
        assert!(simplified.len() < orig.len());

        // numeric fraction reduced
        let s = "12 / 3";
        let ast = parser::parse(s, &fixed, &funcs, None).unwrap();
        let orig = format!("{}", ast);
        let simplified = format!(
            "{}",
            simplify_expr(ast, HashSet::new(), HashMap::new(), None, None, None, false)
        );
        println!("orig: {}", orig);
        println!("simplified: {}", simplified);
        assert!(simplified.len() <= orig.len());

        // sqrt numeric example
        let s = "sqrt(9)";
        let ast = parser::parse(s, &fixed, &funcs, None).unwrap();
        let orig = format!("{}", ast);
        let simplified = format!(
            "{}",
            simplify_expr(ast, HashSet::new(), HashMap::new(), None, None, None, false)
        );
        assert!(simplified.len() < orig.len());
    }
}
