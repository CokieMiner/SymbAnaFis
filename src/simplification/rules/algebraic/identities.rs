use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use crate::{Expr, ExprKind as AstKind};
use std::sync::Arc;

rule!(ExpLnRule, "exp_ln", 80, Algebraic, &[ExprKind::Function], alters_domain: true, |expr: &Expr, _context: &RuleContext| {
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

rule!(LnExpRule, "ln_exp", 80, Algebraic, &[ExprKind::Function], alters_domain: true, |expr: &Expr, _context: &RuleContext| {
    if let AstKind::FunctionCall { name, args } = &expr.kind
        && name == "ln"
        && args.len() == 1
        && let AstKind::FunctionCall {
            name: inner_name,
            args: inner_args,
        } = &args[0].kind
        && inner_name == "exp"
        && inner_args.len() == 1
    {
        return Some(inner_args[0].clone());
    }
    None
});

rule!(ExpMulLnRule, "exp_mul_ln", 80, Algebraic, &[ExprKind::Function], alters_domain: true, |expr: &Expr, _context: &RuleContext| {
    if let AstKind::FunctionCall { name, args } = &expr.kind
        && name == "exp"
        && args.len() == 1
    {
        // Check if arg is a Product containing ln(x)
        if let AstKind::Product(factors) = &args[0].kind {
            // Look for ln(x) among the factors
            for (i, factor) in factors.iter().enumerate() {
                if let AstKind::FunctionCall {
                    name: inner_name,
                    args: inner_args,
                } = &factor.kind
                    && inner_name == "ln"
                    && inner_args.len() == 1
                {
                    // exp(a * b * ln(x)) = x^(a*b)
                    let other_factors: Vec<Arc<Expr>> = factors.iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .map(|(_, f)| f.clone())
                        .collect();

                    let exponent = Expr::product_from_arcs(other_factors);

                    return Some(Expr::pow(inner_args[0].clone(), exponent));
                }
            }
        }
    }
    None
});

rule!(EPowLnRule, "e_pow_ln", 85, Algebraic, &[ExprKind::Pow], alters_domain: true, |expr: &Expr, context: &RuleContext| {
    if let AstKind::Pow(base, exp) = &expr.kind
        && let AstKind::Symbol(s) = &base.kind
            && s == "e"
            && !context.fixed_vars.contains("e")
            && let AstKind::FunctionCall { name, args } = &exp.kind
                && name == "ln"
                && args.len() == 1
            {
                return Some(args[0].clone());
            }
    None
});

rule!(EPowMulLnRule, "e_pow_mul_ln", 85, Algebraic, &[ExprKind::Pow], alters_domain: true, |expr: &Expr, context: &RuleContext| {
    if let AstKind::Pow(base, exp) = &expr.kind
        && let AstKind::Symbol(s) = &base.kind
            && s == "e"
            && !context.fixed_vars.contains("e")
        {
            // Check if exponent is a Product containing ln(x)
            if let AstKind::Product(factors) = &exp.kind {
                for (i, factor) in factors.iter().enumerate() {
                    if let AstKind::FunctionCall {
                        name: inner_name,
                        args: inner_args,
                    } = &factor.kind
                        && inner_name == "ln"
                        && inner_args.len() == 1
                    {
                        // e^(a * b * ln(x)) = x^(a*b)
                        let other_factors: Vec<Arc<Expr>> = factors.iter()
                            .enumerate()
                            .filter(|(j, _)| *j != i)
                            .map(|(_, f)| f.clone())
                            .collect();

                        let exponent = Expr::product_from_arcs(other_factors);

                        return Some(Expr::pow(inner_args[0].clone(), exponent));
                    }
                }
            }
        }
    None
});
