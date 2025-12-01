use crate::ast::Expr;
use crate::simplification::rules::{Rule, RuleCategory, RuleContext};
use std::rc::Rc;

/// Rule for ln(1) = 0
pub struct LnOneRule;

impl Rule for LnOneRule {
    fn name(&self) -> &'static str {
        "ln_one"
    }

    fn priority(&self) -> i32 {
        95
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Exponential
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr {
            if name == "ln" && args.len() == 1 {
                if matches!(args[0], Expr::Number(n) if n == 1.0) {
                    return Some(Expr::Number(0.0));
                }
            }
        }
        None
    }
}

/// Rule for ln(e) = 1
pub struct LnERule;

impl Rule for LnERule {
    fn name(&self) -> &'static str {
        "ln_e"
    }

    fn priority(&self) -> i32 {
        95
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Exponential
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr {
            if name == "ln" && args.len() == 1 {
                // Check for ln(exp(1))
                if matches!(&args[0], Expr::FunctionCall { name: exp_name, args: exp_args }
                           if exp_name == "exp" && exp_args.len() == 1 && matches!(exp_args[0], Expr::Number(n) if n == 1.0))
                {
                    return Some(Expr::Number(1.0));
                }
                // Check for ln(e) where e is a symbol
                if let Expr::Symbol(s) = &args[0] {
                    if s == "e" && !_context.variables.contains("e") {
                        return Some(Expr::Number(1.0));
                    }
                }
            }
        }
        None
    }
}

/// Rule for exp(0) = 1
pub struct ExpZeroRule;

impl Rule for ExpZeroRule {
    fn name(&self) -> &'static str {
        "exp_zero"
    }

    fn priority(&self) -> i32 {
        95
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Exponential
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr {
            if name == "exp" && args.len() == 1 {
                if matches!(args[0], Expr::Number(n) if n == 0.0) {
                    return Some(Expr::Number(1.0));
                }
            }
        }
        None
    }
}

/// Rule for exp(ln(x)) = x (for x > 0)
pub struct ExpLnIdentityRule;

impl Rule for ExpLnIdentityRule {
    fn name(&self) -> &'static str {
        "exp_ln_identity"
    }

    fn priority(&self) -> i32 {
        90
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Exponential
    }

    fn alters_domain(&self) -> bool {
        true
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr {
            if name == "exp" && args.len() == 1 {
                if let Expr::FunctionCall {
                    name: inner_name,
                    args: inner_args,
                } = &args[0]
                {
                    if inner_name == "ln" && inner_args.len() == 1 {
                        return Some(inner_args[0].clone());
                    }
                }
            }
        }
        None
    }
}

/// Rule for ln(exp(x)) = x
pub struct LnExpIdentityRule;

impl Rule for LnExpIdentityRule {
    fn name(&self) -> &'static str {
        "ln_exp_identity"
    }

    fn priority(&self) -> i32 {
        90
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Exponential
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr {
            if name == "ln" && args.len() == 1 {
                // Check for ln(exp(x))
                if let Expr::FunctionCall {
                    name: inner_name,
                    args: inner_args,
                } = &args[0]
                {
                    if inner_name == "exp" && inner_args.len() == 1 {
                        return Some(inner_args[0].clone());
                    }
                }
                // Check for ln(e^x)
                if let Expr::Pow(base, exp) = &args[0] {
                    if let Expr::Symbol(b) = &**base {
                        if b == "e" {
                            return Some(*exp.clone());
                        }
                    }
                }
            }
        }
        None
    }
}

/// Rule for log(x^n) = n * log(x) for ln, log10, log2
pub struct LogPowerRule;

