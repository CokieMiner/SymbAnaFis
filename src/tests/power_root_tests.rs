#[cfg(test)]
mod tests {
    use crate::simplification::simplify_expr;
    use crate::{Expr, ExprKind};
    use std::collections::HashSet;

    #[test]
    fn test_power_collection_mul() {
        // x^2 * y^2 -> (x*y)^2
        let expr = Expr::product(vec![
            Expr::pow(Expr::symbol("x"), Expr::number(2.0)),
            Expr::pow(Expr::symbol("y"), Expr::number(2.0)),
        ]);
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: (x*y)^2
        if let ExprKind::Pow(base, exp) = &simplified.kind {
            if let ExprKind::Number(n) = &exp.kind {
                assert_eq!(*n, 2.0);
            } else {
                panic!("Expected exponent 2.0");
            }

            if let ExprKind::Product(factors) = &base.kind {
                let has_x = factors.iter().any(|f| **f == Expr::symbol("x"));
                let has_y = factors.iter().any(|f| **f == Expr::symbol("y"));
                assert!(has_x && has_y);
            } else {
                panic!("Expected base to be Product");
            }
        } else {
            panic!("Expected power expression");
        }
    }

    #[test]
    fn test_power_collection_div() {
        // x^2 / y^2 -> (x/y)^2
        let expr = Expr::div_expr(
            Expr::pow(Expr::symbol("x"), Expr::number(2.0)),
            Expr::pow(Expr::symbol("y"), Expr::number(2.0)),
        );
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: (x/y)^2
        if let ExprKind::Pow(base, exp) = &simplified.kind {
            if let ExprKind::Number(n) = &exp.kind {
                assert_eq!(*n, 2.0);
            } else {
                panic!("Expected exponent 2.0");
            }

            if let ExprKind::Div(num, den) = &base.kind {
                if let ExprKind::Symbol(s) = &num.kind {
                    assert_eq!(s.as_str(), "x");
                } else {
                    panic!("Expected numerator x");
                }
                if let ExprKind::Symbol(s) = &den.kind {
                    assert_eq!(s.as_str(), "y");
                } else {
                    panic!("Expected denominator y");
                }
            } else {
                panic!("Expected base to be division");
            }
        } else {
            panic!("Expected power expression");
        }
    }

    #[test]
    fn test_root_conversion_sqrt() {
        // x^(1/2) -> sqrt(x)
        let expr = Expr::pow(
            Expr::symbol("x"),
            Expr::div_expr(Expr::number(1.0), Expr::number(2.0)),
        );
        let simplified = simplify_expr(expr, HashSet::new());

        if let ExprKind::FunctionCall { name, args } = &simplified.kind {
            assert_eq!(name.as_str(), "sqrt");
            assert_eq!(args.len(), 1);
            if let ExprKind::Symbol(s) = &args[0].kind {
                assert_eq!(s.as_str(), "x");
            } else {
                panic!("Expected argument x");
            }
        } else {
            panic!("Expected sqrt function call");
        }
    }

    #[test]
    fn test_root_conversion_sqrt_decimal() {
        // x^0.5 -> sqrt(x)
        let expr = Expr::pow(Expr::symbol("x"), Expr::number(0.5));
        let simplified = simplify_expr(expr, HashSet::new());

        if let ExprKind::FunctionCall { name, args } = &simplified.kind {
            assert_eq!(name.as_str(), "sqrt");
            assert_eq!(args.len(), 1);
            if let ExprKind::Symbol(s) = &args[0].kind {
                assert_eq!(s.as_str(), "x");
            } else {
                panic!("Expected argument x");
            }
        } else {
            panic!("Expected sqrt function call");
        }
    }

    #[test]
    fn test_root_conversion_cbrt() {
        // x^(1/3) -> cbrt(x)
        let expr = Expr::pow(
            Expr::symbol("x"),
            Expr::div_expr(Expr::number(1.0), Expr::number(3.0)),
        );
        let simplified = simplify_expr(expr, HashSet::new());

        if let ExprKind::FunctionCall { name, args } = &simplified.kind {
            assert_eq!(name.as_str(), "cbrt");
            assert_eq!(args.len(), 1);
            if let ExprKind::Symbol(s) = &args[0].kind {
                assert_eq!(s.as_str(), "x");
            } else {
                panic!("Expected argument x");
            }
        } else {
            panic!("Expected cbrt function call");
        }
    }
}
