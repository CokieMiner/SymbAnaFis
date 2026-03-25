//! Tests for foundational `core/` modules: poly, error, traits, and visitor.

#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::let_underscore_must_use,
    clippy::no_effect_underscore_binding,
    clippy::float_cmp,
    reason = "Standard test relaxations"
)]
mod poly_tests {
    use super::super::known_symbols as ks;
    use super::super::poly::*;
    use crate::Expr;
    use std::sync::Arc;

    #[test]
    fn test_poly_constant() {
        let p = Polynomial::constant(5.0);
        assert!(p.is_constant());
        assert_eq!(p.as_constant(), Some(5.0));
        assert_eq!(p.degree(), 0);
    }

    #[test]
    fn test_poly_from_base() {
        let x = Expr::symbol("x");
        let p = Polynomial::from_base(x);
        assert!(!p.is_constant());
        assert_eq!(p.degree(), 1);
        assert_eq!(p.coeff(1), 1.0);
    }

    #[test]
    fn test_poly_add() {
        let x = Arc::new(Expr::symbol("x"));
        let p1 = Polynomial::term(Arc::clone(&x), 2, 3.0); // 3x²
        let p2 = Polynomial::term(Arc::clone(&x), 1, 2.0); // 2x
        let sum = p1.add(&p2);

        assert_eq!(sum.term_count(), 2);
        assert_eq!(sum.coeff(2), 3.0);
        assert_eq!(sum.coeff(1), 2.0);
    }

    #[test]
    fn test_poly_mul() {
        // (x + 1) * (x + 1) = x² + 2x + 1
        let x = Arc::new(Expr::symbol("x"));
        let mut p = Polynomial::term(Arc::clone(&x), 1, 1.0);
        p.add_term(0, 1.0); // x + 1

        let result = p.mul(&p);
        assert_eq!(result.degree(), 2);
        assert_eq!(result.coeff(0), 1.0);
        assert_eq!(result.coeff(1), 2.0);
        assert_eq!(result.coeff(2), 1.0);
    }

    #[test]
    fn test_poly_derivative() {
        // d/dx(3x² + 2x + 1) = 6x + 2
        let x = Arc::new(Expr::symbol("x"));
        let mut p = Polynomial::zero(Arc::clone(&x));
        p.add_term(2, 3.0);
        p.add_term(1, 2.0);
        p.add_term(0, 1.0);

        let deriv = p.derivative();
        assert_eq!(deriv.degree(), 1);
        assert_eq!(deriv.coeff(1), 6.0);
        assert_eq!(deriv.coeff(0), 2.0);
    }

    #[test]
    fn test_poly_div_rem() {
        // (x² - 1) / (x - 1) = (x + 1), remainder 0
        let x = Arc::new(Expr::symbol("x"));

        let mut p1 = Polynomial::zero(Arc::clone(&x));
        p1.add_term(2, 1.0);
        p1.add_term(0, -1.0); // x² - 1

        let mut p2 = Polynomial::zero(Arc::clone(&x));
        p2.add_term(1, 1.0);
        p2.add_term(0, -1.0); // x - 1

        let (q, r) = p1.div_rem(&p2).unwrap();
        assert_eq!(q.degree(), 1);
        assert!(r.is_zero() || r.is_constant());
    }

    #[test]
    fn test_poly_gcd() {
        // GCD of (x² - 1) and (x - 1) should be (x - 1)
        let x = Arc::new(Expr::symbol("x"));

        let mut p1 = Polynomial::zero(Arc::clone(&x));
        p1.add_term(2, 1.0);
        p1.add_term(0, -1.0);

        let mut p2 = Polynomial::zero(Arc::clone(&x));
        p2.add_term(1, 1.0);
        p2.add_term(0, -1.0);

        let gcd = p1.gcd(&p2).unwrap();
        assert_eq!(gcd.degree(), 1);
    }

    #[test]
    fn test_poly_with_function_base() {
        // Polynomial in sin(x)
        let sin_x = Expr::func_symbol(ks::get_symbol(ks::KS.sin), Expr::symbol("x"));
        let base = Arc::new(sin_x);

        let mut p = Polynomial::zero(Arc::clone(&base));
        p.add_term(2, 1.0);
        p.add_term(0, -1.0); // sin(x)² - 1

        assert_eq!(p.degree(), 2);
        assert_eq!(p.to_string(), "-1 + sin(x)^2");
    }

