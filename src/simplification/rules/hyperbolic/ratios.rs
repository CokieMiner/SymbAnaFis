use crate::core::expr::{Expr, ExprKind as AstKind};
use crate::core::known_symbols::{KS, get_symbol};
use crate::core::traits::EPSILON;
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};

rule!(
    SinhCoshToTanhRule,
    "sinh_cosh_to_tanh",
    85,
    Hyperbolic,
    &[ExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(num, den) = &expr.kind
            && let AstKind::FunctionCall {
                name: num_name,
                args: num_args,
            } = &num.kind
            && let AstKind::FunctionCall {
                name: den_name,
                args: den_args,
            } = &den.kind
            && num_name.id() == KS.sinh
            && den_name.id() == KS.cosh
            && num_args.len() == 1
            && den_args.len() == 1
            && num_args[0] == den_args[0]
        {
            return Some(Expr::func_symbol(
                get_symbol(KS.tanh),
                (*num_args[0]).clone(),
            ));
        }
        None
    }
);

rule!(
    CoshSinhToCothRule,
    "cosh_sinh_to_coth",
    85,
    Hyperbolic,
    &[ExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(num, den) = &expr.kind
            && let AstKind::FunctionCall {
                name: num_name,
                args: num_args,
            } = &num.kind
            && let AstKind::FunctionCall {
                name: den_name,
                args: den_args,
            } = &den.kind
            && num_name.id() == KS.cosh
            && den_name.id() == KS.sinh
            && num_args.len() == 1
            && den_args.len() == 1
            && num_args[0] == den_args[0]
        {
            return Some(Expr::func_symbol(
                get_symbol(KS.coth),
                (*num_args[0]).clone(),
            ));
        }
        None
    }
);

rule!(
    OneCoshToSechRule,
    "one_cosh_to_sech",
    85,
    Hyperbolic,
    &[ExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(num, den) = &expr.kind
            && let AstKind::Number(n) = &num.kind
            && (*n - 1.0).abs() < EPSILON
            && let AstKind::FunctionCall { name, args } = &den.kind
            && name.id() == KS.cosh
            && args.len() == 1
        {
            return Some(Expr::new(AstKind::FunctionCall {
                name: get_symbol(KS.sech),
                args: args.clone(),
            }));
        }
        None
    }
);

rule!(
    OneSinhToCschRule,
    "one_sinh_to_csch",
    85,
    Hyperbolic,
    &[ExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(num, den) = &expr.kind
            && let AstKind::Number(n) = &num.kind
            && (*n - 1.0).abs() < EPSILON
            && let AstKind::FunctionCall { name, args } = &den.kind
            && name.id() == KS.sinh
            && args.len() == 1
        {
            return Some(Expr::new(AstKind::FunctionCall {
                name: get_symbol(KS.csch),
                args: args.clone(),
            }));
        }
        None
    }
);

rule!(
    OneTanhToCothRule,
    "one_tanh_to_coth",
    85,
    Hyperbolic,
    &[ExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(num, den) = &expr.kind
            && let AstKind::Number(n) = &num.kind
            && (*n - 1.0).abs() < EPSILON
            && let AstKind::FunctionCall { name, args } = &den.kind
            && name.id() == KS.tanh
            && args.len() == 1
        {
            return Some(Expr::new(AstKind::FunctionCall {
                name: get_symbol(KS.coth),
                args: args.clone(),
            }));
        }
        None
    }
);
