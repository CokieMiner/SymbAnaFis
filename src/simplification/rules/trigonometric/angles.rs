use crate::core::expr::{Expr, ExprKind as AstKind};
use crate::core::known_symbols::{KS, get_symbol};
use crate::core::symbol::InternedSymbol;
use crate::core::traits::EPSILON;
use crate::simplification::helpers::extract_coeff_arc;
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use std::sync::Arc;

rule_with_helpers_arc!(
    CosDoubleAngleDifferenceRule,
    "cos_double_angle_difference",
    85,
    Trigonometric,
    &[ExprKind::Sum],
    helpers: {
        // Helper to extract negated term
        fn extract_negated_arc(term: &Expr) -> Option<Arc<Expr>> {
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
                && let AstKind::FunctionCall { name, args } = &base.kind
                && args.len() == 1
            {
                // Exact check for expected power exponent
                #[allow(clippy::float_cmp)] // Comparing against exact constant (power)
                let matches_power = matches!(exp.kind, AstKind::Number(n) if n == power);
                if matches_power {
                    return Some((name.clone(), Arc::clone(&args[0])));
                }
            }
            None
        }
    },
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Sum(terms) = &expr.kind
            && terms.len() == 2
        {
            // Look for patterns like cos^2(x) - sin^2(x) or sin^2(x) - cos^2(x)
            let t1 = &terms[0];
            let t2 = &terms[1];

            // Try t1 = cos^2(x), t2 = -sin^2(x)
            if let (Some((name1, arg1)), Some(negated)) =
                (get_fn_pow_symbol_arc(t1, 2.0), extract_negated_arc(t2))
                && name1.id() == KS.cos
                && let Some((name2, arg2)) = get_fn_pow_symbol_arc(&negated, 2.0)
                && name2.id() == KS.sin
                && arg1 == arg2
            {
                // cos^2(x) - sin^2(x) = cos(2x)
                let arg_doubled = Arc::new(Expr::product_from_arcs(vec![
                    Arc::new(Expr::number(2.0)),
                    arg1
                ]));
                return Some(Arc::new(Expr::func_multi_from_arcs_symbol(
                    get_symbol(KS.cos),
                    vec![arg_doubled],
                )));
            }

            // Try t1 = sin^2(x), t2 = -cos^2(x)
            if let (Some((name1, arg1)), Some(negated)) =
                (get_fn_pow_symbol_arc(t1, 2.0), extract_negated_arc(t2))
                && name1.id() == KS.sin
                && let Some((name2, arg2)) = get_fn_pow_symbol_arc(&negated, 2.0)
                && name2.id() == KS.cos
                && arg1 == arg2
            {
                // sin^2(x) - cos^2(x) = -cos(2x)
                let arg_doubled = Arc::new(Expr::product_from_arcs(vec![
                    Arc::new(Expr::number(2.0)),
                    arg1
                ]));
                let cos_expr = Arc::new(Expr::func_multi_from_arcs_symbol(
                    get_symbol(KS.cos),
                    vec![arg_doubled],
                ));
                return Some(Arc::new(Expr::product_from_arcs(vec![
                    Arc::new(Expr::number(-1.0)),
                    cos_expr,
                ])));
            }
        }
        None
    }
);

