#[cfg(test)]
mod tests {
    use crate::simplification::simplify_expr;
    use crate::{Expr, ExprKind};
    use std::collections::HashSet;

    #[test]
    fn test_nested_fraction_div_div() {
        // (x/y) / (z/a) -> (x*a) / (y*z)
        let expr = Expr::div_expr(
            Expr::div_expr(Expr::symbol("x"), Expr::symbol("y")),
            Expr::div_expr(Expr::symbol("z"), Expr::symbol("a")),
        );
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: (x*a) / (y*z) or Product([x, a]) / Product([y, z])
        if let ExprKind::Div(num, den) = &simplified.kind {
            // Check numerator contains x and a
            let num_str = format!("{}", num);
            assert!(
                num_str.contains("x") && num_str.contains("a"),
                "Expected numerator with x and a, got {}",
                num_str
            );

            // Check denominator contains y and z
            let den_str = format!("{}", den);
            assert!(
                den_str.contains("y") && den_str.contains("z"),
                "Expected denominator with y and z, got {}",
                den_str
            );
        } else {
            panic!("Expected division");
        }
    }

    #[test]
    fn test_nested_fraction_val_div() {
        // x / (y/z) -> (x*z) / y
        let expr = Expr::div_expr(
            Expr::symbol("x"),
            Expr::div_expr(Expr::symbol("y"), Expr::symbol("z")),
        );
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: (x*z) / y
        if let ExprKind::Div(num, den) = &simplified.kind {
            let num_str = format!("{}", num);
            assert!(
                num_str.contains("x") && num_str.contains("z"),
                "Expected numerator with x and z, got {}",
                num_str
            );

            if let ExprKind::Symbol(s) = &den.kind {
                assert_eq!(s.as_str(), "y");
            } else {
                panic!("Expected denominator y");
            }
        } else {
            panic!("Expected division");
        }
    }

    #[test]
    fn test_nested_fraction_div_val() {
        // (x/y) / z -> x / (y*z)
        let expr = Expr::div_expr(
            Expr::div_expr(Expr::symbol("x"), Expr::symbol("y")),
            Expr::symbol("z"),
        );
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: x / (y*z)
        if let ExprKind::Div(num, den) = &simplified.kind {
            if let ExprKind::Symbol(s) = &num.kind {
                assert_eq!(s.as_str(), "x");
            } else {
                panic!("Expected numerator to be x");
            }

            let den_str = format!("{}", den);
            assert!(
                den_str.contains("y") && den_str.contains("z"),
                "Expected denominator with y and z, got {}",
                den_str
            );
        } else {
            panic!("Expected division");
        }
    }

    #[test]
    fn test_nested_fraction_numbers() {
        // (1/2) / (1/3) -> (1*3) / (2*1) -> 3/2
        let expr = Expr::div_expr(
            Expr::div_expr(Expr::number(1.0), Expr::number(2.0)),
            Expr::div_expr(Expr::number(1.0), Expr::number(3.0)),
        );
        let simplified = simplify_expr(expr, HashSet::new());

        if let ExprKind::Div(num, den) = &simplified.kind {
            if let (ExprKind::Number(n), ExprKind::Number(d)) = (&num.kind, &den.kind) {
                assert_eq!(*n, 3.0);
                assert_eq!(*d, 2.0);
            } else {
                panic!("Expected numerator and denominator to be numbers");
            }
        } else {
            panic!("Expected division 3/2, got {:?}", simplified);
        }
    }

    #[test]
    fn test_fraction_cancellation_products() {
        // (C * R) / (C * R^2) -> 1 / R
        let expr = Expr::div_expr(
            Expr::product(vec![Expr::symbol("C"), Expr::symbol("R")]),
            Expr::product(vec![
                Expr::symbol("C"),
                Expr::pow(Expr::symbol("R"), Expr::number(2.0)),
            ]),
        );
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: 1 / R
        if let ExprKind::Div(num, den) = &simplified.kind {
            assert_eq!(**num, Expr::number(1.0));
            if let ExprKind::Symbol(s) = &den.kind {
                assert_eq!(s.as_str(), "R");
            } else {
                panic!("Expected denominator R, got {:?}", den);
            }
        } else if let ExprKind::Pow(base, exp) = &simplified.kind {
            // R^-1
            if let ExprKind::Symbol(s) = &base.kind {
                assert_eq!(s.as_str(), "R");
                assert_eq!(**exp, Expr::number(-1.0));
            } else {
                panic!("Expected R^-1");
            }
        } else {
            panic!("Expected 1/R or R^-1, got {:?}", simplified);
        }
    }
}
