use crate::Expr;
use std::f64::consts::PI;

pub fn apply_trig_rules(expr: Expr) -> Expr {
    match &expr {
        Expr::FunctionCall { name, args } => {
            if args.len() != 1 {
                return expr;
            }
            let content = &args[0];
            match name.as_str() {
                "sin" => {
                    // Standard values
                    if matches!(content, Expr::Number(n) if *n == 0.0) {
                        return Expr::Number(0.0);
                    }
                    if matches!(content, Expr::Number(n) if (n - PI).abs() < 1e-10) {
                        return Expr::Number(0.0);
                    }
                    if matches!(content, Expr::Number(n) if (n - PI/2.0).abs() < 1e-10) {
                        return Expr::Number(1.0);
                    }
                    // Extended Exact Values
                    if matches!(content, Expr::Number(n) if (n - PI/6.0).abs() < 1e-10) {
                        return Expr::Number(0.5);
                    }
                    if matches!(content, Expr::Number(n) if (n - PI/4.0).abs() < 1e-10) {
                        return Expr::Number(2.0f64.sqrt() / 2.0);
                    }
                    if matches!(content, Expr::Number(n) if (n - PI/3.0).abs() < 1e-10) {
                        return Expr::Number(3.0f64.sqrt() / 2.0);
                    }

                    // sin(-x) = -sin(x)
                    if let Expr::Mul(a, b) = content
                        && matches!(**a, Expr::Number(n) if n == -1.0)
                    {
                        return Expr::Mul(
                            Box::new(Expr::Number(-1.0)),
                            Box::new(Expr::FunctionCall {
                                name: "sin".to_string(),
                                args: vec![*b.clone()],
                            }),
                        );
                    }

                    // sin(asin(x)) = x
                    if let Expr::FunctionCall {
                        name: inner_name,
                        args: inner_args,
                    } = content
                        && inner_name == "asin"
                        && inner_args.len() == 1
                    {
                        return inner_args[0].clone();
                    }

                    // Cofunction: sin(pi/2 - x) = cos(x)
                    if let Expr::Sub(lhs, rhs) = content
                        && matches!(**lhs, Expr::Number(n) if (n - PI/2.0).abs() < 1e-10)
                    {
                        return Expr::FunctionCall {
                            name: "cos".to_string(),
                            args: vec![*rhs.clone()],
                        };
                    }
                    // Cofunction: sin(pi/2 + (-1 * x)) = cos(x)  (after algebraic simplification)
                    if let Expr::Add(lhs, rhs) = content
                        && matches!(**lhs, Expr::Number(n) if (n - PI/2.0).abs() < 1e-10)
                        && let Expr::Mul(a, b) = &**rhs
                        && matches!(**a, Expr::Number(n) if n == -1.0)
                    {
                        return Expr::FunctionCall {
                            name: "cos".to_string(),
                            args: vec![*b.clone()],
                        };
                    }
                    // Cofunction: sin((-1 * x) + pi/2) = cos(x)  (after algebraic simplification)
                    if let Expr::Add(lhs, rhs) = content
                        && matches!(**rhs, Expr::Number(n) if (n - PI/2.0).abs() < 1e-10)
                        && let Expr::Mul(a, b) = &**lhs
                        && matches!(**a, Expr::Number(n) if n == -1.0)
                    {
                        return Expr::FunctionCall {
                            name: "cos".to_string(),
                            args: vec![*b.clone()],
                        };
                    }

                    // Periodicity: sin(x + 2k*pi) = sin(x)
                    if let Expr::Add(lhs, rhs) = content {
                        if is_multiple_of_two_pi(rhs) {
                            return Expr::FunctionCall {
                                name: "sin".to_string(),
                                args: vec![*lhs.clone()],
                            };
                        }
                        if is_multiple_of_two_pi(lhs) {
                            return Expr::FunctionCall {
                                name: "sin".to_string(),
                                args: vec![*rhs.clone()],
                            };
                        }
                    }

                    // Reflection/Shifts
                    // sin(pi - x) = sin(x)
                    if let Expr::Sub(lhs, rhs) = content {
                        if is_pi(lhs) {
                            return Expr::FunctionCall {
                                name: "sin".to_string(),
                                args: vec![*rhs.clone()],
                            };
                        }
                        // sin(3pi/2 - x) = -cos(x)
                        if is_three_pi_over_two(lhs) {
                            return Expr::Mul(
                                Box::new(Expr::Number(-1.0)),
                                Box::new(Expr::FunctionCall {
                                    name: "cos".to_string(),
                                    args: vec![*rhs.clone()],
                                }),
                            );
                        }
                    }
                    // sin(pi + x) = -sin(x)
                    if let Expr::Add(lhs, rhs) = content {
                        if is_pi(lhs) || is_pi(rhs) {
                            let arg = if is_pi(lhs) { rhs } else { lhs };
                            return Expr::Mul(
                                Box::new(Expr::Number(-1.0)),
                                Box::new(Expr::FunctionCall {
                                    name: "sin".to_string(),
                                    args: vec![*arg.clone()],
                                }),
                            );
                        }
                        // sin(3pi/2 + x) = -cos(x)
                        if is_three_pi_over_two(lhs) || is_three_pi_over_two(rhs) {
                            let arg = if is_three_pi_over_two(lhs) { rhs } else { lhs };
                            return Expr::Mul(
                                Box::new(Expr::Number(-1.0)),
                                Box::new(Expr::FunctionCall {
                                    name: "cos".to_string(),
                                    args: vec![*arg.clone()],
                                }),
                            );
                        }
                    }

                    // Double angle: sin(2x) = 2*sin(x)*cos(x)
                    if let Expr::Mul(a, b) = content
                        && matches!(**a, Expr::Number(n) if n == 2.0)
                    {
                        return Expr::Mul(
                            Box::new(Expr::Mul(
                                Box::new(Expr::Number(2.0)),
                                Box::new(Expr::FunctionCall {
                                    name: "sin".to_string(),
                                    args: vec![*b.clone()],
                                }),
                            )),
                            Box::new(Expr::FunctionCall {
                                name: "cos".to_string(),
                                args: vec![*b.clone()],
                            }),
                        );
                    }
                }
                "cos" => {
                    // Standard values
                    if matches!(content, Expr::Number(n) if *n == 0.0) {
                        return Expr::Number(1.0);
                    }
                    if matches!(content, Expr::Number(n) if (n - PI).abs() < 1e-10) {
                        return Expr::Number(-1.0);
                    }
                    if matches!(content, Expr::Number(n) if (n - PI/2.0).abs() < 1e-10) {
                        return Expr::Number(0.0);
                    }
                    // Extended Exact Values
                    if matches!(content, Expr::Number(n) if (n - PI/6.0).abs() < 1e-10) {
                        return Expr::Number(3.0f64.sqrt() / 2.0);
                    }
                    if matches!(content, Expr::Number(n) if (n - PI/4.0).abs() < 1e-10) {
                        return Expr::Number(2.0f64.sqrt() / 2.0);
                    }
                    if matches!(content, Expr::Number(n) if (n - PI/3.0).abs() < 1e-10) {
                        return Expr::Number(0.5);
                    }

                    // cos(-x) = cos(x)
                    if let Expr::Mul(a, b) = content
                        && matches!(**a, Expr::Number(n) if n == -1.0)
                    {
                        return Expr::FunctionCall {
                            name: "cos".to_string(),
                            args: vec![*b.clone()],
                        };
                    }

                    // cos(acos(x)) = x
                    if let Expr::FunctionCall {
                        name: inner_name,
                        args: inner_args,
                    } = content
                        && inner_name == "acos"
                        && inner_args.len() == 1
                    {
                        return inner_args[0].clone();
                    }

                    // Cofunction: cos(pi/2 - x) = sin(x)
                    if let Expr::Sub(lhs, rhs) = content
                        && matches!(**lhs, Expr::Number(n) if (n - PI/2.0).abs() < 1e-10)
                    {
                        return Expr::FunctionCall {
                            name: "sin".to_string(),
                            args: vec![*rhs.clone()],
                        };
                    }
                    // Cofunction: cos(pi/2 + (-1 * x)) = sin(x)  (after algebraic simplification)
                    if let Expr::Add(lhs, rhs) = content
                        && matches!(**lhs, Expr::Number(n) if (n - PI/2.0).abs() < 1e-10)
                        && let Expr::Mul(a, b) = &**rhs
                        && matches!(**a, Expr::Number(n) if n == -1.0)
                    {
                        return Expr::FunctionCall {
                            name: "sin".to_string(),
                            args: vec![*b.clone()],
                        };
                    }
                    // Cofunction: cos((-1 * x) + pi/2) = sin(x)  (after algebraic simplification)
                    if let Expr::Add(lhs, rhs) = content
                        && matches!(**rhs, Expr::Number(n) if (n - PI/2.0).abs() < 1e-10)
                        && let Expr::Mul(a, b) = &**lhs
                        && matches!(**a, Expr::Number(n) if n == -1.0)
                    {
                        return Expr::FunctionCall {
                            name: "sin".to_string(),
                            args: vec![*b.clone()],
                        };
                    }

                    // Periodicity: cos(x + 2k*pi) = cos(x)
                    if let Expr::Add(lhs, rhs) = content {
                        if is_multiple_of_two_pi(rhs) {
                            return Expr::FunctionCall {
                                name: "cos".to_string(),
                                args: vec![*lhs.clone()],
                            };
                        }
                        if is_multiple_of_two_pi(lhs) {
                            return Expr::FunctionCall {
                                name: "cos".to_string(),
                                args: vec![*rhs.clone()],
                            };
                        }
                    }

                    // Reflection/Shifts
                    // cos(pi - x) = -cos(x)
                    if let Expr::Sub(lhs, rhs) = content {
                        if is_pi(lhs) {
                            return Expr::Mul(
                                Box::new(Expr::Number(-1.0)),
                                Box::new(Expr::FunctionCall {
                                    name: "cos".to_string(),
                                    args: vec![*rhs.clone()],
                                }),
                            );
                        }
                        // cos(3pi/2 - x) = -sin(x)
                        if is_three_pi_over_two(lhs) {
                            return Expr::Mul(
                                Box::new(Expr::Number(-1.0)),
                                Box::new(Expr::FunctionCall {
                                    name: "sin".to_string(),
                                    args: vec![*rhs.clone()],
                                }),
                            );
                        }
                    }
                    // cos(pi + x) = -cos(x)
                    if let Expr::Add(lhs, rhs) = content {
                        if is_pi(lhs) || is_pi(rhs) {
                            let arg = if is_pi(lhs) { rhs } else { lhs };
                            return Expr::Mul(
                                Box::new(Expr::Number(-1.0)),
                                Box::new(Expr::FunctionCall {
                                    name: "cos".to_string(),
                                    args: vec![*arg.clone()],
                                }),
                            );
                        }
                        // cos(3pi/2 + x) = sin(x)
                        if is_three_pi_over_two(lhs) || is_three_pi_over_two(rhs) {
                            let arg = if is_three_pi_over_two(lhs) { rhs } else { lhs };
                            return Expr::FunctionCall {
                                name: "sin".to_string(),
                                args: vec![*arg.clone()],
                            };
                        }
                    }
                }
                "tan" => {
                    // Standard values
                    if matches!(content, Expr::Number(n) if *n == 0.0) {
                        return Expr::Number(0.0);
                    }
                    if matches!(content, Expr::Number(n) if (n - PI).abs() < 1e-10) {
                        return Expr::Number(0.0);
                    }
                    // Extended Exact Values
                    if matches!(content, Expr::Number(n) if (n - PI/6.0).abs() < 1e-10) {
                        return Expr::Number(3.0f64.sqrt() / 3.0);
                    }
                    if matches!(content, Expr::Number(n) if (n - PI/4.0).abs() < 1e-10) {
                        return Expr::Number(1.0);
                    }
                    if matches!(content, Expr::Number(n) if (n - PI/3.0).abs() < 1e-10) {
                        return Expr::Number(3.0f64.sqrt());
                    }

                    // tan(-x) = -tan(x)
                    if let Expr::Mul(a, b) = content
                        && matches!(**a, Expr::Number(n) if n == -1.0)
                    {
                        return Expr::Mul(
                            Box::new(Expr::Number(-1.0)),
                            Box::new(Expr::FunctionCall {
                                name: "tan".to_string(),
                                args: vec![*b.clone()],
                            }),
                        );
                    }

                    // tan(atan(x)) = x
                    if let Expr::FunctionCall {
                        name: inner_name,
                        args: inner_args,
                    } = content
                        && inner_name == "atan"
                        && inner_args.len() == 1
                    {
                        return inner_args[0].clone();
                    }

                    // Cofunction: tan(pi/2 - x) = cot(x)
                    if let Expr::Sub(lhs, rhs) = content
                        && matches!(**lhs, Expr::Number(n) if (n - PI/2.0).abs() < 1e-10)
                    {
                        return Expr::FunctionCall {
                            name: "cot".to_string(),
                            args: vec![*rhs.clone()],
                        };
                    }
                    // Cofunction: tan(pi/2 + (-1 * x)) = cot(x)  (after algebraic simplification)
                    if let Expr::Add(lhs, rhs) = content
                        && matches!(**lhs, Expr::Number(n) if (n - PI/2.0).abs() < 1e-10)
                        && let Expr::Mul(a, b) = &**rhs
                        && matches!(**a, Expr::Number(n) if n == -1.0)
                    {
                        return Expr::FunctionCall {
                            name: "cot".to_string(),
                            args: vec![*b.clone()],
                        };
                    }
                    // Cofunction: tan((-1 * x) + pi/2) = cot(x)  (after algebraic simplification)
                    if let Expr::Add(lhs, rhs) = content
                        && matches!(**rhs, Expr::Number(n) if (n - PI/2.0).abs() < 1e-10)
                        && let Expr::Mul(a, b) = &**lhs
                        && matches!(**a, Expr::Number(n) if n == -1.0)
                    {
                        return Expr::FunctionCall {
                            name: "cot".to_string(),
                            args: vec![*b.clone()],
                        };
                    }

                    // Double angle: tan(2x) = 2*tan(x) / (1 - tan^2(x))
                    if let Expr::Mul(a, b) = content
                        && matches!(**a, Expr::Number(n) if n == 2.0)
                    {
                        let tan_x = Expr::FunctionCall {
                            name: "tan".to_string(),
                            args: vec![*b.clone()],
                        };
                        let tan_x_sq =
                            Expr::Pow(Box::new(tan_x.clone()), Box::new(Expr::Number(2.0)));
                        return Expr::Div(
                            Box::new(Expr::Mul(Box::new(Expr::Number(2.0)), Box::new(tan_x))),
                            Box::new(Expr::Sub(Box::new(Expr::Number(1.0)), Box::new(tan_x_sq))),
                        );
                    }
                }
                "cot" => {
                    // cot(x) = 1 / tan(x) = cos(x) / sin(x)
                    // cot(0) is undefined, but cot(π/4) = 1, cot(π/2) = 0
                    if matches!(content, Expr::Number(n) if (n - PI/4.0).abs() < 1e-10) {
                        return Expr::Number(1.0);
                    }
                    if matches!(content, Expr::Number(n) if (n - PI/2.0).abs() < 1e-10) {
                        return Expr::Number(0.0);
                    }

                    // Cofunction: cot(pi/2 - x) = tan(x)
                    if let Expr::Sub(lhs, rhs) = content
                        && matches!(**lhs, Expr::Number(n) if (n - PI/2.0).abs() < 1e-10)
                    {
                        return Expr::FunctionCall {
                            name: "tan".to_string(),
                            args: vec![*rhs.clone()],
                        };
                    }
                    // Cofunction: cot(pi/2 + (-1 * x)) = tan(x)  (after algebraic simplification)
                    if let Expr::Add(lhs, rhs) = content
                        && matches!(**lhs, Expr::Number(n) if (n - PI/2.0).abs() < 1e-10)
                        && let Expr::Mul(a, b) = &**rhs
                        && matches!(**a, Expr::Number(n) if n == -1.0)
                    {
                        return Expr::FunctionCall {
                            name: "tan".to_string(),
                            args: vec![*b.clone()],
                        };
                    }
                    // Cofunction: cot((-1 * x) + pi/2) = tan(x)  (after algebraic simplification)
                    if let Expr::Add(lhs, rhs) = content
                        && matches!(**rhs, Expr::Number(n) if (n - PI/2.0).abs() < 1e-10)
                        && let Expr::Mul(a, b) = &**lhs
                        && matches!(**a, Expr::Number(n) if n == -1.0)
                    {
                        return Expr::FunctionCall {
                            name: "tan".to_string(),
                            args: vec![*b.clone()],
                        };
                    }
                }
                "sec" => {
                    // sec(x) = 1 / cos(x)
                    // sec(0) = 1, sec(π/2) is undefined
                    if matches!(content, Expr::Number(n) if *n == 0.0) {
                        return Expr::Number(1.0);
                    }

                    // sec(-x) = sec(x)
                    if let Expr::Mul(a, b) = content
                        && matches!(**a, Expr::Number(n) if n == -1.0)
                    {
                        return Expr::FunctionCall {
                            name: "sec".to_string(),
                            args: vec![*b.clone()],
                        };
                    }
                }
                "csc" => {
                    // csc(x) = 1 / sin(x)
                    // csc(π/2) = 1, csc(π/6) = 2
                    if matches!(content, Expr::Number(n) if (n - PI/2.0).abs() < 1e-10) {
                        return Expr::Number(1.0);
                    }
                    if matches!(content, Expr::Number(n) if (n - PI/6.0).abs() < 1e-10) {
                        return Expr::Number(2.0);
                    }

                    // csc(-x) = -csc(x)
                    if let Expr::Mul(a, b) = content
                        && matches!(**a, Expr::Number(n) if n == -1.0)
                    {
                        return Expr::Mul(
                            Box::new(Expr::Number(-1.0)),
                            Box::new(Expr::FunctionCall {
                                name: "csc".to_string(),
                                args: vec![*b.clone()],
                            }),
                        );
                    }
                }

                // Inverse Trig
                "asin" => {
                    if matches!(content, Expr::Number(n) if *n == 0.0) {
                        return Expr::Number(0.0);
                    }
                    // asin(sin(x)) = x
                    if let Expr::FunctionCall {
                        name: inner_name,
                        args: inner_args,
                    } = content
                        && inner_name == "sin"
                        && inner_args.len() == 1
                    {
                        return inner_args[0].clone();
                    }
                }
                "acos" => {
                    if matches!(content, Expr::Number(n) if *n == 1.0) {
                        return Expr::Number(0.0);
                    }
                    if matches!(content, Expr::Number(n) if *n == 0.0) {
                        return Expr::Number(PI / 2.0);
                    }
                    // acos(cos(x)) = x
                    if let Expr::FunctionCall {
                        name: inner_name,
                        args: inner_args,
                    } = content
                        && inner_name == "cos"
                        && inner_args.len() == 1
                    {
                        return inner_args[0].clone();
                    }
                }
                "atan" => {
                    if matches!(content, Expr::Number(n) if *n == 0.0) {
                        return Expr::Number(0.0);
                    }
                    if matches!(content, Expr::Number(n) if *n == 1.0) {
                        return Expr::Number(PI / 4.0);
                    }
                    // atan(tan(x)) = x
                    if let Expr::FunctionCall {
                        name: inner_name,
                        args: inner_args,
                    } = content
                        && inner_name == "tan"
                        && inner_args.len() == 1
                    {
                        return inner_args[0].clone();
                    }
                }
                _ => {}
            }
        }
        Expr::Add(u, v) => {
            // Pythagorean: sin^2(x) + cos^2(x) = 1
            if is_sin_sq_plus_cos_sq(u, v).is_some() {
                return Expr::Number(1.0);
            }

            // Pythagorean: 1 + tan^2(x) = sec^2(x)
            if let Some(arg) = is_one_plus_tan_sq(u, v) {
                return Expr::Pow(
                    Box::new(Expr::FunctionCall {
                        name: "sec".to_string(),
                        args: vec![arg],
                    }),
                    Box::new(Expr::Number(2.0)),
                );
            }

            // Pythagorean: 1 + cot^2(x) = csc^2(x)
            if let Some(arg) = is_one_plus_cot_sq(u, v) {
                return Expr::Pow(
                    Box::new(Expr::FunctionCall {
                        name: "csc".to_string(),
                        args: vec![arg],
                    }),
                    Box::new(Expr::Number(2.0)),
                );
            }

            // Double angle: cos^2(x) - sin^2(x) = cos(2x)
            if let Some(arg) = is_cos_sq_minus_sin_sq(u, v) {
                return Expr::FunctionCall {
                    name: "cos".to_string(),
                    args: vec![Expr::Mul(Box::new(Expr::Number(2.0)), Box::new(arg))],
                };
            }
            // Double angle: cos^2(x) + (-sin^2(x)) = cos(2x)
            if let Some(arg) = is_cos_sq_plus_neg_sin_sq(u, v) {
                return Expr::FunctionCall {
                    name: "cos".to_string(),
                    args: vec![Expr::Mul(Box::new(Expr::Number(2.0)), Box::new(arg))],
                };
            }

            // Sum/Difference Formulas
            // sin(x)cos(y) + cos(x)sin(y) = sin(x + y)
            if let Some((x, y)) = is_sin_sum(u, v) {
                return Expr::FunctionCall {
                    name: "sin".to_string(),
                    args: vec![Expr::Add(Box::new(x), Box::new(y))],
                };
            }
            // cos(x)cos(y) + sin(x)sin(y) = cos(x - y)
            if let Some((x, y)) = is_cos_diff(u, v) {
                return Expr::FunctionCall {
                    name: "cos".to_string(),
                    args: vec![Expr::Sub(Box::new(x), Box::new(y))],
                };
            }
        }
        Expr::Sub(u, v) => {
            // Double angle: cos^2(x) - sin^2(x) = cos(2x)
            if let Some(arg) = is_cos_sq_minus_sin_sq(u, v) {
                return Expr::FunctionCall {
                    name: "cos".to_string(),
                    args: vec![Expr::Mul(Box::new(Expr::Number(2.0)), Box::new(arg))],
                };
            }

            // Sum/Difference Formulas
            // sin(x)cos(y) - cos(x)sin(y) = sin(x - y)
            if let Some((x, y)) = is_sin_diff(u, v) {
                return Expr::FunctionCall {
                    name: "sin".to_string(),
                    args: vec![Expr::Sub(Box::new(x), Box::new(y))],
                };
            }
            // cos(x)cos(y) - sin(x)sin(y) = cos(x + y)
            if let Some((x, y)) = is_cos_sum(u, v) {
                return Expr::FunctionCall {
                    name: "cos".to_string(),
                    args: vec![Expr::Add(Box::new(x), Box::new(y))],
                };
            }
        }
        _ => {}
    }
    expr
}

