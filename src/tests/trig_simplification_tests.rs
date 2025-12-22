use crate::simplification::simplify_expr;
use crate::{Expr, ExprKind};
use std::collections::HashSet;

#[test]
fn test_trig_symmetry_extended() {
    // tan(-x) = -tan(x)
    let expr = Expr::func(
        "tan",
        Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
    );
    let simplified = simplify_expr(expr, HashSet::new());
    // Should be -1 * tan(x) or Product([-1, tan(x)])
    if let ExprKind::Product(factors) = &simplified.kind {
        assert!(factors.len() == 2);
        assert!(matches!(&factors[0].kind, ExprKind::Number(n) if *n == -1.0));
        if let ExprKind::FunctionCall { name, args } = &factors[1].kind {
            assert_eq!(name.as_str(), "tan");
            assert_eq!(*args[0], Expr::symbol("x"));
        } else {
            panic!("Expected function call");
        }
    } else {
        panic!("Expected Product, got {:?}", simplified);
    }

    // sec(-x) = sec(x)
    let expr = Expr::func(
        "sec",
        Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
    );
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name.as_str(), "sec");
        assert_eq!(*args[0], Expr::symbol("x"));
    } else {
        panic!("Expected sec(x)");
    }
}

#[test]
fn test_inverse_composition() {
    // sin(asin(x)) = x
    let expr = Expr::func("sin", Expr::func("asin", Expr::symbol("x")));
    assert_eq!(simplify_expr(expr, HashSet::new()), Expr::symbol("x"));

    // cos(acos(x)) = x
    let expr = Expr::func("cos", Expr::func("acos", Expr::symbol("x")));
    assert_eq!(simplify_expr(expr, HashSet::new()), Expr::symbol("x"));

    // tan(atan(x)) = x
    let expr = Expr::func("tan", Expr::func("atan", Expr::symbol("x")));
    assert_eq!(simplify_expr(expr, HashSet::new()), Expr::symbol("x"));
}

#[test]
fn test_inverse_composition_reverse() {
    // asin(sin(x)) = x
    let expr = Expr::func("asin", Expr::func("sin", Expr::symbol("x")));
    assert_eq!(simplify_expr(expr, HashSet::new()), Expr::symbol("x"));

    // acos(cos(x)) = x
    let expr = Expr::func("acos", Expr::func("cos", Expr::symbol("x")));
    assert_eq!(simplify_expr(expr, HashSet::new()), Expr::symbol("x"));
}

#[test]
fn test_pythagorean_identities() {
    // sin^2(x) + cos^2(x) = 1
    let expr = Expr::sum(vec![
        Expr::pow(Expr::func("sin", Expr::symbol("x")), Expr::number(2.0)),
        Expr::pow(Expr::func("cos", Expr::symbol("x")), Expr::number(2.0)),
    ]);
    assert_eq!(simplify_expr(expr, HashSet::new()), Expr::number(1.0));

    // 1 + tan^2(x) = sec^2(x)
    let expr = Expr::sum(vec![
        Expr::number(1.0),
        Expr::pow(Expr::func("tan", Expr::symbol("x")), Expr::number(2.0)),
    ]);
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::Pow(base, exp) = &simplified.kind {
        assert_eq!(**exp, Expr::number(2.0));
        if let ExprKind::FunctionCall { name, args } = &base.kind {
            assert_eq!(name.as_str(), "sec");
            assert_eq!(*args[0], Expr::symbol("x"));
        } else {
            panic!("Expected sec(x)");
        }
    } else {
        panic!("Expected sec^2(x)");
    }

    // 1 + cot^2(x) = csc^2(x)
    let expr = Expr::sum(vec![
        Expr::number(1.0),
        Expr::pow(Expr::func("cot", Expr::symbol("x")), Expr::number(2.0)),
    ]);
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::Pow(base, exp) = &simplified.kind {
        assert_eq!(**exp, Expr::number(2.0));
        if let ExprKind::FunctionCall { name, args } = &base.kind {
            assert_eq!(name.as_str(), "csc");
            assert_eq!(*args[0], Expr::symbol("x"));
        } else {
            panic!("Expected csc(x)");
        }
    } else {
        panic!("Expected csc^2(x)");
    }
}

