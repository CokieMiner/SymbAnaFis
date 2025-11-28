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
                // sqrt(x^2) -> x (assuming positive)
                if let Expr::Pow(base, exp) = content
                    && matches!(**exp, Expr::Number(n) if n == 2.0)
                {
                    return *base.clone();
                }
            }
            "cbrt" => {
                if matches!(content, Expr::Number(n) if *n == 0.0) {
                    return Expr::Number(0.0);
                }
                if matches!(content, Expr::Number(n) if *n == 1.0) {
                    return Expr::Number(1.0);
                }
                // cbrt(x^3) -> x
                if let Expr::Pow(base, exp) = content
                    && matches!(**exp, Expr::Number(n) if n == 3.0)
                {
                    return *base.clone();
                }
            }
            _ => {}
        }
    }
    expr
}
