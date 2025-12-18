#[cfg(test)]
mod tests {
    use crate::{Expr, simplification::simplify_expr};
    use std::collections::HashSet;

    #[test]
    fn test_perfect_square_factoring() {
        // x^2 + 2x + 1 -> (x + 1)^2
        let expr = Expr::sum(vec![
            Expr::pow(Expr::symbol("x"), Expr::number(2.0)),
            Expr::product(vec![Expr::number(2.0), Expr::symbol("x")]),
            Expr::number(1.0),
        ]);
        let simplified = simplify_expr(expr, HashSet::new());
        println!("Simplified: {:?}", simplified);

        // Expected: (x + 1)^2
        let expected = Expr::pow(
            Expr::sum(vec![Expr::symbol("x"), Expr::number(1.0)]),
            Expr::number(2.0),
        );

        assert_eq!(simplified, expected);
    }
}
