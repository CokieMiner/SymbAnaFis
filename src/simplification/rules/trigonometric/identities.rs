use crate::core::expr::{Expr, ExprKind as AstKind};
use crate::core::known_symbols::{KS, get_symbol};
use crate::core::symbol::InternedSymbol;
use crate::core::traits::EPSILON;
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use std::sync::Arc;

rule_arc!(
    PythagoreanIdentityRule,
    "pythagorean_identity",
    80,
    Trigonometric,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Sum(terms) = &expr.kind
            && terms.len() == 2
        {
            let u = &terms[0];
            let v = &terms[1];

            // sin^2(x) + cos^2(x) = 1
            if let (AstKind::Pow(sin_base, sin_exp), AstKind::Pow(cos_base, cos_exp)) =
                (&u.kind, &v.kind)
                && {
                    // Exact check for power 2.0 (square)
                    #[allow(clippy::float_cmp, reason = "Comparing against exact constant 2.0")]
                    let is_two = matches!(&sin_exp.kind, AstKind::Number(n) if *n == 2.0);
                    is_two
                }
                && {
                    // Exact check for power 2.0 (square)
                    #[allow(clippy::float_cmp, reason = "Comparing against exact constant 2.0")]
                    let is_two = matches!(&cos_exp.kind, AstKind::Number(n) if *n == 2.0);
                    is_two
                }
                && let (
                    AstKind::FunctionCall {
                        name: sin_name,
                        args: sin_args,
                    },
                    AstKind::FunctionCall {
                        name: cos_name,
                        args: cos_args,
                    },
                ) = (&sin_base.kind, &cos_base.kind)
                && sin_name.id() == KS.sin
                && cos_name.id() == KS.cos
                && sin_args.len() == 1
                && cos_args.len() == 1
                && sin_args[0] == cos_args[0]
            {
                return Some(Arc::new(Expr::number(1.0)));
            }

            // cos^2(x) + sin^2(x) = 1
            if let (AstKind::Pow(cos_base, cos_exp), AstKind::Pow(sin_base, sin_exp)) =
                (&u.kind, &v.kind)
                && {
                    // Exact check for power 2.0 (square)
                    #[allow(clippy::float_cmp, reason = "Comparing against exact constant 2.0")]
                    let is_two = matches!(&cos_exp.kind, AstKind::Number(n) if *n == 2.0);
                    is_two
                }
                && {
                    // Exact check for power 2.0 (square)
                    #[allow(clippy::float_cmp, reason = "Comparing against exact constant 2.0")]
                    let is_two = matches!(&sin_exp.kind, AstKind::Number(n) if *n == 2.0);
                    is_two
                }
                && let (
                    AstKind::FunctionCall {
                        name: cos_name,
                        args: cos_args,
                    },
                    AstKind::FunctionCall {
                        name: sin_name,
                        args: sin_args,
                    },
                ) = (&cos_base.kind, &sin_base.kind)
                && cos_name.id() == KS.cos
                && sin_name.id() == KS.sin
                && cos_args.len() == 1
                && sin_args.len() == 1
                && cos_args[0] == sin_args[0]
            {
                return Some(Arc::new(Expr::number(1.0)));
            }
        }
        None
    }
);

rule_with_helpers_arc!(
    PythagoreanComplementsRule,
    "pythagorean_complements",
    70,
    Trigonometric,
    &[ExprKind::Sum],
    helpers: {
         // Helper to extract negated term from Product([-1, x])
        fn extract_negated(term: &Expr) -> Option<Arc<Expr>> {
            if let AstKind::Product(factors) = &term.kind
                && factors.len() == 2
                && let AstKind::Number(n) = &factors[0].kind
                && (*n + 1.0).abs() < EPSILON
            {
                return Some(Arc::clone(&factors[1]));
            }
            None
        }

        fn get_fn_pow_symbol_arc(expr: &Expr, power: f64) -> Option<(InternedSymbol, Arc<Expr>)> {
            if let AstKind::Pow(base, exp) = &expr.kind
                && {
                    // Exact check for power constant
                    #[allow(clippy::float_cmp, reason = "Comparing against exact constant (power)")]
                    let is_power = matches!(exp.kind, AstKind::Number(n) if n == power);
                    is_power
                }
                && let AstKind::FunctionCall { name, args } = &base.kind
                && args.len() == 1
            {
                return Some((name.clone(), Arc::clone(&args[0])));
            }
            None
        }
    },
    |expr: &Expr, _context: &RuleContext| {
        // 1 - cos^2(x) = sin^2(x)
        // 1 - sin^2(x) = cos^2(x)
        if let AstKind::Sum(terms) = &expr.kind
            && terms.len() == 2
        {
            let lhs = &terms[0];
            let rhs = &terms[1];

            // 1 + (-cos^2(x)) = sin^2(x)
            if {
                // Exact check for constant 1.0
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
                let is_one = matches!(&lhs.kind, AstKind::Number(n) if *n == 1.0);
                is_one
            }
                && let Some(negated) = extract_negated(rhs)
            {
                if let Some((name, arg)) = get_fn_pow_symbol_arc(&negated, 2.0)
                    && name.id() == KS.cos
                {
                    return Some(Arc::new(Expr::pow_static(
                        Expr::func_symbol(get_symbol(KS.sin), arg.as_ref().clone()),
                        Expr::number(2.0),
                    )));
                }
                if let Some((name, arg)) = get_fn_pow_symbol_arc(&negated, 2.0)
                    && name.id() == KS.sin
                {
                    return Some(Arc::new(Expr::pow_static(
                        Expr::func_symbol(get_symbol(KS.cos), arg.as_ref().clone()),
                        Expr::number(2.0),
                    )));
                }
            }

            // (-cos^2(x)) + 1 = sin^2(x)
            if {
                // Exact check for constant 1.0
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
                let is_one = matches!(&rhs.kind, AstKind::Number(n) if *n == 1.0);
                is_one
            }
                && let Some(negated) = extract_negated(lhs)
            {
                if let Some((name, arg)) = get_fn_pow_symbol_arc(&negated, 2.0)
                    && name.id() == KS.cos
                {
                    return Some(Arc::new(Expr::pow_static(
                        Expr::func_symbol(get_symbol(KS.sin), arg.as_ref().clone()),
                        Expr::number(2.0),
                    )));
                }
                if let Some((name, arg)) = get_fn_pow_symbol_arc(&negated, 2.0)
                    && name.id() == KS.sin
                {
                    return Some(Arc::new(Expr::pow_static(
                        Expr::func_symbol(get_symbol(KS.cos), arg.as_ref().clone()),
                        Expr::number(2.0),
                    )));
                }
            }
        }
        None
    }
);

