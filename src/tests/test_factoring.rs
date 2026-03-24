#[cfg(test)]
mod tests {
    use crate::{Expr, core::ExprKind, simplification::simplify_expr};
    use std::collections::{HashMap, HashSet};

    #[test]
    fn test_perfect_square_factoring() {
        // x^2 + 2x + 1 -> (x + 1)^2
        let expr = Expr::sum(vec![
            Expr::pow(Expr::symbol("x"), Expr::number(2.0)),
            Expr::product(vec![Expr::number(2.0), Expr::symbol("x")]),
            Expr::number(1.0),
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
        println!("Simplified: {:?}", simplified);

        // Must be factored to (x + 1)^2
        if let ExprKind::Pow(base, exp) = &simplified.kind {
            assert_eq!(**exp, Expr::number(2.0), "Expected exponent 2");
            // Check base contains x and 1
            let base_str = base.to_string();
            assert!(
                base_str.contains("x") && base_str.contains("1"),
                "Expected base (x + 1), got: {}",
                base_str
            );
        } else {
            panic!("Expected factored form (x+1)^2, but got: {:?}", simplified);
        }
    }
}
