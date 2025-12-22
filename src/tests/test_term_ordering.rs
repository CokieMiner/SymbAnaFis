// Test to verify that hyperbolic conversion patterns handle different term orderings
use crate::simplification::simplify_expr;
use crate::{Expr, ExprKind};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_cosh_reversed_order() {
        // (exp(-x) + exp(x)) / 2 -> cosh(x)
        // Testing REVERSED order: e^(-x) first, e^x second
        let expr = Expr::div_expr(
            Expr::sum(vec![
                Expr::func(
                    "exp",
                    Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
                ),
                Expr::func("exp", Expr::symbol("x")),
            ]),
            Expr::number(2.0),
        );

        let simplified = simplify_expr(expr, HashSet::new());
        if let ExprKind::FunctionCall { name, args } = &simplified.kind {
            assert_eq!(name.as_str(), "cosh");
            assert_eq!(*args[0], Expr::symbol("x"));
        } else {
            panic!("Expected cosh(x), got {:?}", simplified);
        }
    }

    #[test]
    fn test_cosh_normal_order() {
        // (exp(x) + exp(-x)) / 2 -> cosh(x)
        // Testing NORMAL order: e^x first, e^(-x) second
        let expr = Expr::div_expr(
            Expr::sum(vec![
                Expr::func("exp", Expr::symbol("x")),
                Expr::func(
                    "exp",
                    Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
                ),
            ]),
            Expr::number(2.0),
        );

        let simplified = simplify_expr(expr, HashSet::new());
        if let ExprKind::FunctionCall { name, args } = &simplified.kind {
            assert_eq!(name.as_str(), "cosh");
            assert_eq!(*args[0], Expr::symbol("x"));
        } else {
            panic!("Expected cosh(x), got {:?}", simplified);
        }
    }

    #[test]
    fn test_coth_reversed_numerator() {
        // (exp(-x) + exp(x)) / (exp(x) - exp(-x)) -> coth(x)
        // Testing REVERSED order in numerator
        let numerator = Expr::sum(vec![
            Expr::func(
                "exp",
                Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
            ),
            Expr::func("exp", Expr::symbol("x")),
        ]);
        let denominator = Expr::sum(vec![
            Expr::func("exp", Expr::symbol("x")),
            Expr::product(vec![
                Expr::number(-1.0),
                Expr::func(
                    "exp",
                    Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
                ),
            ]),
        ]);
        let expr = Expr::div_expr(numerator, denominator);

        let simplified = simplify_expr(expr, HashSet::new());
        if let ExprKind::FunctionCall { name, args } = &simplified.kind {
            assert_eq!(name.as_str(), "coth");
            assert_eq!(*args[0], Expr::symbol("x"));
        } else {
            panic!("Expected coth(x), got {:?}", simplified);
        }
    }

    #[test]
    fn test_tanh_reversed_denominator() {
        // (exp(x) - exp(-x)) / (exp(-x) + exp(x)) -> tanh(x)
        // Testing REVERSED order in denominator
        let numerator = Expr::sum(vec![
            Expr::func("exp", Expr::symbol("x")),
            Expr::product(vec![
                Expr::number(-1.0),
                Expr::func(
                    "exp",
                    Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
                ),
            ]),
        ]);
        let denominator = Expr::sum(vec![
            Expr::func(
                "exp",
                Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
            ),
            Expr::func("exp", Expr::symbol("x")),
        ]);
        let expr = Expr::div_expr(numerator, denominator);

        let simplified = simplify_expr(expr, HashSet::new());
        if let ExprKind::FunctionCall { name, args } = &simplified.kind {
            assert_eq!(name.as_str(), "tanh");
            assert_eq!(*args[0], Expr::symbol("x"));
        } else {
            panic!("Expected tanh(x), got {:?}", simplified);
        }
    }

    #[test]
    fn test_sech_reversed_denominator() {
        // 2 / (exp(-x) + exp(x)) -> sech(x)
        // Testing REVERSED order in denominator
        let expr = Expr::div_expr(
            Expr::number(2.0),
            Expr::sum(vec![
                Expr::func(
                    "exp",
                    Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
                ),
                Expr::func("exp", Expr::symbol("x")),
            ]),
        );

        let simplified = simplify_expr(expr, HashSet::new());
        if let ExprKind::FunctionCall { name, args } = &simplified.kind {
            assert_eq!(name.as_str(), "sech");
            assert_eq!(*args[0], Expr::symbol("x"));
        } else {
            panic!("Expected sech(x), got {:?}", simplified);
        }
    }

    #[test]
    fn test_sinh_reversed_numerator() {
        // (exp(x) + (-1)*exp(-x)) / 2 -> sinh(x)
        let expr = Expr::div_expr(
            Expr::sum(vec![
                Expr::func("exp", Expr::symbol("x")),
                Expr::product(vec![
                    Expr::number(-1.0),
                    Expr::func(
                        "exp",
                        Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
                    ),
                ]),
            ]),
            Expr::number(2.0),
        );

        let simplified = simplify_expr(expr, HashSet::new());
        if let ExprKind::FunctionCall { name, args } = &simplified.kind {
            assert_eq!(name.as_str(), "sinh");
            assert_eq!(*args[0], Expr::symbol("x"));
        } else {
            panic!("Expected sinh(x), got {:?}", simplified);
        }
    }

    #[test]
    fn test_sinh_reversed_add_pattern() {
        // (-1*exp(-x)) + exp(x) -> sinh pattern
        // Testing REVERSED order in Add pattern with negation first
        let expr = Expr::div_expr(
            Expr::sum(vec![
                Expr::product(vec![
                    Expr::number(-1.0),
                    Expr::func(
                        "exp",
                        Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
                    ),
                ]),
                Expr::func("exp", Expr::symbol("x")),
            ]),
            Expr::number(2.0),
        );

        let simplified = simplify_expr(expr, HashSet::new());
        if let ExprKind::FunctionCall { name, args } = &simplified.kind {
            assert_eq!(name.as_str(), "sinh");
            assert_eq!(*args[0], Expr::symbol("x"));
        } else {
            panic!("Expected sinh(x), got {:?}", simplified);
        }
    }
}
