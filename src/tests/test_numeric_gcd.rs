#[cfg(test)]
mod tests {
    use crate::{Expr, core::ExprKind, simplification::simplify_expr};
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;

    #[test]
    fn test_numeric_gcd() {
        // 2/4 -> 1/2
        let expr = Expr::new(ExprKind::Div(
            Arc::new(Expr::number(2.0)),
            Arc::new(Expr::number(4.0)),
        ));
        let simplified = simplify_expr(
            expr,
            HashSet::new(),
            HashMap::new(),
            None,
            None,
            None,
            false,
        );
        if let ExprKind::Div(num, den) = &simplified.kind {
            assert_eq!(**num, Expr::number(1.0));
            assert_eq!(**den, Expr::number(2.0));
        } else {
            panic!("Expected Div, got {:?}", simplified);
        }

        // 10/2 -> 5
        let expr = Expr::new(ExprKind::Div(
            Arc::new(Expr::number(10.0)),
            Arc::new(Expr::number(2.0)),
        ));
        let simplified = simplify_expr(
            expr,
            HashSet::new(),
            HashMap::new(),
            None,
            None,
            None,
            false,
        );
        assert_eq!(simplified, Expr::number(5.0));

        // 6/9 -> 2/3
        let expr = Expr::new(ExprKind::Div(
            Arc::new(Expr::number(6.0)),
            Arc::new(Expr::number(9.0)),
        ));
        let simplified = simplify_expr(
            expr,
            HashSet::new(),
            HashMap::new(),
            None,
            None,
            None,
            false,
        );
        if let ExprKind::Div(num, den) = &simplified.kind {
            assert_eq!(**num, Expr::number(2.0));
            assert_eq!(**den, Expr::number(3.0));
        } else {
            panic!("Expected Div, got {:?}", simplified);
        }
    }
}
