use crate::simplification::simplify_expr;
use crate::{Expr, ExprKind};
use std::collections::HashSet;
use std::sync::Arc;

#[test]
fn test_simplify_to_sinh() {
    // (exp(x) - exp(-x)) / 2 -> sinh(x)
    let expr = Expr::new(ExprKind::Div(
        Arc::new(Expr::new(ExprKind::Sub(
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "exp".to_string(),
                args: vec![Expr::symbol("x")],
            })),
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "exp".to_string(),
                args: vec![Expr::new(ExprKind::Mul(
                    Arc::new(Expr::number(-1.0)),
                    Arc::new(Expr::symbol("x")),
                ))],
            })),
        ))),
        Arc::new(Expr::number(2.0)),
    ));

    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name, "sinh");
        assert_eq!(args[0], Expr::symbol("x"));
    } else {
        panic!("Expected sinh(x), got {:?}", simplified);
    }
}

#[test]
fn test_simplify_to_cosh() {
    // (exp(x) + exp(-x)) / 2 -> cosh(x)
    let expr = Expr::new(ExprKind::Div(
        Arc::new(Expr::new(ExprKind::Add(
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "exp".to_string(),
                args: vec![Expr::symbol("x")],
            })),
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "exp".to_string(),
                args: vec![Expr::new(ExprKind::Mul(
                    Arc::new(Expr::number(-1.0)),
                    Arc::new(Expr::symbol("x")),
                ))],
            })),
        ))),
        Arc::new(Expr::number(2.0)),
    ));

    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name, "cosh");
        assert_eq!(args[0], Expr::symbol("x"));
    } else {
        panic!("Expected cosh(x), got {:?}", simplified);
    }
}

#[test]
fn test_simplify_to_tanh() {
    // (exp(x) - exp(-x)) / (exp(x) + exp(-x)) -> tanh(x)
    let expr = Expr::new(ExprKind::Div(
        Arc::new(Expr::new(ExprKind::Sub(
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "exp".to_string(),
                args: vec![Expr::symbol("x")],
            })),
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "exp".to_string(),
                args: vec![Expr::new(ExprKind::Mul(
                    Arc::new(Expr::number(-1.0)),
                    Arc::new(Expr::symbol("x")),
                ))],
            })),
        ))),
        Arc::new(Expr::new(ExprKind::Add(
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "exp".to_string(),
                args: vec![Expr::symbol("x")],
            })),
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "exp".to_string(),
                args: vec![Expr::new(ExprKind::Mul(
                    Arc::new(Expr::number(-1.0)),
                    Arc::new(Expr::symbol("x")),
                ))],
            })),
        ))),
    ));

    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name, "tanh");
        assert_eq!(args[0], Expr::symbol("x"));
    } else {
        panic!("Expected tanh(x), got {:?}", simplified);
    }
}

#[test]
fn test_simplify_to_coth() {
    // (exp(x) + exp(-x)) / (exp(x) - exp(-x)) -> coth(x)
    let expr = Expr::new(ExprKind::Div(
        Arc::new(Expr::new(ExprKind::Add(
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "exp".to_string(),
                args: vec![Expr::symbol("x")],
            })),
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "exp".to_string(),
                args: vec![Expr::new(ExprKind::Mul(
                    Arc::new(Expr::number(-1.0)),
                    Arc::new(Expr::symbol("x")),
                ))],
            })),
        ))),
        Arc::new(Expr::new(ExprKind::Sub(
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "exp".to_string(),
                args: vec![Expr::symbol("x")],
            })),
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "exp".to_string(),
                args: vec![Expr::new(ExprKind::Mul(
                    Arc::new(Expr::number(-1.0)),
                    Arc::new(Expr::symbol("x")),
                ))],
            })),
        ))),
    ));

    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name, "coth");
        assert_eq!(args[0], Expr::symbol("x"));
    } else {
        panic!("Expected coth(x), got {:?}", simplified);
    }
}

#[test]
fn test_simplify_to_sech() {
    // 2 / (exp(x) + exp(-x)) -> sech(x)
    let expr = Expr::new(ExprKind::Div(
        Arc::new(Expr::number(2.0)),
        Arc::new(Expr::new(ExprKind::Add(
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "exp".to_string(),
                args: vec![Expr::symbol("x")],
            })),
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "exp".to_string(),
                args: vec![Expr::new(ExprKind::Mul(
                    Arc::new(Expr::number(-1.0)),
                    Arc::new(Expr::symbol("x")),
                ))],
            })),
        ))),
    ));

    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name, "sech");
        assert_eq!(args[0], Expr::symbol("x"));
    } else {
        panic!("Expected sech(x), got {:?}", simplified);
    }
}

