#[cfg(test)]
mod tests {
    use crate::simplification::simplify_expr;
    use crate::{Expr, ExprKind};
    use std::collections::HashSet;
    use std::sync::Arc;
    #[test]
    fn test_nested_fraction_div_div() {
        // (x/y) / (z/a) -> (x*a) / (y*z)
        let expr = Expr::new(ExprKind::Div(
            Arc::new(Expr::new(ExprKind::Div(
                Arc::new(Expr::symbol("x")),
                Arc::new(Expr::symbol("y")),
            ))),
            Arc::new(Expr::new(ExprKind::Div(
                Arc::new(Expr::symbol("z")),
                Arc::new(Expr::symbol("a")),
            ))),
        ));
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: (x*a) / (y*z)
        // Note: ordering of multiplication might vary, so we check structure
        if let ExprKind::Div(num, den) = &simplified.kind {
            // Check numerator: x*a or a*x
            if let ExprKind::Mul(n1, n2) = &num.kind {
                let s1 = format!("{}", n1);
                let s2 = format!("{}", n2);
                assert!((s1 == "x" && s2 == "a") || (s1 == "a" && s2 == "x"));
            } else {
                panic!("Expected numerator to be multiplication");
            }

            // Check denominator: y*z or z*y
            if let ExprKind::Mul(d1, d2) = &den.kind {
                let s1 = format!("{}", d1);
                let s2 = format!("{}", d2);
                assert!((s1 == "y" && s2 == "z") || (s1 == "z" && s2 == "y"));
            } else {
                panic!("Expected denominator to be multiplication");
            }
        } else {
            panic!("Expected division");
        }
    }

    #[test]
    fn test_nested_fraction_val_div() {
        // x / (y/z) -> (x*z) / y
        let expr = Expr::new(ExprKind::Div(
            Arc::new(Expr::symbol("x")),
            Arc::new(Expr::new(ExprKind::Div(
                Arc::new(Expr::symbol("y")),
                Arc::new(Expr::symbol("z")),
            ))),
        ));
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: (x*z) / y
        if let ExprKind::Div(num, den) = &simplified.kind {
            if let ExprKind::Mul(n1, n2) = &num.kind {
                let s1 = format!("{}", n1);
                let s2 = format!("{}", n2);
                assert!((s1 == "x" && s2 == "z") || (s1 == "z" && s2 == "x"));
            } else {
                panic!("Expected numerator to be multiplication");
            }

            if let ExprKind::Symbol(s) = &den.kind {
                assert_eq!(s, "y");
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
        let expr = Expr::new(ExprKind::Div(
            Arc::new(Expr::new(ExprKind::Div(
                Arc::new(Expr::symbol("x")),
                Arc::new(Expr::symbol("y")),
            ))),
            Arc::new(Expr::symbol("z")),
        ));
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: x / (y*z)
        if let ExprKind::Div(num, den) = &simplified.kind {
            if let ExprKind::Symbol(s) = &num.kind {
                assert_eq!(s, "x");
            } else {
                panic!("Expected numerator to be x");
            }

            if let ExprKind::Mul(d1, d2) = &den.kind {
                let s1 = format!("{}", d1);
                let s2 = format!("{}", d2);
                assert!((s1 == "y" && s2 == "z") || (s1 == "z" && s2 == "y"));
            } else {
                panic!("Expected denominator to be multiplication");
            }
        } else {
            panic!("Expected division");
        }
    }

    #[test]
    fn test_nested_fraction_numbers() {
        // (1/2) / (1/3) -> (1*3) / (2*1) -> 3/2 -> 1.5
        let expr = Expr::new(ExprKind::Div(
            Arc::new(Expr::new(ExprKind::Div(
                Arc::new(Expr::number(1.0)),
                Arc::new(Expr::number(2.0)),
            ))),
            Arc::new(Expr::new(ExprKind::Div(
                Arc::new(Expr::number(1.0)),
                Arc::new(Expr::number(3.0)),
            ))),
        ));
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
        let expr = Expr::new(ExprKind::Div(
            Arc::new(Expr::new(ExprKind::Mul(
                Arc::new(Expr::symbol("C")),
                Arc::new(Expr::symbol("R")),
            ))),
            Arc::new(Expr::new(ExprKind::Mul(
                Arc::new(Expr::symbol("C")),
                Arc::new(Expr::new(ExprKind::Pow(
                    Arc::new(Expr::symbol("R")),
                    Arc::new(Expr::number(2.0)),
                ))),
            ))),
        ));
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: 1 / R
        if let ExprKind::Div(num, den) = &simplified.kind {
            assert_eq!(**num, Expr::number(1.0));
            if let ExprKind::Symbol(s) = &den.kind {
                assert_eq!(s, "R");
            } else {
                panic!("Expected denominator R, got {:?}", den);
            }
        } else if let ExprKind::Pow(base, exp) = &simplified.kind {
            // R^-1
            if let ExprKind::Symbol(s) = &base.kind {
                assert_eq!(s, "R");
                assert_eq!(**exp, Expr::number(-1.0));
            } else {
                panic!("Expected R^-1");
            }
        } else {
            panic!("Expected 1/R or R^-1, got {:?}", simplified);
        }
    }
}
