#[cfg(test)]
mod tests {
    use crate::{Expr, ExprKind};

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

        // Sum constructor automatically sorts
        let sum = Expr::sum(vec![y.clone(), x2.clone(), x.clone(), y2.clone()]);

        // Expected order: x, x^2, y, y^2
        let terms = get_term_strings(&sum);
        assert_eq!(terms, vec!["x", "x^2", "y", "y^2"]);
    }

    #[test]
    fn test_mixed_coefficient_adjacency() {
        let x = Expr::symbol("x");

        // x, 2x, x^2, 3x^2
        let term1 = x.clone();
        let term2 = Expr::product(vec![Expr::number(2.0), x.clone()]);
        let term3 = Expr::pow(x.clone(), Expr::number(2.0));
        let term4 = Expr::product(vec![
            Expr::number(3.0),
            Expr::pow(x.clone(), Expr::number(2.0)),
        ]);

        let sum = Expr::sum(vec![term4, term2, term1, term3]);

        // Expected: x, 2x, x^2, 3x^2
        // (Base x < Base x^2)
        // Within Base x: x (coeff 1) < 2x (coeff 2)
        // Within Base x^2: x^2 (coeff 1) < 3x^2 (coeff 3)
        let terms = get_term_strings(&sum);
        assert_eq!(terms, vec!["x", "2x", "x^2", "3x^2"]);
    }
}
