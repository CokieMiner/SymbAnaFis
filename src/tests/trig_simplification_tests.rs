use crate::simplification::simplify_expr;
use crate::{Expr, ExprKind};
use std::collections::HashSet;
use std::sync::Arc;

#[test]
fn test_trig_symmetry_extended() {
    // tan(-x) = -tan(x)
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "tan".to_string(),
        args: vec![Expr::new(ExprKind::Mul(
            Arc::new(Expr::number(-1.0)),
            Arc::new(Expr::symbol("x")),
        ))],
    });
    let simplified = simplify_expr(expr, HashSet::new());
    // Should be -1 * tan(x)
    if let ExprKind::Mul(a, b) = &simplified.kind {
        assert_eq!(**a, Expr::number(-1.0));
        if let ExprKind::FunctionCall { name, args } = &b.kind {
            assert_eq!(name, "tan");
            assert_eq!(args[0], Expr::symbol("x"));
        } else {
            panic!("Expected function call");
        }
    } else {
        panic!("Expected multiplication");
    }

    // sec(-x) = sec(x)
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "sec".to_string(),
        args: vec![Expr::new(ExprKind::Mul(
            Arc::new(Expr::number(-1.0)),
            Arc::new(Expr::symbol("x")),
        ))],
    });
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name, "sec");
        assert_eq!(args[0], Expr::symbol("x"));
    } else {
        panic!("Expected sec(x)");
    }
}

#[test]
fn test_inverse_composition() {
    // sin(asin(x)) = x
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "sin".to_string(),
        args: vec![Expr::new(ExprKind::FunctionCall {
            name: "asin".to_string(),
            args: vec![Expr::symbol("x")],
        })],
    });
    assert_eq!(simplify_expr(expr, HashSet::new()), Expr::symbol("x"));

    // cos(acos(x)) = x
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "cos".to_string(),
        args: vec![Expr::new(ExprKind::FunctionCall {
            name: "acos".to_string(),
            args: vec![Expr::symbol("x")],
        })],
    });
    assert_eq!(simplify_expr(expr, HashSet::new()), Expr::symbol("x"));

    // tan(atan(x)) = x
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "tan".to_string(),
        args: vec![Expr::new(ExprKind::FunctionCall {
            name: "atan".to_string(),
            args: vec![Expr::symbol("x")],
        })],
    });
    assert_eq!(simplify_expr(expr, HashSet::new()), Expr::symbol("x"));
}

#[test]
fn test_inverse_composition_reverse() {
    // asin(sin(x)) = x
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "asin".to_string(),
        args: vec![Expr::new(ExprKind::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::symbol("x")],
        })],
    });
    assert_eq!(simplify_expr(expr, HashSet::new()), Expr::symbol("x"));

    // acos(cos(x)) = x
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "acos".to_string(),
        args: vec![Expr::new(ExprKind::FunctionCall {
            name: "cos".to_string(),
            args: vec![Expr::symbol("x")],
        })],
    });
    assert_eq!(simplify_expr(expr, HashSet::new()), Expr::symbol("x"));
}

