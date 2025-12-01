/// Rule for cbrt(x^n) = x^(n/3)
pub struct CbrtPowerRule;

impl Rule for CbrtPowerRule {
    fn name(&self) -> &'static str {
        "cbrt_power"
    }

    fn priority(&self) -> i32 {
        85
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Root
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr {
            if name == "cbrt" && args.len() == 1 {
                if let Expr::Pow(base, exp) = &args[0] {
                    // Create new exponent: exp / 3
                    let new_exp = Expr::Div(exp.clone(), Box::new(Expr::Number(3.0)));

                    // Simplify the division immediately
                    // This handles cases like 3/3 → 1, 6/3 → 2
                    let simplified_exp = crate::simplification::simplify(new_exp);

                    // If exponent simplified to 1, return base directly
                    if matches!(simplified_exp, Expr::Number(n) if n == 1.0) {
                        return Some((**base).clone());
                    }

                    return Some(Expr::Pow(base.clone(), Box::new(simplified_exp)));
                }
            }
        }
        None
    }
}
