use crate::core::expr::{Expr, ExprKind as AstKind};
use crate::core::known_symbols::{
    ACOSH, ASINH, ATANH, COSH, COTH, CSCH, SECH, SINH, TANH, get_symbol,
};
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};

rule!(
    SinhZeroRule,
    "sinh_zero",
    95,
    Hyperbolic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == *SINH
            && args.len() == 1
            && matches!(args[0].kind, AstKind::Number(n) if n == 0.0)
        {
            return Some(Expr::number(0.0));
        }
        None
    }
);

rule!(
    CoshZeroRule,
    "cosh_zero",
    95,
    Hyperbolic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == *COSH
            && args.len() == 1
            && matches!(args[0].kind, AstKind::Number(n) if n == 0.0)
        {
            return Some(Expr::number(1.0));
        }
        None
    }
);

rule!(
    SinhNegationRule,
    "sinh_negation",
    90,
    Hyperbolic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == *SINH
            && args.len() == 1
            && let AstKind::Product(factors) = &args[0].kind
            && factors.len() == 2
        {
            if let AstKind::Number(n) = &factors[0].kind
                && *n == -1.0
            {
                return Some(Expr::product(vec![
                    Expr::number(-1.0),
                    Expr::func_symbol(get_symbol(&SINH), (*factors[1]).clone()),
                ]));
            }
            if let AstKind::Number(n) = &factors[1].kind
                && *n == -1.0
            {
                return Some(Expr::product(vec![
                    Expr::number(-1.0),
                    Expr::func_symbol(get_symbol(&SINH), (*factors[0]).clone()),
                ]));
            }
        }
        None
    }
);

rule!(
    CoshNegationRule,
    "cosh_negation",
    90,
    Hyperbolic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == *COSH
            && args.len() == 1
            && let AstKind::Product(factors) = &args[0].kind
            && factors.len() == 2
        {
            if let AstKind::Number(n) = &factors[0].kind
                && *n == -1.0
            {
                return Some(Expr::func_symbol(get_symbol(&COSH), (*factors[1]).clone()));
            }
            if let AstKind::Number(n) = &factors[1].kind
                && *n == -1.0
            {
                return Some(Expr::func_symbol(get_symbol(&COSH), (*factors[0]).clone()));
            }
        }
        None
    }
);

rule!(
    TanhNegationRule,
    "tanh_negation",
    90,
    Hyperbolic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == *TANH
            && args.len() == 1
            && let AstKind::Product(factors) = &args[0].kind
            && factors.len() == 2
        {
            if let AstKind::Number(n) = &factors[0].kind
                && *n == -1.0
            {
                return Some(Expr::product(vec![
                    Expr::number(-1.0),
                    Expr::func_symbol(get_symbol(&TANH), (*factors[1]).clone()),
                ]));
            }
            if let AstKind::Number(n) = &factors[1].kind
                && *n == -1.0
            {
                return Some(Expr::product(vec![
                    Expr::number(-1.0),
                    Expr::func_symbol(get_symbol(&TANH), (*factors[0]).clone()),
                ]));
            }
        }
        None
    }
);

rule!(
    SinhAsinhIdentityRule,
    "sinh_asinh_identity",
    95,
    Hyperbolic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == *SINH
            && args.len() == 1
            && let AstKind::FunctionCall {
                name: inner_name,
                args: inner_args,
            } = &args[0].kind
            && inner_name.id() == *ASINH
            && inner_args.len() == 1
        {
            return Some((*inner_args[0]).clone());
        }
        None
    }
);

rule!(
    CoshAcoshIdentityRule,
    "cosh_acosh_identity",
    95,
    Hyperbolic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == *COSH
            && args.len() == 1
            && let AstKind::FunctionCall {
                name: inner_name,
                args: inner_args,
            } = &args[0].kind
            && inner_name.id() == *ACOSH
            && inner_args.len() == 1
        {
            return Some((*inner_args[0]).clone());
        }
        None
    }
);

rule!(
    TanhAtanhIdentityRule,
    "tanh_atanh_identity",
    95,
    Hyperbolic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == *TANH
            && args.len() == 1
            && let AstKind::FunctionCall {
                name: inner_name,
                args: inner_args,
            } = &args[0].kind
            && inner_name.id() == *ATANH
            && inner_args.len() == 1
        {
            return Some((*inner_args[0]).clone());
        }
        None
    }
);

