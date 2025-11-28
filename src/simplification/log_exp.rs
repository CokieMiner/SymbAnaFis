use crate::Expr;

pub fn apply_log_exp_rules(expr: Expr) -> Expr {
    if let Expr::FunctionCall { name, args } = &expr
        && args.len() == 1 {
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
                    && inner_name == "ln" && inner_args.len() == 1 {
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
                    && inner_name == "exp" && inner_args.len() == 1 {
                    return inner_args[0].clone();
                }
            }
            "log10" => {
                if matches!(content, Expr::Number(n) if *n == 1.0) {
                    return Expr::Number(0.0);
                }
                if matches!(content, Expr::Number(n) if *n == 10.0) {
                    return Expr::Number(1.0);
                }
            }
            "log2" => {
                if matches!(content, Expr::Number(n) if *n == 1.0) {
                    return Expr::Number(0.0);
                }
                if matches!(content, Expr::Number(n) if *n == 2.0) {
                    return Expr::Number(1.0);
                }
            }
            _ => {}
        }
    }
    expr
}
