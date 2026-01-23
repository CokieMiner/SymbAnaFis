use crate::core::known_symbols::KS;
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use crate::{Expr, ExprKind as AstKind};
use std::sync::Arc;

rule_arc!(ExpLnRule, "exp_ln", 80, Algebraic, &[ExprKind::Function], alters_domain: true, |expr: &Expr, _context: &RuleContext| {
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
        return Some(Arc::clone(&inner_args[0]));
    }
    None
});

rule_arc!(LnExpRule, "ln_exp", 80, Algebraic, &[ExprKind::Function], alters_domain: true, |expr: &Expr, _context: &RuleContext| {
    if let AstKind::FunctionCall { name, args } = &expr.kind
        && name.id() == KS.ln
        && args.len() == 1
        && let AstKind::FunctionCall {
            name: inner_name,
            args: inner_args,
        } = &args[0].kind
        && inner_name.id() == KS.exp
        && inner_args.len() == 1
    {
        return Some(Arc::clone(&inner_args[0]));
    }
    None
});

rule_arc!(ExpMulLnRule, "exp_mul_ln", 80, Algebraic, &[ExprKind::Function], alters_domain: true, |expr: &Expr, _context: &RuleContext| {
    if let AstKind::FunctionCall { name, args } = &expr.kind
        && name.id() == KS.exp
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
                    && inner_name.id() == KS.ln
                    && inner_args.len() == 1
                {
                    // exp(a * b * ln(x)) = x^(a*b)
                    let other_factors: Vec<Arc<Expr>> = factors.iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .map(|(_, f)| Arc::clone(f))
                        .collect();

                    // Optimize construction: if only 1 factor, unwrap it (arc clone)
                    let exponent = if other_factors.len() == 1 {
                        Arc::clone(&other_factors[0])
                    } else {
                        Arc::new(Expr::product_from_arcs(other_factors))
                    };

                    return Some(Arc::new(Expr::pow_from_arcs(Arc::clone(&inner_args[0]), exponent)));
                }
            }
        }
    }
    None
});

rule_arc!(EPowLnRule, "e_pow_ln", 85, Algebraic, &[ExprKind::Pow], alters_domain: true, |expr: &Expr, context: &RuleContext| {
    if let AstKind::Pow(base, exp) = &expr.kind
        && let AstKind::Symbol(s) = &base.kind
        && s.id() == KS.e
        && !context.known_symbols.contains("e")
        && let AstKind::FunctionCall { name, args } = &exp.kind
        && name.id() == KS.ln
        && args.len() == 1
    {
        return Some(Arc::clone(&args[0]));
    }
    None
});

rule_arc!(EPowMulLnRule, "e_pow_mul_ln", 85, Algebraic, &[ExprKind::Pow], alters_domain: true, |expr: &Expr, context: &RuleContext| {
    if let AstKind::Pow(base, exp) = &expr.kind
        && let AstKind::Symbol(s) = &base.kind
        && s.id() == KS.e
        && !context.known_symbols.contains("e")
    {
        // Check if exponent is a Product containing ln(x)
        if let AstKind::Product(factors) = &exp.kind {
            for (i, factor) in factors.iter().enumerate() {
                if let AstKind::FunctionCall {
                    name: inner_name,
                    args: inner_args,
                } = &factor.kind
                    && inner_name.id() == KS.ln
                    && inner_args.len() == 1
                {
                    // e^(a * b * ln(x)) = x^(a*b)
                    let other_factors: Vec<Arc<Expr>> = factors.iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .map(|(_, f)| Arc::clone(f))
                        .collect();

                    let exponent = if other_factors.len() == 1 {
                        Arc::clone(&other_factors[0])
                    } else {
                        Arc::new(Expr::product_from_arcs(other_factors))
                    };

                    return Some(Arc::new(Expr::pow_from_arcs(Arc::clone(&inner_args[0]), exponent)));
                }
            }
        }
    }
    None
});