// Helper to check if expression is k * 2*pi where k is an integer
fn is_multiple_of_two_pi(expr: &Expr) -> bool {
    if let Expr::Number(n) = expr {
        let two_pi = 2.0 * PI;
        let k = n / two_pi;
        return (k - k.round()).abs() < 1e-10;
    }
    false
}

fn is_pi(expr: &Expr) -> bool {
    if let Expr::Number(n) = expr {
        return (n - PI).abs() < 1e-10;
    }
    false
}

fn is_three_pi_over_two(expr: &Expr) -> bool {
    if let Expr::Number(n) = expr {
        return (n - 3.0 * PI / 2.0).abs() < 1e-10;
    }
    false
}

// Helper for sin^2(x) + cos^2(x)
fn is_sin_sq_plus_cos_sq(u: &Expr, v: &Expr) -> Option<Expr> {
    if let (Some((name1, arg1)), Some((name2, arg2))) = (get_trig_sq(u), get_trig_sq(v))
        && arg1 == arg2
        && ((name1 == "sin" && name2 == "cos") || (name1 == "cos" && name2 == "sin"))
    {
        return Some(arg1);
    }
    None
}

// Helper for cos^2(x) - sin^2(x)
fn is_cos_sq_minus_sin_sq(u: &Expr, v: &Expr) -> Option<Expr> {
    if let (Some(("cos", arg1)), Some(("sin", arg2))) = (get_trig_sq(u), get_trig_sq(v))
        && arg1 == arg2
    {
        return Some(arg1);
    }
    if let (Some(("sin", arg2)), Some(("cos", arg1))) = (get_trig_sq(u), get_trig_sq(v))
        && arg1 == arg2
    {
        return Some(arg1);
    }
    None
}

