use crate::ast::{Expr, ExprKind as AstKind};
use crate::simplification::helpers;
use crate::simplification::patterns::common::extract_coefficient;
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};

rule!(
    TrigDoubleAngleRule,
    "trig_double_angle",
    85,
    Trigonometric,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name == "sin"
            && args.len() == 1
        {
            let (coeff, rest) = extract_coefficient(&args[0]);
            if coeff == 2.0 {
                // sin(2x) = 2*sin(x)*cos(x)
                return Some(Expr::product(vec![
                    Expr::number(2.0),
                    Expr::func("sin", rest.clone()),
                    Expr::func("cos", rest),
                ]));
            }
        }
        None
    }
);

rule!(
    CosDoubleAngleDifferenceRule,
    "cos_double_angle_difference",
    85,
    Trigonometric,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Sum(terms) = &expr.kind {
            if terms.len() == 2 {
                // Look for patterns like cos^2(x) - sin^2(x) or sin^2(x) - cos^2(x)
                let t1 = &terms[0];
                let t2 = &terms[1];

                // Helper to extract negated term
                fn extract_negated(term: &Expr) -> Option<Expr> {
                    if let AstKind::Product(factors) = &term.kind {
                        if factors.len() == 2 {
                            if let AstKind::Number(n) = &factors[0].kind {
                                if (*n + 1.0).abs() < 1e-10 {
                                    return Some((*factors[1]).clone());
                                }
                            }
                        }
                    }
                    None
                }

                // Try t1 = cos^2(x), t2 = -sin^2(x)
                if let (Some(("cos", arg1)), Some(negated)) =
                    (helpers::get_fn_pow_named(t1, 2.0), extract_negated(t2))
                {
                    if let Some(("sin", arg2)) = helpers::get_fn_pow_named(&negated, 2.0) {
                        if arg1 == arg2 {
                            // cos^2(x) - sin^2(x) = cos(2x)
                            return Some(Expr::func(
                                "cos",
                                Expr::product(vec![Expr::number(2.0), arg1]),
                            ));
                        }
                    }
                }

                // Try t1 = sin^2(x), t2 = -cos^2(x)
                if let (Some(("sin", arg1)), Some(negated)) =
                    (helpers::get_fn_pow_named(t1, 2.0), extract_negated(t2))
                {
                    if let Some(("cos", arg2)) = helpers::get_fn_pow_named(&negated, 2.0) {
                        if arg1 == arg2 {
                            // sin^2(x) - cos^2(x) = -cos(2x)
                            return Some(Expr::product(vec![
                                Expr::number(-1.0),
                                Expr::func("cos", Expr::product(vec![Expr::number(2.0), arg1])),
                            ]));
                        }
                    }
                }
            }
        }
        None
    }
);

rule!(
    TrigSumDifferenceRule,
    "trig_sum_difference",
    70,
    Trigonometric,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        // k*sin(x)cos(y) + k*cos(x)sin(y) = k*sin(x+y)
        // k*sin(x)cos(y) - k*cos(x)sin(y) = k*sin(x-y)
        if let AstKind::Sum(terms) = &expr.kind {
            if terms.len() == 2 {
                let u = &terms[0];
                let v = &terms[1];

                let (c1, r1) = extract_coefficient(u);
                let (c2, r2) = extract_coefficient(v);

                // Helper to check if expr is exactly sin(a)*cos(b)
                // Returns Some((a, b))
                let parse_sin_cos = |e: &Expr| -> Option<(Expr, Expr)> {
                    // Helper to get name/arg from FunctionCall OR Pow(FunctionCall, 1)
                    let get_op = |t: &Expr| -> Option<(String, Expr)> {
                        if let AstKind::FunctionCall { name, args } = &t.kind {
                            if args.len() == 1 {
                                return Some((name.to_string(), args[0].clone()));
                            }
                        }
                        helpers::get_fn_pow_named(t, 1.0).map(|(n, e)| (n.to_string(), e))
                    };

                    if let AstKind::Product(factors) = &e.kind {
                        if factors.len() == 2 {
                            let f1 = &factors[0];
                            let f2 = &factors[1];
                            // Try matches
                            if let (Some((n1, a)), Some((n2, b))) = (get_op(f1), get_op(f2)) {
                                if n1 == "sin" && n2 == "cos" {
                                    return Some((a, b));
                                }
                                if n1 == "cos" && n2 == "sin" {
                                    return Some((b, a));
                                }
                            }
                        }
                    }
                    None
                };

                // Pattern 1: Match sin(x)cos(y) + cos(x)sin(y)
                // We need to identify x and y consistently
                if let (Some((x1, y1)), Some((x2, y2))) = (parse_sin_cos(&r1), parse_sin_cos(&r2)) {
                    // Check logic for sin(x+y): needs sin(x)cos(y) + sin(y)cos(x)
                    // r1 has (x1, y1) -> sin(x1)cos(y1)
                    // r2 has (x2, y2) -> sin(x2)cos(y2)
                    // We need x2=y1 and y2=x1

                    if x1 == y2 && y1 == x2 {
                        // Found terms: sin(x)cos(y) and sin(y)cos(x) (aka cos(x)sin(y))
                        if (c1 - c2).abs() < 1e-10 {
                            // c1 == c2: k * sin(x+y)
                            return Some(Expr::product(vec![
                                Expr::number(c1),
                                Expr::func("sin", Expr::sum(vec![x1, y1])),
                            ]));
                        } else if (c1 + c2).abs() < 1e-10 {
                            // c1 == -c2: k * sin(x-y)
                            // Term 1 is k*sin(x)cos(y). Term 2 is -k*cos(x)sin(y).
                            // Result is k*sin(x-y)
                            return Some(Expr::product(vec![
                                Expr::number(c1),
                                Expr::func(
                                    "sin",
                                    Expr::sum(vec![
                                        x1,
                                        Expr::product(vec![Expr::number(-1.0), y1]),
                                    ]),
                                ),
                            ]));
                        }
                    }
                }
            }
        }
        None
    }
);