#[test]
fn test_pythagorean_identities() {
    // sin^2(x) + cos^2(x) = 1
    let expr = Expr::new(ExprKind::Add(
        Arc::new(Expr::new(ExprKind::Pow(
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "sin".to_string(),
                args: vec![Expr::symbol("x")],
            })),
            Arc::new(Expr::number(2.0)),
        ))),
        Arc::new(Expr::new(ExprKind::Pow(
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "cos".to_string(),
                args: vec![Expr::symbol("x")],
            })),
            Arc::new(Expr::number(2.0)),
        ))),
    ));
    assert_eq!(simplify_expr(expr, HashSet::new()), Expr::number(1.0));

    // 1 + tan^2(x) = sec^2(x)
    let expr = Expr::new(ExprKind::Add(
        Arc::new(Expr::number(1.0)),
        Arc::new(Expr::new(ExprKind::Pow(
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "tan".to_string(),
                args: vec![Expr::symbol("x")],
            })),
            Arc::new(Expr::number(2.0)),
        ))),
    ));
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::Pow(base, exp) = &simplified.kind {
        assert_eq!(**exp, Expr::number(2.0));
        if let ExprKind::FunctionCall { name, args } = &base.kind {
            assert_eq!(name, "sec");
            assert_eq!(args[0], Expr::symbol("x"));
        } else {
            panic!("Expected sec(x)");
        }
    } else {
        panic!("Expected sec^2(x)");
    }

    // 1 + cot^2(x) = csc^2(x)
    let expr = Expr::new(ExprKind::Add(
        Arc::new(Expr::number(1.0)),
        Arc::new(Expr::new(ExprKind::Pow(
            Arc::new(Expr::new(ExprKind::FunctionCall {
                name: "cot".to_string(),
                args: vec![Expr::symbol("x")],
            })),
            Arc::new(Expr::number(2.0)),
        ))),
    ));
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::Pow(base, exp) = &simplified.kind {
        assert_eq!(**exp, Expr::number(2.0));
        if let ExprKind::FunctionCall { name, args } = &base.kind {
            assert_eq!(name, "csc");
            assert_eq!(args[0], Expr::symbol("x"));
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
    // sin(pi/2 - x) = cos(x)
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "sin".to_string(),
        args: vec![Expr::new(ExprKind::Sub(
            Arc::new(Expr::number(PI / 2.0)),
            Arc::new(Expr::symbol("x")),
        ))],
    });
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name, "cos");
        assert_eq!(args[0], Expr::symbol("x"));
    } else {
        panic!("Expected cos(x)");
    }

    // cos(pi/2 - x) = sin(x)
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "cos".to_string(),
        args: vec![Expr::new(ExprKind::Sub(
            Arc::new(Expr::number(PI / 2.0)),
            Arc::new(Expr::symbol("x")),
        ))],
    });
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name, "sin");
        assert_eq!(args[0], Expr::symbol("x"));
    } else {
        panic!("Expected sin(x)");
    }
}

#[test]
fn test_trig_periodicity() {
    use std::f64::consts::PI;
    // sin(x + 2pi) = sin(x)
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "sin".to_string(),
        args: vec![Expr::new(ExprKind::Add(
            Arc::new(Expr::symbol("x")),
            Arc::new(Expr::number(2.0 * PI)),
        ))],
    });
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name, "sin");
        assert_eq!(args[0], Expr::symbol("x"));
    } else {
        panic!("Expected sin(x)");
    }

    // cos(x + 2pi) = cos(x)
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "cos".to_string(),
        args: vec![Expr::new(ExprKind::Add(
            Arc::new(Expr::symbol("x")),
            Arc::new(Expr::number(2.0 * PI)),
        ))],
    });
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name, "cos");
        assert_eq!(args[0], Expr::symbol("x"));
    } else {
        panic!("Expected cos(x)");
    }
}

#[test]
fn test_trig_periodicity_general() {
    use std::f64::consts::PI;
    // sin(x + 4pi) = sin(x)
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "sin".to_string(),
        args: vec![Expr::new(ExprKind::Add(
            Arc::new(Expr::symbol("x")),
            Arc::new(Expr::number(4.0 * PI)),
        ))],
    });
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name, "sin");
        assert_eq!(args[0], Expr::symbol("x"));
    } else {
        panic!("Expected sin(x)");
    }

    // cos(x - 2pi) = cos(x)
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "cos".to_string(),
        args: vec![Expr::new(ExprKind::Add(
            Arc::new(Expr::symbol("x")),
            Arc::new(Expr::number(-2.0 * PI)),
        ))],
    });
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name, "cos");
        assert_eq!(args[0], Expr::symbol("x"));
    } else {
        panic!("Expected cos(x)");
    }
}

