use crate::core::expr::{Expr, ExprKind as AstExprKind};
use crate::core::known_symbols::KS;
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};

rule!(InverseTrigIdentityRule, "inverse_trig_identity", 90, Trigonometric, &[ExprKind::Function], alters_domain: true, |expr: &Expr, _context: &RuleContext| {
    if let AstExprKind::FunctionCall { name, args } = &expr.kind
        && args.len() == 1
        && let AstExprKind::FunctionCall {
            name: inner_name,
            args: inner_args,
        } = &args[0].kind
        && inner_args.len() == 1
    {
        let inner_arg = &inner_args[0];
        // sin(asin(x)) = x, etc.  O(1) ID comparison
        if (name.id() == KS.sin && inner_name.id() == KS.asin)
            || (name.id() == KS.cos && inner_name.id() == KS.acos)
            || (name.id() == KS.tan && inner_name.id() == KS.atan)
        {
            return Some((**inner_arg).clone());
        }
    }
    None
});

rule!(InverseTrigCompositionRule, "inverse_trig_composition", 85, Trigonometric, &[ExprKind::Function], alters_domain: true, |expr: &Expr, _context: &RuleContext| {
    if let AstExprKind::FunctionCall { name, args } = &expr.kind
        && args.len() == 1
        && let AstExprKind::FunctionCall {
            name: inner_name,
            args: inner_args,
        } = &args[0].kind
        && inner_args.len() == 1
    {
        let inner_arg = &inner_args[0];
        // asin(sin(x)) = x, etc.  O(1) ID comparison
        if (name.id() == KS.asin && inner_name.id() == KS.sin)
            || (name.id() == KS.acos && inner_name.id() == KS.cos)
            || (name.id() == KS.atan && inner_name.id() == KS.tan)
        {
            return Some((**inner_arg).clone());
        }
    }
    None
});
