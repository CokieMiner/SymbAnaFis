use crate::core::expr::{Expr, ExprKind as AstKind};
use crate::core::known_symbols::{get_symbol, COSH, COTH, CSCH, SECH, SINH, TANH};
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
            && num_name.id() == *SINH
            && den_name.id() == *COSH
            && num_args.len() == 1
            && den_args.len() == 1
            && num_args[0] == den_args[0]
        {
            return Some(Expr::func_symbol(get_symbol(&TANH), (*num_args[0]).clone()));
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
            && num_name.id() == *COSH
            && den_name.id() == *SINH
            && num_args.len() == 1
            && den_args.len() == 1
            && num_args[0] == den_args[0]
        {
            return Some(Expr::func_symbol(get_symbol(&COTH), (*num_args[0]).clone()));
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
            && (*n - 1.0).abs() < 1e-10
            && let AstKind::FunctionCall { name, args } = &den.kind
            && name.id() == *COSH
            && args.len() == 1
        {
            return Some(Expr::new(AstKind::FunctionCall {
                name: get_symbol(&SECH),
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
            && (*n - 1.0).abs() < 1e-10
            && let AstKind::FunctionCall { name, args } = &den.kind
            && name.id() == *SINH
            && args.len() == 1
        {
            return Some(Expr::new(AstKind::FunctionCall {
                name: get_symbol(&CSCH),
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
            && (*n - 1.0).abs() < 1e-10
            && let AstKind::FunctionCall { name, args } = &den.kind
            && name.id() == *TANH
            && args.len() == 1
        {
            return Some(Expr::new(AstKind::FunctionCall {
                name: get_symbol(&COTH),
                args: args.clone(),
            }));
        }
        None
    }
);
