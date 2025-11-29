use crate::Expr;

pub fn apply_root_rules(expr: Expr) -> Expr {
    if let Expr::FunctionCall { name, args } = &expr
        && args.len() == 1
    {
        let content = &args[0];
        match name.as_str() {
            "sqrt" => {
                if matches!(content, Expr::Number(n) if *n == 0.0) {
                    return Expr::Number(0.0);
                }
                if matches!(content, Expr::Number(n) if *n == 1.0) {
                    return Expr::Number(1.0);
                }
                // sqrt(x^n) -> x^(n/2)
                if let Expr::Pow(base, exp) = content
                    && let Expr::Number(n) = **exp
                {
                    // If n is even, result is x^(n/2)
                    if n % 2.0 == 0.0 {
                        let new_exp = n / 2.0;
                        if new_exp == 1.0 {
                            return *base.clone();
                        }
                        return Expr::Pow(base.clone(), Box::new(Expr::Number(new_exp)));
                    }
                }
                // sqrt(sqrt(x)) -> x^(1/4)
                if let Expr::FunctionCall {
                    name: inner_name,
                    args: inner_args,
                } = content
                    && inner_name == "sqrt"
                    && inner_args.len() == 1
                {
                    return Expr::Pow(
                        Box::new(inner_args[0].clone()),
                        Box::new(Expr::Number(0.25)),
                    );
                }
            }
            "cbrt" => {
                if matches!(content, Expr::Number(n) if *n == 0.0) {
                    return Expr::Number(0.0);
                }
                if matches!(content, Expr::Number(n) if *n == 1.0) {
                    return Expr::Number(1.0);
                }
                // cbrt(x^n) -> x^(n/3)
                if let Expr::Pow(base, exp) = content
                    && let Expr::Number(n) = **exp
                {
                    // If n is multiple of 3, result is x^(n/3)
                    if n % 3.0 == 0.0 {
                        let new_exp = n / 3.0;
                        if new_exp == 1.0 {
                            return *base.clone();
                        }
                        return Expr::Pow(base.clone(), Box::new(Expr::Number(new_exp)));
                    }
                }
            }
            _ => {}
        }
    }
    // Convert fractional powers to roots
    // x^(1/2) -> sqrt(x)
    if let Expr::Pow(base, exp) = &expr
        && let Expr::Div(num, den) = &**exp
        && matches!(**num, Expr::Number(n) if n == 1.0)
        && matches!(**den, Expr::Number(n) if n == 2.0)
    {
        return Expr::FunctionCall {
            name: "sqrt".to_string(),
            args: vec![*base.clone()],
        };
    }
    // x^0.5 -> sqrt(x)
    if let Expr::Pow(base, exp) = &expr
        && matches!(**exp, Expr::Number(n) if n == 0.5)
    {
        return Expr::FunctionCall {
            name: "sqrt".to_string(),
            args: vec![*base.clone()],
        };
    }

    // x^(1/3) -> cbrt(x)
    if let Expr::Pow(base, exp) = &expr
        && let Expr::Div(num, den) = &**exp
        && matches!(**num, Expr::Number(n) if n == 1.0)
        && matches!(**den, Expr::Number(n) if n == 3.0)
    {
        return Expr::FunctionCall {
            name: "cbrt".to_string(),
            args: vec![*base.clone()],
        };
    }
    // x^(1/3) approx 0.333... (handle if needed, but usually we keep fraction)

    expr
}
