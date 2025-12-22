use crate::core::known_symbols::{ABS, ABS_CAP, SGN, SIGN, get_symbol};
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use crate::{Expr, ExprKind as AstKind};
use std::sync::Arc;

rule!(
    AbsNumericRule,
    "abs_numeric",
    95,
    Algebraic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && (name.id() == *ABS || name.id() == *ABS_CAP)
            && args.len() == 1
            && let AstKind::Number(n) = &args[0].kind
        {
            return Some(Expr::number(n.abs()));
        }
        None
    }
);

rule!(
    AbsAbsRule,
    "abs_abs",
    90,
    Algebraic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && (name.id() == *ABS || name.id() == *ABS_CAP)
            && args.len() == 1
            && let AstKind::FunctionCall {
                name: inner_name,
                args: inner_args,
            } = &args[0].kind
            && (inner_name.id() == *ABS || inner_name.id() == *ABS_CAP)
            && inner_args.len() == 1
        {
            return Some((*args[0]).clone());
        }
        None
    }
);

rule!(
    AbsNegRule,
    "abs_neg",
    90,
    Algebraic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && (name.id() == *ABS || name.id() == *ABS_CAP)
            && args.len() == 1
        {
            // Check for -x (represented as Product([-1, x]))
            if let AstKind::Product(factors) = &args[0].kind
                && factors.len() >= 2
                && let Some(first) = factors.first()
                && let AstKind::Number(n) = &first.kind
                && *n == -1.0
            {
                // Get the rest of the factors
                let rest: Vec<Arc<Expr>> = factors.iter().skip(1).cloned().collect();
                let inner = Expr::product_from_arcs(rest);
                return Some(Expr::func_symbol(get_symbol(&ABS), inner));
            }
        }
        None
    }
);

rule!(
    AbsSquareRule,
    "abs_square",
    85,
    Algebraic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && (name.id() == *ABS || name.id() == *ABS_CAP)
            && args.len() == 1
        {
            // Check for x^(even number)
            if let AstKind::Pow(_, exp) = &args[0].kind
                && let AstKind::Number(n) = &exp.kind
            {
                // Check if exponent is a positive even integer
                if *n > 0.0 && n.fract() == 0.0 && (*n as i64) % 2 == 0 {
                    return Some((*args[0]).clone());
                }
            }
        }
        None
    }
);

rule!(
    AbsPowEvenRule,
    "abs_pow_even",
    85,
    Algebraic,
    &[ExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        // abs(x)^n where n is positive even integer -> x^n
        if let AstKind::Pow(base, exp) = &expr.kind
            && let AstKind::FunctionCall { name, args } = &base.kind
            && (name.id() == *ABS || name.id() == *ABS_CAP)
            && args.len() == 1
            && let AstKind::Number(n) = &exp.kind
        {
            // Check if exponent is a positive even integer
            if *n > 0.0 && n.fract() == 0.0 && (*n as i64) % 2 == 0 {
                return Some(Expr::pow((*args[0]).clone(), exp.as_ref().clone()));
            }
        }
        None
    }
);

rule!(
    SignNumericRule,
    "sign_numeric",
    95,
    Algebraic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && (name.id() == *SIGN || name.id() == *SGN)
            && args.len() == 1
            && let AstKind::Number(n) = &args[0].kind
        {
            if *n > 0.0 {
                return Some(Expr::number(1.0));
            } else if *n < 0.0 {
                return Some(Expr::number(-1.0));
            } else {
                return Some(Expr::number(0.0));
            }
        }
        None
    }
);

rule!(
    SignSignRule,
    "sign_sign",
    90,
    Algebraic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && (name.id() == *SIGN || name.id() == *SGN)
            && args.len() == 1
            && let AstKind::FunctionCall {
                name: inner_name, ..
            } = &args[0].kind
            && (inner_name.id() == *SIGN || inner_name.id() == *SGN)
        {
            return Some((*args[0]).clone());
        }
        None
    }
);

rule!(
    SignAbsRule,
    "sign_abs",
    85,
    Algebraic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && (name.id() == *SIGN || name.id() == *SGN)
            && args.len() == 1
            && let AstKind::FunctionCall {
                name: inner_name, ..
            } = &args[0].kind
            && (inner_name.id() == *ABS || inner_name.id() == *ABS_CAP)
        {
            // sign(abs(x)) = 1 for x != 0
            return Some(Expr::number(1.0));
        }
        None
    }
);

rule!(
    AbsSignMulRule,
    "abs_sign_mul",
    85,
    Algebraic,
    &[ExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Product(factors) = &expr.kind {
            // Check for abs(x) * sign(x) pattern within factors
            for (i, f1) in factors.iter().enumerate() {
                for (j, f2) in factors.iter().enumerate() {
                    if i >= j {
                        continue;
                    }
                    // Check if f1 is abs and f2 is sign (or vice versa)
                    if let (
                        AstKind::FunctionCall {
                            name: name1,
                            args: args1,
                        },
                        AstKind::FunctionCall {
                            name: name2,
                            args: args2,
                        },
                    ) = (&f1.kind, &f2.kind)
                        && args1.len() == 1
                        && args2.len() == 1
                        && args1[0] == args2[0]
                    {
                        let is_abs_sign = (name1.id() == *ABS || name1.id() == *ABS_CAP)
                            && (name2.id() == *SIGN || name2.id() == *SGN);
                        let is_sign_abs = (name1.id() == *SIGN || name1.id() == *SGN)
                            && (name2.id() == *ABS || name2.id() == *ABS_CAP);
                        if is_abs_sign || is_sign_abs {
                            // abs(x) * sign(x) = x, replace these two factors with x
                            let mut new_factors: Vec<Expr> = factors
                                .iter()
                                .enumerate()
                                .filter(|(k, _)| *k != i && *k != j)
                                .map(|(_, f)| (**f).clone())
                                .collect();
                            new_factors.push((*args1[0]).clone());
                            if new_factors.len() == 1 {
                                return Some(new_factors.into_iter().next().unwrap());
                            } else {
                                return Some(Expr::product(new_factors));
                            }
                        }
                    }
                }
            }
        }
        None
    }
);
