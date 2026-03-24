#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::items_after_statements,
    reason = "Standard test relaxations"
)]
mod api_tests {
    use crate::Diff;
    use crate::core::symbol::symb;

    #[test]
    fn test_diff_builder_basic() {
        let result = Diff::new().diff_str("x^2", "x", &[]).unwrap();
        assert_eq!(result, "2*x");
    }

    #[test]
    fn test_diff_with_known_symbol() {
        let result = Diff::new().diff_str("alpha*x", "x", &["alpha"]).unwrap();
        assert_eq!(result, "alpha");
    }

    #[test]
    fn test_diff_domain_safe() {
        let result = Diff::new()
            .domain_safe(true)
            .diff_str("x^2", "x", &[])
            .unwrap();
        assert_eq!(result, "2*x");
    }

    #[test]
    fn test_diff_expr() {
        let x = symb("test_diff_x");
        let expr = x.pow(2.0);
        let result = Diff::new().differentiate(&expr, &x).unwrap();
        assert_eq!(format!("{result}"), "2*test_diff_x");
    }

    #[test]
    fn test_custom_eval_with_evaluate() {
        use crate::Expr;
        use std::collections::HashMap;
        use std::sync::Arc;

        let x = symb("test_custom_x");
        let f_of_x = Expr::func("f", x.to_expr());

        let mut vars: HashMap<&str, f64> = HashMap::new();
        vars.insert("test_custom_x", 3.0);

        type CustomEval = Arc<dyn Fn(&[f64]) -> Option<f64> + Send + Sync>;
        let mut custom_evals: HashMap<String, CustomEval> = HashMap::new();
        custom_evals.insert(
            "f".to_owned(),
            Arc::new(|args: &[f64]| Some(args[0].mul_add(args[0], 1.0))),
        );

        let result = f_of_x.evaluate(&vars, &custom_evals);
        assert_eq!(format!("{result}"), "10");
    }

    #[test]
    fn test_custom_eval_without_evaluator() {
        use crate::Expr;
        use std::collections::HashMap;

        let x = symb("test_noeval_x");
        let f_of_x = Expr::func("f", x.to_expr());

        let mut vars: HashMap<&str, f64> = HashMap::new();
        vars.insert("test_noeval_x", 3.0);

        let result = f_of_x.evaluate(&vars, &HashMap::new());
        assert_eq!(format!("{result}"), "f(3)");
    }

    #[test]
    fn test_fixed_var() {
        let a = symb("a");
        let _x = symb("x");

        let result = Diff::new().fixed_var(&a).diff_str("a*x", "x", &[]).unwrap();
        assert_eq!(result, "a");

        let b = symb("b");
        let result2 = Diff::new()
            .fixed_vars(&[&a, &b])
            .diff_str("a*x + b", "x", &[])
            .unwrap();
        assert_eq!(result2, "a");
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::let_underscore_must_use,
    clippy::no_effect_underscore_binding,
    reason = "Standard test relaxations"
)]
mod engine_tests {
    use crate::core::known_symbols as ks;
    use crate::{Expr, core::ExprKind};

    #[test]
    fn test_derive_sinh() {
        let expr = Expr::func_symbol(ks::get_symbol(ks::KS.sinh), Expr::symbol("x"));
        let result = expr.derive("x", None);
        match &result.kind {
            ExprKind::FunctionCall { name, .. } => assert_eq!(name.id(), ks::KS.cosh),
            ExprKind::Product(factors) => {
                let has_cosh = factors.iter().any(
                    |f| matches!(&f.kind, ExprKind::FunctionCall { name, .. } if name.id() == ks::KS.cosh),
                );
                assert!(has_cosh, "Expected cosh in Product, got {factors:?}");
            }
            _ => panic!("Expected cosh or Product with cosh, got {result:?}"),
        }
    }

    #[test]
    fn test_custom_derivative_fallback() {
        let x = Expr::symbol("x");
        let expr = Expr::func("custom", x);
        let result = expr.derive("x", None);
        assert!(
            !matches!(result.kind, ExprKind::Number(_)),
            "Expected non-constant derivative, got: {:?}",
            result.kind
        );
    }

    #[test]
    fn test_derive_subtraction() {
        let expr = Expr::sub_expr(Expr::symbol("x"), Expr::number(1.0));
        let result = expr.derive("x", None);
        assert!(matches!(result.kind, ExprKind::Number(n) if n == 1.0));
    }

    #[test]
    fn test_derive_division() {
        let expr = Expr::div_expr(Expr::symbol("x"), Expr::number(2.0));
        let result = expr.derive("x", None);
        assert!(matches!(result.kind, ExprKind::Div(_, _)));
    }

    #[test]
    fn test_logarithmic_differentiation() {
        let expr = Expr::pow_static(Expr::symbol("x"), Expr::symbol("x"));
        let result = expr.derive("x", None);
        assert!(matches!(result.kind, ExprKind::Product(_)));
    }

    #[test]
    fn test_derivative_order_increment() {
        let inner_func = Expr::func("f", Expr::symbol("x"));
        let derivative_expr = Expr::derivative(inner_func, "x", 1);
        let result = derivative_expr.derive("x", None);
        match &result.kind {
            ExprKind::Derivative { order, var, .. } => {
                assert_eq!(*order, 2);
                assert_eq!(var.as_str(), "x");
            }
            _ => panic!("Expected Derivative, got {result:?}"),
        }
    }

