#[cfg(test)]
mod tests {
    use crate::simplification::simplify_expr;
    use crate::{Expr, ExprKind};
    use std::collections::HashSet;
    use std::sync::Arc;

    #[test]
    fn test_ln_power() {
        // ln(x^2) -> 2 * ln(abs(x)) (mathematically correct for all x ≠ 0)
        let expr = Expr::new(ExprKind::FunctionCall {
            name: "ln".to_string(),
            args: vec![Expr::new(ExprKind::Pow(
                Arc::new(Expr::symbol("x")),
                Arc::new(Expr::number(2.0)),
            ))],
        });
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: 2 * ln(abs(x))
        if let ExprKind::Mul(coeff, func) = &simplified.kind {
            assert_eq!(**coeff, Expr::number(2.0));
            if let ExprKind::FunctionCall { name, args } = &func.kind {
                assert_eq!(name, "ln");
                // The argument should be abs(x)
                if let ExprKind::FunctionCall {
                    name: abs_name,
                    args: abs_args,
                } = &args[0].kind
                {
                    assert_eq!(abs_name, "abs");
                    assert_eq!(abs_args[0], Expr::symbol("x"));
                } else {
                    panic!("Expected abs(x), got {:?}", args[0]);
                }
            } else {
                panic!("Expected ln function call");
            }
        } else {
            panic!("Expected multiplication, got {:?}", simplified);
        }
    }

    #[test]
    fn test_log10_power_odd() {
        // log10(x^3) -> 3 * log10(x) (odd power, no abs needed but assumes x > 0)
        let expr = Expr::new(ExprKind::FunctionCall {
            name: "log10".to_string(),
            args: vec![Expr::new(ExprKind::Pow(
                Arc::new(Expr::symbol("x")),
                Arc::new(Expr::number(3.0)),
            ))],
        });
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: 3 * log10(x)
        if let ExprKind::Mul(coeff, func) = &simplified.kind {
            assert_eq!(**coeff, Expr::number(3.0));
            if let ExprKind::FunctionCall { name, args } = &func.kind {
                assert_eq!(name, "log10");
                assert_eq!(args[0], Expr::symbol("x"));
            } else {
                panic!("Expected log10 function call");
            }
        } else {
            panic!("Expected multiplication");
        }
    }

    #[test]
    fn test_log10_power_even() {
        // log10(x^4) -> 4 * log10(abs(x)) (even power, needs abs)
        let expr = Expr::new(ExprKind::FunctionCall {
            name: "log10".to_string(),
            args: vec![Expr::new(ExprKind::Pow(
                Arc::new(Expr::symbol("x")),
                Arc::new(Expr::number(4.0)),
            ))],
        });
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: 4 * log10(abs(x))
        if let ExprKind::Mul(coeff, func) = &simplified.kind {
            assert_eq!(**coeff, Expr::number(4.0));
            if let ExprKind::FunctionCall { name, args } = &func.kind {
                assert_eq!(name, "log10");
                if let ExprKind::FunctionCall {
                    name: abs_name,
                    args: abs_args,
                } = &args[0].kind
                {
                    assert_eq!(abs_name, "abs");
                    assert_eq!(abs_args[0], Expr::symbol("x"));
                } else {
                    panic!("Expected abs(x), got {:?}", args[0]);
                }
            } else {
                panic!("Expected log10 function call");
            }
        } else {
            panic!("Expected multiplication");
        }
    }

    #[test]
    fn test_log2_power() {
        // log2(x^0.5) -> 0.5 * log2(x)
        let expr = Expr::new(ExprKind::FunctionCall {
            name: "log2".to_string(),
            args: vec![Expr::new(ExprKind::Pow(
                Arc::new(Expr::symbol("x")),
                Arc::new(Expr::number(0.5)),
            ))],
        });
        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: log2(x) / 2
        if let ExprKind::Div(num, den) = &simplified.kind {
            if let ExprKind::FunctionCall { name, args } = &num.kind {
                assert_eq!(name, "log2");
                assert_eq!(args[0], Expr::symbol("x"));
            } else {
                panic!("Expected numerator to be log2(x)");
            }

            if let ExprKind::Number(n) = &den.kind {
                assert_eq!(*n, 2.0);
            } else {
                panic!("Expected denominator to be 2");
            }
        } else {
            // It might also be (1/2) * log2(x) if flattening didn't happen,
            // but based on analysis it should be Div.
            // Let's print what we got if it fails
            panic!("Expected division log2(x)/2, got {:?}", simplified);
        }
    }
}
