#[cfg(test)]
mod tests {
    use crate::simplification::simplify_expr;
    use crate::{Expr, ExprKind};
    use std::collections::HashSet;

    // Helper to get string representation of terms
    fn get_term_strings(e: &Expr) -> Vec<String> {
        if let ExprKind::Sum(terms) = &e.kind {
            terms.iter().map(|t| t.to_string()).collect()
        } else {
            vec![e.to_string()]
        }
    }

    #[test]
    fn test_polynomial_term_adjacency() {
        let x = Expr::symbol("x");
        let y = Expr::symbol("y");

        // Create mixed terms: x, y, x^2, y^2
        let x2 = Expr::pow(x.clone(), Expr::number(2.0));
        let y2 = Expr::pow(y.clone(), Expr::number(2.0));

        // Sum constructor flattens; simplify() sorts for canonical form
        let sum = simplify_expr(
            Expr::sum(vec![y.clone(), x2.clone(), x.clone(), y2.clone()]),
            HashSet::new(),
        );

        // Expected order after simplify: x, x^2, y, y^2
        let terms = get_term_strings(&sum);
        assert_eq!(terms, vec!["x", "x^2", "y", "y^2"]);
    }

    #[test]
    fn test_mixed_coefficient_adjacency() {
        let x = Expr::symbol("x");

        // x, 2x, x^2, 3x^2 - these now get combined in simplify!
        let term1 = x.clone();
        let term2 = Expr::product(vec![Expr::number(2.0), x.clone()]);
        let term3 = Expr::pow(x.clone(), Expr::number(2.0));
        let term4 = Expr::product(vec![
            Expr::number(3.0),
            Expr::pow(x.clone(), Expr::number(2.0)),
        ]);

        let sum = simplify_expr(Expr::sum(vec![term4, term2, term1, term3]), HashSet::new());

        // Accept either combined form or factored form - both mathematically equivalent
        // 3x + 4x^2 or x*(3 + 4x)
        let result_str = sum.to_string();
        assert!(
            result_str.contains("3x") && result_str.contains("4x") || result_str.contains("x*("),
            "Expected combined or factored form, got: {}",
            result_str
        );
    }
}
