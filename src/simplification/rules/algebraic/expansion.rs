use crate::core::known_symbols::{CBRT, SQRT};
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use crate::{Expr, ExprKind as AstKind};

rule!(
    ExpandPowerForCancellationRule,
    "expand_power_for_cancellation",
    92,
    Algebraic,
    &[ExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(num, den) = &expr.kind {
            // Helper to check if a factor is present in an expression
            let contains_factor = |expr: &Expr, factor: &Expr| -> bool {
                match &expr.kind {
                    AstKind::Product(factors) => factors.iter().any(|f| **f == *factor),
                    _ => expr == factor,
                }
            };

            // Helper to check if expansion is useful
            let check_and_expand = |target: &Expr, other: &Expr| -> Option<Expr> {
                if let AstKind::Pow(base, exp) = &target.kind
                    && let AstKind::Product(base_factors) = &base.kind
                {
                    // Check if any base factor is present in 'other'
                    let mut useful = false;
                    for factor in base_factors {
                        if contains_factor(other, factor) {
                            useful = true;
                            break;
                        }
                    }

                    if useful {
                        let pow_factors: Vec<Expr> = base_factors
                            .iter()
                            .map(|f| Expr::pow_static((**f).clone(), (**exp).clone()))
                            .collect();
                        return Some(Expr::product(pow_factors));
                    }
                }
                None
            };

            // Try expanding powers in numerator
            if let Some(expanded) = check_and_expand(num, den) {
                return Some(Expr::div_expr(expanded, (**den).clone()));
            }

            // Try expanding powers in denominator
            if let Some(expanded) = check_and_expand(den, num) {
                return Some(Expr::div_expr((**num).clone(), expanded));
            }
        }
        None
    }
);

rule!(
    PowerExpansionRule,
    "power_expansion",
    86,
    Algebraic,
    &[ExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Pow(base, exp) = &expr.kind {
            // Expand (a*b)^n -> a^n * b^n ONLY if expansion enables simplification
            if let AstKind::Product(base_factors) = &base.kind
                && let AstKind::Number(n) = &exp.kind
                && *n > 1.0
                && n.fract() == 0.0
                && (*n as i64) < 10
            {
                // Check if expansion would enable simplification
                let has_simplifiable = base_factors.iter().any(|f| match &f.kind {
                    AstKind::Pow(_, inner_exp) => {
                        if let AstKind::Number(inner_n) = &inner_exp.kind {
                            (inner_n * n).fract().abs() < 1e-10
                        } else if let AstKind::Div(num, den) = &inner_exp.kind {
                            if let (AstKind::Number(a), AstKind::Number(b)) = (&num.kind, &den.kind)
                            {
                                ((a * n) / b).fract().abs() < 1e-10
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    }
                    AstKind::FunctionCall { name, .. } => {
                        (name.id() == *SQRT || name.id() == *CBRT) && *n >= 2.0
                    }
                    AstKind::Number(_) => true,
                    _ => false,
                });

                if has_simplifiable {
                    let factors: Vec<Expr> = base_factors
                        .iter()
                        .map(|f| Expr::pow_static((**f).clone(), (**exp).clone()))
                        .collect();
                    return Some(Expr::product(factors));
                }
            }

            // Expand (a/b)^n -> a^n / b^n ONLY if expansion enables simplification
            if let AstKind::Div(a, b) = &base.kind
                && let AstKind::Number(n) = &exp.kind
                && *n > 1.0
                && n.fract() == 0.0
                && (*n as i64) < 10
            {
                // Helper to check if a term would simplify when raised to power n
                let would_simplify = |term: &Expr| -> bool {
                    match &term.kind {
                        AstKind::Pow(_, inner_exp) => {
                            if let AstKind::Number(inner_n) = &inner_exp.kind {
                                (inner_n * n).fract().abs() < 1e-10
                            } else if let AstKind::Div(num, den) = &inner_exp.kind {
                                if let (AstKind::Number(a_val), AstKind::Number(b_val)) =
                                    (&num.kind, &den.kind)
                                {
                                    ((a_val * n) / b_val).fract().abs() < 1e-10
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        }
                        AstKind::FunctionCall { name, .. } => {
                            (name.id() == *SQRT || name.id() == *CBRT) && *n >= 2.0
                        }
                        AstKind::Number(_) => true,
                        AstKind::Product(factors) => factors.iter().any(|f| match &f.kind {
                            AstKind::Number(_) => true,
                            AstKind::FunctionCall { name, .. } => {
                                name.id() == *SQRT || name.id() == *CBRT
                            }
                            AstKind::Pow(_, inner_exp) => {
                                if let AstKind::Number(inner_n) = &inner_exp.kind {
                                    (inner_n * n).fract().abs() < 1e-10
                                } else {
                                    false
                                }
                            }
                            _ => false,
                        }),
                        _ => false,
                    }
                };

                // Only expand if numerator or denominator would simplify
                if would_simplify(a) || would_simplify(b) {
                    let a_pow = Expr::pow_static((**a).clone(), (**exp).clone());
                    let b_pow = Expr::pow_static((**b).clone(), (**exp).clone());
                    return Some(Expr::div_expr(a_pow, b_pow));
                }
            }
        }
        None
    }
);