impl Rule for LogPowerRule {
    fn name(&self) -> &'static str {
        "log_power"
    }

    fn priority(&self) -> i32 {
        90
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Exponential
    }

    fn alters_domain(&self) -> bool {
        true
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr {
            if args.len() == 1 && (name == "ln" || name == "log10" || name == "log2") {
                let content = &args[0];
                // log(x^n) = n * log(x)
                if let Expr::Pow(base, exp) = content {
                    return Some(Expr::Mul(
                        exp.clone(),
                        Box::new(Expr::FunctionCall {
                            name: name.clone(),
                            args: vec![*base.clone()],
                        }),
                    ));
                }
                // log(sqrt(x)) = 0.5 * log(x)
                if let Expr::FunctionCall {
                    name: inner_name,
                    args: inner_args,
                } = content
                {
                    if inner_name == "sqrt" && inner_args.len() == 1 {
                        return Some(Expr::Mul(
                            Box::new(Expr::Number(0.5)),
                            Box::new(Expr::FunctionCall {
                                name: name.clone(),
                                args: vec![inner_args[0].clone()],
                            }),
                        ));
                    }
                    // log(cbrt(x)) = (1/3) * log(x)
                    if inner_name == "cbrt" && inner_args.len() == 1 {
                        return Some(Expr::Mul(
                            Box::new(Expr::Div(
                                Box::new(Expr::Number(1.0)),
                                Box::new(Expr::Number(3.0)),
                            )),
                            Box::new(Expr::FunctionCall {
                                name: name.clone(),
                                args: vec![inner_args[0].clone()],
                            }),
                        ));
                    }
                }
            }
        }
        None
    }
}

/// Rule for specific log values: log10(1)=0, log10(10)=1, log2(1)=0, log2(2)=1
pub struct LogBaseRules;

impl Rule for LogBaseRules {
    fn name(&self) -> &'static str {
        "log_base_values"
    }

    fn priority(&self) -> i32 {
        95
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Exponential
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr {
            if args.len() == 1 {
                if name == "log10" {
                    if matches!(args[0], Expr::Number(n) if n == 1.0) {
                        return Some(Expr::Number(0.0));
                    }
                    if matches!(args[0], Expr::Number(n) if n == 10.0) {
                        return Some(Expr::Number(1.0));
                    }
                } else if name == "log2" {
                    if matches!(args[0], Expr::Number(n) if n == 1.0) {
                        return Some(Expr::Number(0.0));
                    }
                    if matches!(args[0], Expr::Number(n) if n == 2.0) {
                        return Some(Expr::Number(1.0));
                    }
                }
            }
        }
        None
    }
}

/// Rule for exp(x) = e^x
pub struct ExpToEPowRule;

impl Rule for ExpToEPowRule {
    fn name(&self) -> &'static str {
        "exp_to_e_pow"
    }

    fn priority(&self) -> i32 {
        95
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Exponential
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr {
            if name == "exp" && args.len() == 1 {
                return Some(Expr::Pow(
                    Box::new(Expr::Symbol("e".to_string())),
                    Box::new(args[0].clone()),
                ));
            }
        }
        None
    }
}

/// Rule for ln(a) + ln(b) = ln(a*b) and ln(a) - ln(b) = ln(a/b)
pub struct LogCombinationRule;

impl Rule for LogCombinationRule {
    fn name(&self) -> &'static str {
        "log_combination"
    }

    fn priority(&self) -> i32 {
        85
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Exponential
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        match expr {
            Expr::Add(u, v) => {
                // ln(a) + ln(b) = ln(a * b)
                if let (Some(arg1), Some(arg2)) = (Self::get_ln_arg(u), Self::get_ln_arg(v)) {
                    return Some(Expr::FunctionCall {
                        name: "ln".to_string(),
                        args: vec![Expr::Mul(Box::new(arg1), Box::new(arg2))],
                    });
                }
            }
            Expr::Sub(u, v) => {
                // ln(a) - ln(b) = ln(a / b)
                if let (Some(arg1), Some(arg2)) = (Self::get_ln_arg(u), Self::get_ln_arg(v)) {
                    return Some(Expr::FunctionCall {
                        name: "ln".to_string(),
                        args: vec![Expr::Div(Box::new(arg1), Box::new(arg2))],
                    });
                }
            }
            _ => {}
        }
        None
    }
}

impl LogCombinationRule {
    fn get_ln_arg(expr: &Expr) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr {
            if name == "ln" && args.len() == 1 {
                return Some(args[0].clone());
            }
        }
        None
    }
}

/// Get all exponential/logarithmic rules in priority order
pub fn get_exponential_rules() -> Vec<Rc<dyn Rule>> {
    vec![
        Rc::new(LnOneRule),
        Rc::new(LnERule),
        Rc::new(ExpZeroRule),
        Rc::new(ExpToEPowRule),
        Rc::new(ExpLnIdentityRule),
        Rc::new(LnExpIdentityRule),
        Rc::new(LogPowerRule),
        Rc::new(LogBaseRules),
        Rc::new(LogCombinationRule),
    ]
}
