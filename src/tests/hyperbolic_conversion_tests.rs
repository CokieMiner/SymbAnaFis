use crate::simplification::simplify_expr;
use crate::{Expr, ExprKind};
use std::collections::HashSet;

#[test]
fn test_simplify_to_sinh() {
    // (exp(x) - exp(-x)) / 2 -> sinh(x)
    // Represented as Sum([exp(x), Product([-1, exp(-x)])]) / 2
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
fn test_simplify_to_cosh() {
    // (exp(x) + exp(-x)) / 2 -> cosh(x)
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
fn test_simplify_to_tanh() {
    // (exp(x) - exp(-x)) / (exp(x) + exp(-x)) -> tanh(x)
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
        Expr::func("exp", Expr::symbol("x")),
        Expr::func(
            "exp",
            Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
        ),
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
fn test_simplify_to_coth() {
    // (exp(x) + exp(-x)) / (exp(x) - exp(-x)) -> coth(x)
    let numerator = Expr::sum(vec![
        Expr::func("exp", Expr::symbol("x")),
        Expr::func(
            "exp",
            Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
        ),
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
fn test_simplify_to_sech() {
    // 2 / (exp(x) + exp(-x)) -> sech(x)
    let expr = Expr::div_expr(
        Expr::number(2.0),
        Expr::sum(vec![
            Expr::func("exp", Expr::symbol("x")),
            Expr::func(
                "exp",
                Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
            ),
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
fn test_simplify_to_csch() {
    // 2 / (exp(x) - exp(-x)) -> csch(x)
    let expr = Expr::div_expr(
        Expr::number(2.0),
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
    );

    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name.as_str(), "csch");
        assert_eq!(*args[0], Expr::symbol("x"));
    } else {
        panic!("Expected csch(x), got {:?}", simplified);
    }
}

#[test]
fn test_hyperbolic_identities() {
    // sinh(-x) = -sinh(x)
    let expr = Expr::func(
        "sinh",
        Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
    );
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::Product(factors) = &simplified.kind {
        assert!(factors.len() == 2);
        assert!(matches!(&factors[0].kind, ExprKind::Number(n) if *n == -1.0));
        if let ExprKind::FunctionCall { name, args } = &factors[1].kind {
            assert_eq!(name.as_str(), "sinh");
            assert_eq!(*args[0], Expr::symbol("x"));
        } else {
            panic!("Expected sinh(x)");
        }
    } else {
        panic!("Expected -sinh(x)");
    }

    // cosh(-x) = cosh(x)
    let expr = Expr::func(
        "cosh",
        Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
    );
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name.as_str(), "cosh");
        assert_eq!(*args[0], Expr::symbol("x"));
    } else {
        panic!("Expected cosh(x)");
    }

    // cosh^2(x) - sinh^2(x) = 1 represented as Sum([cosh^2(x), Product([-1, sinh^2(x)])])
    let expr = Expr::sum(vec![
        Expr::pow(Expr::func("cosh", Expr::symbol("x")), Expr::number(2.0)),
        Expr::product(vec![
            Expr::number(-1.0),
            Expr::pow(Expr::func("sinh", Expr::symbol("x")), Expr::number(2.0)),
        ]),
    ]);
    assert_eq!(simplify_expr(expr, HashSet::new()), Expr::number(1.0));

    // 1 - tanh^2(x) = sech^2(x) represented as Sum([1, Product([-1, tanh^2(x)])])
    let expr = Expr::sum(vec![
        Expr::number(1.0),
        Expr::product(vec![
            Expr::number(-1.0),
            Expr::pow(Expr::func("tanh", Expr::symbol("x")), Expr::number(2.0)),
        ]),
    ]);
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::Pow(base, exp) = &simplified.kind {
        assert_eq!(**exp, Expr::number(2.0));
        if let ExprKind::FunctionCall { name, args } = &base.kind {
            assert_eq!(name.as_str(), "sech");
            assert_eq!(*args[0], Expr::symbol("x"));
        } else {
            panic!("Expected sech(x)");
        }
    } else {
        panic!("Expected sech^2(x)");
    }

    // coth^2(x) - 1 = csch^2(x) represented as Sum([coth^2(x), -1])
    let expr = Expr::sum(vec![
        Expr::pow(Expr::func("coth", Expr::symbol("x")), Expr::number(2.0)),
        Expr::number(-1.0),
    ]);
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::Pow(base, exp) = &simplified.kind {
        assert_eq!(**exp, Expr::number(2.0));
        if let ExprKind::FunctionCall { name, args } = &base.kind {
            assert_eq!(name.as_str(), "csch");
            assert_eq!(*args[0], Expr::symbol("x"));
        } else {
            panic!("Expected csch(x)");
        }
    } else {
        panic!("Expected csch^2(x), got {:?}", simplified);
    }
}
