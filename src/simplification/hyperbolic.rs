use crate::Expr;

pub fn apply_hyperbolic_rules(expr: Expr) -> Expr {
    match &expr {
        Expr::FunctionCall { name, args } => {
            if args.len() == 1 {
                let content = &args[0];
                match name.as_str() {
                    "sinh" | "tanh" => {
                        if matches!(content, Expr::Number(n) if *n == 0.0) {
                            return Expr::Number(0.0);
                        }
                        if name == "sinh" {
                            // sinh(-x) = -sinh(x)
                            if let Expr::Mul(a, b) = content
                                && matches!(**a, Expr::Number(n) if n == -1.0)
                            {
                                return Expr::Mul(
                                    Box::new(Expr::Number(-1.0)),
                                    Box::new(Expr::FunctionCall {
                                        name: "sinh".to_string(),
                                        args: vec![*b.clone()],
                                    }),
                                );
                            }
                            // sinh(asinh(x)) = x
                            if let Expr::FunctionCall {
                                name: inner_name,
                                args: inner_args,
                            } = content
                                && inner_name == "asinh"
                                && inner_args.len() == 1
                            {
                                return inner_args[0].clone();
                            }
                        } else {
                            // tanh(-x) = -tanh(x)
                            if let Expr::Mul(a, b) = content
                                && matches!(**a, Expr::Number(n) if n == -1.0)
                            {
                                return Expr::Mul(
                                    Box::new(Expr::Number(-1.0)),
                                    Box::new(Expr::FunctionCall {
                                        name: "tanh".to_string(),
                                        args: vec![*b.clone()],
                                    }),
                                );
                            }
                            // tanh(atanh(x)) = x
                            if let Expr::FunctionCall {
                                name: inner_name,
                                args: inner_args,
                            } = content
                                && inner_name == "atanh"
                                && inner_args.len() == 1
                            {
                                return inner_args[0].clone();
                            }
                        }
                    }
                    "cosh" | "sech" => {
                        if matches!(content, Expr::Number(n) if *n == 0.0) {
                            return Expr::Number(1.0);
                        }
                        if name == "cosh" {
                            // cosh(-x) = cosh(x)
                            if let Expr::Mul(a, b) = content
                                && matches!(**a, Expr::Number(n) if n == -1.0)
                            {
                                return Expr::FunctionCall {
                                    name: "cosh".to_string(),
                                    args: vec![*b.clone()],
                                };
                            }
                            // cosh(acosh(x)) = x
                            if let Expr::FunctionCall {
                                name: inner_name,
                                args: inner_args,
                            } = content
                                && inner_name == "acosh"
                                && inner_args.len() == 1
                            {
                                return inner_args[0].clone();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Expr::Sub(u, v) => {
            // cosh^2(x) - sinh^2(x) = 1
            if let (Some((name1, arg1)), Some((name2, arg2))) = (get_hyp_sq(u), get_hyp_sq(v))
                && arg1 == arg2
                && name1 == "cosh"
                && name2 == "sinh"
            {
                return Expr::Number(1.0);
            }
            // 1 - tanh^2(x) = sech^2(x)
            if let (Expr::Number(n), Some((name, arg))) = (&**u, get_hyp_sq(v))
                && *n == 1.0
                && name == "tanh"
            {
                return Expr::Pow(
                    Box::new(Expr::FunctionCall {
                        name: "sech".to_string(),
                        args: vec![arg],
                    }),
                    Box::new(Expr::Number(2.0)),
                );
            }
            // coth^2(x) - 1 = csch^2(x)
            if let (Some((name, arg)), Expr::Number(n)) = (get_hyp_sq(u), &**v)
                && *n == 1.0
                && name == "coth"
            {
                return Expr::Pow(
                    Box::new(Expr::FunctionCall {
                        name: "csch".to_string(),
                        args: vec![arg],
                    }),
                    Box::new(Expr::Number(2.0)),
                );
            }
        }
        Expr::Add(u, v) => {
            // cosh^2(x) + sinh^2(x) = cosh(2x)
            if let (Some((name1, arg1)), Some((name2, arg2))) = (get_hyp_sq(u), get_hyp_sq(v))
                && arg1 == arg2
                && ((name1 == "sinh" && name2 == "cosh") || (name1 == "cosh" && name2 == "sinh"))
            {
                return Expr::FunctionCall {
                    name: "cosh".to_string(),
                    args: vec![Expr::Mul(Box::new(Expr::Number(2.0)), Box::new(arg1))],
                };
            }
            // cosh^2(x) + (-1 * sinh^2(x)) = 1  (after algebraic simplification)
            if let (Some((name1, arg1)), Expr::Mul(lhs, rhs)) = (get_hyp_sq(u), &**v)
                && let Expr::Number(n) = **lhs
                && n == -1.0
                && let Some((name2, arg2)) = get_hyp_sq(rhs)
                && arg1 == arg2
                && name1 == "cosh"
                && name2 == "sinh"
            {
                return Expr::Number(1.0);
            }
            // (-1 * sinh^2(x)) + cosh^2(x) = 1  (after algebraic simplification)
            if let (Expr::Mul(lhs, rhs), Some((name2, arg2))) = (&**u, get_hyp_sq(v))
                && let Expr::Number(n) = **lhs
                && n == -1.0
                && let Some((name1, arg1)) = get_hyp_sq(rhs)
                && arg1 == arg2
                && name1 == "sinh"
                && name2 == "cosh"
            {
                return Expr::Number(1.0);
            }
            // (-1 * sinh^2(x)) + cosh^2(x) = 1  (after algebraic simplification)
            if let (Expr::Mul(lhs, rhs), Some((name2, arg2))) = (&**u, get_hyp_sq(v))
                && let Expr::Number(n) = **lhs
                && n == -1.0
                && let Some((name1, arg1)) = get_hyp_sq(rhs)
                && arg1 == arg2
                && name1 == "sinh"
                && name2 == "cosh"
            {
                return Expr::Number(1.0);
            }
            // 1 + (-1 * tanh^2(x)) = sech^2(x)  (after algebraic simplification)
            if let (Expr::Number(n), Expr::Mul(lhs, rhs)) = (&**u, &**v)
                && *n == 1.0
                && let Expr::Number(m) = **lhs
                && m == -1.0
                && let Some((name, arg)) = get_hyp_sq(rhs)
                && name == "tanh"
            {
                return Expr::Pow(
                    Box::new(Expr::FunctionCall {
                        name: "sech".to_string(),
                        args: vec![arg],
                    }),
                    Box::new(Expr::Number(2.0)),
                );
            }
            // (-1 * tanh^2(x)) + 1 = sech^2(x)  (after algebraic simplification)
            if let (Expr::Mul(lhs, rhs), Expr::Number(n)) = (&**u, &**v)
                && *n == 1.0
                && let Expr::Number(m) = **lhs
                && m == -1.0
                && let Some((name, arg)) = get_hyp_sq(rhs)
                && name == "tanh"
            {
                return Expr::Pow(
                    Box::new(Expr::FunctionCall {
                        name: "sech".to_string(),
                        args: vec![arg],
                    }),
                    Box::new(Expr::Number(2.0)),
                );
            }
            // coth^2(x) + (-1) = csch^2(x)  (after algebraic simplification)
            if let (Some((name, arg)), Expr::Number(n)) = (get_hyp_sq(u), &**v)
                && name == "coth"
                && *n == -1.0
            {
                return Expr::Pow(
                    Box::new(Expr::FunctionCall {
                        name: "csch".to_string(),
                        args: vec![arg],
                    }),
                    Box::new(Expr::Number(2.0)),
                );
            }
            // (-1) + coth^2(x) = csch^2(x)  (after algebraic simplification)
            if let (Expr::Number(n), Some((name, arg))) = (&**u, get_hyp_sq(v))
                && name == "coth"
                && *n == -1.0
            {
                return Expr::Pow(
                    Box::new(Expr::FunctionCall {
                        name: "csch".to_string(),
                        args: vec![arg],
                    }),
                    Box::new(Expr::Number(2.0)),
                );
            }
        }
        Expr::Div(numerator, denominator) => {
            // Check for sinh: (e^x - e^-x) / 2
            if let (Expr::Sub(u, v), Expr::Number(d)) = (&**numerator, &**denominator)
                && *d == 2.0
                && let Some(arg) = is_exp_diff(u, v)
            {
                return Expr::FunctionCall {
                    name: "sinh".to_string(),
                    args: vec![arg],
                };
            }
            // Check for sinh: (e^x + (-1 * e^-x)) / 2  (after algebraic simplification)
            if let (Expr::Add(u, v), Expr::Number(d)) = (&**numerator, &**denominator)
                && *d == 2.0
                && let Some(arg) = is_exp_diff_negated(u, v)
            {
                return Expr::FunctionCall {
                    name: "sinh".to_string(),
                    args: vec![arg],
                };
            }
            // Check for cosh: (e^x + e^-x) / 2
            if let (Expr::Add(u, v), Expr::Number(d)) = (&**numerator, &**denominator)
                && *d == 2.0
                && let Some(arg) = is_exp_sum(u, v)
            {
                return Expr::FunctionCall {
                    name: "cosh".to_string(),
                    args: vec![arg],
                };
            }
            // Check for tanh: (e^x - e^-x) / (e^x + e^-x)
            if let (Expr::Sub(u_sub, v_sub), Expr::Add(u_add, v_add)) =
                (&**numerator, &**denominator)
                && let Some(arg1) = is_exp_diff(u_sub, v_sub)
                && let Some(arg2) = is_exp_sum(u_add, v_add)
                && arg1 == arg2
            {
                return Expr::FunctionCall {
                    name: "tanh".to_string(),
                    args: vec![arg1],
                };
            }
            // Check for tanh: (e^x + (-1 * e^-x)) / (e^x + e^-x)  (after algebraic simplification)
            if let (Expr::Add(u_sub, v_sub), Expr::Add(u_add, v_add)) =
                (&**numerator, &**denominator)
                && let Some(arg1) = is_exp_diff_negated(u_sub, v_sub)
                && let Some(arg2) = is_exp_sum(u_add, v_add)
                && arg1 == arg2
            {
                return Expr::FunctionCall {
                    name: "tanh".to_string(),
                    args: vec![arg1],
                };
            }
            // Check for coth: (e^x + e^-x) / (e^x - e^-x)
            if let (Expr::Add(u_add, v_add), Expr::Sub(u_sub, v_sub)) =
                (&**numerator, &**denominator)
                && let Some(arg1) = is_exp_sum(u_add, v_add)
                && let Some(arg2) = is_exp_diff(u_sub, v_sub)
                && arg1 == arg2
            {
                return Expr::FunctionCall {
                    name: "coth".to_string(),
                    args: vec![arg1],
                };
            }
            // Check for coth: (e^x + e^-x) / (e^x + (-1 * e^-x))  (after algebraic simplification)
            if let (Expr::Add(u_add, v_add), Expr::Add(u_sub, v_sub)) =
                (&**numerator, &**denominator)
                && let Some(arg1) = is_exp_sum(u_add, v_add)
                && let Some(arg2) = is_exp_diff_negated(u_sub, v_sub)
                && arg1 == arg2
            {
                return Expr::FunctionCall {
                    name: "coth".to_string(),
                    args: vec![arg1],
                };
            }
            // Check for sech: 2 / (e^x + e^-x)
            if let (Expr::Number(n), Expr::Add(u, v)) = (&**numerator, &**denominator)
                && *n == 2.0
                && let Some(arg) = is_exp_sum(u, v)
            {
                return Expr::FunctionCall {
                    name: "sech".to_string(),
                    args: vec![arg],
                };
            }
            // Check for csch: 2 / (e^x - e^-x)
            if let (Expr::Number(n), Expr::Sub(u, v)) = (&**numerator, &**denominator)
                && *n == 2.0
                && let Some(arg) = is_exp_diff(u, v)
            {
                return Expr::FunctionCall {
                    name: "csch".to_string(),
                    args: vec![arg],
                };
            }
            // Check for csch: 2 / (e^x + (-1 * e^-x))  (after algebraic simplification)
            if let (Expr::Number(n), Expr::Add(u, v)) = (&**numerator, &**denominator)
                && *n == 2.0
                && let Some(arg) = is_exp_diff_negated(u, v)
            {
                return Expr::FunctionCall {
                    name: "csch".to_string(),
                    args: vec![arg],
                };
            }
            // sinh(x) / cosh(x) = tanh(x)
            if let (
                Expr::FunctionCall { name: n1, args: a1 },
                Expr::FunctionCall { name: n2, args: a2 },
            ) = (&**numerator, &**denominator)
                && n1 == "sinh"
                && n2 == "cosh"
                && a1.len() == 1
                && a2.len() == 1
                && a1[0] == a2[0]
            {
                return Expr::FunctionCall {
                    name: "tanh".to_string(),
                    args: a1.clone(),
                };
            }
            // cosh(x) / sinh(x) = coth(x)
            if let (
                Expr::FunctionCall { name: n1, args: a1 },
                Expr::FunctionCall { name: n2, args: a2 },
            ) = (&**numerator, &**denominator)
                && n1 == "cosh"
                && n2 == "sinh"
                && a1.len() == 1
                && a2.len() == 1
                && a1[0] == a2[0]
            {
                return Expr::FunctionCall {
                    name: "coth".to_string(),
                    args: a1.clone(),
                };
            }
            // 1 / cosh(x) = sech(x)
            if let (Expr::Number(n), Expr::FunctionCall { name, args }) =
                (&**numerator, &**denominator)
                && *n == 1.0
                && name == "cosh"
                && args.len() == 1
            {
                return Expr::FunctionCall {
                    name: "sech".to_string(),
                    args: args.clone(),
                };
            }
            // 1 / sinh(x) = csch(x)
            if let (Expr::Number(n), Expr::FunctionCall { name, args }) =
                (&**numerator, &**denominator)
                && *n == 1.0
                && name == "sinh"
                && args.len() == 1
            {
                return Expr::FunctionCall {
                    name: "csch".to_string(),
                    args: args.clone(),
                };
            }
        }
        _ => {}
    }
    expr
}

// Helper to extract (name, arg) from hyp(arg)^2
fn get_hyp_sq(expr: &Expr) -> Option<(&str, Expr)> {
    if let Expr::Pow(base, exp) = expr
        && matches!(**exp, Expr::Number(n) if n == 2.0)
        && let Expr::FunctionCall { name, args } = &**base
        && args.len() == 1
    {
        return Some((name.as_str(), args[0].clone()));
    }
    None
}

// Helper to check if u - v is exp(x) - exp(-x)
// Returns Some(x) if match found.
// If u=exp(x) and v=exp(-x), returns Some(x).
// If u=exp(-x) and v=exp(x), returns Some(-x).
fn is_exp_diff(u: &Expr, v: &Expr) -> Option<Expr> {
    if let (
        Expr::FunctionCall {
            name: n1,
            args: args1,
        },
        Expr::FunctionCall {
            name: n2,
            args: args2,
        },
    ) = (u, v)
        && n1 == "exp"
        && n2 == "exp"
        && args1.len() == 1
        && args2.len() == 1
        && is_negation(&args1[0], &args2[0])
    {
        return Some(args1[0].clone());
    }
    None
}

// Helper to check if u + v is exp(x) + exp(-x)
// Returns Some(x) if match found.
fn is_exp_sum(u: &Expr, v: &Expr) -> Option<Expr> {
    if let (
        Expr::FunctionCall {
            name: n1,
            args: args1,
        },
        Expr::FunctionCall {
            name: n2,
            args: args2,
        },
    ) = (u, v)
        && n1 == "exp"
        && n2 == "exp"
        && args1.len() == 1
        && args2.len() == 1
        && is_negation(&args1[0], &args2[0])
    {
        return Some(args1[0].clone());
    }
    None
}

// Helper to check if u + v is exp(x) + (-1 * exp(-x))
// Returns Some(x) if match found.
fn is_exp_diff_negated(u: &Expr, v: &Expr) -> Option<Expr> {
    // Check if u is exp(x) and v is -1 * exp(-x)
    if let (
        Expr::FunctionCall {
            name: n1,
            args: args1,
        },
        Expr::Mul(lhs, rhs),
    ) = (u, v)
        && n1 == "exp"
        && args1.len() == 1
        && let Expr::Number(n) = **lhs
        && n == -1.0
        && let Expr::FunctionCall {
            name: n2,
            args: args2,
        } = &**rhs
        && n2 == "exp"
        && args2.len() == 1
        && is_negation(&args1[0], &args2[0])
    {
        return Some(args1[0].clone());
    }
    // Check if v is exp(x) and u is -1 * exp(-x)
    if let (
        Expr::Mul(lhs, rhs),
        Expr::FunctionCall {
            name: n2,
            args: args2,
        },
    ) = (u, v)
        && n2 == "exp"
        && args2.len() == 1
        && let Expr::Number(n) = **lhs
        && n == -1.0
        && let Expr::FunctionCall {
            name: n1,
            args: args1,
        } = &**rhs
        && n1 == "exp"
        && args1.len() == 1
        && is_negation(&args2[0], &args1[0])
    {
        return Some(args2[0].clone());
    }
    None
}

// Check if b is negation of a, or a is negation of b
fn is_negation(a: &Expr, b: &Expr) -> bool {
    // Check b = -1 * a
    if let Expr::Mul(lhs, rhs) = b {
        if let Expr::Number(n) = **lhs
            && n == -1.0
            && **rhs == *a
        {
            return true;
        }
        if let Expr::Number(n) = **rhs
            && n == -1.0
            && **lhs == *a
        {
            return true;
        }
    }

    // Check a = -1 * b
    if let Expr::Mul(lhs, rhs) = a {
        if let Expr::Number(n) = **lhs
            && n == -1.0
            && **rhs == *b
        {
            return true;
        }
        if let Expr::Number(n) = **rhs
            && n == -1.0
            && **lhs == *b
        {
            return true;
        }
    }

    false
}