#[test]
fn test_cofunction_identities() {
    use std::f64::consts::PI;
    // sin(pi/2 - x) = cos(x) represented as Sum([pi/2, Product([-1, x])])
    let expr = Expr::func(
        "sin",
        Expr::sum(vec![
            Expr::number(PI / 2.0),
            Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
        ]),
    );
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name.as_str(), "cos");
        assert_eq!(*args[0], Expr::symbol("x"));
    } else {
        panic!("Expected cos(x)");
    }

    // cos(pi/2 - x) = sin(x)
    let expr = Expr::func(
        "cos",
        Expr::sum(vec![
            Expr::number(PI / 2.0),
            Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
        ]),
    );
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name.as_str(), "sin");
        assert_eq!(*args[0], Expr::symbol("x"));
    } else {
        panic!("Expected sin(x)");
    }
}

#[test]
fn test_trig_periodicity() {
    use std::f64::consts::PI;
    // sin(x + 2pi) = sin(x)
    let expr = Expr::func(
        "sin",
        Expr::sum(vec![Expr::symbol("x"), Expr::number(2.0 * PI)]),
    );
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name.as_str(), "sin");
        assert_eq!(*args[0], Expr::symbol("x"));
    } else {
        panic!("Expected sin(x)");
    }

    // cos(x + 2pi) = cos(x)
    let expr = Expr::func(
        "cos",
        Expr::sum(vec![Expr::symbol("x"), Expr::number(2.0 * PI)]),
    );
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name.as_str(), "cos");
        assert_eq!(*args[0], Expr::symbol("x"));
    } else {
        panic!("Expected cos(x)");
    }
}

#[test]
fn test_trig_periodicity_general() {
    use std::f64::consts::PI;
    // sin(x + 4pi) = sin(x)
    let expr = Expr::func(
        "sin",
        Expr::sum(vec![Expr::symbol("x"), Expr::number(4.0 * PI)]),
    );
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name.as_str(), "sin");
        assert_eq!(*args[0], Expr::symbol("x"));
    } else {
        panic!("Expected sin(x)");
    }

    // cos(x - 2pi) = cos(x) represented as Sum([x, -2pi])
    let expr = Expr::func(
        "cos",
        Expr::sum(vec![Expr::symbol("x"), Expr::number(-2.0 * PI)]),
    );
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name.as_str(), "cos");
        assert_eq!(*args[0], Expr::symbol("x"));
    } else {
        panic!("Expected cos(x)");
    }
}

#[test]
fn test_trig_reflection_shifts() {
    use std::f64::consts::PI;
    // sin(pi - x) = sin(x) represented as Sum([pi, Product([-1, x])])
    let expr = Expr::func(
        "sin",
        Expr::sum(vec![
            Expr::number(PI),
            Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
        ]),
    );
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name.as_str(), "sin");
        assert_eq!(*args[0], Expr::symbol("x"));
    } else {
        panic!("Expected sin(x)");
    }

    // cos(pi + x) = -cos(x)
    let expr = Expr::func("cos", Expr::sum(vec![Expr::number(PI), Expr::symbol("x")]));
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::Product(factors) = &simplified.kind {
        assert!(factors.len() == 2);
        assert!(matches!(&factors[0].kind, ExprKind::Number(n) if *n == -1.0));
        if let ExprKind::FunctionCall { name, args } = &factors[1].kind {
            assert_eq!(name.as_str(), "cos");
            assert_eq!(*args[0], Expr::symbol("x"));
        } else {
            panic!("Expected cos(x)");
        }
    } else {
        panic!("Expected -cos(x)");
    }

    // sin(3pi/2 - x) = -cos(x) represented as Sum([3pi/2, Product([-1, x])])
    let expr = Expr::func(
        "sin",
        Expr::sum(vec![
            Expr::number(3.0 * PI / 2.0),
            Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
        ]),
    );
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::Product(factors) = &simplified.kind {
        assert!(factors.len() == 2);
        assert!(matches!(&factors[0].kind, ExprKind::Number(n) if *n == -1.0));
        if let ExprKind::FunctionCall { name, args } = &factors[1].kind {
            assert_eq!(name.as_str(), "cos");
            assert_eq!(*args[0], Expr::symbol("x"));
        } else {
            panic!("Expected cos(x)");
        }
    } else {
        panic!("Expected -cos(x)");
    }
}

