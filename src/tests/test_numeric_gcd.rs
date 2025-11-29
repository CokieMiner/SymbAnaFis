#[cfg(test)]
mod tests {
    use crate::{Expr, simplify};

    #[test]
    fn test_numeric_gcd() {
        // 2/4 -> 1/2
        let expr = Expr::Div(Box::new(Expr::Number(2.0)), Box::new(Expr::Number(4.0)));
        let simplified = simplify(expr);
        if let Expr::Div(num, den) = simplified {
            assert_eq!(num, Box::new(Expr::Number(1.0)));
            assert_eq!(den, Box::new(Expr::Number(2.0)));
        } else {
            panic!("Expected Div, got {:?}", simplified);
        }

        // 10/2 -> 5
        let expr = Expr::Div(Box::new(Expr::Number(10.0)), Box::new(Expr::Number(2.0)));
        let simplified = simplify(expr);
        assert_eq!(simplified, Expr::Number(5.0));

        // 6/9 -> 2/3
        let expr = Expr::Div(Box::new(Expr::Number(6.0)), Box::new(Expr::Number(9.0)));
        let simplified = simplify(expr);
        if let Expr::Div(num, den) = simplified {
            assert_eq!(num, Box::new(Expr::Number(2.0)));
            assert_eq!(den, Box::new(Expr::Number(3.0)));
        } else {
            panic!("Expected Div, got {:?}", simplified);
        }
    }
}
