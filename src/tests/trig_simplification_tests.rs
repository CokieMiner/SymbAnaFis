use crate::Expr;
use crate::simplification::simplify;

#[test]
fn test_trig_symmetry_extended() {
    // tan(-x) = -tan(x)
    let expr = Expr::FunctionCall {
        name: "tan".to_string(),
        args: vec![Expr::Mul(
            Box::new(Expr::Number(-1.0)),
            Box::new(Expr::Symbol("x".to_string())),
        )],
    };
    let simplified = simplify(expr);
    // Should be -1 * tan(x)
    if let Expr::Mul(a, b) = simplified {
        assert_eq!(*a, Expr::Number(-1.0));
        if let Expr::FunctionCall { name, args } = *b {
            assert_eq!(name, "tan");
            assert_eq!(args[0], Expr::Symbol("x".to_string()));
        } else {
            panic!("Expected function call");
        }
    } else {
        panic!("Expected multiplication");
    }

    // sec(-x) = sec(x)
    let expr = Expr::FunctionCall {
        name: "sec".to_string(),
        args: vec![Expr::Mul(
            Box::new(Expr::Number(-1.0)),
            Box::new(Expr::Symbol("x".to_string())),
        )],
    };
    let simplified = simplify(expr);
    if let Expr::FunctionCall { name, args } = simplified {
        assert_eq!(name, "sec");
        assert_eq!(args[0], Expr::Symbol("x".to_string()));
    } else {
        panic!("Expected sec(x)");
    }
}

#[test]
fn test_inverse_composition() {
    // sin(asin(x)) = x
    let expr = Expr::FunctionCall {
        name: "sin".to_string(),
        args: vec![Expr::FunctionCall {
            name: "asin".to_string(),
            args: vec![Expr::Symbol("x".to_string())],
        }],
    };
    assert_eq!(simplify(expr), Expr::Symbol("x".to_string()));

    // cos(acos(x)) = x
    let expr = Expr::FunctionCall {
        name: "cos".to_string(),
        args: vec![Expr::FunctionCall {
            name: "acos".to_string(),
            args: vec![Expr::Symbol("x".to_string())],
        }],
    };
    assert_eq!(simplify(expr), Expr::Symbol("x".to_string()));

    // tan(atan(x)) = x
    let expr = Expr::FunctionCall {
        name: "tan".to_string(),
        args: vec![Expr::FunctionCall {
            name: "atan".to_string(),
            args: vec![Expr::Symbol("x".to_string())],
        }],
    };
    assert_eq!(simplify(expr), Expr::Symbol("x".to_string()));
}

#[test]
fn test_inverse_composition_reverse() {
    // asin(sin(x)) = x
    let expr = Expr::FunctionCall {
        name: "asin".to_string(),
        args: vec![Expr::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::Symbol("x".to_string())],
        }],
    };
    assert_eq!(simplify(expr), Expr::Symbol("x".to_string()));

    // acos(cos(x)) = x
    let expr = Expr::FunctionCall {
        name: "acos".to_string(),
        args: vec![Expr::FunctionCall {
            name: "cos".to_string(),
            args: vec![Expr::Symbol("x".to_string())],
        }],
    };
    assert_eq!(simplify(expr), Expr::Symbol("x".to_string()));
}

