use crate::core::expr::{Expr, ExprKind as AstKind};
use crate::core::known_symbols::{get_symbol, COS, SIN};
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};

fn check_sin_triple(u: &Expr, v: &Expr, eps: f64) -> Option<Expr> {
    // Check for 3*sin(x) - 4*sin^3(x) pattern
    if let AstKind::Product(factors) = &u.kind {
        // Look for coefficient 3 and sin(x)
        if factors.len() == 2
            && let AstKind::Number(n) = &factors[0].kind
            && (*n - 3.0).abs() < eps
            && let AstKind::FunctionCall { name, args } = &factors[1].kind
            && name.id() == *SIN
            && args.len() == 1
        {
            let x = &args[0];
            if let Some((coeff, _is_neg)) = extract_sin_cubed(v, x, eps)
                && (coeff - 4.0).abs() < eps
            {
                return Some(Expr::func_symbol(
                    get_symbol(&SIN),
                    Expr::product(vec![Expr::number(3.0), (**x).clone()]),
                ));
            }
        }
    }
    None
}

fn check_sin_triple_add(u: &Expr, v: &Expr, eps: f64) -> Option<Expr> {
    // Check for 3*sin(x) + (-4*sin^3(x)) pattern
    if let AstKind::Product(factors) = &u.kind
        && factors.len() == 2
        && let AstKind::Number(n) = &factors[0].kind
        && (*n - 3.0).abs() < eps
        && let AstKind::FunctionCall { name, args } = &factors[1].kind
        && name.id() == *SIN
        && args.len() == 1
    {
        let x = &args[0];
        if let Some((coeff, is_neg)) = extract_sin_cubed(v, x, eps)
            && is_neg
            && (coeff - 4.0).abs() < eps
        {
            return Some(Expr::func_symbol(
                get_symbol(&SIN),
                Expr::product(vec![Expr::number(3.0), (**x).clone()]),
            ));
        }
    }
    // Check reversed
    if let AstKind::Product(factors) = &v.kind
        && factors.len() == 2
        && let AstKind::Number(n) = &factors[0].kind
        && (*n - 3.0).abs() < eps
        && let AstKind::FunctionCall { name, args } = &factors[1].kind
        && name.id() == *SIN
        && args.len() == 1
    {
        let x = &args[0];
        if let Some((coeff, is_neg)) = extract_sin_cubed(u, x, eps)
            && is_neg
            && (coeff - 4.0).abs() < eps
        {
            return Some(Expr::func_symbol(
                get_symbol(&SIN),
                Expr::product(vec![Expr::number(3.0), (**x).clone()]),
            ));
        }
    }
    None
}

fn check_cos_triple(u: &Expr, v: &Expr, eps: f64) -> Option<Expr> {
    // Check for 4*cos^3(x) - 3*cos(x) pattern
    if let AstKind::Product(factors) = &u.kind
        && factors.len() == 2
        && let AstKind::Number(n) = &factors[0].kind
        && (*n - 4.0).abs() < eps
        && let AstKind::Pow(base, exp) = &factors[1].kind
        && let AstKind::Number(e) = &exp.kind
        && *e == 3.0
        && let AstKind::FunctionCall { name, args } = &base.kind
        && name.id() == *COS
        && args.len() == 1
    {
        let x = &args[0];
        if let Some((coeff, _is_neg)) = extract_cos(v, x, eps)
            && (coeff - 3.0).abs() < eps
        {
            return Some(Expr::func_symbol(
                get_symbol(&COS),
                Expr::product(vec![Expr::number(3.0), (**x).clone()]),
            ));
        }
    }
    None
}

