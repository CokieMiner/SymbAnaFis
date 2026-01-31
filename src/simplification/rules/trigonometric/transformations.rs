use crate::core::expr::{Expr, ExprKind as AstKind};
use crate::core::known_symbols::{KS, get_symbol};
use crate::core::traits::EPSILON;
use crate::simplification::helpers;
use crate::simplification::patterns::trigonometric::get_trig_function;
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};

rule!(
    CofunctionIdentityRule,
    "cofunction_identity",
    85,
    Trigonometric,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        // Helper to extract negated term from Product([-1, x])
        fn extract_negated(term: &Expr) -> Option<Expr> {
            if let AstKind::Product(factors) = &term.kind
                && factors.len() == 2
                && let AstKind::Number(n) = &factors[0].kind
                && (*n + 1.0).abs() < EPSILON
            {
                return Some((*factors[1]).clone());
            }
            None
        }

        if let AstKind::FunctionCall { name, args } = &expr.kind
            && args.len() == 1
        {
            // Check for Sum pattern: pi/2 + (-x) = pi/2 - x
            if let AstKind::Sum(terms) = &args[0].kind
                && terms.len() == 2
            {
                let u = &terms[0];
                let v = &terms[1];

                // Check for pi/2
                let is_pi_div_2 = |e: &Expr| {
                    if let AstKind::Div(num, den) = &e.kind {
                        // Exact check for denominator 2.0 (PI/2)
                        #[allow(clippy::float_cmp, reason = "Comparing against exact constant 2.0")]
                        let is_two = matches!(&den.kind, AstKind::Number(n) if *n == 2.0);
                        helpers::is_pi(num) && is_two
                    } else {
                        helpers::get_numeric_value(e).is_some_and(|value| {
                            helpers::approx_eq(value, std::f64::consts::PI / 2.0)
                        })
                    }
                };

                // Check pi/2 + (-x) pattern
                if is_pi_div_2(u)
                    && let Some(x) = extract_negated(v)
                {
                    match name {
                        n if n.id() == KS.sin => {
                            return Some(Expr::func_symbol(get_symbol(KS.cos), x));
                        }
                        n if n.id() == KS.cos => {
                            return Some(Expr::func_symbol(get_symbol(KS.sin), x));
                        }
                        n if n.id() == KS.tan => {
                            return Some(Expr::func_symbol(get_symbol(KS.cot), x));
                        }
                        n if n.id() == KS.cot => {
                            return Some(Expr::func_symbol(get_symbol(KS.tan), x));
                        }
                        n if n.id() == KS.sec => {
                            return Some(Expr::func_symbol(get_symbol(KS.csc), x));
                        }
                        n if n.id() == KS.csc => {
                            return Some(Expr::func_symbol(get_symbol(KS.sec), x));
                        }
                        _ => {}
                    }
                }

                // Check (-x) + pi/2 pattern
                if is_pi_div_2(v)
                    && let Some(x) = extract_negated(u)
                {
                    match name {
                        n if n.id() == KS.sin => {
                            return Some(Expr::func_symbol(get_symbol(KS.cos), x));
                        }
                        n if n.id() == KS.cos => {
                            return Some(Expr::func_symbol(get_symbol(KS.sin), x));
                        }
                        n if n.id() == KS.tan => {
                            return Some(Expr::func_symbol(get_symbol(KS.cot), x));
                        }
                        n if n.id() == KS.cot => {
                            return Some(Expr::func_symbol(get_symbol(KS.tan), x));
                        }
                        n if n.id() == KS.sec => {
                            return Some(Expr::func_symbol(get_symbol(KS.csc), x));
                        }
                        n if n.id() == KS.csc => {
                            return Some(Expr::func_symbol(get_symbol(KS.sec), x));
                        }
                        _ => {}
                    }
                }
            }
        }
        None
    }
);

rule!(
    TrigPeriodicityRule,
    "trig_periodicity",
    85,
    Trigonometric,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && (name.id() == KS.sin || name.id() == KS.cos)
            && args.len() == 1
            && let AstKind::Sum(terms) = &args[0].kind
            && terms.len() == 2
        {
            let lhs = &terms[0];
            let rhs = &terms[1];

            // x + 2πk = x (for trig functions)
            if helpers::is_multiple_of_two_pi(rhs) {
                return Some(Expr::func(name.clone(), (**lhs).clone()));
            }
            if helpers::is_multiple_of_two_pi(lhs) {
                return Some(Expr::func(name.clone(), (**rhs).clone()));
            }
        }
        None
    }
);

