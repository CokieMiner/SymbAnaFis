#[cfg(test)]
mod tests {
    use crate::simplification::simplify_expr;
    use crate::{Expr, ExprKind};
    use std::collections::HashSet;
    use std::sync::Arc;

    #[test]
    fn test_power_of_power() {
        // (x^2)^2 -> x^4
        let expr = Expr::new(ExprKind::Pow(
            Arc::new(Expr::new(ExprKind::Pow(
                Arc::new(Expr::symbol("x")),
                Arc::new(Expr::number(2.0)),
            ))),
            Arc::new(Expr::number(2.0)),
        ));
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: x^4
        if let ExprKind::Pow(base, exp) = &simplified.kind {
            if let ExprKind::Symbol(s) = &base.kind {
                assert_eq!(s, "x");
            } else {
                panic!("Expected base x");
            }
            if let ExprKind::Number(n) = &exp.kind {
                assert_eq!(*n, 4.0);
            } else {
                panic!("Expected exponent 4.0");
            }
        } else {
            panic!("Expected power expression, got {:?}", simplified);
        }
    }

    #[test]
    fn test_power_of_power_symbolic() {
        // (x^a)^b -> x^(a*b)
        let expr = Expr::new(ExprKind::Pow(
            Arc::new(Expr::new(ExprKind::Pow(
                Arc::new(Expr::symbol("x")),
                Arc::new(Expr::symbol("a")),
            ))),
            Arc::new(Expr::symbol("b")),
        ));
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: x^(a*b)
        if let ExprKind::Pow(base, exp) = &simplified.kind {
            if let ExprKind::Symbol(s) = &base.kind {
                assert_eq!(s, "x");
            } else {
                panic!("Expected base x");
            }
            // Exponent should be a * b (or b * a depending on sorting)
            if let ExprKind::Mul(lhs, rhs) = &exp.kind {
                // Check for a*b or b*a
                let s1 = format!("{}", lhs);
                let s2 = format!("{}", rhs);
                assert!((s1 == "a" && s2 == "b") || (s1 == "b" && s2 == "a"));
            } else {
                panic!("Expected multiplication in exponent");
            }
        } else {
            panic!("Expected power expression");
        }
    }

    #[test]
    fn test_sigma_power_of_power() {
        // (sigma^2)^2 -> sigma^4
        let expr = Expr::new(ExprKind::Pow(
            Arc::new(Expr::new(ExprKind::Pow(
                Arc::new(Expr::symbol("sigma")),
                Arc::new(Expr::number(2.0)),
            ))),
            Arc::new(Expr::number(2.0)),
        ));
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: sigma^4
        if let ExprKind::Pow(base, exp) = &simplified.kind {
            if let ExprKind::Symbol(s) = &base.kind {
                assert_eq!(s, "sigma");
            } else {
                panic!("Expected base sigma");
            }
            if let ExprKind::Number(n) = &exp.kind {
                assert_eq!(*n, 4.0);
            } else {
                panic!("Expected exponent 4.0");
            }
        } else {
            panic!("Expected power expression");
        }
    }
}
