use crate::Expr;

pub fn apply_log_exp_rules(expr: Expr) -> Expr {
    if let Expr::FunctionCall { name, args } = &expr
        && args.len() == 1
    {
        let content = &args[0];
        match name.as_str() {
            "exp" => {
                if matches!(content, Expr::Number(n) if *n == 0.0) {
                    return Expr::Number(1.0);
                }
                // exp(ln(x)) = x
                if let Expr::FunctionCall {
                    name: inner_name,
                    args: inner_args,
                } = content
                    && inner_name == "ln"
                    && inner_args.len() == 1
                {
                    return inner_args[0].clone();
                }
            }
            "ln" => {
                if matches!(content, Expr::Number(n) if *n == 1.0) {
                    return Expr::Number(0.0);
                }
                // ln(exp(x)) = x
                if let Expr::FunctionCall {
                    name: inner_name,
                    args: inner_args,
                } = content
                    && inner_name == "exp"
                    && inner_args.len() == 1
                {
                    return inner_args[0].clone();
                }
                // ln(x^n) = n * ln(x)
                if let Expr::Pow(base, exp) = content {
                    return Expr::Mul(
                        exp.clone(),
                        Box::new(Expr::FunctionCall {
                            name: "ln".to_string(),
                            args: vec![*base.clone()],
                        }),
                    );
                }
                // ln(sqrt(x)) = 0.5 * ln(x)
                if let Expr::FunctionCall {
                    name: inner_name,
                    args: inner_args,
                } = content
                    && inner_name == "sqrt"
                    && inner_args.len() == 1
                {
                    return Expr::Mul(
                        Box::new(Expr::Div(
                            Box::new(Expr::Number(1.0)),
                            Box::new(Expr::Number(2.0)),
                        )),
                        Box::new(Expr::FunctionCall {
                            name: "ln".to_string(),
                            args: vec![inner_args[0].clone()],
                        }),
                    );
                }
                // ln(cbrt(x)) = (1/3) * ln(x)
                if let Expr::FunctionCall {
                    name: inner_name,
                    args: inner_args,
                } = content
                    && inner_name == "cbrt"
                    && inner_args.len() == 1
                {
                    return Expr::Mul(
                        Box::new(Expr::Div(
                            Box::new(Expr::Number(1.0)),
                            Box::new(Expr::Number(3.0)),
                        )),
                        Box::new(Expr::FunctionCall {
                            name: "ln".to_string(),
                            args: vec![inner_args[0].clone()],
                        }),
                    );
                }
            }
            "log10" => {
                if matches!(content, Expr::Number(n) if *n == 1.0) {
                    return Expr::Number(0.0);
                }
                if matches!(content, Expr::Number(n) if *n == 10.0) {
                    return Expr::Number(1.0);
                }
                // log10(x^n) = n * log10(x)
                if let Expr::Pow(base, exp) = content {
                    return Expr::Mul(
                        exp.clone(),
                        Box::new(Expr::FunctionCall {
                            name: "log10".to_string(),
                            args: vec![*base.clone()],
                        }),
                    );
                }
                // log10(sqrt(x)) = 0.5 * log10(x)
                if let Expr::FunctionCall {
                    name: inner_name,
                    args: inner_args,
                } = content
                    && inner_name == "sqrt"
                    && inner_args.len() == 1
                {
                    return Expr::Mul(
                        Box::new(Expr::Div(
                            Box::new(Expr::Number(1.0)),
                            Box::new(Expr::Number(2.0)),
                        )),
                        Box::new(Expr::FunctionCall {
                            name: "log10".to_string(),
                            args: vec![inner_args[0].clone()],
                        }),
                    );
                }
                // log10(cbrt(x)) = (1/3) * log10(x)
                if let Expr::FunctionCall {
                    name: inner_name,
                    args: inner_args,
                } = content
                    && inner_name == "cbrt"
                    && inner_args.len() == 1
                {
                    return Expr::Mul(
                        Box::new(Expr::Div(
                            Box::new(Expr::Number(1.0)),
                            Box::new(Expr::Number(3.0)),
                        )),
                        Box::new(Expr::FunctionCall {
                            name: "log10".to_string(),
                            args: vec![inner_args[0].clone()],
                        }),
                    );
                }
            }
            "log2" => {
                if matches!(content, Expr::Number(n) if *n == 1.0) {
                    return Expr::Number(0.0);
                }
                if matches!(content, Expr::Number(n) if *n == 2.0) {
                    return Expr::Number(1.0);
                }
                // log2(x^n) = n * log2(x)
                if let Expr::Pow(base, exp) = content {
                    return Expr::Mul(
                        exp.clone(),
                        Box::new(Expr::FunctionCall {
                            name: "log2".to_string(),
                            args: vec![*base.clone()],
                        }),
                    );
                }
                // log2(sqrt(x)) = 0.5 * log2(x)
                if let Expr::FunctionCall {
                    name: inner_name,
                    args: inner_args,
                } = content
                    && inner_name == "sqrt"
                    && inner_args.len() == 1
                {
                    return Expr::Mul(
                        Box::new(Expr::Div(
                            Box::new(Expr::Number(1.0)),
                            Box::new(Expr::Number(2.0)),
                        )),
                        Box::new(Expr::FunctionCall {
                            name: "log2".to_string(),
                            args: vec![inner_args[0].clone()],
                        }),
                    );
                }
                // log2(cbrt(x)) = (1/3) * log2(x)
                if let Expr::FunctionCall {
                    name: inner_name,
                    args: inner_args,
                } = content
                    && inner_name == "cbrt"
                    && inner_args.len() == 1
                {
                    return Expr::Mul(
                        Box::new(Expr::Div(
                            Box::new(Expr::Number(1.0)),
                            Box::new(Expr::Number(3.0)),
                        )),
                        Box::new(Expr::FunctionCall {
                            name: "log2".to_string(),
                            args: vec![inner_args[0].clone()],
                        }),
                    );
                }
            }
            _ => {}
        }
    }

    // Combination rules
    match &expr {
        // ln(a) + ln(b) = ln(a * b)
        Expr::Add(u, v) => {
            if let (Some(arg1), Some(arg2)) = (get_ln_arg(u), get_ln_arg(v)) {
                return Expr::FunctionCall {
                    name: "ln".to_string(),
                    args: vec![Expr::Mul(Box::new(arg1), Box::new(arg2))],
                };
            }
            // ln(a) - ln(b) = ln(a / b)
            // Handled by Add rule if subtraction is converted to Add(a, -b)
            // But we also have explicit Sub
        }
        Expr::Sub(u, v) => {
            if let (Some(arg1), Some(arg2)) = (get_ln_arg(u), get_ln_arg(v)) {
                return Expr::FunctionCall {
                    name: "ln".to_string(),
                    args: vec![Expr::Div(Box::new(arg1), Box::new(arg2))],
                };
            }
        }
        // exp(a) * exp(b) = exp(a + b)
        Expr::Mul(u, v) => {
            if let (Some(arg1), Some(arg2)) = (get_exp_arg(u), get_exp_arg(v)) {
                return Expr::FunctionCall {
                    name: "exp".to_string(),
                    args: vec![Expr::Add(Box::new(arg1), Box::new(arg2))],
                };
            }
        }
        _ => {}
    }

    expr
}

fn get_ln_arg(expr: &Expr) -> Option<Expr> {
    if let Expr::FunctionCall { name, args } = expr
        && name == "ln"
        && args.len() == 1
    {
        return Some(args[0].clone());
    }
    None
}

fn get_exp_arg(expr: &Expr) -> Option<Expr> {
    if let Expr::FunctionCall { name, args } = expr
        && name == "exp"
        && args.len() == 1
    {
        return Some(args[0].clone());
    }
    None
}