rule_with_helpers_arc!(
    TrigSumDifferenceRule,
    "trig_sum_difference",
    70,
    Trigonometric,
    &[ExprKind::Sum],
    helpers: {
        fn get_fn_pow_symbol_arc(expr: &Expr, power: f64) -> Option<(InternedSymbol, Arc<Expr>)> {
            if let AstKind::Pow(base, exp) = &expr.kind
                && let AstKind::FunctionCall { name, args } = &base.kind
                && args.len() == 1
            {
                // Exact check for expected power exponent
                #[allow(clippy::float_cmp)] // Comparing against exact constant (power)
                let matches_power = matches!(exp.kind, AstKind::Number(n) if n == power);
                if matches_power {
                    return Some((name.clone(), Arc::clone(&args[0])));
                }
            }
            None
        }
    },
    |expr: &Expr, _context: &RuleContext| {
        // k*sin(x)cos(y) + k*cos(x)sin(y) = k*sin(x+y)
        // k*sin(x)cos(y) - k*cos(x)sin(y) = k*sin(x-y)
        if let AstKind::Sum(terms) = &expr.kind
            && terms.len() == 2
        {
            let u = &terms[0];
            let v = &terms[1];

            let (c1, r1) = extract_coeff_arc(u);
            let (c2, r2) = extract_coeff_arc(v);

            // Helper to check if expr is exactly sin(a)*cos(b)
            // Returns Some((a, b))
            let parse_sin_cos = |e: &Expr| -> Option<(Arc<Expr>, Arc<Expr>)> {
                // Helper to get name/arg from FunctionCall OR Pow(FunctionCall, 1)
                let get_op = |t: &Expr| -> Option<(InternedSymbol, Arc<Expr>)> {
                    if let AstKind::FunctionCall { name, args } = &t.kind
                        && args.len() == 1
                    {
                        return Some((name.clone(), Arc::clone(&args[0])));
                    }
                    get_fn_pow_symbol_arc(t, 1.0)
                };

                if let AstKind::Product(factors) = &e.kind
                    && factors.len() == 2
                {
                    let f1 = &factors[0];
                    let f2 = &factors[1];
                    // Try matches
                    if let (Some((n1, a)), Some((n2, b))) = (get_op(f1), get_op(f2)) {
                        if n1.id() == KS.sin && n2.id() == KS.cos {
                            return Some((a, b));
                        }
                        if n1.id() == KS.cos && n2.id() == KS.sin {
                            return Some((b, a));
                        }
                    }
                }
                None
            };

            // Pattern 1: Match sin(x)cos(y) + cos(x)sin(y)
            // We need to identify x and y consistently
            if let (Some((x1, y1)), Some((x2, y2))) = (parse_sin_cos(&r1), parse_sin_cos(&r2)) {
                // Check logic for sin(x+y): needs sin(x)cos(y) + sin(y)cos(x)
                if x1 == y2 && y1 == x2 {
                    // Found terms: sin(x)cos(y) and sin(y)cos(x) (aka cos(x)sin(y))
                    if (c1 - c2).abs() < EPSILON {
                        // c1 == c2: k * sin(x+y)
                        return Some(Arc::new(Expr::product_from_arcs(vec![
                            Arc::new(Expr::number(c1)),
                            Arc::new(Expr::func_multi_from_arcs_symbol(get_symbol(KS.sin), vec![
                                Arc::new(Expr::sum_from_arcs(vec![x1, y1]))
                            ])),
                        ])));
                    } else if (c1 + c2).abs() < EPSILON {
                        // c1 == -c2: k * sin(x-y)
                        // Result is k*sin(x-y)
                        return Some(Arc::new(Expr::product_from_arcs(vec![
                            Arc::new(Expr::number(c1)),
                            Arc::new(Expr::func_multi_from_arcs_symbol(
                                get_symbol(KS.sin),
                                vec![Arc::new(Expr::sum_from_arcs(vec![
                                    x1,
                                    Arc::new(Expr::product_from_arcs(vec![Arc::new(Expr::number(-1.0)), y1]))
                                ]))]
                            )),
                        ])));
                    }
                }
            }
        }
        None
    }
);

fn is_cos_minus_sin(expr: &Expr) -> bool {
    // In n-ary, cos(x) - sin(x) = Sum([cos(x), Product([-1, sin(x)])])
    if let AstKind::Sum(terms) = &expr.kind
        && terms.len() == 2
    {
        let a = &terms[0];
        let b = &terms[1];

        if is_cos(a)
            && let AstKind::Product(factors) = &b.kind
            && factors.len() == 2
            && let AstKind::Number(n) = &factors[0].kind
            && (*n + 1.0).abs() < EPSILON
        {
            return is_sin(&factors[1]);
        }
    }
    false
}

fn is_cos_plus_sin(expr: &Expr) -> bool {
    if let AstKind::Sum(terms) = &expr.kind {
        if terms.len() == 2 {
            is_cos(&terms[0]) && is_sin(&terms[1])
        } else {
            false
        }
    } else {
        false
    }
}

fn is_cos(expr: &Expr) -> bool {
    matches!(&expr.kind, AstKind::FunctionCall { name, args } if name.id() == KS.cos && args.len() == 1)
}

fn is_sin(expr: &Expr) -> bool {
    matches!(&expr.kind, AstKind::FunctionCall { name, args } if name.id() == KS.sin && args.len() == 1)
}

fn get_cos_arg_arc(expr: &Expr) -> Option<Arc<Expr>> {
    if let AstKind::FunctionCall { name, args } = &expr.kind {
        (name.id() == KS.cos && args.len() == 1).then(|| Arc::clone(&args[0]))
    } else if let AstKind::Sum(terms) = &expr.kind {
        if terms.is_empty() {
            None
        } else {
            get_cos_arg_arc(&terms[0])
        }
    } else {
        None
    }
}