    #[test]
    fn test_poly_roundtrip() {
        let expr = Expr::sum(vec![
            Expr::pow_static(Expr::symbol("x"), Expr::number(2.0)),
            Expr::product(vec![Expr::number(2.0), Expr::symbol("x")]),
            Expr::number(1.0),
        ]);

        let poly = Polynomial::try_from_expr(&expr);
        assert!(poly.is_some());

        let p = poly.unwrap();
        assert_eq!(p.degree(), 2);
    }

    #[test]
    fn test_poly_display() {
        let x = Arc::new(Expr::symbol("x"));
        let mut p = Polynomial::zero(x);
        p.add_term(2, 3.0);
        p.add_term(1, -2.0);
        p.add_term(0, 1.0);

        let s = p.to_string();
        assert!(s.contains('3') && s.contains('x'));
    }
}

#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::let_underscore_must_use,
    clippy::no_effect_underscore_binding,
    reason = "Standard test relaxations"
)]
mod error_tests {
    use super::super::error::*;

    #[test]
    fn test_span_creation() {
        let span = Span::new(5, 10);
        assert_eq!(span.start(), 5);
        assert_eq!(span.end(), 10);
        assert!(span.is_valid());
    }

    #[test]
    fn test_span_swap() {
        let span = Span::new(10, 5);
        assert_eq!(span.start(), 5);
        assert_eq!(span.end(), 10);
        assert!(span.is_valid());
    }

    #[test]
    fn test_span_at() {
        let span = Span::at(7);
        assert_eq!(span.start(), 7);
        assert_eq!(span.end(), 8);
        assert!(span.is_valid());
    }

    #[test]
    fn test_span_empty() {
        let span = Span::empty();
        assert_eq!(span.start(), 0);
        assert_eq!(span.end(), 0);
        assert!(!span.is_valid());
    }

    #[test]
    fn test_span_display() {
        let span1 = Span::new(4, 8);
        assert_eq!(span1.display(), " at positions 5-8");

        let span2 = Span::at(9);
        assert_eq!(span2.display(), " at position 10");

        let span3 = Span::empty();
        assert_eq!(span3.display(), "");
    }

    #[test]
    fn test_diff_error_display() {
        let err1 = DiffError::EmptyFormula;
        assert_eq!(format!("{err1}"), "Formula cannot be empty");

        let err2 = DiffError::invalid_syntax("test message");
        assert_eq!(format!("{err2}"), "Invalid syntax: test message");

        let err3 = DiffError::invalid_syntax_at("spanned message", Span::new(1, 3));
        assert_eq!(
            format!("{err3}"),
            "Invalid syntax: spanned message at positions 2-3"
        );

        let err4 = DiffError::MaxDepthExceeded;
        assert_eq!(
            format!("{err4}"),
            "Expression nesting depth exceeds maximum limit"
        );
    }

    #[test]
    fn test_diff_error_constructors() {
        let err5 = DiffError::invalid_syntax("msg");
        match err5 {
            DiffError::InvalidSyntax { msg, span: None } => assert_eq!(msg, "msg"),
            _ => panic!("Wrong error type"),
        }

        let err6 = DiffError::invalid_syntax_at("msg", Span::at(5));
        match err6 {
            DiffError::InvalidSyntax {
                msg,
                span: Some(span),
            } => {
                assert_eq!(msg, "msg");
                assert_eq!(span.start(), 5);
            }
            _ => panic!("Wrong error type"),
        }
    }
}

#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::let_underscore_must_use,
    clippy::no_effect_underscore_binding,
    reason = "Standard test relaxations"
)]
mod traits_tests {
    use super::super::traits::*;

    #[test]
    fn test_is_zero() {
        assert!(is_zero(0.0));
        assert!(is_zero(1e-15));
        assert!(is_zero(-1e-15));
        assert!(!is_zero(0.1));
        assert!(!is_zero(-0.1));
    }

    #[test]
    fn test_is_one() {
        assert!(is_one(1.0));
        assert!(is_one(1.0 + 1e-15));
        assert!(is_one(1.0 - 1e-15));
        assert!(!is_one(1.1));
        assert!(!is_one(0.9));
    }

    #[test]
    fn test_is_neg_one() {
        assert!(is_neg_one(-1.0));
        assert!(is_neg_one(-1.0 + 1e-15));
        assert!(!is_neg_one(1.0));
    }
}

#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::let_underscore_must_use,
    clippy::no_effect_underscore_binding,
    reason = "Standard test relaxations"
)]
mod visitor_tests {
    use super::super::visitor::*;
    use crate::Expr;
    use crate::symb;