// Helper for cos^2(x) + (-sin^2(x))
fn is_cos_sq_plus_neg_sin_sq(u: &Expr, v: &Expr) -> Option<Expr> {
    if let Some(("cos", arg1)) = get_trig_sq(u)
        && let Expr::Mul(coeff, sin_sq) = v
        && matches!(**coeff, Expr::Number(n) if n == -1.0)
        && let Some(("sin", arg2)) = get_trig_sq(sin_sq)
        && arg1 == arg2
    {
        return Some(arg1);
    }
    if let Some(("cos", arg1)) = get_trig_sq(v)
        && let Expr::Mul(coeff, sin_sq) = u
        && matches!(**coeff, Expr::Number(n) if n == -1.0)
        && let Some(("sin", arg2)) = get_trig_sq(sin_sq)
        && arg1 == arg2
    {
        return Some(arg1);
    }
    None
}

// Helper for 1 + tan^2(x)
fn is_one_plus_tan_sq(u: &Expr, v: &Expr) -> Option<Expr> {
    if is_one(u)
        && let Some(("tan", arg)) = get_trig_sq(v)
    {
        return Some(arg);
    }
    if is_one(v)
        && let Some(("tan", arg)) = get_trig_sq(u)
    {
        return Some(arg);
    }
    None
}

