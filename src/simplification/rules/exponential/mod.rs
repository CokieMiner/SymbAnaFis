use crate::ast::{Expr, ExprKind as AstKind};
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use std::sync::Arc;

rule!(
    LnOneRule,
    "ln_one",
    95,
    Exponential,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name == "ln"
            && args.len() == 1
            && matches!(&args[0].kind, AstKind::Number(n) if *n == 1.0)
        {
            return Some(Expr::number(0.0));
        }
        None
    }
);

rule!(
    LnERule,
    "ln_e",
    95,
    Exponential,
    &[ExprKind::Function],
    |expr: &Expr, context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name == "ln"
            && args.len() == 1
        {
            // Check for ln(exp(1))
            if matches!(&args[0].kind, AstKind::FunctionCall { name: exp_name, args: exp_args }
                       if exp_name == "exp" && exp_args.len() == 1 && matches!(&exp_args[0].kind, AstKind::Number(n) if *n == 1.0))
            {
                return Some(Expr::number(1.0));
            }
            // Check for ln(e) where e is a symbol (and not a user-defined variable)
            if let AstKind::Symbol(s) = &args[0].kind
                && s == "e"
                && !context.fixed_vars.contains("e")
            {
                return Some(Expr::number(1.0));
            }
        }
        None
    }
);

rule!(
    ExpZeroRule,
    "exp_zero",
    95,
    Exponential,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name == "exp"
            && args.len() == 1
            && matches!(&args[0].kind, AstKind::Number(n) if *n == 0.0)
        {
            return Some(Expr::number(1.0));
        }
        None
    }
);

rule!(ExpLnIdentityRule, "exp_ln_identity", 90, Exponential, &[ExprKind::Function], alters_domain: true, |expr: &Expr, _context: &RuleContext| {
    if let AstKind::FunctionCall { name, args } = &expr.kind
        && name == "exp"
        && args.len() == 1
        && let AstKind::FunctionCall {
            name: inner_name,
            args: inner_args,
        } = &args[0].kind
        && inner_name == "ln"
        && inner_args.len() == 1
    {
        return Some(inner_args[0].clone());
    }
    None
});

rule!(LnExpIdentityRule, "ln_exp_identity", 90, Exponential, &[ExprKind::Function], alters_domain: true, |expr: &Expr, _context: &RuleContext| {
    if let AstKind::FunctionCall { name, args } = &expr.kind
        && name == "ln"
        && args.len() == 1
    {
        // Check for ln(exp(x))
        if let AstKind::FunctionCall {
            name: inner_name,
            args: inner_args,
        } = &args[0].kind
            && inner_name == "exp"
            && inner_args.len() == 1
        {
            return Some(inner_args[0].clone());
        }
        // Check for ln(e^x)
        if let AstKind::Pow(base, exp) = &args[0].kind
            && let AstKind::Symbol(b) = &base.kind
            && b == "e"
        {
            return Some((**exp).clone());
        }
    }
    None
});

rule!(
    LogPowerRule,
    "log_power",
    90,
    Exponential,
    &[ExprKind::Function],
    |expr: &Expr, context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && args.len() == 1
            && (name == "ln" || name == "log10" || name == "log2")
        {
            let content = &args[0];
            // log(x^n) = n * log(x)
            if let AstKind::Pow(base, exp) = &content.kind {
                // Check if exponent is an even integer
                if let AstKind::Number(n) = &exp.kind {
                    let is_even_int = n.fract() == 0.0 && (*n as i64) % 2 == 0;
                    let is_odd_int = n.fract() == 0.0 && (*n as i64) % 2 != 0;

                    if is_even_int && *n != 0.0 {
                        // ln(x^2) = 2*ln(|x|) - always correct for x ≠ 0
                        return Some(Expr::product(vec![
                            (**exp).clone(),
                            Expr::func(name.clone(), Expr::func("abs", (**base).clone())),
                        ]));
                    } else if is_odd_int {
                        // ln(x^3) = 3*ln(x) - only valid for x > 0
                        if context.domain_safe {
                            let is_positive = matches!(&base.kind,
                                AstKind::FunctionCall { name: fn_name, .. }
                                if fn_name == "exp" || fn_name == "cosh"
                            );
                            if !is_positive {
                                return None;
                            }
                        }
                        return Some(Expr::product(vec![
                            (**exp).clone(),
                            Expr::func(name.clone(), (**base).clone()),
                        ]));
                    }
                }

                // For non-integer or symbolic exponents, only apply in aggressive mode
                if context.domain_safe {
                    return None;
                }

                return Some(Expr::product(vec![
                    (**exp).clone(),
                    Expr::func(name.clone(), (**base).clone()),
                ]));
            }
            // log(sqrt(x)) = 0.5 * log(x) - only for x > 0
            if let AstKind::FunctionCall {
                name: inner_name,
                args: inner_args,
            } = &content.kind
            {
                if inner_name == "sqrt" && inner_args.len() == 1 {
                    if context.domain_safe {
                        return None;
                    }
                    return Some(Expr::product(vec![
                        Expr::number(0.5),
                        Expr::func(name.clone(), inner_args[0].clone()),
                    ]));
                }
                // log(cbrt(x)) = (1/3) * log(x)
                if inner_name == "cbrt" && inner_args.len() == 1 {
                    if context.domain_safe {
                        return None;
                    }
                    return Some(Expr::product(vec![
                        Expr::div_expr(Expr::number(1.0), Expr::number(3.0)),
                        Expr::func(name.clone(), inner_args[0].clone()),
                    ]));
                }
            }
        }
        None
    }
);

