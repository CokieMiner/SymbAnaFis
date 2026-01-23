use crate::core::known_symbols::{KS, get_symbol};
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use crate::{Expr, ExprKind as AstKind};
use std::sync::Arc;

rule_arc!(
    AbsNumericRule,
    "abs_numeric",
    95,
    Algebraic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.abs
            && args.len() == 1
            && let AstKind::Number(n) = &args[0].kind
        {
            return Some(Arc::new(Expr::number(n.abs())));
        }
        None
    }
);

rule_arc!(
    AbsAbsRule,
    "abs_abs",
    90,
    Algebraic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.abs
            && args.len() == 1
            && let AstKind::FunctionCall {
                name: inner_name,
                args: inner_args,
            } = &args[0].kind
            && inner_name.id() == KS.abs
            && inner_args.len() == 1
        {
            return Some(Arc::clone(&args[0]));
        }
        None
    }
);

rule_arc!(
    AbsNegRule,
    "abs_neg",
    90,
    Algebraic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.abs
            && args.len() == 1
        {
            // Check for -x (represented as Product([-1, x]))
            if let AstKind::Product(factors) = &args[0].kind
                && factors.len() >= 2
                && let Some(first) = factors.first()
                && let AstKind::Number(n) = &first.kind
                // Exact check for -1.0 to identify negative argument
                && {
                    #[allow(clippy::float_cmp)] // Comparing against exact constant -1.0
                    let is_neg_one = *n == -1.0;
                    is_neg_one
                }
            {
                // Get the rest of the factors
                let rest: Vec<Arc<Expr>> = factors.iter().skip(1).cloned().collect();
                let inner = Expr::product_from_arcs(rest);
                return Some(Arc::new(Expr::func_symbol(get_symbol(KS.abs), inner)));
            }
        }
        None
    }
);

rule_arc!(
    AbsSquareRule,
    "abs_square",
    85,
    Algebraic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.abs
            && args.len() == 1
        {
            // Check for x^(even number)
            if let AstKind::Pow(_, exp) = &args[0].kind
                && let AstKind::Number(n) = &exp.kind
            {
                // Check if exponent is a positive even integer
                if *n > 0.0 && n.fract() == 0.0 {
                    // Checked fract() == 0.0, so cast is safe
                    // Split logic to avoid experimental attribute on boolean expression
                    #[allow(clippy::cast_possible_truncation)]
                    // Checked fract()==0.0, so cast is safe
                    if (*n as i64) % 2 == 0 {
                        return Some(Arc::clone(&args[0]));
                    }
                }
            }
        }
        None
    }
);

rule_arc!(
    AbsPowEvenRule,
    "abs_pow_even",
    85,
    Algebraic,
    &[ExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        // abs(x)^n where n is positive even integer -> x^n
        if let AstKind::Pow(base, exp) = &expr.kind
            && let AstKind::FunctionCall { name, args } = &base.kind
            && name.id() == KS.abs
            && args.len() == 1
            && let AstKind::Number(n) = &exp.kind
        {
            // Check if exponent is a positive even integer
            if *n > 0.0 && n.fract() == 0.0 {
                // Checked fract() == 0.0, so cast is safe
                #[allow(clippy::cast_possible_truncation)] // Checked fract()==0.0, so cast is safe
                if (*n as i64) % 2 == 0 {
                    return Some(Arc::new(Expr::pow_from_arcs(
                        Arc::clone(&args[0]),
                        Arc::clone(exp),
                    )));
                }
            }
        }
        None
    }
);

rule_arc!(
    SignNumericRule,
    "sign_numeric",
    95,
    Algebraic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && (name.id() == KS.sign || name.id() == KS.sgn)
            && args.len() == 1
            && let AstKind::Number(n) = &args[0].kind
        {
            if *n > 0.0 {
                return Some(Arc::new(Expr::number(1.0)));
            } else if *n < 0.0 {
                return Some(Arc::new(Expr::number(-1.0)));
            }
            return Some(Arc::new(Expr::number(0.0)));
        }
        None
    }
);

rule_arc!(
    SignSignRule,
    "sign_sign",
    90,
    Algebraic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && (name.id() == KS.sign || name.id() == KS.sgn)
            && args.len() == 1
            && let AstKind::FunctionCall {
                name: inner_name, ..
            } = &args[0].kind
            && (inner_name.id() == KS.sign || inner_name.id() == KS.sgn)
        {
            return Some(Arc::clone(&args[0]));
        }
        None
    }
);

rule_arc!(
    SignAbsRule,
    "sign_abs",
    85,
    Algebraic,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && (name.id() == KS.sign || name.id() == KS.sgn)
            && args.len() == 1
            && let AstKind::FunctionCall {
                name: inner_name, ..
            } = &args[0].kind
            && inner_name.id() == KS.abs
        {
            // sign(abs(x)) = 1 for x != 0
            return Some(Arc::new(Expr::number(1.0)));
        }
        None
    }
);

rule_arc!(
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
                        let is_abs_sign =
                            name1.id() == KS.abs && (name2.id() == KS.sign || name2.id() == KS.sgn);
                        let is_sign_abs =
                            (name1.id() == KS.sign || name1.id() == KS.sgn) && name2.id() == KS.abs;
                        if is_abs_sign || is_sign_abs {
                            // abs(x) * sign(x) = x, replace these two factors with x
                            let mut new_factors: Vec<Arc<Expr>> = factors
                                .iter()
                                .enumerate()
                                .filter(|(k, _)| *k != i && *k != j)
                                .map(|(_, f)| Arc::clone(f))
                                .collect();
                            new_factors.push(Arc::clone(&args1[0]));
                            if new_factors.len() == 1 {
                                return Some(
                                    new_factors
                                        .into_iter()
                                        .next()
                                        .expect("New factors guaranteed to have one element"),
                                );
                            }
                            return Some(Arc::new(Expr::product_from_arcs(new_factors)));
                        }
                    }
                }
            }
        }
        None
    }
);