// Helper for 1 + cot^2(x)
fn is_one_plus_cot_sq(u: &Expr, v: &Expr) -> Option<Expr> {
    if is_one(u)
        && let Some(("cot", arg)) = get_trig_sq(v)
    {
        return Some(arg);
    }
    if is_one(v)
        && let Some(("cot", arg)) = get_trig_sq(u)
    {
        return Some(arg);
    }
    None
}

fn is_one(expr: &Expr) -> bool {
    matches!(expr, Expr::Number(n) if (n - 1.0).abs() < 1e-10)
}

// Helper to extract (name, arg) from trig(arg)^2
fn get_trig_sq(expr: &Expr) -> Option<(&str, Expr)> {
    if let Expr::Pow(base, exp) = expr
        && matches!(**exp, Expr::Number(n) if n == 2.0)
        && let Expr::FunctionCall { name, args } = &**base
        && args.len() == 1
    {
        return Some((name.as_str(), args[0].clone()));
    }
    None
}

// Helper for sin(x)cos(y) + cos(x)sin(y)
fn is_sin_sum(u: &Expr, v: &Expr) -> Option<(Expr, Expr)> {
    if let (Some((s1, c1)), Some((s2, c2))) = (get_sin_cos_args(u), get_sin_cos_args(v))
        && s1 == c2
        && c1 == s2
    {
        return Some((s1, c1));
    }
    None
}