fn get_sin_arg_arc(expr: &Expr) -> Option<Arc<Expr>> {
    if let AstKind::FunctionCall { name, args } = &expr.kind {
        (name.id() == KS.sin && args.len() == 1).then(|| Arc::clone(&args[0]))
    } else if let AstKind::Sum(terms) = &expr.kind {
        if terms.len() >= 2 {
            // Look in second term (which may be -sin(x) = Product([-1, sin(x)]))
            let second = &terms[1];
            if let AstKind::Product(factors) = &second.kind
                && factors.len() == 2
                && let AstKind::Number(n) = &factors[0].kind
                && (*n + 1.0).abs() < EPSILON
            {
                return get_sin_arg_arc(&factors[1]);
            }
            get_sin_arg_arc(second)
        } else {
            None
        }
    } else {
        None
    }
}

rule_arc!(
    TrigProductToDoubleAngleRule,
    "trig_product_to_double_angle",
    90,
    Trigonometric,
    &[ExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Product(factors) = &expr.kind
            && factors.len() == 2
        {
            let a = &factors[0];
            let b = &factors[1];

            let (cos_minus_sin, cos_plus_sin) = if is_cos_minus_sin(a) && is_cos_plus_sin(b) {
                (a, b)
            } else if is_cos_minus_sin(b) && is_cos_plus_sin(a) {
                (b, a)
            } else {
                return None;
            };

            if let Some(arg) = get_cos_arg_arc(cos_minus_sin)
                && get_cos_arg_arc(cos_plus_sin) == Some(Arc::clone(&arg))
                && get_sin_arg_arc(cos_minus_sin) == Some(Arc::clone(&arg))
                && get_sin_arg_arc(cos_plus_sin) == Some(Arc::clone(&arg))
            {
                // (cos(x) - sin(x))(cos(x) + sin(x)) = cos(2x)
                let doubled_arg = Arc::new(Expr::product_from_arcs(vec![
                    Arc::new(Expr::number(2.0)),
                    arg,
                ]));
                return Some(Arc::new(Expr::func_multi_from_arcs_symbol(
                    get_symbol(KS.cos),
                    vec![doubled_arg],
                )));
            }
        }
        None
    }
);

// Contracts k*sin(x)*cos(x) to (k/2)*sin(2*x) for any coefficient k
// Handles both orderings: sin(x)*cos(x) and cos(x)*sin(x)
rule_arc!(
    SinProductToDoubleAngleRule,
    "sin_product_to_double_angle",
    90,
    Trigonometric,
    &[ExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Product(factors) = &expr.kind {
            // Look for patterns: k * sin(x) * cos(x) in any order
            // The identity: sin(x)*cos(x) = sin(2x)/2
            // So: k * sin(x) * cos(x) = (k/2) * sin(2x)

            let mut coeff = 1.0;
            let mut sin_arg: Option<Arc<Expr>> = None;
            let mut cos_arg: Option<Arc<Expr>> = None;
            let mut other_factors: Vec<Arc<Expr>> = Vec::new();

            for f in factors {
                match &f.kind {
                    AstKind::Number(n) => {
                        coeff *= n;
                    }
                    AstKind::FunctionCall { name, args }
                        if name.id() == KS.sin && args.len() == 1 =>
                    {
                        if sin_arg.is_some() {
                            // Multiple sin factors - don't handle
                            other_factors.push(Arc::clone(f));
                        } else {
                            sin_arg = Some(Arc::clone(&args[0]));
                        }
                    }
                    AstKind::FunctionCall { name, args }
                        if name.id() == KS.cos && args.len() == 1 =>
                    {
                        if cos_arg.is_some() {
                            // Multiple cos factors - don't handle
                            other_factors.push(Arc::clone(f));
                        } else {
                            cos_arg = Some(Arc::clone(&args[0]));
                        }
                    }
                    _ => {
                        other_factors.push(Arc::clone(f));
                    }
                }
            }

            // Check if we have sin(x) * cos(x) with same argument
            if let Some(s_arg) = sin_arg
                && let Some(c_arg) = cos_arg
                && s_arg == c_arg
                && other_factors.is_empty()
            {
                // k * sin(x) * cos(x) = (k/2) * sin(2*x)
                let doubled_arg = Arc::new(Expr::product_from_arcs(vec![
                    Arc::new(Expr::number(2.0)),
                    s_arg,
                ]));
                let sin_2x = Arc::new(Expr::func_multi_from_arcs_symbol(
                    get_symbol(KS.sin),
                    vec![doubled_arg],
                ));

                let new_coeff = coeff / 2.0;
                if (new_coeff - 1.0).abs() < EPSILON {
                    // Coefficient is 1, just return sin(2x)
                    return Some(sin_2x);
                }
                // Return (k/2) * sin(2x)
                return Some(Arc::new(Expr::product_from_arcs(vec![
                    Arc::new(Expr::number(new_coeff)),
                    sin_2x,
                ])));
            }
        }
        None
    }
);
