#[cfg(test)]
mod tests {
    use crate::simplification::simplify_expr;
    use crate::{Expr, ExprKind};
    use std::collections::HashSet;

    #[test]
    fn test_ln_power() {
        // ln(x^2) -> 2 * ln(abs(x)) (mathematically correct for all x ≠ 0)
        let expr = Expr::func("ln", Expr::pow(Expr::symbol("x"), Expr::number(2.0)));
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: 2 * ln(abs(x))
        if let ExprKind::Product(factors) = &simplified.kind {
            // Should have 2 factors: 2.0 and ln(abs(x))
            let has_2 = factors.iter().any(|f| **f == Expr::number(2.0));
            assert!(has_2, "Expected factor 2.0");

            let has_ln = factors.iter().any(|f| {
                if let ExprKind::FunctionCall { name, args } = &f.kind
                    && name.as_str() == "ln"
                {
                    // The argument should be abs(x)
                    if let ExprKind::FunctionCall {
                        name: abs_name,
                        args: abs_args,
                    } = &args[0].kind
                    {
                        return abs_name.as_str() == "abs" && *abs_args[0] == Expr::symbol("x");
                    }
                }
                false
            });
            assert!(has_ln, "Expected ln(abs(x))");
        } else {
            panic!("Expected Product, got {:?}", simplified);
        }
    }

    #[test]
    fn test_log10_power_odd() {
        // log10(x^3) -> 3 * log10(x) (odd power, no abs needed but assumes x > 0)
        let expr = Expr::func("log10", Expr::pow(Expr::symbol("x"), Expr::number(3.0)));
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: 3 * log10(x)
        if let ExprKind::Product(factors) = &simplified.kind {
            let has_3 = factors.iter().any(|f| **f == Expr::number(3.0));
            assert!(has_3, "Expected factor 3.0");

            let has_log = factors.iter().any(|f| {
                if let ExprKind::FunctionCall { name, args } = &f.kind {
                    name.as_str() == "log10" && *args[0] == Expr::symbol("x")
                } else {
                    false
                }
            });
            assert!(has_log, "Expected log10(x)");
        } else {
            panic!("Expected Product");
        }
    }

    #[test]
    fn test_log10_power_even() {
        // log10(x^4) -> 4 * log10(abs(x)) (even power, needs abs)
        let expr = Expr::func("log10", Expr::pow(Expr::symbol("x"), Expr::number(4.0)));
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: 4 * log10(abs(x))
        if let ExprKind::Product(factors) = &simplified.kind {
            let has_4 = factors.iter().any(|f| **f == Expr::number(4.0));
            assert!(has_4, "Expected factor 4.0");

            let has_log = factors.iter().any(|f| {
                if let ExprKind::FunctionCall { name, args } = &f.kind
                    && name.as_str() == "log10"
                    && let ExprKind::FunctionCall {
                        name: abs_name,
                        args: abs_args,
                    } = &args[0].kind
                {
                    return abs_name.as_str() == "abs" && *abs_args[0] == Expr::symbol("x");
                }
                false
            });
            assert!(has_log, "Expected log10(abs(x))");
        } else {
            panic!("Expected Product");
        }
    }

    #[test]
    fn test_log2_power() {
        // log2(x^0.5) -> 0.5 * log2(x)
        let expr = Expr::func("log2", Expr::pow(Expr::symbol("x"), Expr::number(0.5)));
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: log2(x) / 2
        // Or 0.5 * log2(x)
        match &simplified.kind {
            ExprKind::Div(num, den) => {
                // Check for log2(x) / 2
                let is_log2 = if let ExprKind::FunctionCall { name, args } = &num.kind {
                    name.as_str() == "log2" && *args[0] == Expr::symbol("x")
                } else {
                    false
                };

                let is_2 = matches!(den.kind, ExprKind::Number(n) if n == 2.0);

                assert!(is_log2 && is_2, "Expected log2(x)/2");
            }
            ExprKind::Product(factors) => {
                // Check for 0.5 * log2(x)
                let has_half = factors.iter().any(|f| **f == Expr::number(0.5));
                let has_log2 = factors.iter().any(|f| {
                    if let ExprKind::FunctionCall { name, args } = &f.kind {
                        name.as_str() == "log2" && *args[0] == Expr::symbol("x")
                    } else {
                        false
                    }
                });
                assert!(has_half && has_log2, "Expected 0.5 * log2(x)");
            }
            _ => panic!("Expected division or product, got {:?}", simplified),
        }
    }
}
