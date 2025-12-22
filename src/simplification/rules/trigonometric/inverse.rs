use crate::core::expr::{Expr, ExprKind as AstExprKind};
use crate::core::known_symbols::{ACOS, ASIN, ATAN, COS, SIN, TAN};
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
        if (name.id() == *SIN && inner_name.id() == *ASIN)
            || (name.id() == *COS && inner_name.id() == *ACOS)
            || (name.id() == *TAN && inner_name.id() == *ATAN)
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
        if (name.id() == *ASIN && inner_name.id() == *SIN)
            || (name.id() == *ACOS && inner_name.id() == *COS)
            || (name.id() == *ATAN && inner_name.id() == *TAN)
        {
            return Some((**inner_arg).clone());
        }
    }
    None
});
