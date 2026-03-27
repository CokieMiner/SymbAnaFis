use super::{Rule, RuleCategory, RuleContext, RuleExprKind};
use crate::EPSILON;
use crate::core::known_symbols::KS;
use crate::core::{Expr, ExprKind};
use std::sync::Arc;

rule!(
    ExpandPowerForCancellationRule,
    "expand_power_for_cancellation",
    92,
    Algebraic,
    &[RuleExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::Div(num, den) = &expr.kind {
            // Helper to check if a factor is present in an expression
            let contains_factor = |e: &Expr, factor: &Expr| -> bool {
                match &e.kind {
                    ExprKind::Product(factors) => factors.iter().any(|f| **f == *factor),
                    _ => e == factor,
                }
            };

            // Helper to check if expansion is useful
            let check_and_expand = |target: &Expr, other: &Expr| -> Option<Expr> {
                if let ExprKind::Pow(base, exp) = &target.kind
                    && let ExprKind::Product(base_factors) = &base.kind
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
    &[RuleExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::Pow(base, exp) = &expr.kind {
            // Expand (a*b)^n -> a^n * b^n ONLY if expansion enables simplification
            if let ExprKind::Product(base_factors) = &base.kind
                && let ExprKind::Number(n) = &exp.kind
                && *n > 1.0
                && n.fract() == 0.0
                && {
                    #[allow(
                        clippy::cast_possible_truncation,
                        reason = "n is integer-checked and bounded by small_pow < 10"
                    )]
                    let small_pow: i64 = n.trunc() as i64;
                    small_pow < 10
                }
            {
                // Check if expansion would enable simplification
                let has_simplifiable = base_factors.iter().any(|f| match &f.kind {
                    ExprKind::Pow(_, inner_exp) => {
                        if let ExprKind::Number(inner_n) = &inner_exp.kind {
                            (inner_n * n).fract().abs() < EPSILON
                        } else if let ExprKind::Div(num, den) = &inner_exp.kind {
                            if let (ExprKind::Number(a), ExprKind::Number(b)) =
                                (&num.kind, &den.kind)
                            {
                                ((a * n) / b).fract().abs() < EPSILON
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    }
                    ExprKind::FunctionCall { name, .. } => {
                        (name.id() == KS.sqrt || name.id() == KS.cbrt) && *n >= 2.0
                    }
                    ExprKind::Number(_) => true,
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
            if let ExprKind::Div(a, b) = &base.kind
                && let ExprKind::Number(n) = &exp.kind
                && *n > 1.0
                && n.fract() == 0.0
                && {
                    #[allow(
                        clippy::cast_possible_truncation,
                        reason = "n is integer-checked and bounded by small_pow < 10"
                    )]
                    let small_pow: i64 = (*n).trunc() as i64;
                    small_pow < 10
                }
            {
                // Helper to check if a term would simplify when raised to power n
                let would_simplify = |term: &Expr| -> bool {
                    match &term.kind {
                        ExprKind::Pow(_, inner_exp) => {
                            if let ExprKind::Number(inner_n) = &inner_exp.kind {
                                (inner_n * n).fract().abs() < EPSILON
                            } else if let ExprKind::Div(num, den) = &inner_exp.kind {
                                if let (ExprKind::Number(a_val), ExprKind::Number(b_val)) =
                                    (&num.kind, &den.kind)
                                {
                                    ((a_val * n) / b_val).fract().abs() < EPSILON
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        }
                        ExprKind::FunctionCall { name, .. } => {
                            (name.id() == KS.sqrt || name.id() == KS.cbrt) && *n >= 2.0
                        }
                        ExprKind::Number(_) => true,
                        ExprKind::Product(factors) => factors.iter().any(|f| match &f.kind {
                            ExprKind::Number(_) => true,
                            ExprKind::FunctionCall { name, .. } => {
                                name.id() == KS.sqrt || name.id() == KS.cbrt
                            }
                            ExprKind::Pow(_, inner_exp) => {
                                if let ExprKind::Number(inner_n) = &inner_exp.kind {
                                    (inner_n * n).fract().abs() < EPSILON
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