// Hyperbolic identity: cosh^2(x) - sinh^2(x) = 1 and related
rule!(
    HyperbolicIdentityRule,
    "hyperbolic_identity",
    95,
    Hyperbolic,
    &[ExprKind::Sum, ExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        // cosh^2(x) - sinh^2(x) = 1 (as Sum([cosh^2(x), Product([-1, sinh^2(x)])]))
        if let AstKind::Sum(terms) = &expr.kind
            && terms.len() == 2
        {
            let u = &terms[0];
            let v = &terms[1];

            // Helper to extract negated term
            fn extract_negated(term: &Expr) -> Option<Expr> {
                if let AstKind::Product(factors) = &term.kind
                    && factors.len() == 2
                    && let AstKind::Number(n) = &factors[0].kind
                    && (*n + 1.0).abs() < 1e-10
                {
                    return Some((*factors[1]).clone());
                }
                None
            }

            // cosh^2(x) + (-sinh^2(x)) = 1
            if let Some((name1, arg1)) = get_hyperbolic_power(u, 2.0)
                && name1.id() == *COSH
                && let Some(negated) = extract_negated(v)
                && let Some((name2, arg2)) = get_hyperbolic_power(&negated, 2.0)
                && name2.id() == *SINH
                && arg1 == arg2
            {
                return Some(Expr::number(1.0));
            }

            // 1 + (-tanh^2(x)) = sech^2(x)
            if let AstKind::Number(n) = &u.kind
                && *n == 1.0
                && let Some(negated) = extract_negated(v)
                && let Some((name, arg)) = get_hyperbolic_power(&negated, 2.0)
                && name.id() == *TANH
            {
                return Some(Expr::pow(
                    Expr::func_symbol(get_symbol(&SECH), arg),
                    Expr::number(2.0),
                ));
            }

            // coth^2(x) - 1 = csch^2(x)
            if let (Some((n1, a1)), AstKind::Number(num)) = (get_hyperbolic_power(u, 2.0), &v.kind)
                && n1.id() == *COTH
                && (*num + 1.0).abs() < 1e-10
            {
                return Some(Expr::pow(
                    Expr::func_symbol(get_symbol(&CSCH), a1),
                    Expr::number(2.0),
                ));
            }
            if let (AstKind::Number(num), Some((n2, a2))) = (&u.kind, get_hyperbolic_power(v, 2.0))
                && n2.id() == *COTH
                && (*num + 1.0).abs() < 1e-10
            {
                return Some(Expr::pow(
                    Expr::func_symbol(get_symbol(&CSCH), a2),
                    Expr::number(2.0),
                ));
            }
        }

        // (cosh(x) - sinh(x)) * (cosh(x) + sinh(x)) = 1
        if let AstKind::Product(factors) = &expr.kind
            && factors.len() == 2
        {
            let u = &factors[0];
            let v = &factors[1];

            if let (Some(arg1), Some(arg2)) =
                (is_cosh_minus_sinh_term(u), is_cosh_plus_sinh_term(v))
                && arg1 == arg2
            {
                return Some(Expr::number(1.0));
            }
            if let (Some(arg1), Some(arg2)) =
                (is_cosh_minus_sinh_term(v), is_cosh_plus_sinh_term(u))
                && arg1 == arg2
            {
                return Some(Expr::number(1.0));
            }
        }

        None
    }
);

fn get_hyperbolic_power(
    expr: &Expr,
    power: f64,
) -> Option<(crate::core::symbol::InternedSymbol, Expr)> {
    if let AstKind::Pow(base, exp) = &expr.kind
        && let AstKind::Number(p) = &exp.kind
        && *p == power
        && let AstKind::FunctionCall { name, args } = &base.kind
        && args.len() == 1
        && (name.id() == *SINH || name.id() == *COSH || name.id() == *TANH || name.id() == *COTH)
    {
        return Some((name.clone(), (*args[0]).clone()));
    }
    None
}

fn is_cosh_minus_sinh_term(expr: &Expr) -> Option<Expr> {
    // Sum([cosh(x), Product([-1, sinh(x)])])
    if let AstKind::Sum(terms) = &expr.kind
        && terms.len() == 2
        && let AstKind::FunctionCall { name: n1, args: a1 } = &terms[0].kind
        && n1.id() == *COSH
        && a1.len() == 1
        && let AstKind::Product(factors) = &terms[1].kind
        && factors.len() == 2
        && let AstKind::Number(n) = &factors[0].kind
        && *n == -1.0
        && let AstKind::FunctionCall { name: n2, args: a2 } = &factors[1].kind
        && n2.id() == *SINH
        && a2.len() == 1
        && a1[0] == a2[0]
    {
        return Some((*a1[0]).clone());
    }
    None
}