#[test]
fn test_trig_exact_values_extended() {
    use std::f64::consts::PI;

    // sin(pi/6) = 0.5
    let expr = Expr::func("sin", Expr::number(PI / 6.0));
    assert_eq!(simplify_expr(expr, HashSet::new()), Expr::number(0.5));

    // cos(pi/3) = 0.5
    let expr = Expr::func("cos", Expr::number(PI / 3.0));
    assert_eq!(simplify_expr(expr, HashSet::new()), Expr::number(0.5));

    // tan(pi/4) = 1.0
    let expr = Expr::func("tan", Expr::number(PI / 4.0));
    assert_eq!(simplify_expr(expr, HashSet::new()), Expr::number(1.0));

    // sin(pi/4) = sqrt(2)/2 approx 0.70710678
    let expr = Expr::func("sin", Expr::number(PI / 4.0));
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::Number(n) = simplified.kind {
        assert!((n - (2.0f64.sqrt() / 2.0)).abs() < 1e-10);
    } else {
        panic!("Expected number");
    }
}

#[test]
fn test_double_angle_formulas() {
    // sin(2x) = 2*sin(x)*cos(x)
    let expr = Expr::func(
        "sin",
        Expr::product(vec![Expr::number(2.0), Expr::symbol("x")]),
    );
    let simplified = simplify_expr(expr, HashSet::new());
    // Should be Product([2, cos(x), sin(x)]) or similar
    if let ExprKind::Product(factors) = &simplified.kind {
        let has_2 = factors
            .iter()
            .any(|f| matches!(&f.kind, ExprKind::Number(n) if *n == 2.0));
        let has_sin = factors.iter().any(|f| {
            if let ExprKind::FunctionCall { name, args } = &f.kind {
                name.as_str() == "sin" && *args[0] == Expr::symbol("x")
            } else {
                false
            }
        });
        let has_cos = factors.iter().any(|f| {
            if let ExprKind::FunctionCall { name, args } = &f.kind {
                name.as_str() == "cos" && *args[0] == Expr::symbol("x")
            } else {
                false
            }
        });
        assert!(
            has_2 && has_sin && has_cos,
            "Expected 2*sin(x)*cos(x), got {:?}",
            simplified
        );
    } else {
        panic!("Expected Product, got {:?}", simplified);
    }

    // cos(2x) stays as cos(2x) (no expansion for simplification)
    let expr = Expr::func(
        "cos",
        Expr::product(vec![Expr::number(2.0), Expr::symbol("x")]),
    );
    let simplified = simplify_expr(expr, HashSet::new());
    // Should stay as cos(2*x) or cos(Product([2, x]))
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name.as_str(), "cos");
        if let ExprKind::Product(factors) = &args[0].kind {
            let has_2 = factors
                .iter()
                .any(|f| matches!(&f.kind, ExprKind::Number(n) if *n == 2.0));
            let has_x = factors
                .iter()
                .any(|f| matches!(&f.kind, ExprKind::Symbol(s) if s.as_str() == "x"));
            assert!(has_2 && has_x);
        } else {
            panic!("Expected 2*x");
        }
    } else {
        panic!("Expected cos(2*x), got {:?}", simplified);
    }

    // tan(2x) stays as tan(2x) (no expansion for simplification)
    let expr = Expr::func(
        "tan",
        Expr::product(vec![Expr::number(2.0), Expr::symbol("x")]),
    );
    let simplified = simplify_expr(expr, HashSet::new());
    // Should stay as tan(2*x)
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name.as_str(), "tan");
        if let ExprKind::Product(factors) = &args[0].kind {
            let has_2 = factors
                .iter()
                .any(|f| matches!(&f.kind, ExprKind::Number(n) if *n == 2.0));
            let has_x = factors
                .iter()
                .any(|f| matches!(&f.kind, ExprKind::Symbol(s) if s.as_str() == "x"));
            assert!(has_2 && has_x);
        } else {
            panic!("Expected 2*x");
        }
    } else {
        panic!("Expected tan(2*x), got {:?}", simplified);
    }
}
