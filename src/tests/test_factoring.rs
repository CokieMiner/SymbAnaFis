#[cfg(test)]
mod tests {
    use crate::{Expr, simplification::simplify};

    #[test]
    fn test_perfect_square_factoring() {
        // x^2 + 2x + 1 -> (x + 1)^2
        let expr = Expr::Add(
            Box::new(Expr::Add(
                Box::new(Expr::Pow(
                    Box::new(Expr::Symbol("x".to_string())),
                    Box::new(Expr::Number(2.0)),
                )),
                Box::new(Expr::Mul(
                    Box::new(Expr::Number(2.0)),
                    Box::new(Expr::Symbol("x".to_string())),
                )),
            )),
            Box::new(Expr::Number(1.0)),
        );
        let simplified = simplify(expr);
        println!("Simplified: {:?}", simplified);

        // Expected: (x + 1)^2
        let expected = Expr::Pow(
            Box::new(Expr::Add(
                Box::new(Expr::Number(1.0)),
                Box::new(Expr::Symbol("x".to_string())),
            )),
            Box::new(Expr::Number(2.0)),
        );

        assert_eq!(simplified, expected);
    }
}