#[test]
fn test_trig_reflection_shifts() {
    use std::f64::consts::PI;
    // sin(pi - x) = sin(x)
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "sin".to_string(),
        args: vec![Expr::new(ExprKind::Sub(
            Arc::new(Expr::number(PI)),
            Arc::new(Expr::symbol("x")),
        ))],
    });
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name, "sin");
        assert_eq!(args[0], Expr::symbol("x"));
    } else {
        panic!("Expected sin(x)");
    }

    // cos(pi + x) = -cos(x)
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "cos".to_string(),
        args: vec![Expr::new(ExprKind::Add(
            Arc::new(Expr::number(PI)),
            Arc::new(Expr::symbol("x")),
        ))],
    });
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::Mul(a, b) = &simplified.kind {
        assert_eq!(**a, Expr::number(-1.0));
        if let ExprKind::FunctionCall { name, args } = &b.kind {
            assert_eq!(name, "cos");
            assert_eq!(args[0], Expr::symbol("x"));
        } else {
            panic!("Expected cos(x)");
        }
    } else {
        panic!("Expected -cos(x)");
    }

    // sin(3pi/2 - x) = -cos(x)
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "sin".to_string(),
        args: vec![Expr::new(ExprKind::Sub(
            Arc::new(Expr::number(3.0 * PI / 2.0)),
            Arc::new(Expr::symbol("x")),
        ))],
    });
    let simplified = simplify_expr(expr, HashSet::new());
    if let ExprKind::Mul(a, b) = &simplified.kind {
        assert_eq!(**a, Expr::number(-1.0));
        if let ExprKind::FunctionCall { name, args } = &b.kind {
            assert_eq!(name, "cos");
            assert_eq!(args[0], Expr::symbol("x"));
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
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "sin".to_string(),
        args: vec![Expr::number(PI / 6.0)],
    });
    assert_eq!(simplify_expr(expr, HashSet::new()), Expr::number(0.5));

    // cos(pi/3) = 0.5
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "cos".to_string(),
        args: vec![Expr::number(PI / 3.0)],
    });
    assert_eq!(simplify_expr(expr, HashSet::new()), Expr::number(0.5));

    // tan(pi/4) = 1.0
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "tan".to_string(),
        args: vec![Expr::number(PI / 4.0)],
    });
    assert_eq!(simplify_expr(expr, HashSet::new()), Expr::number(1.0));

    // sin(pi/4) = sqrt(2)/2 approx 0.70710678
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "sin".to_string(),
        args: vec![Expr::number(PI / 4.0)],
    });
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
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "sin".to_string(),
        args: vec![Expr::new(ExprKind::Mul(
            Arc::new(Expr::number(2.0)),
            Arc::new(Expr::symbol("x")),
        ))],
    });
    let simplified = simplify_expr(expr, HashSet::new());
    // Should be 2*sin(x)*cos(x) - the structure is ((2*cos(x))*sin(x))
    if let ExprKind::Mul(a, b) = &simplified.kind {
        // a should be (2*cos(x))
        if let ExprKind::Mul(c, d) = &a.kind {
            assert_eq!(**c, Expr::number(2.0));
            if let ExprKind::FunctionCall { name, args } = &d.kind {
                assert_eq!(name, "cos");
                assert_eq!(args[0], Expr::symbol("x"));
            } else {
                panic!("Expected cos(x)");
            }
        } else {
            panic!("Expected 2*cos(x)");
        }
        // b should be sin(x)
        if let ExprKind::FunctionCall { name, args } = &b.kind {
            assert_eq!(name, "sin");
            assert_eq!(args[0], Expr::symbol("x"));
        } else {
            panic!("Expected sin(x)");
        }
    } else {
        panic!("Expected 2*cos(x)*sin(x), got {:?}", simplified);
    }

    // cos(2x) stays as cos(2x) (no expansion for simplification)
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "cos".to_string(),
        args: vec![Expr::new(ExprKind::Mul(
            Arc::new(Expr::number(2.0)),
            Arc::new(Expr::symbol("x")),
        ))],
    });
    let simplified = simplify_expr(expr, HashSet::new());
    // Should stay as cos(2x)
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name, "cos");
        if let ExprKind::Mul(a, b) = &args[0].kind {
            assert_eq!(**a, Expr::number(2.0));
            assert_eq!(**b, Expr::symbol("x"));
        } else {
            panic!("Expected 2*x");
        }
    } else {
        panic!("Expected cos(2*x), got {:?}", simplified);
    }

    // tan(2x) stays as tan(2x) (no expansion for simplification)
    let expr = Expr::new(ExprKind::FunctionCall {
        name: "tan".to_string(),
        args: vec![Expr::new(ExprKind::Mul(
            Arc::new(Expr::number(2.0)),
            Arc::new(Expr::symbol("x")),
        ))],
    });
    let simplified = simplify_expr(expr, HashSet::new());
    // Should stay as tan(2x)
    if let ExprKind::FunctionCall { name, args } = &simplified.kind {
        assert_eq!(name, "tan");
        if let ExprKind::Mul(a, b) = &args[0].kind {
            assert_eq!(**a, Expr::number(2.0));
            assert_eq!(**b, Expr::symbol("x"));
        } else {
            panic!("Expected 2*x");
        }
    } else {
        panic!("Expected tan(2*x), got {:?}", simplified);
    }
}