#[test]
fn test_simplify_to_csch() {
    // 2 / (exp(x) - exp(-x)) -> csch(x)
    let expr = Expr::new(ExprKind::Div(
        Arc::new(Expr::number(2.0)),
        Arc::new(Expr::new(ExprKind::Sub(
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "exp".to_string(),
                args: vec![Expr::symbol("x")],
            })),
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "exp".to_string(),
                args: vec![Expr::new(ExprKind::Mul(
                    Arc::new(Expr::number(-1.0)),
                    Arc::new(Expr::symbol("x")),
                ))],
            })),
        ))),
    ));

    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name, "csch");
        assert_eq!(args[0], Expr::symbol("x"));
    } else {
        panic!("Expected csch(x)");
    }
}

#[test]
fn test_hyperbolic_identities() {
    // sinh(-x) = -sinh(x)
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "sinh".to_string(),
        args: vec![Expr::new(ExprKind::Mul(
            Arc::new(Expr::number(-1.0)),
            Arc::new(Expr::symbol("x")),
        ))],
    });
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::Mul(a, b) = &simplified.kind {
        assert_eq!(**a, Expr::number(-1.0));
        if let ExprKind::FunctionCall { name, args } = &b.kind {
            assert_eq!(name, "sinh");
            assert_eq!(args[0], Expr::symbol("x"));
        } else {
            panic!("Expected sinh(x)");
        }
    } else {
        panic!("Expected -sinh(x)");
    }

    // cosh(-x) = cosh(x)
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "cosh".to_string(),
        args: vec![Expr::new(ExprKind::Mul(
            Arc::new(Expr::number(-1.0)),
            Arc::new(Expr::symbol("x")),
        ))],
    });
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name, "cosh");
        assert_eq!(args[0], Expr::symbol("x"));
    } else {
        panic!("Expected cosh(x)");
    }

    // cosh^2(x) - sinh^2(x) = 1
    let expr = Expr::new(ExprKind::Sub(
        Arc::new(Expr::new(ExprKind::Pow(
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "cosh".to_string(),
                args: vec![Expr::symbol("x")],
            })),
            Arc::new(Expr::number(2.0)),
        ))),
        Arc::new(Expr::new(ExprKind::Pow(
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "sinh".to_string(),
                args: vec![Expr::symbol("x")],
            })),
            Arc::new(Expr::number(2.0)),
        ))),
    ));
    assert_eq!(simplify_expr(expr, HashSet::new()), Expr::number(1.0));

    // 1 - tanh^2(x) = sech^2(x)
    let expr = Expr::new(ExprKind::Sub(
        Arc::new(Expr::number(1.0)),
        Arc::new(Expr::new(ExprKind::Pow(
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "tanh".to_string(),
                args: vec![Expr::symbol("x")],
            })),
            Arc::new(Expr::number(2.0)),
        ))),
    ));
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::Pow(base, exp) = &simplified.kind {
        assert_eq!(**exp, Expr::number(2.0));
        if let ExprKind::FunctionCall { name, args } = &base.kind {
            assert_eq!(name, "sech");
            assert_eq!(args[0], Expr::symbol("x"));
        } else {
            panic!("Expected sech(x)");
        }
    } else {
        panic!("Expected sech^2(x)");
    }

    // coth^2(x) - 1 = csch^2(x)
    let expr = Expr::new(ExprKind::Sub(
        Arc::new(Expr::new(ExprKind::Pow(
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "coth".to_string(),
                args: vec![Expr::symbol("x")],
            })),
            Arc::new(Expr::number(2.0)),
        ))),
        Arc::new(Expr::number(1.0)),
    ));
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::Pow(base, exp) = &simplified.kind {
        assert_eq!(**exp, Expr::number(2.0));
        if let ExprKind::FunctionCall { name, args } = &base.kind {
            assert_eq!(name, "csch");
            assert_eq!(args[0], Expr::symbol("x"));
        } else {
            panic!("Expected csch(x)");
        }
    } else {
        panic!("Expected csch^2(x)");
    }
}