// Helper for sin(x)cos(y) - cos(x)sin(y)
fn is_sin_diff(u: &Expr, v: &Expr) -> Option<(Expr, Expr)> {
    if let (Some((s1, c1)), Some((c2, s2))) = (get_sin_cos_args(u), get_sin_cos_args(v)) {
        // u = sin(x)cos(y), v = cos(x)sin(y)
        // x = s1, y = c1. Check if c2=x, s2=y
        if s1 == c2 && c1 == s2 {
            return Some((s1, c1));
        }
    }
    None
}

// Helper for cos(x)cos(y) - sin(x)sin(y)
fn is_cos_sum(u: &Expr, v: &Expr) -> Option<(Expr, Expr)> {
    if let (Some((c1, c2)), Some((s1, s2))) = (get_cos_cos_args(u), get_sin_sin_args(v)) {
        // u = cos(x)cos(y), v = sin(x)sin(y)
        // Check if {c1, c2} == {s1, s2}
        if (c1 == s1 && c2 == s2) || (c1 == s2 && c2 == s1) {
            return Some((c1, c2));
        }
    }
    None
}

// Helper for cos(x)cos(y) + sin(x)sin(y)
fn is_cos_diff(u: &Expr, v: &Expr) -> Option<(Expr, Expr)> {
    if let (Some((c1, c2)), Some((s1, s2))) = (get_cos_cos_args(u), get_sin_sin_args(v)) {
        // u = cos(x)cos(y), v = sin(x)sin(y)
        // Check if {c1, c2} == {s1, s2}
        if (c1 == s1 && c2 == s2) || (c1 == s2 && c2 == s1) {
            return Some((c1, c2));
        }
    }
    None
}