rule!(
    LogBaseRules,
    "log_base_values",
    95,
    Exponential,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && args.len() == 1
        {
            if name == "log10" {
                if matches!(&args[0].kind, AstKind::Number(n) if *n == 1.0) {
                    return Some(Expr::number(0.0));
                }
                if matches!(&args[0].kind, AstKind::Number(n) if *n == 10.0) {
                    return Some(Expr::number(1.0));
                }
            } else if name == "log2" {
                if matches!(&args[0].kind, AstKind::Number(n) if *n == 1.0) {
                    return Some(Expr::number(0.0));
                }
                if matches!(&args[0].kind, AstKind::Number(n) if *n == 2.0) {
                    return Some(Expr::number(1.0));
                }
            }
        }
        None
    }
);

rule!(
    ExpToEPowRule,
    "exp_to_e_pow",
    95,
    Exponential,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name == "exp"
            && args.len() == 1
        {
            return Some(Expr::pow(Expr::symbol("e"), args[0].clone()));
        }
        None
    }
);

/// Helper function to extract ln argument
fn get_ln_arg(expr: &Expr) -> Option<Expr> {
    if let AstKind::FunctionCall { name, args } = &expr.kind
        && name == "ln"
        && args.len() == 1
    {
        return Some(args[0].clone());
    }
    None
}

rule_with_helpers!(LogCombinationRule, "log_combination", 85, Exponential, &[ExprKind::Sum],
    helpers: {
        // Helper defined above
    },
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Sum(terms) = &expr.kind {
            if terms.len() == 2 {
                let u = &terms[0];
                let v = &terms[1];

                // ln(a) + ln(b) = ln(a * b)
                if let (Some(arg1), Some(arg2)) = (get_ln_arg(u), get_ln_arg(v)) {
                    return Some(Expr::func(
                        "ln",
                        Expr::product(vec![arg1, arg2]),
                    ));
                }

                // ln(a) + (-ln(b)) = ln(a/b)
                // Check if v is Product([-1, ln(b)])
                if let AstKind::Product(factors) = &v.kind {
                    if factors.len() == 2 {
                        if let AstKind::Number(n) = &factors[0].kind {
                            if (*n + 1.0).abs() < 1e-10 {
                                if let (Some(arg1), Some(arg2)) = (get_ln_arg(u), get_ln_arg(&factors[1])) {
                                    return Some(Expr::func(
                                        "ln",
                                        Expr::div_expr(arg1, arg2),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }
);

/// Get all exponential/logarithmic rules in priority order
pub(crate) fn get_exponential_rules() -> Vec<Arc<dyn Rule + Send + Sync>> {
    vec![
        Arc::new(LnOneRule),
        Arc::new(LnERule),
        Arc::new(ExpZeroRule),
        Arc::new(ExpToEPowRule),
        Arc::new(ExpLnIdentityRule),
        Arc::new(LnExpIdentityRule),
        Arc::new(LogPowerRule),
        Arc::new(LogBaseRules),
        Arc::new(LogCombinationRule),
    ]
}
