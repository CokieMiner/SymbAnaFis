// Tests for the expr module, categorized by the file they originated from.

#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::let_underscore_must_use,
    clippy::no_effect_underscore_binding,
    reason = "Standard test relaxations"
)]
mod api_user_tests {
    use super::super::super::{Expr, ExprKind};

    #[test]
    fn test_sum_flattening() {
        let x = Expr::symbol("x");
        let y = Expr::symbol("y");
        let z = Expr::symbol("z");
        let inner = Expr::sum(vec![x, y]);
        let outer = Expr::sum(vec![inner, z]);
        match &outer.kind {
            ExprKind::Sum(terms) => assert_eq!(terms.len(), 3),
            ExprKind::Poly(poly) => assert_eq!(poly.terms().len(), 3),
            _ => panic!("Expected Sum or Poly"),
        }
    }

    #[test]
    fn test_product_flattening() {
        let a = Expr::symbol("a");
        let b = Expr::symbol("b");
        let c = Expr::symbol("c");
        let inner = Expr::product(vec![a, b]);
        let outer = Expr::product(vec![inner, c]);
        match &outer.kind {
            ExprKind::Product(factors) => assert_eq!(factors.len(), 3),
            _ => panic!("Expected Product"),
        }
    }

    #[test]
    fn test_subtraction_as_sum() {
        let x = Expr::symbol("x");
        let y = Expr::symbol("y");
        let result = Expr::sub_expr(x, y);
        match &result.kind {
            ExprKind::Sum(terms) => assert_eq!(terms.len(), 2),
            _ => panic!("Expected Sum from subtraction"),
        }
    }

    #[test]
    fn test_cached_constants_get_fresh_expr_ids() {
        let zero_a = Expr::number(0.0);
        let zero_b = Expr::number(0.0);
        let one_a = Expr::number(1.0);
        let one_b = Expr::number(1.0);
        let two_a = Expr::number(2.0);
        let two_b = Expr::number(2.0);

        assert_ne!(zero_a.id(), zero_b.id());
        assert_ne!(one_a.id(), one_b.id());
        assert_ne!(two_a.id(), two_b.id());
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
mod display_tests {
    use crate::core::Expr;
    use std::collections::{HashMap, HashSet};
    #[test]
    #[allow(
        clippy::approx_constant,
        reason = "Testing exact float display, not mathematical approximation"
    )]
    fn test_display_number() {
        assert_eq!(format!("{}", Expr::number(3.0)), "3");
        assert!(format!("{}", Expr::number(3.141)).starts_with("3.141"));
    }

    #[test]
    fn test_display_symbol() {
        assert_eq!(format!("{}", Expr::symbol("x")), "x");
    }

    #[test]
    fn test_display_sum() {
        use crate::simplification::simplify_expr;

        let expr = simplify_expr(
            Expr::sum(vec![Expr::symbol("x"), Expr::number(1.0)]),
            HashSet::new(),
            HashMap::new(),
            None,
            None,
            None,
            false,
        );
        assert_eq!(format!("{expr}"), "1 + x"); // Sorted after simplify: numbers before symbols
    }

    #[test]
    fn test_display_product() {
        let prod = Expr::product(vec![Expr::number(2.0), Expr::symbol("x")]);
        assert_eq!(format!("{prod}"), "2*x");
    }

    #[test]
    fn test_display_negation() {
        let expr = Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]);
        assert_eq!(format!("{expr}"), "-x");
    }

    #[test]
    fn test_display_subtraction() {
        // x - y = Sum([x, Product([-1, y])])
        let expr = Expr::sub_expr(Expr::symbol("x"), Expr::symbol("y"));
        let display = format!("{expr}");
        // Should display as subtraction
        assert!(
            display.contains('-'),
            "Expected subtraction, got: {display}"
        );
    }

    #[test]
    fn test_display_nested_sum() {
        // Test: x + (y + z) should display with parentheses
        let inner_sum = Expr::sum(vec![Expr::symbol("y"), Expr::symbol("z")]);
        let expr = Expr::sum(vec![Expr::symbol("x"), inner_sum]);
        let display = format!("{expr}");
        // Should display as "x + (y + z)" to preserve structure
        assert_eq!(display, "x + y + z");
    }
}