rule!(
    TrigReflectionRule,
    "trig_reflection",
    80,
    Trigonometric,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        // Helper to extract negated term from Product([-1, x])
        fn extract_negated(term: &Expr) -> Option<Expr> {
            if let AstKind::Product(factors) = &term.kind
                && factors.len() == 2
                && let AstKind::Number(n) = &factors[0].kind
                && (*n + 1.0).abs() < EPSILON
            {
                return Some((*factors[1]).clone());
            }
            None
        }

        if let AstKind::FunctionCall { name, args } = &expr.kind
            && (name.id() == KS.sin || name.id() == KS.cos)
            && args.len() == 1
            && let AstKind::Sum(terms) = &args[0].kind
            && terms.len() == 2
        {
            let u = &terms[0];
            let v = &terms[1];

            // Check π + (-x) pattern: sin(π - x) = sin(x), cos(π - x) = -cos(x)
            if helpers::is_pi(u)
                && let Some(x) = extract_negated(v)
            {
                match name {
                    n if n.id() == KS.sin => return Some(Expr::func_symbol(get_symbol(KS.sin), x)),
                    n if n.id() == KS.cos => {
                        return Some(Expr::product(vec![
                            Expr::number(-1.0),
                            Expr::func_symbol(get_symbol(KS.cos), x),
                        ]));
                    }
                    _ => {}
                }
            }

            // Check π + x pattern: sin(π + x) = -sin(x), cos(π + x) = -cos(x)
            if helpers::is_pi(u) {
                match name {
                    n if n.id() == KS.sin || n.id() == KS.cos => {
                        return Some(Expr::product(vec![
                            Expr::number(-1.0),
                            Expr::func_symbol(name.clone(), (**v).clone()),
                        ]));
                    }
                    _ => {}
                }
            }

            if helpers::is_pi(v) {
                match name {
                    n if n.id() == KS.sin || n.id() == KS.cos => {
                        return Some(Expr::product(vec![
                            Expr::number(-1.0),
                            Expr::func_symbol(name.clone(), (**u).clone()),
                        ]));
                    }
                    _ => {}
                }
            }
        }
        None
    }
);

rule!(
    TrigThreePiOverTwoRule,
    "trig_three_pi_over_two",
    80,
    Trigonometric,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        // Helper to extract negated term from Product([-1, x])
        fn extract_negated(term: &Expr) -> Option<Expr> {
            if let AstKind::Product(factors) = &term.kind
                && factors.len() == 2
                && let AstKind::Number(n) = &factors[0].kind
                && (*n + 1.0).abs() < EPSILON
            {
                return Some((*factors[1]).clone());
            }
            None
        }

        if let AstKind::FunctionCall { name, args } = &expr.kind
            && (name.id() == KS.sin || name.id() == KS.cos)
            && args.len() == 1
            && let AstKind::Sum(terms) = &args[0].kind
            && terms.len() == 2
        {
            let u = &terms[0];
            let v = &terms[1];

            // Check 3π/2 + (-x) pattern
            if helpers::is_three_pi_over_two(u)
                && let Some(x) = extract_negated(v)
            {
                match name {
                    n if n.id() == KS.sin => {
                        return Some(Expr::product(vec![
                            Expr::number(-1.0),
                            Expr::func_symbol(get_symbol(KS.cos), x),
                        ]));
                    }
                    n if n.id() == KS.cos => {
                        return Some(Expr::product(vec![
                            Expr::number(-1.0),
                            Expr::func_symbol(get_symbol(KS.sin), x),
                        ]));
                    }
                    _ => {}
                }
            }

            if helpers::is_three_pi_over_two(v)
                && let Some(x) = extract_negated(u)
            {
                match name {
                    n if n.id() == KS.sin => {
                        return Some(Expr::product(vec![
                            Expr::number(-1.0),
                            Expr::func_symbol(get_symbol(KS.cos), x),
                        ]));
                    }
                    n if n.id() == KS.cos => {
                        return Some(Expr::product(vec![
                            Expr::number(-1.0),
                            Expr::func_symbol(get_symbol(KS.sin), x),
                        ]));
                    }
                    _ => {}
                }
            }
        }
        None
    }
);

rule!(
    TrigNegArgRule,
    "trig_neg_arg",
    90,
    Trigonometric,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let Some((name, arg)) = get_trig_function(expr) {
            // Check for Product([-1, x]) pattern
            if let AstKind::Product(factors) = &arg.kind
                && factors.len() == 2
                && let AstKind::Number(n) = &factors[0].kind
                && {
                    // Exact check for -1.0 coefficient
                    #[allow(clippy::float_cmp, reason = "Comparing against exact constant -1.0")]
                    let is_neg_one = *n == -1.0;
                    is_neg_one
                }
            {
                let inner = (*factors[1]).clone();
                match &name {
                    func if func.id() == KS.sin || func.id() == KS.tan => {
                        return Some(Expr::product(vec![
                            Expr::number(-1.0),
                            Expr::func_symbol(name.clone(), inner),
                        ]));
                    }
                    func if func.id() == KS.cos || func.id() == KS.sec => {
                        return Some(Expr::func_symbol(name.clone(), inner));
                    }
                    _ => {}
                }
            }
        }
        None
    }
);