    #[test]
    fn test_node_counter() {
        let x = symb("x");
        let expr = x + x.pow(2.0); // x + x^2 = Poly with base x
        let mut counter = NodeCounter::default();
        walk_expr(&expr, &mut counter);
        // With optimized Poly: only walks the base expression (x = 1 symbol node)
        // Note: This test is dependent on Poly optimization. If Poly is disabled, this count will change.
        assert_eq!(counter.count, 1);
    }

    #[test]
    fn test_variable_collector() {
        let x = symb("x");
        let y = symb("y");
        let expr = x + y;
        let mut collector = VariableCollector::default();
        walk_expr(&expr, &mut collector);
        // Use variable_names() for string-based checking
        let names = collector.variable_names();
        assert!(names.contains("x"));
        assert!(names.contains("y"));
        assert_eq!(collector.variables.len(), 2);
    }

    #[test]
    fn test_expr_view_number() {
        let expr = Expr::number(42.0);
        let view = expr.view();
        assert!(view.is_number());
        assert_eq!(view.as_number(), Some(42.0));
    }

    #[test]
    fn test_expr_view_symbol() {
        let x: Expr = symb("view_test_x").into();
        let view = x.view();
        assert!(view.is_symbol());
        assert_eq!(view.as_symbol(), Some("view_test_x"));
    }

    #[test]
    fn test_expr_view_sum() {
        let x: Expr = symb("view_sum_x").into();
        let expr = x + 1.0;
        let view = expr.view();
        assert!(view.is_sum());
        if let ExprView::Sum(terms) = view {
            assert_eq!(terms.len(), 2);
        } else {
            panic!("Expected Sum view");
        }
    }

    #[test]
    fn test_expr_view_poly_as_sum() {
        // Create a polynomial expression (x^2 + 2x + 1)
        let x: Expr = symb("poly_view_x").into();
        let expr = x.clone().pow(2.0) + 2.0 * x + 1.0;

        // View should present it as a Sum, even if internally stored as Poly
        let view = expr.view();
        assert!(view.is_sum(), "Poly should be viewed as Sum");

        if let ExprView::Sum(terms) = view {
            assert!(!terms.is_empty(), "Should have at least one term");
        } else {
            panic!("Expected Sum view for polynomial");
        }
    }

    #[test]
    fn test_expr_view_div() {
        let x: Expr = symb("div_view_x").into();
        let expr = x / 2.0;
        if let ExprView::Div(num, den) = expr.view() {
            assert_eq!(num.to_string(), "div_view_x");
            assert_eq!(den.to_string(), "2");
        } else {
            panic!("Expected Div view");
        }
    }

    #[test]
    fn test_expr_view_function() {
        let x: Expr = symb("func_view_x").into();
        let expr = x.sin();
        if let ExprView::Function { name, args } = expr.view() {
            assert_eq!(name, "sin");
            assert_eq!(args.len(), 1);
        } else {
            panic!("Expected Function view");
        }
    }

    #[test]
    fn test_expr_view_pattern_matching() {
        let x: Expr = symb("pattern_x").into();
        let y: Expr = symb("pattern_y").into();

        let exprs = vec![
            (Expr::number(42.0), "number"),
            (x.clone(), "symbol"),
            (x.clone() + y.clone(), "sum"),
            (x.clone() * y.clone(), "product"),
            (x.clone() / y, "div"),
            (x.clone().pow(2.0), "pow"),
            (x.sin(), "function"),
        ];

        for (expr, expected) in exprs {
            let kind = match expr.view() {
                ExprView::Number(_) => "number",
                ExprView::Symbol(_) => "symbol",
                ExprView::Sum(_) => "sum",
                ExprView::Product(_) => "product",
                ExprView::Div(_, _) => "div",
                ExprView::Pow(_, _) => "pow",
                ExprView::Function { .. } => "function",
                ExprView::Derivative { .. } => "derivative",
            };
            assert_eq!(kind, expected, "Failed for expression: {expr}");
        }
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "Expression tree too deep")]
    fn test_depth_limit() {
        // Create a deeply nested expression that exceeds MAX_DEPTH
        let x = symb("x");
        let mut expr = x.sin(); // Start with Expr
        for _ in 0..1200 {
            // Exceed the 1000 limit
            expr = expr.sin();
        }

        let mut counter = NodeCounter::default();
        // This should panic in debug builds to prevent stack overflow
        walk_expr(&expr, &mut counter);
    }
}