fn is_cos_minus_sin(expr: &Expr) -> bool {
    // In n-ary, cos(x) - sin(x) = Sum([cos(x), Product([-1, sin(x)])])
    if let AstKind::Sum(terms) = &expr.kind {
        if terms.len() == 2 {
            let a = &terms[0];
            let b = &terms[1];

            if is_cos(a) {
                if let AstKind::Product(factors) = &b.kind {
                    if factors.len() == 2 {
                        if let AstKind::Number(n) = &factors[0].kind {
                            if (*n + 1.0).abs() < 1e-10 {
                                return is_sin(&factors[1]);
                            }
                        }
                    }
                }
            }
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
    matches!(&expr.kind, AstKind::FunctionCall { name, args } if name == "cos" && args.len() == 1)
}

fn is_sin(expr: &Expr) -> bool {
    matches!(&expr.kind, AstKind::FunctionCall { name, args } if name == "sin" && args.len() == 1)
}

fn get_cos_arg(expr: &Expr) -> Option<Expr> {
    if let AstKind::FunctionCall { name, args } = &expr.kind {
        if name == "cos" && args.len() == 1 {
            Some(args[0].clone())
        } else {
            None
        }
    } else if let AstKind::Sum(terms) = &expr.kind {
        if !terms.is_empty() {
            get_cos_arg(&terms[0])
        } else {
            None
        }
    } else {
        None
    }
}

fn get_sin_arg(expr: &Expr) -> Option<Expr> {
    if let AstKind::FunctionCall { name, args } = &expr.kind {
        if name == "sin" && args.len() == 1 {
            Some(args[0].clone())
        } else {
            None
        }
    } else if let AstKind::Sum(terms) = &expr.kind {
        if terms.len() >= 2 {
            // Look in second term (which may be -sin(x) = Product([-1, sin(x)]))
            let second = &terms[1];
            if let AstKind::Product(factors) = &second.kind {
                if factors.len() == 2 {
                    if let AstKind::Number(n) = &factors[0].kind {
                        if (*n + 1.0).abs() < 1e-10 {
                            return get_sin_arg(&factors[1]);
                        }
                    }
                }
            }
            get_sin_arg(second)
        } else {
            None
        }
    } else {
        None
    }
}

rule!(
    TrigProductToDoubleAngleRule,
    "trig_product_to_double_angle",
    90,
    Trigonometric,
    &[ExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Product(factors) = &expr.kind {
            if factors.len() == 2 {
                let a = &factors[0];
                let b = &factors[1];

                let (cos_minus_sin, cos_plus_sin) = if is_cos_minus_sin(a) && is_cos_plus_sin(b) {
                    (a, b)
                } else if is_cos_minus_sin(b) && is_cos_plus_sin(a) {
                    (b, a)
                } else {
                    return None;
                };

                if let Some(arg) = get_cos_arg(cos_minus_sin)
                    && get_cos_arg(cos_plus_sin) == Some(arg.clone())
                    && get_sin_arg(cos_minus_sin) == Some(arg.clone())
                    && get_sin_arg(cos_plus_sin) == Some(arg.clone())
                {
                    // (cos(x) - sin(x))(cos(x) + sin(x)) = cos(2x)
                    return Some(Expr::func(
                        "cos",
                        Expr::product(vec![Expr::number(2.0), arg]),
                    ));
                }
            }
        }
        None
    }
);