fn is_cosh_plus_sinh_term(expr: &Expr) -> Option<Expr> {
    // Sum([cosh(x), sinh(x)])
    if let AstKind::Sum(terms) = &expr.kind
        && terms.len() == 2
        && let AstKind::FunctionCall { name: n1, args: a1 } = &terms[0].kind
        && n1.id() == *COSH
        && a1.len() == 1
        && let AstKind::FunctionCall { name: n2, args: a2 } = &terms[1].kind
        && n2.id() == *SINH
        && a2.len() == 1
        && a1[0] == a2[0]
    {
        return Some((*a1[0]).clone());
    }
    None
}

rule!(
    HyperbolicTripleAngleRule,
    "hyperbolic_triple_angle",
    70,
    Hyperbolic,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Sum(terms) = &expr.kind
            && terms.len() == 2
        {
            let u = &terms[0];
            let v = &terms[1];

            // 4*sinh(x)^3 + 3*sinh(x) -> sinh(3x)
            if let (Some((c1, arg1, p1)), Some((c2, arg2, p2))) = (
                parse_fn_term(u, get_symbol(&SINH)),
                parse_fn_term(v, get_symbol(&SINH)),
            ) && arg1 == arg2
            {
                let eps = 1e-10;
                if ((c1 - 4.0).abs() < eps && p1 == 3.0 && c2 == 3.0 && p2 == 1.0)
                    || ((c2 - 4.0).abs() < eps && p2 == 3.0 && c1 == 3.0 && p1 == 1.0)
                {
                    return Some(Expr::func_symbol(
                        get_symbol(&SINH),
                        Expr::product(vec![Expr::number(3.0), arg1]),
                    ));
                }
            }

            // 4*cosh(x)^3 + (-3*cosh(x)) -> cosh(3x) (subtraction represented as sum with negated term)
            if let Some((c1, arg1, p1)) = parse_fn_term(u, get_symbol(&COSH))
                && let Some((c2, arg2, p2)) = parse_fn_term(v, get_symbol(&COSH))
                && arg1 == arg2
            {
                let eps = 1e-10;
                // 4*cosh^3(x) - 3*cosh(x) -> cosh(3x)
                if (c1 - 4.0).abs() < eps && p1 == 3.0 && c2 == -3.0 && p2 == 1.0 {
                    return Some(Expr::func_symbol(
                        get_symbol(&COSH),
                        Expr::product(vec![Expr::number(3.0), arg1]),
                    ));
                }
            }
        }
        None
    }
);

fn parse_fn_term(
    expr: &Expr,
    func_name: crate::core::symbol::InternedSymbol,
) -> Option<(f64, Expr, f64)> {
    // Direct function call: func(x) -> (1.0, x, 1.0)
    if let AstKind::FunctionCall { name, args } = &expr.kind
        && *name == func_name
        && args.len() == 1
    {
        return Some((1.0, (*args[0]).clone(), 1.0));
    }

    // Power: func(x)^p -> (1.0, x, p)
    if let AstKind::Pow(base, exp) = &expr.kind
        && let AstKind::Number(p) = &exp.kind
        && let AstKind::FunctionCall { name, args } = &base.kind
        && *name == func_name
        && args.len() == 1
    {
        return Some((1.0, (*args[0]).clone(), *p));
    }

    // Product with coefficient: c * func(x) or c * func(x)^p
    if let AstKind::Product(factors) = &expr.kind
        && factors.len() == 2
        && let AstKind::Number(c) = &factors[0].kind
    {
        // c * func(x)
        if let AstKind::FunctionCall { name, args } = &factors[1].kind
            && *name == func_name
            && args.len() == 1
        {
            return Some((*c, (*args[0]).clone(), 1.0));
        }
        // c * func(x)^p
        if let AstKind::Pow(base, exp) = &factors[1].kind
            && let AstKind::Number(p) = &exp.kind
            && let AstKind::FunctionCall { name, args } = &base.kind
            && *name == func_name
            && args.len() == 1
        {
            return Some((*c, (*args[0]).clone(), *p));
        }
    }

    None
}