    #[test]
    fn test_derivative_order_increment_multi_digit() {
        let inner_func = Expr::func("f", Expr::symbol("x"));
        let ninth_deriv = Expr::derivative(inner_func.clone(), "x", 9);
        let result = ninth_deriv.derive("x", None);
        match result.kind {
            ExprKind::Derivative { order, .. } => assert_eq!(order, 10),
            _ => panic!("Expected Derivative, got {result:?}"),
        }

        let ninety_ninth_deriv = Expr::derivative(inner_func, "x", 99);
        let result2 = ninety_ninth_deriv.derive("x", None);
        match result2.kind {
            ExprKind::Derivative { order, .. } => assert_eq!(order, 100),
            _ => panic!("Expected Derivative, got {result2:?}"),
        }
    }

    #[test]
    fn test_derivative_variable_not_present_returns_zero() {
        let inner_func = Expr::func("f", Expr::symbol("x"));
        let derivative_expr = Expr::derivative(inner_func, "x", 1);
        let result = derivative_expr.derive("y", None);
        assert_eq!(result.as_number(), Some(0.0));
    }

    #[test]
    fn test_derivative_mixed_partial_when_variable_present() {
        let inner_func = Expr::func_multi("f", vec![Expr::symbol("x"), Expr::symbol("y")]);
        let derivative_expr = Expr::derivative(inner_func, "x", 1);
        let result = derivative_expr.derive("y", None);

        match &result.kind {
            ExprKind::Derivative { var, order, inner } => {
                assert_eq!(var.as_str(), "y");
                assert_eq!(*order, 1);
                match &inner.kind {
                    ExprKind::Derivative {
                        var: inner_var,
                        order: inner_order,
                        ..
                    } => {
                        assert_eq!(inner_var.as_str(), "x");
                        assert_eq!(*inner_order, 1);
                    }
                    _ => panic!("Expected inner Derivative, got {inner:?}"),
                }
            }
            _ => panic!("Expected Derivative for mixed partial, got {result:?}"),
        }
    }

    #[test]
    fn test_derivative_multivar_function() {
        let inner_func = Expr::func_multi("f", vec![Expr::symbol("x"), Expr::symbol("y")]);
        let partial_deriv = Expr::derivative(inner_func, "x", 1);
        let result = partial_deriv.derive("x", None);
        match &result.kind {
            ExprKind::Derivative { order, var, .. } => {
                assert_eq!(*order, 2);
                assert_eq!(var.as_str(), "x");
            }
            _ => panic!("Expected Derivative, got {result:?}"),
        }
    }

    #[test]
    fn test_mixed_partial_display() {
        let f = Expr::func_multi("f", vec![Expr::symbol("x"), Expr::symbol("y")]);
        let df_dx = Expr::derivative(f, "x", 1);
        let d2f_dxdy = df_dx.derive("y", None);
        let display = format!("{d2f_dxdy}");
        assert!(
            display.contains("\u{2202}^"),
            "Display should contain derivative notation, got: {display}"
        );
    }

    #[test]
    fn test_deeply_nested_derivatives() {
        let f_xyz = Expr::func_multi(
            "f",
            vec![Expr::symbol("x"), Expr::symbol("y"), Expr::symbol("z")],
        );
        let deriv_x = Expr::derivative(f_xyz.clone(), "x", 1);
        let deriv_xy = deriv_x.derive("y", None);
        let deriv_xyz = deriv_xy.derive("z", None);

        match &deriv_xyz.kind {
            ExprKind::Derivative { var, .. } => {
                assert_eq!(var.as_str(), "z");
            }
            _ => panic!("Expected Derivative for triple mixed partial, got {deriv_xyz:?}"),
        }

        let deriv_w = f_xyz.derive("w", None);
        assert_eq!(deriv_w.as_number(), Some(0.0));

        let deriv_w_x = deriv_x.derive("w", None);
        assert_eq!(deriv_w_x.as_number(), Some(0.0));
    }

    #[test]
    fn test_derive_erfc() {
        let expr = Expr::func_symbol(ks::get_symbol(ks::KS.erfc), Expr::symbol("x"));
        let result = expr.derive("x", None);
        let s = format!("{result}");
        assert!(s.contains("exp"), "Result should contain exp: {s}");
        assert!(s.contains("pi"), "Result should contain pi: {s}");
        assert!(s.contains("-2"), "Result should contain -2: {s}");
    }

    #[test]
    fn test_empty_product_derivative() {
        use crate::core::expr::ExprKind as AstKind;
        let empty_prod = Expr::new(AstKind::Product(vec![]));
        let result = empty_prod.derive("x", None);
        assert_eq!(result.as_number(), Some(0.0));
    }

    #[test]
    fn test_empty_sum_derivative() {
        use crate::core::expr::ExprKind as AstKind;
        let empty_sum = Expr::new(AstKind::Sum(vec![]));
        let result = empty_sum.derive("x", None);
        assert_eq!(result.as_number(), Some(0.0));
    }

    #[test]
    fn test_power_rule_edge_cases() {
        let zero_pow_two = Expr::pow_static(Expr::number(0.0), Expr::number(2.0));
        let result = zero_pow_two.derive("x", None);
        assert_eq!(result.as_number(), Some(0.0));

        let x_pow_zero = Expr::pow_static(Expr::symbol("x"), Expr::number(0.0));
        let result3 = x_pow_zero.derive("x", None);
        let simplified = result3.to_string();
        assert!(
            simplified.contains('0') || result3.as_number() == Some(0.0),
            "x^0 derivative should simplify to 0, got: {simplified}"
        );
    }
}