// Extracts (x, y) from sin(x)*cos(y) or cos(y)*sin(x)
fn get_sin_cos_args(expr: &Expr) -> Option<(Expr, Expr)> {
    if let Expr::Mul(lhs, rhs) = expr {
        // Check sin(x)*cos(y)
        if let (
            Expr::FunctionCall { name: n1, args: a1 },
            Expr::FunctionCall { name: n2, args: a2 },
        ) = (&**lhs, &**rhs)
            && n1 == "sin"
            && n2 == "cos"
            && a1.len() == 1
            && a2.len() == 1
        {
            return Some((a1[0].clone(), a2[0].clone()));
        }
        // Check cos(y)*sin(x)
        if let (
            Expr::FunctionCall { name: n1, args: a1 },
            Expr::FunctionCall { name: n2, args: a2 },
        ) = (&**lhs, &**rhs)
            && n1 == "cos"
            && n2 == "sin"
            && a1.len() == 1
            && a2.len() == 1
        {
            return Some((a2[0].clone(), a1[0].clone()));
        }
    }
    None
}

// Extracts (x, y) from cos(x)*cos(y)
fn get_cos_cos_args(expr: &Expr) -> Option<(Expr, Expr)> {
    if let Expr::Mul(lhs, rhs) = expr
        && let (
            Expr::FunctionCall { name: n1, args: a1 },
            Expr::FunctionCall { name: n2, args: a2 },
        ) = (&**lhs, &**rhs)
        && n1 == "cos"
        && n2 == "cos"
        && a1.len() == 1
        && a2.len() == 1
    {
        return Some((a1[0].clone(), a2[0].clone()));
    }
    None
}

// Extracts (x, y) from sin(x)*sin(y)
fn get_sin_sin_args(expr: &Expr) -> Option<(Expr, Expr)> {
    if let Expr::Mul(lhs, rhs) = expr
        && let (
            Expr::FunctionCall { name: n1, args: a1 },
            Expr::FunctionCall { name: n2, args: a2 },
        ) = (&**lhs, &**rhs)
        && n1 == "sin"
        && n2 == "sin"
        && a1.len() == 1
        && a2.len() == 1
    {
        return Some((a1[0].clone(), a2[0].clone()));
    }
    None
}