rule_with_helpers_arc!(
    PythagoreanTangentRule,
    "pythagorean_tangent",
    70,
    Trigonometric,
    &[ExprKind::Sum],
    helpers: {
        fn get_fn_pow_symbol_arc(expr: &Expr, power: f64) -> Option<(InternedSymbol, Arc<Expr>)> {
            if let AstKind::Pow(base, exp) = &expr.kind
                && {
                    // Exact check for power constant
                    #[allow(clippy::float_cmp, reason = "Comparing against exact constant (power)")]
                    let is_power = matches!(exp.kind, AstKind::Number(n) if n == power);
                    is_power
                }
                && let AstKind::FunctionCall { name, args } = &base.kind
                && args.len() == 1
            {
                return Some((name.clone(), Arc::clone(&args[0])));
            }
            None
        }
    },
    |expr: &Expr, _context: &RuleContext| {
        // tan^2(x) + 1 = sec^2(x)
        // 1 + tan^2(x) = sec^2(x)
        // cot^2(x) + 1 = csc^2(x)
        // 1 + cot^2(x) = csc^2(x)
        if let AstKind::Sum(terms) = &expr.kind
            && terms.len() == 2
        {
            let lhs = &terms[0];
            let rhs = &terms[1];

            // tan^2(x) + 1 = sec^2(x)
            if let Some((name, arg)) = get_fn_pow_symbol_arc(lhs, 2.0)
                && name.id() == KS.tan
                && {
                    // Exact check for constant 1.0
                    #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
                    let is_one = matches!(&rhs.kind, AstKind::Number(n) if *n == 1.0);
                    is_one
                }
            {
                return Some(Arc::new(Expr::pow_static(
                    Expr::func_symbol(get_symbol(KS.sec), arg.as_ref().clone()),
                    Expr::number(2.0),
                )));
            }

            // 1 + tan^2(x) = sec^2(x)
            if {
                // Exact check for constant 1.0
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
                let is_one = matches!(&lhs.kind, AstKind::Number(n) if *n == 1.0);
                is_one
            }
                && let Some((name, arg)) = get_fn_pow_symbol_arc(rhs, 2.0)
                && name.id() == KS.tan
            {
                return Some(Arc::new(Expr::pow_static(
                    Expr::func_symbol(get_symbol(KS.sec), arg.as_ref().clone()),
                    Expr::number(2.0),
                )));
            }

            // cot^2(x) + 1 = csc^2(x)
            if let Some((name, arg)) = get_fn_pow_symbol_arc(lhs, 2.0)
                && name.id() == KS.cot
                && {
                    // Exact check for constant 1.0
                    #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
                    let is_one = matches!(&rhs.kind, AstKind::Number(n) if *n == 1.0);
                    is_one
                }
            {
                return Some(Arc::new(Expr::pow_static(
                    Expr::func_symbol(get_symbol(KS.csc), arg.as_ref().clone()),
                    Expr::number(2.0),
                )));
            }

            // 1 + cot^2(x) = csc^2(x)
            if {
                // Exact check for constant 1.0
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
                let is_one = matches!(&lhs.kind, AstKind::Number(n) if *n == 1.0);
                is_one
            }
                && let Some((name, arg)) = get_fn_pow_symbol_arc(rhs, 2.0)
                && name.id() == KS.cot
            {
                return Some(Arc::new(Expr::pow_static(
                    Expr::func_symbol(get_symbol(KS.csc), arg.as_ref().clone()),
                    Expr::number(2.0),
                )));
            }
        }
        None
    }
);
