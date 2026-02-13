use crate::core::expr::{Expr, ExprKind as AstKind};
use crate::core::known_symbols::{KS, get_symbol};
use crate::core::traits::EPSILON;
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use std::sync::Arc;

rule!(
    LnOneRule,
    "ln_one",
    95,
    Exponential,
    &[ExprKind::Function],
    targets: &[KS.ln],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.ln
            && args.len() == 1
        {
            #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
            let is_one = matches!(&args[0].kind, AstKind::Number(n) if *n == 1.0);
            if is_one
        {
            return Some(Expr::number(0.0));
        }
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
    targets: &[KS.ln],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.ln
            && args.len() == 1
        {
            // Check for ln(exp(1))
            let is_exp_one = if let AstKind::FunctionCall { name: exp_name, args: exp_args } = &args[0].kind
                && exp_name.id() == KS.exp && exp_args.len() == 1
            {
                if let AstKind::Number(n) = &exp_args[0].kind {
                    #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
                    let is_one = *n == 1.0;
                    is_one
                } else {
                    false
                }
            } else {
                false
            };
            if is_exp_one
            {
                return Some(Expr::number(1.0));
            }
            // Check for ln(e) where e is a symbol (and not a user-defined variable)
            if let AstKind::Symbol(s) = &args[0].kind
                && s.id() == KS.e
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
    targets: &[KS.exp],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.exp
            && args.len() == 1
        {
            #[allow(clippy::float_cmp, reason = "Comparing against exact constant 0.0")]
            let is_zero = matches!(&args[0].kind, AstKind::Number(n) if *n == 0.0);
            if is_zero
        {
            return Some(Expr::number(1.0));
        }
        }
        None
    }
);

rule!(ExpLnIdentityRule, "exp_ln_identity", 90, Exponential, &[ExprKind::Function], alters_domain: true, targets: &[KS.exp], |expr: &Expr, _context: &RuleContext| {
    if let AstKind::FunctionCall { name, args } = &expr.kind
        && name.id() == KS.exp
        && args.len() == 1
        && let AstKind::FunctionCall {
            name: inner_name,
            args: inner_args,
        } = &args[0].kind
        && inner_name.id() == KS.ln
        && inner_args.len() == 1
    {
        return Some((*inner_args[0]).clone());
    }
    None
});

rule!(LnExpIdentityRule, "ln_exp_identity", 90, Exponential, &[ExprKind::Function], alters_domain: true, targets: &[KS.ln], |expr: &Expr, _context: &RuleContext| {
    if let AstKind::FunctionCall { name, args } = &expr.kind
        && name.id() == KS.ln
        && args.len() == 1
    {
        // Check for ln(exp(x))
        if let AstKind::FunctionCall {
            name: inner_name,
            args: inner_args,
        } = &args[0].kind
            && inner_name.id() == KS.exp
            && inner_args.len() == 1
        {
            return Some((*inner_args[0]).clone());
        }
        // Check for ln(e^x)
        if let AstKind::Pow(base, exp) = &args[0].kind
            && let AstKind::Symbol(b) = &base.kind
            && b.id() == KS.e
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
    targets: &[KS.ln, KS.log10, KS.log2, KS.log],
    |expr: &Expr, context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && (name.id() == KS.ln || name.id() == KS.log10 || name.id() == KS.log2 || name.id() == KS.log)
        {
            let content = if args.len() == 1 {
                &args[0]
            } else if args.len() == 2 {
                &args[1]
            } else {
                return None;
            };

            // log(x^n) = n * log(x)
            if let AstKind::Pow(base, exp) = &content.kind {
                // Check if exponent is an even integer
                if let AstKind::Number(n) = &exp.kind {
                    // Exact comparison for integer check and cast for parity
                    #[allow(clippy::float_cmp, clippy::cast_possible_truncation, reason = "Comparing against exact constant 0.0 for integer check")]
                    let is_even_int = n.fract() == 0.0 && (*n as i64) % 2 == 0;
                    #[allow(clippy::float_cmp, clippy::cast_possible_truncation, reason = "Comparing against exact constant 0.0 for integer check")]
                    let is_odd_int = n.fract() == 0.0 && (*n as i64) % 2 != 0;

                    if is_even_int && *n != 0.0 {
                        // ln(x^2) = 2*ln(|x|) - always correct for x ≠ 0
                        return Some(Expr::product(vec![
                            (**exp).clone(),
                            match args.len() {
                                1 => Expr::func_multi_from_arcs_symbol(name.clone(), vec![Arc::new(Expr::func_multi_from_arcs_symbol(get_symbol(KS.abs), vec![Arc::clone(base)]))]),
                                2 => Expr::func_multi_from_arcs_symbol(name.clone(), vec![Arc::clone(&args[0]), Arc::new(Expr::func_multi_from_arcs_symbol(get_symbol(KS.abs), vec![Arc::clone(base)]))]),
                                _ => return None, // Invalid number of arguments
                            }
                        ]));
                    } else if is_odd_int {
                        // ln(x^3) = 3*ln(x) - only valid for x > 0
                        if context.domain_safe {
                            let is_positive = matches!(&base.kind,
                                AstKind::FunctionCall { name: fn_name, .. }
                                if fn_name.id() == KS.exp || fn_name.id() == KS.cosh
                            );
                            if !is_positive {
                                return None;
                            }
                        }
                        return Some(Expr::product(vec![
                            (**exp).clone(),
                            match args.len() {
                                1 => Expr::func_multi_from_arcs_symbol(name.clone(), vec![Arc::clone(base)]),
                                2 => Expr::func_multi_from_arcs_symbol(name.clone(), vec![Arc::clone(&args[0]), Arc::clone(base)]),
                                _ => return None, // Invalid number of arguments
                            }
                        ]));
                    }
                }

                // For non-integer or symbolic exponents, only apply in aggressive mode
                if context.domain_safe {
                    return None;
                }

                return Some(Expr::product(vec![
                    (**exp).clone(),
                    match args.len() {
                        1 => Expr::func_multi_from_arcs_symbol(name.clone(), vec![Arc::clone(base)]),
                        2 => Expr::func_multi_from_arcs_symbol(name.clone(), vec![Arc::clone(&args[0]), Arc::clone(base)]),
                        _ => return None, // Invalid number of arguments
                    }
                ]));
            }
            // log(sqrt(x)) = 0.5 * log(x) - only for x > 0
            if let AstKind::FunctionCall {
                name: inner_name,
                args: inner_args,
            } = &content.kind
            {
                if inner_name.id() == KS.sqrt && inner_args.len() == 1 {
                    if context.domain_safe {
                        return None;
                    }
                    return Some(Expr::product(vec![
                        Expr::number(0.5),
                        match args.len() {
                            1 => Expr::func_multi_from_arcs_symbol(name.clone(), vec![Arc::clone(&inner_args[0])]),
                            2 => Expr::func_multi_from_arcs_symbol(name.clone(), vec![Arc::clone(&args[0]), Arc::clone(&inner_args[0])]),
                            _ => return None, // Invalid number of arguments
                        }
                    ]));
                }
                // log(cbrt(x)) = (1/3) * log(x)
                if inner_name.id() == KS.cbrt && inner_args.len() == 1 {
                    if context.domain_safe {
                        return None;
                    }
                    return Some(Expr::product(vec![
                        Expr::div_expr(Expr::number(1.0), Expr::number(3.0)),
                        match args.len() {
                            1 => Expr::func_multi_from_arcs_symbol(name.clone(), vec![Arc::clone(&inner_args[0])]),
                            2 => Expr::func_multi_from_arcs_symbol(name.clone(), vec![Arc::clone(&args[0]), Arc::clone(&inner_args[0])]),
                            _ => return None, // Invalid number of arguments
                        }
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
    targets: &[KS.log10, KS.log2, KS.log],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind {
            if args.len() == 1 {
                if name.id() == KS.log10 {
                    #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
                    let is_one = matches!(&args[0].kind, AstKind::Number(n) if *n == 1.0);
                    if is_one {
                        return Some(Expr::number(0.0));
                    }
                    #[allow(clippy::float_cmp, reason = "Comparing against exact constant 10.0")]
                    let is_ten = matches!(&args[0].kind, AstKind::Number(n) if *n == 10.0);
                    if is_ten {
                        return Some(Expr::number(1.0));
                    }
                } else if name.id() == KS.log2 {
                    #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
                    let is_one = matches!(&args[0].kind, AstKind::Number(n) if *n == 1.0);
                    if is_one {
                        return Some(Expr::number(0.0));
                    }
                    #[allow(clippy::float_cmp, reason = "Comparing against exact constant 2.0")]
                    let is_two = matches!(&args[0].kind, AstKind::Number(n) if *n == 2.0);
                    if is_two {
                        return Some(Expr::number(1.0));
                    }
                }
            } else if args.len() == 2 && name.id() == KS.log {
                // log(base, 1) = 0
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
                let is_one = matches!(&args[1].kind, AstKind::Number(n) if *n == 1.0);
                if is_one {
                     return Some(Expr::number(0.0));
                }
            // log(base, base) = 1
            if args[0] == args[1] {
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
    targets: &[KS.exp],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.exp
            && args.len() == 1
        {
            return Some(Expr::pow_from_arcs(Arc::new(Expr::symbol("e")), Arc::clone(&args[0])));
        }
        None
    }
);

/// Helper function to extract ln argument
fn get_ln_arg(expr: &Expr) -> Option<Expr> {
    if let AstKind::FunctionCall { name, args } = &expr.kind
        && name.id() == KS.ln
        && args.len() == 1
    {
        return Some((*args[0]).clone());
    }
    None
}

rule_with_helpers!(LogCombinationRule, "log_combination", 85, Exponential, &[ExprKind::Sum],
    helpers: {
        // Helper defined above
    },
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Sum(terms) = &expr.kind
            && terms.len() == 2 {
                let u = &terms[0];
                let v = &terms[1];

                // ln(a) + ln(b) = ln(a * b)
                if let (Some(arg1), Some(arg2)) = (get_ln_arg(u), get_ln_arg(v)) {
                    return Some(Expr::func_symbol(
                        get_symbol(KS.ln),
                        Expr::product(vec![arg1, arg2]),
                    ));
                }

                // ln(a) + (-ln(b)) = ln(a/b)
                // Check if v is Product([-1, ln(b)])
                if let AstKind::Product(factors) = &v.kind
                    && factors.len() == 2
                        && let AstKind::Number(n) = &factors[0].kind
                            && (*n + 1.0).abs() < EPSILON
                                && let (Some(arg1), Some(arg2)) = (get_ln_arg(u), get_ln_arg(&factors[1])) {
                                    return Some(Expr::func_symbol(
                                        get_symbol(KS.ln),
                                        Expr::div_expr(arg1, arg2),
                                    ));
                                }
            }
        None
    }
);

/// Get all exponential/logarithmic rules in priority order
pub fn get_exponential_rules() -> Vec<Arc<dyn Rule + Send + Sync>> {
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