fn check_cos_triple_add(u: &Expr, v: &Expr, eps: f64) -> Option<Expr> {
    // Check for 4*cos^3(x) + (-3*cos(x)) pattern
    if let AstKind::Product(factors) = &u.kind
        && factors.len() == 2
        && let AstKind::Number(n) = &factors[0].kind
        && (*n - 4.0).abs() < eps
        && let AstKind::Pow(base, exp) = &factors[1].kind
        && let AstKind::Number(e) = &exp.kind
        && *e == 3.0
        && let AstKind::FunctionCall { name, args } = &base.kind
        && name.id() == *COS
        && args.len() == 1
    {
        let x = &args[0];
        if let Some((coeff, is_neg)) = extract_cos(v, x, eps)
            && is_neg
            && (coeff - 3.0).abs() < eps
        {
            return Some(Expr::func_symbol(
                get_symbol(&COS),
                Expr::product(vec![Expr::number(3.0), (**x).clone()]),
            ));
        }
    }
    // Check reversed
    if let AstKind::Product(factors) = &v.kind
        && factors.len() == 2
        && let AstKind::Number(n) = &factors[0].kind
        && (*n - 4.0).abs() < eps
        && let AstKind::Pow(base, exp) = &factors[1].kind
        && let AstKind::Number(e) = &exp.kind
        && *e == 3.0
        && let AstKind::FunctionCall { name, args } = &base.kind
        && name.id() == *COS
        && args.len() == 1
    {
        let x = &args[0];
        if let Some((coeff, is_neg)) = extract_cos(u, x, eps)
            && is_neg
            && (coeff - 3.0).abs() < eps
        {
            return Some(Expr::func_symbol(
                get_symbol(&COS),
                Expr::product(vec![Expr::number(3.0), (**x).clone()]),
            ));
        }
    }
    None
}

fn extract_sin_cubed(expr: &Expr, x: &std::sync::Arc<Expr>, _eps: f64) -> Option<(f64, bool)> {
    // Match c * sin^3(x) pattern
    if let AstKind::Product(factors) = &expr.kind
        && factors.len() == 2
        && let AstKind::Number(n) = &factors[0].kind
        && let AstKind::Pow(base, exp) = &factors[1].kind
        && let AstKind::Number(e) = &exp.kind
        && *e == 3.0
        && let AstKind::FunctionCall { name, args } = &base.kind
        && name.id() == *SIN
        && args.len() == 1
        && &args[0] == x
    {
        return Some((n.abs(), *n < 0.0));
    }
    None
}

fn extract_cos(expr: &Expr, x: &std::sync::Arc<Expr>, _eps: f64) -> Option<(f64, bool)> {
    // Match c * cos(x) pattern
    if let AstKind::Product(factors) = &expr.kind
        && factors.len() == 2
        && let AstKind::Number(n) = &factors[0].kind
        && let AstKind::FunctionCall { name, args } = &factors[1].kind
        && name.id() == *COS
        && args.len() == 1
        && &args[0] == x
    {
        return Some((n.abs(), *n < 0.0));
    }
    None
}

rule!(
    TrigTripleAngleRule,
    "trig_triple_angle",
    70,
    Trigonometric,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        let eps = 1e-10;
        if let AstKind::Sum(terms) = &expr.kind
            && terms.len() == 2
        {
            let u = &terms[0];
            let v = &terms[1];

            // Helper to extract negated term from Product([-1, x])
            fn extract_negated(term: &Expr) -> Option<Expr> {
                if let AstKind::Product(factors) = &term.kind
                    && factors.len() >= 2
                    && let AstKind::Number(n) = &factors[0].kind
                    && (*n + 1.0).abs() < 1e-10
                {
                    // Rebuild without the -1 factor
                    let remaining: Vec<Expr> =
                        factors.iter().skip(1).map(|f| (**f).clone()).collect();
                    if remaining.len() == 1 {
                        return Some(remaining.into_iter().next().unwrap());
                    } else {
                        return Some(Expr::product(remaining));
                    }
                }
                None
            }

            // Try sin triple angle: 3*sin(x) + (-4*sin^3(x)) = sin(3x)
            if let Some(result) = check_sin_triple_add(u, v, eps) {
                return Some(result);
            }

            // Try with negated term for subtraction pattern
            if let Some(negated_v) = extract_negated(v) {
                if let Some(result) = check_sin_triple(u, &negated_v, eps) {
                    return Some(result);
                }
                if let Some(result) = check_cos_triple(u, &negated_v, eps) {
                    return Some(result);
                }
            }

            // Try cos triple angle
            if let Some(result) = check_cos_triple_add(u, v, eps) {
                return Some(result);
            }
        }
        None
    }
);
