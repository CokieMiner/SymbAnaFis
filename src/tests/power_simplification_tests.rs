#[cfg(test)]
mod tests {
    use crate::simplification::simplify_expr;
    use crate::{Expr, ExprKind};
    use std::collections::HashSet;

    #[test]
    fn test_power_of_power() {
        // (x^2)^2 -> x^4
        let expr = Expr::pow(
            Expr::pow(Expr::symbol("x"), Expr::number(2.0)),
            Expr::number(2.0),
        );
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: x^4
        if let ExprKind::Pow(base, exp) = &simplified.kind {
            if let ExprKind::Symbol(s) = &base.kind {
                assert_eq!(s.as_str(), "x");
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
        let expr = Expr::pow(
            Expr::pow(Expr::symbol("x"), Expr::symbol("a")),
            Expr::symbol("b"),
        );
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: x^(a*b) or x^Product([a, b])
        if let ExprKind::Pow(base, exp) = &simplified.kind {
            if let ExprKind::Symbol(s) = &base.kind {
                assert_eq!(s.as_str(), "x");
            } else {
                panic!("Expected base x");
            }
            // Exponent should be a * b (Product([a, b]))
            if let ExprKind::Product(factors) = &exp.kind {
                let has_a = factors.iter().any(|f| **f == Expr::symbol("a"));
                let has_b = factors.iter().any(|f| **f == Expr::symbol("b"));
                assert!(has_a && has_b, "Expected a*b in exponent");
            } else {
                panic!("Expected Product in exponent");
            }
        } else {
            panic!("Expected power expression");
        }
    }

    #[test]
    fn test_sigma_power_of_power() {
        // (sigma^2)^2 -> sigma^4
        let expr = Expr::pow(
            Expr::pow(Expr::symbol("sigma"), Expr::number(2.0)),
            Expr::number(2.0),
        );
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: sigma^4
        if let ExprKind::Pow(base, exp) = &simplified.kind {
            if let ExprKind::Symbol(s) = &base.kind {
                assert_eq!(s.as_str(), "sigma");
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
