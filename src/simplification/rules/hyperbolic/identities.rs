use crate::core::expr::{Expr, ExprKind as AstKind};
use crate::core::known_symbols::{KS, get_symbol};
use crate::core::traits::EPSILON;
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};

rule!(
    SinhZeroRule,
    "sinh_zero",
    95,
    Hyperbolic,
    &[ExprKind::Function],
    targets: &[KS.sinh],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.sinh
            && args.len() == 1
            && {
                // Exact check for constant 0.0
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 0.0")]
                let is_zero = matches!(args[0].kind, AstKind::Number(n) if n == 0.0);
                is_zero
            }
        {
            return Some(Expr::number(0.0_f64));
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
    targets: &[KS.cosh],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.cosh
            && args.len() == 1
            && {
                // Exact check for constant 0.0
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 0.0")]
                let is_zero = matches!(args[0].kind, AstKind::Number(n) if n == 0.0_f64);
                is_zero
            }
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
    targets: &[KS.sinh],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.sinh
            && args.len() == 1
            && let AstKind::Product(factors) = &args[0].kind
            && factors.len() == 2
        {
            #[allow(clippy::float_cmp, reason = "Comparing against exact constant -1.0")]
            let is_neg_one = matches!(&factors[0].kind, AstKind::Number(n) if *n == -1.0);
            if is_neg_one
            {
                return Some(Expr::product(vec![
                    Expr::number(-1.0),
                    Expr::func_symbol(get_symbol(KS.sinh), (*factors[1]).clone()),
                ]));
            }
            #[allow(clippy::float_cmp, reason = "Comparing against exact constant -1.0")]
            let is_neg_one_alt = matches!(&factors[1].kind, AstKind::Number(n) if *n == -1.0);
            if is_neg_one_alt
            {
                return Some(Expr::product(vec![
                    Expr::number(-1.0),
                    Expr::func_symbol(get_symbol(KS.sinh), (*factors[0]).clone()),
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
    targets: &[KS.cosh],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.cosh
            && args.len() == 1
            && let AstKind::Product(factors) = &args[0].kind
            && factors.len() == 2
        {
            #[allow(clippy::float_cmp, reason = "Comparing against exact constant -1.0")]
            let is_neg_one = matches!(&factors[0].kind, AstKind::Number(n) if *n == -1.0);
            if is_neg_one
            {
                return Some(Expr::func_symbol(get_symbol(KS.cosh), (*factors[1]).clone()));
            }
            #[allow(clippy::float_cmp, reason = "Comparing against exact constant -1.0")]
            let is_neg_one_alt = matches!(&factors[1].kind, AstKind::Number(n) if *n == -1.0);
            if is_neg_one_alt
            {
                return Some(Expr::func_symbol(get_symbol(KS.cosh), (*factors[0]).clone()));
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
    targets: &[KS.tanh],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.tanh
            && args.len() == 1
            && let AstKind::Product(factors) = &args[0].kind
            && factors.len() == 2
        {
            #[allow(clippy::float_cmp, reason = "Comparing against exact constant -1.0")]
            let is_neg_one = matches!(&factors[0].kind, AstKind::Number(n) if *n == -1.0);
            if is_neg_one
            {
                return Some(Expr::product(vec![
                    Expr::number(-1.0),
                    Expr::func_symbol(get_symbol(KS.tanh), (*factors[1]).clone()),
                ]));
            }
            #[allow(clippy::float_cmp, reason = "Comparing against exact constant -1.0")]
            let is_neg_one_alt = matches!(&factors[1].kind, AstKind::Number(n) if *n == -1.0);
            if is_neg_one_alt
            {
                return Some(Expr::product(vec![
                    Expr::number(-1.0),
                    Expr::func_symbol(get_symbol(KS.tanh), (*factors[0]).clone()),
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
    targets: &[KS.sinh],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.sinh
            && args.len() == 1
            && let AstKind::FunctionCall {
                name: inner_name,
                args: inner_args,
            } = &args[0].kind
            && inner_name.id() == KS.asinh
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
    targets: &[KS.cosh],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.cosh
            && args.len() == 1
            && let AstKind::FunctionCall {
                name: inner_name,
                args: inner_args,
            } = &args[0].kind
            && inner_name.id() == KS.acosh
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
    targets: &[KS.tanh],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.tanh
            && args.len() == 1
            && let AstKind::FunctionCall {
                name: inner_name,
                args: inner_args,
            } = &args[0].kind
            && inner_name.id() == KS.atanh
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
            // Helper to extract negated term
            fn extract_negated(term: &Expr) -> Option<Expr> {
                if let AstKind::Product(factors) = &term.kind
                    && factors.len() == 2
                    && let AstKind::Number(n) = &factors[0].kind
                    && (*n + 1.0).abs() < EPSILON
                {
                    return Some((*factors[1]).clone());
                }
                None
            }

            let u = &terms[0];
            let v = &terms[1];

            // cosh^2(x) + (-sinh^2(x)) = 1
            if let Some((name1, arg1)) = get_hyperbolic_power(u, 2.0)
                && name1.id() == KS.cosh
                && let Some(negated) = extract_negated(v)
                && let Some((name2, arg2)) = get_hyperbolic_power(&negated, 2.0)
                && name2.id() == KS.sinh
                && arg1 == arg2
            {
                return Some(Expr::number(1.0));
            }

            // 1 + (-tanh^2(x)) = sech^2(x)
            if {
                // Exact check for constant 1.0
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
                let is_one = matches!(&u.kind, AstKind::Number(n) if *n == 1.0);
                is_one
            } && let Some(negated) = extract_negated(v)
                && let Some((name, arg)) = get_hyperbolic_power(&negated, 2.0)
                && name.id() == KS.tanh
            {
                return Some(Expr::pow_static(
                    Expr::func_symbol(get_symbol(KS.sech), arg),
                    Expr::number(2.0),
                ));
            }

            // coth^2(x) - 1 = csch^2(x)
            if let (Some((n1, a1)), AstKind::Number(num)) = (get_hyperbolic_power(u, 2.0), &v.kind)
                && n1.id() == KS.coth
                && (*num + 1.0).abs() < EPSILON
            {
                return Some(Expr::pow_static(
                    Expr::func_symbol(get_symbol(KS.csch), a1),
                    Expr::number(2.0),
                ));
            }
            if let (AstKind::Number(num), Some((n2, a2))) = (&u.kind, get_hyperbolic_power(v, 2.0))
                && n2.id() == KS.coth
                && (*num + 1.0).abs() < EPSILON
            {
                return Some(Expr::pow_static(
                    Expr::func_symbol(get_symbol(KS.csch), a2),
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
        && {
            // Exact check for power (e.g. 2.0)
            #[allow(clippy::float_cmp, reason = "Comparing against exact constant power")]
            let is_match = *p == power;
            is_match
        }
        && let AstKind::FunctionCall { name, args } = &base.kind
        && args.len() == 1
        && (name.id() == KS.sinh
            || name.id() == KS.cosh
            || name.id() == KS.tanh
            || name.id() == KS.coth)
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
        && n1.id() == KS.cosh
        && a1.len() == 1
        && let AstKind::Product(factors) = &terms[1].kind
        && factors.len() == 2
        && {
            // Exact check for constant -1.0
            #[allow(clippy::float_cmp, reason = "Comparing against exact constant -1.0")]
            let is_neg_one = matches!(&factors[0].kind, AstKind::Number(n) if *n == -1.0);
            is_neg_one
        }
        && let AstKind::FunctionCall { name: n2, args: a2 } = &factors[1].kind
        && n2.id() == KS.sinh
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
        && n1.id() == KS.cosh
        && a1.len() == 1
        && let AstKind::FunctionCall { name: n2, args: a2 } = &terms[1].kind
        && n2.id() == KS.sinh
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
    &[ExprKind::Sum, ExprKind::Poly],
    |expr: &Expr, _context: &RuleContext| {
        let eps = EPSILON; // Use reasonable floating point tolerance for triple angle identities

        // Handle Poly form: Poly(sinh(x), [(1, 3.0), (3, 4.0)]) -> sinh(3x)
        if let AstKind::Poly(poly) = &expr.kind {
            let base = poly.base();
            let terms = poly.terms();

            // Check if base is sinh(x) or cosh(x)
            if let AstKind::FunctionCall { name, args } = &base.kind {
                if args.len() != 1 {
                    return None;
                }
                let x = &args[0];

                // Check for sinh triple angle: 4*sinh(x)^3 + 3*sinh(x)
                // terms should be [(1, 3.0), (3, 4.0)] or similar
                if name.id() == KS.sinh && terms.len() == 2 {
                    let mut has_linear_3 = false;
                    let mut has_cubic_4 = false;

                    for &(power, coeff) in terms {
                        if power == 1 && (coeff - 3.0).abs() < eps {
                            has_linear_3 = true;
                        } else if power == 3 && (coeff - 4.0).abs() < eps {
                            has_cubic_4 = true;
                        }
                    }

                    if has_linear_3 && has_cubic_4 {
                        return Some(Expr::func_symbol(
                            get_symbol(KS.sinh),
                            Expr::product(vec![Expr::number(3.0), (**x).clone()]),
                        ));
                    }
                }

                // Check for cosh triple angle: 4*cosh(x)^3 - 3*cosh(x)
                // terms should be [(1, -3.0), (3, 4.0)] or similar
                if name.id() == KS.cosh && terms.len() == 2 {
                    let mut has_linear_neg3 = false;
                    let mut has_cubic_4 = false;

                    for &(power, coeff) in terms {
                        if power == 1 && (coeff + 3.0).abs() < eps {
                            has_linear_neg3 = true;
                        } else if power == 3 && (coeff - 4.0).abs() < eps {
                            has_cubic_4 = true;
                        }
                    }

                    if has_linear_neg3 && has_cubic_4 {
                        return Some(Expr::func_symbol(
                            get_symbol(KS.cosh),
                            Expr::product(vec![Expr::number(3.0), (**x).clone()]),
                        ));
                    }
                }
            }
        }

        // Original Sum handling
        if let AstKind::Sum(terms) = &expr.kind
            && terms.len() == 2
        {
            let u = &terms[0];
            let v = &terms[1];

            if let (Some((c1, arg1, p1)), Some((c2, arg2, p2))) = (
                parse_fn_term(u, &get_symbol(KS.sinh)),
                parse_fn_term(v, &get_symbol(KS.sinh)),
            ) && arg1 == arg2
            {
                // Constants in triple angle identity
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 3.0")]
                let is_pow1_cubed = p1 == 3.0;
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 3.0")]
                let is_coef2_three = c2 == 3.0;
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
                let is_pow2_one = p2 == 1.0;
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 3.0")]
                let is_pow2_cubed = p2 == 3.0;
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 3.0")]
                let is_coef1_three = c1 == 3.0;
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
                let is_pow1_one = p1 == 1.0;

                if (c1 - 4.0).abs() < eps && is_pow1_cubed && is_coef2_three && is_pow2_one
                    || (c2 - 4.0).abs() < eps && is_pow2_cubed && is_coef1_three && is_pow1_one
                {
                    return Some(Expr::func_symbol(
                        get_symbol(KS.sinh),
                        Expr::product(vec![Expr::number(3.0), arg1]),
                    ));
                }
            }

            // 4*cosh(x)^3 + (-3*cosh(x)) -> cosh(3x) (subtraction represented as sum with negated term)
            if let Some((c1, arg1, p1)) = parse_fn_term(u, &get_symbol(KS.cosh))
                && let Some((c2, arg2, p2)) = parse_fn_term(v, &get_symbol(KS.cosh))
                && arg1 == arg2
            {
                // 4*cosh^3(x) - 3*cosh(x) -> cosh(3x)
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 3.0")]
                let is_p1_3 = p1 == 3.0;
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant -3.0")]
                let is_c2_neg3 = c2 == -3.0;
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
                let is_p2_1 = p2 == 1.0;

                if (c1 - 4.0).abs() < eps && is_p1_3 && is_c2_neg3 && is_p2_1 {
                    return Some(Expr::func_symbol(
                        get_symbol(KS.cosh),
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
    func_name: &crate::core::symbol::InternedSymbol,
) -> Option<(f64, Expr, f64)> {
    // Direct function call: func(x) -> (1.0, x, 1.0)
    if let AstKind::FunctionCall { name, args } = &expr.kind
        && name.id() == func_name.id()
        && args.len() == 1
    {
        return Some((1.0, (*args[0]).clone(), 1.0));
    }

    // Power: func(x)^p -> (1.0, x, p)
    if let AstKind::Pow(base, exp) = &expr.kind
        && let AstKind::Number(p) = &exp.kind
        && let AstKind::FunctionCall { name, args } = &base.kind
        && name.id() == func_name.id()
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
            && name.id() == func_name.id()
            && args.len() == 1
        {
            return Some((*c, (*args[0]).clone(), 1.0));
        }
        // c * func(x)^p
        if let AstKind::Pow(base, exp) = &factors[1].kind
            && let AstKind::Number(p) = &exp.kind
            && let AstKind::FunctionCall { name, args } = &base.kind
            && name.id() == func_name.id()
            && args.len() == 1
        {
            return Some((*c, (*args[0]).clone(), *p));
        }
    }

    None
}