#[test]
fn test_pythagorean_identities() {
    // sin^2(x) + cos^2(x) = 1
    let expr = Expr::Add(
        Box::new(Expr::Pow(
            Box::new(Expr::FunctionCall {
                name: "sin".to_string(),
                args: vec![Expr::Symbol("x".to_string())],
            }),
            Box::new(Expr::Number(2.0)),
        )),
        Box::new(Expr::Pow(
            Box::new(Expr::FunctionCall {
                name: "cos".to_string(),
                args: vec![Expr::Symbol("x".to_string())],
            }),
            Box::new(Expr::Number(2.0)),
        )),
    );
    assert_eq!(simplify(expr), Expr::Number(1.0));

    // 1 + tan^2(x) = sec^2(x)
    let expr = Expr::Add(
        Box::new(Expr::Number(1.0)),
        Box::new(Expr::Pow(
            Box::new(Expr::FunctionCall {
                name: "tan".to_string(),
                args: vec![Expr::Symbol("x".to_string())],
            }),
            Box::new(Expr::Number(2.0)),
        )),
    );
    let simplified = simplify(expr);
    if let Expr::Pow(base, exp) = simplified {
        assert_eq!(*exp, Expr::Number(2.0));
        if let Expr::FunctionCall { name, args } = *base {
            assert_eq!(name, "sec");
            assert_eq!(args[0], Expr::Symbol("x".to_string()));
        } else {
            panic!("Expected sec(x)");
        }
    } else {
        panic!("Expected sec^2(x)");
    }

    // 1 + cot^2(x) = csc^2(x)
    let expr = Expr::Add(
        Box::new(Expr::Number(1.0)),
        Box::new(Expr::Pow(
            Box::new(Expr::FunctionCall {
                name: "cot".to_string(),
                args: vec![Expr::Symbol("x".to_string())],
            }),
            Box::new(Expr::Number(2.0)),
        )),
    );
    let simplified = simplify(expr);
    if let Expr::Pow(base, exp) = simplified {
        assert_eq!(*exp, Expr::Number(2.0));
        if let Expr::FunctionCall { name, args } = *base {
            assert_eq!(name, "csc");
            assert_eq!(args[0], Expr::Symbol("x".to_string()));
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
    let expr = Expr::FunctionCall {
        name: "sin".to_string(),
        args: vec![Expr::Sub(
            Box::new(Expr::Number(PI / 2.0)),
            Box::new(Expr::Symbol("x".to_string())),
        )],
    };
    let simplified = simplify(expr);
    if let Expr::FunctionCall { name, args } = simplified {
        assert_eq!(name, "cos");
        assert_eq!(args[0], Expr::Symbol("x".to_string()));
    } else {
        panic!("Expected cos(x)");
    }

    // cos(pi/2 - x) = sin(x)
    let expr = Expr::FunctionCall {
        name: "cos".to_string(),
        args: vec![Expr::Sub(
            Box::new(Expr::Number(PI / 2.0)),
            Box::new(Expr::Symbol("x".to_string())),
        )],
    };
    let simplified = simplify(expr);
    if let Expr::FunctionCall { name, args } = simplified {
        assert_eq!(name, "sin");
        assert_eq!(args[0], Expr::Symbol("x".to_string()));
    } else {
        panic!("Expected sin(x)");
    }
}

#[test]
fn test_trig_periodicity() {
    use std::f64::consts::PI;
    // sin(x + 2pi) = sin(x)
    let expr = Expr::FunctionCall {
        name: "sin".to_string(),
        args: vec![Expr::Add(
            Box::new(Expr::Symbol("x".to_string())),
            Box::new(Expr::Number(2.0 * PI)),
        )],
    };
    let simplified = simplify(expr);
    if let Expr::FunctionCall { name, args } = simplified {
        assert_eq!(name, "sin");
        assert_eq!(args[0], Expr::Symbol("x".to_string()));
    } else {
        panic!("Expected sin(x)");
    }

    // cos(x + 2pi) = cos(x)
    let expr = Expr::FunctionCall {
        name: "cos".to_string(),
        args: vec![Expr::Add(
            Box::new(Expr::Symbol("x".to_string())),
            Box::new(Expr::Number(2.0 * PI)),
        )],
    };
    let simplified = simplify(expr);
    if let Expr::FunctionCall { name, args } = simplified {
        assert_eq!(name, "cos");
        assert_eq!(args[0], Expr::Symbol("x".to_string()));
    } else {
        panic!("Expected cos(x)");
    }
}

#[test]
fn test_trig_periodicity_general() {
    use std::f64::consts::PI;
    // sin(x + 4pi) = sin(x)
    let expr = Expr::FunctionCall {
        name: "sin".to_string(),
        args: vec![Expr::Add(
            Box::new(Expr::Symbol("x".to_string())),
            Box::new(Expr::Number(4.0 * PI)),
        )],
    };
    let simplified = simplify(expr);
    if let Expr::FunctionCall { name, args } = simplified {
        assert_eq!(name, "sin");
        assert_eq!(args[0], Expr::Symbol("x".to_string()));
    } else {
        panic!("Expected sin(x)");
    }

    // cos(x - 2pi) = cos(x)
    let expr = Expr::FunctionCall {
        name: "cos".to_string(),
        args: vec![Expr::Add(
            Box::new(Expr::Symbol("x".to_string())),
            Box::new(Expr::Number(-2.0 * PI)),
        )],
    };
    let simplified = simplify(expr);
    if let Expr::FunctionCall { name, args } = simplified {
        assert_eq!(name, "cos");
        assert_eq!(args[0], Expr::Symbol("x".to_string()));
    } else {
        panic!("Expected cos(x)");
    }
}

#[test]
fn test_trig_reflection_shifts() {
    use std::f64::consts::PI;
    // sin(pi - x) = sin(x)
    let expr = Expr::FunctionCall {
        name: "sin".to_string(),
        args: vec![Expr::Sub(
            Box::new(Expr::Number(PI)),
            Box::new(Expr::Symbol("x".to_string())),
        )],
    };
    let simplified = simplify(expr);
    if let Expr::FunctionCall { name, args } = simplified {
        assert_eq!(name, "sin");
        assert_eq!(args[0], Expr::Symbol("x".to_string()));
    } else {
        panic!("Expected sin(x)");
    }

    // cos(pi + x) = -cos(x)
    let expr = Expr::FunctionCall {
        name: "cos".to_string(),
        args: vec![Expr::Add(
            Box::new(Expr::Number(PI)),
            Box::new(Expr::Symbol("x".to_string())),
        )],
    };
    let simplified = simplify(expr);
    if let Expr::Mul(a, b) = simplified {
        assert_eq!(*a, Expr::Number(-1.0));
        if let Expr::FunctionCall { name, args } = *b {
            assert_eq!(name, "cos");
            assert_eq!(args[0], Expr::Symbol("x".to_string()));
        } else {
            panic!("Expected cos(x)");
        }
    } else {
        panic!("Expected -cos(x)");
    }

    // sin(3pi/2 - x) = -cos(x)
    let expr = Expr::FunctionCall {
        name: "sin".to_string(),
        args: vec![Expr::Sub(
            Box::new(Expr::Number(3.0 * PI / 2.0)),
            Box::new(Expr::Symbol("x".to_string())),
        )],
    };
    let simplified = simplify(expr);
    if let Expr::Mul(a, b) = simplified {
        assert_eq!(*a, Expr::Number(-1.0));
        if let Expr::FunctionCall { name, args } = *b {
            assert_eq!(name, "cos");
            assert_eq!(args[0], Expr::Symbol("x".to_string()));
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
    let expr = Expr::FunctionCall {
        name: "sin".to_string(),
        args: vec![Expr::Number(PI / 6.0)],
    };
    assert_eq!(simplify(expr), Expr::Number(0.5));

    // cos(pi/3) = 0.5
    let expr = Expr::FunctionCall {
        name: "cos".to_string(),
        args: vec![Expr::Number(PI / 3.0)],
    };
    assert_eq!(simplify(expr), Expr::Number(0.5));

    // tan(pi/4) = 1.0
    let expr = Expr::FunctionCall {
        name: "tan".to_string(),
        args: vec![Expr::Number(PI / 4.0)],
    };
    assert_eq!(simplify(expr), Expr::Number(1.0));

    // sin(pi/4) = sqrt(2)/2 approx 0.70710678
    let expr = Expr::FunctionCall {
        name: "sin".to_string(),
        args: vec![Expr::Number(PI / 4.0)],
    };
    let simplified = simplify(expr);
    if let Expr::Number(n) = simplified {
        assert!((n - (2.0f64.sqrt() / 2.0)).abs() < 1e-10);
    } else {
        panic!("Expected number");
    }
}

#[test]
fn test_double_angle_formulas() {
    // sin(2x) = 2*sin(x)*cos(x)
    let expr = Expr::FunctionCall {
        name: "sin".to_string(),
        args: vec![Expr::Mul(
            Box::new(Expr::Number(2.0)),
            Box::new(Expr::Symbol("x".to_string())),
        )],
    };
    let simplified = simplify(expr);
    // Should be 2*sin(x)*cos(x) - the structure is ((2*cos(x))*sin(x))
    if let Expr::Mul(a, b) = &simplified {
        // a should be (2*cos(x))
        if let Expr::Mul(c, d) = &**a {
            assert_eq!(**c, Expr::Number(2.0));
            if let Expr::FunctionCall { name, args } = &**d {
                assert_eq!(name, "cos");
                assert_eq!(args[0], Expr::Symbol("x".to_string()));
            } else {
                panic!("Expected cos(x)");
            }
        } else {
            panic!("Expected 2*cos(x)");
        }
        // b should be sin(x)
        if let Expr::FunctionCall { name, args } = &**b {
            assert_eq!(name, "sin");
            assert_eq!(args[0], Expr::Symbol("x".to_string()));
        } else {
            panic!("Expected sin(x)");
        }
    } else {
        panic!("Expected 2*cos(x)*sin(x), got {:?}", simplified);
    }

    // cos(2x) = cos^2(x) - sin^2(x)
    let expr = Expr::FunctionCall {
        name: "cos".to_string(),
        args: vec![Expr::Mul(
            Box::new(Expr::Number(2.0)),
            Box::new(Expr::Symbol("x".to_string())),
        )],
    };
    let simplified = simplify(expr);
    // Should be cos^2(x) + (-1)*sin^2(x)  (canonical form after algebraic simplification)
    if let Expr::Add(a, b) = simplified {
        // The terms are sorted, so Mul comes before Pow
        // Check -sin^2(x)
        if let Expr::Mul(coeff, sin_sq) = *a {
            assert_eq!(*coeff, Expr::Number(-1.0));
            if let Expr::Pow(base1, exp1) = *sin_sq {
                assert_eq!(*exp1, Expr::Number(2.0));
                if let Expr::FunctionCall {
                    name: name1,
                    args: args1,
                } = *base1
                {
                    assert_eq!(name1, "sin");
                    assert_eq!(args1[0], Expr::Symbol("x".to_string()));
                } else {
                    panic!("Expected sin(x)");
                }
            } else {
                panic!("Expected sin^2(x)");
            }
        } else {
            panic!("Expected -sin^2(x)");
        }
        // Check cos^2(x)
        if let Expr::Pow(base2, exp2) = *b {
            assert_eq!(*exp2, Expr::Number(2.0));
            if let Expr::FunctionCall {
                name: name2,
                args: args2,
            } = *base2
            {
                assert_eq!(name2, "cos");
                assert_eq!(args2[0], Expr::Symbol("x".to_string()));
            } else {
                panic!("Expected cos(x)");
            }
        } else {
            panic!("Expected cos^2(x)");
        }
    } else {
        panic!("Expected cos^2(x) + (-1)*sin^2(x), got {:?}", simplified);
    }

    // tan(2x) = 2*tan(x) / (1 - tan^2(x))
    let expr = Expr::FunctionCall {
        name: "tan".to_string(),
        args: vec![Expr::Mul(
            Box::new(Expr::Number(2.0)),
            Box::new(Expr::Symbol("x".to_string())),
        )],
    };
    let simplified = simplify(expr);
    // Should be 2*tan(x) / (1 - tan^2(x))
    if let Expr::Div(num, den) = simplified {
        // Check numerator: 2*tan(x)
        if let Expr::Mul(a, b) = *num {
            assert_eq!(*a, Expr::Number(2.0));
            if let Expr::FunctionCall { name, args } = *b {
                assert_eq!(name, "tan");
                assert_eq!(args[0], Expr::Symbol("x".to_string()));
            } else {
                panic!("Expected tan(x)");
            }
        } else {
            panic!("Expected 2*tan(x)");
        }
        // Check denominator: 1 + (-1)*tan^2(x)  (canonical form)
        if let Expr::Add(c, d) = *den {
            assert_eq!(*c, Expr::Number(1.0));
            if let Expr::Mul(coeff, tan_sq) = *d {
                assert_eq!(*coeff, Expr::Number(-1.0));
                if let Expr::Pow(base, exp) = *tan_sq {
                    assert_eq!(*exp, Expr::Number(2.0));
                    if let Expr::FunctionCall { name, args } = *base {
                        assert_eq!(name, "tan");
                        assert_eq!(args[0], Expr::Symbol("x".to_string()));
                    } else {
                        panic!("Expected tan(x)");
                    }
                } else {
                    panic!("Expected tan^2(x)");
                }
            } else {
                panic!("Expected -tan^2(x)");
            }
        } else {
            panic!("Expected 1 + (-1)*tan^2(x)");
        }
    } else {
        panic!("Expected 2*tan(x)/(1-tan^2(x))");
    }
}
