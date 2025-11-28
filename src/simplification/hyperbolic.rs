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
                            if let Expr::Mul(a, b) = content {
                                if matches!(**a, Expr::Number(n) if n == -1.0) {
                                    return Expr::Mul(
                                        Box::new(Expr::Number(-1.0)),
                                        Box::new(Expr::FunctionCall {
                                            name: "sinh".to_string(),
                                            args: vec![*b.clone()],
                                        }),
                                    );
                                }
                            }
                        } else {
                            // tanh(-x) = -tanh(x)
                            if let Expr::Mul(a, b) = content {
                                if matches!(**a, Expr::Number(n) if n == -1.0) {
                                    return Expr::Mul(
                                        Box::new(Expr::Number(-1.0)),
                                        Box::new(Expr::FunctionCall {
                                            name: "tanh".to_string(),
                                            args: vec![*b.clone()],
                                        }),
                                    );
                                }
                            }
                        }
                    }
                    "cosh" | "sech" => {
                        if matches!(content, Expr::Number(n) if *n == 0.0) {
                            return Expr::Number(1.0);
                        }
                        if name == "cosh" {
                            // cosh(-x) = cosh(x)
                            if let Expr::Mul(a, b) = content {
                                if matches!(**a, Expr::Number(n) if n == -1.0) {
                                    return Expr::FunctionCall {
                                        name: "cosh".to_string(),
                                        args: vec![*b.clone()],
                                    };
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Expr::Sub(u, v) => {
            // cosh^2(x) - sinh^2(x) = 1
            if let (Some((name1, arg1)), Some((name2, arg2))) = (get_hyp_sq(u), get_hyp_sq(v)) {
                if arg1 == arg2 && name1 == "cosh" && name2 == "sinh" {
                    return Expr::Number(1.0);
                }
            }
            // 1 - tanh^2(x) = sech^2(x)
            if let (Expr::Number(n), Some((name, arg))) = (&**u, get_hyp_sq(v)) {
                if *n == 1.0 && name == "tanh" {
                    return Expr::Pow(
                        Box::new(Expr::FunctionCall {
                            name: "sech".to_string(),
                            args: vec![arg],
                        }),
                        Box::new(Expr::Number(2.0)),
                    );
                }
            }
            // coth^2(x) - 1 = csch^2(x)
            if let (Some((name, arg)), Expr::Number(n)) = (get_hyp_sq(u), &**v) {
                if *n == 1.0 && name == "coth" {
                    return Expr::Pow(
                        Box::new(Expr::FunctionCall {
                            name: "csch".to_string(),
                            args: vec![arg],
                        }),
                        Box::new(Expr::Number(2.0)),
                    );
                }
            }
        }
        Expr::Div(numerator, denominator) => {
            // Check for sinh: (e^x - e^-x) / 2
            if let (Expr::Sub(u, v), Expr::Number(d)) = (&**numerator, &**denominator) {
                if *d == 2.0 {
                    if let Some(arg) = is_exp_diff(u, v) {
                        return Expr::FunctionCall {
                            name: "sinh".to_string(),
                            args: vec![arg],
                        };
                    }
                }
            }
            // Check for cosh: (e^x + e^-x) / 2
            if let (Expr::Add(u, v), Expr::Number(d)) = (&**numerator, &**denominator) {
                if *d == 2.0 {
                    if let Some(arg) = is_exp_sum(u, v) {
                        return Expr::FunctionCall {
                            name: "cosh".to_string(),
                            args: vec![arg],
                        };
                    }
                }
            }
            // Check for tanh: (e^x - e^-x) / (e^x + e^-x)
            if let (Expr::Sub(u_sub, v_sub), Expr::Add(u_add, v_add)) =
                (&**numerator, &**denominator)
            {
                if let Some(arg1) = is_exp_diff(u_sub, v_sub) {
                    if let Some(arg2) = is_exp_sum(u_add, v_add) {
                        if arg1 == arg2 {
                            return Expr::FunctionCall {
                                name: "tanh".to_string(),
                                args: vec![arg1],
                            };
                        }
                    }
                }
            }
            // Check for coth: (e^x + e^-x) / (e^x - e^-x)
            if let (Expr::Add(u_add, v_add), Expr::Sub(u_sub, v_sub)) =
                (&**numerator, &**denominator)
            {
                if let Some(arg1) = is_exp_sum(u_add, v_add) {
                    if let Some(arg2) = is_exp_diff(u_sub, v_sub) {
                        if arg1 == arg2 {
                            return Expr::FunctionCall {
                                name: "coth".to_string(),
                                args: vec![arg1],
                            };
                        }
                    }
                }
            }
            // Check for sech: 2 / (e^x + e^-x)
            if let (Expr::Number(n), Expr::Add(u, v)) = (&**numerator, &**denominator) {
                if *n == 2.0 {
                    if let Some(arg) = is_exp_sum(u, v) {
                        return Expr::FunctionCall {
                            name: "sech".to_string(),
                            args: vec![arg],
                        };
                    }
                }
            }
            // Check for csch: 2 / (e^x - e^-x)
            if let (Expr::Number(n), Expr::Sub(u, v)) = (&**numerator, &**denominator) {
                if *n == 2.0 {
                    if let Some(arg) = is_exp_diff(u, v) {
                        return Expr::FunctionCall {
                            name: "csch".to_string(),
                            args: vec![arg],
                        };
                    }
                }
            }
        }
        _ => {}
    }
    expr
}

// Helper to extract (name, arg) from hyp(arg)^2
fn get_hyp_sq(expr: &Expr) -> Option<(&str, Expr)> {
    if let Expr::Pow(base, exp) = expr {
        if matches!(**exp, Expr::Number(n) if n == 2.0) {
            if let Expr::FunctionCall { name, args } = &**base {
                if args.len() == 1 {
                    return Some((name.as_str(), args[0].clone()));
                }
            }
        }
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
    {
        if n1 == "exp" && n2 == "exp" && args1.len() == 1 && args2.len() == 1 {
            if is_negation(&args1[0], &args2[0]) {
                return Some(args1[0].clone());
            }
        }
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
    {
        if n1 == "exp" && n2 == "exp" && args1.len() == 1 && args2.len() == 1 {
            if is_negation(&args1[0], &args2[0]) {
                return Some(args1[0].clone());
            }
        }
    }
    None
}

// Check if b is negation of a, or a is negation of b
fn is_negation(a: &Expr, b: &Expr) -> bool {
    // Check b = -1 * a
    if let Expr::Mul(lhs, rhs) = b {
        if let Expr::Number(n) = **lhs {
            if n == -1.0 && **rhs == *a {
                return true;
            }
        }
        if let Expr::Number(n) = **rhs {
            if n == -1.0 && **lhs == *a {
                return true;
            }
        }
    }

    // Check a = -1 * b
    if let Expr::Mul(lhs, rhs) = a {
        if let Expr::Number(n) = **lhs {
            if n == -1.0 && **rhs == *b {
                return true;
            }
        }
        if let Expr::Number(n) = **rhs {
            if n == -1.0 && **lhs == *b {
                return true;
            }
        }
    }

    false
}
