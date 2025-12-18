use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use crate::{Expr, ExprKind as AstKind};

rule!(
    ExpandPowerForCancellationRule,
    "expand_power_for_cancellation",
    92,
    Algebraic,
    &[ExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(num, den) = &expr.kind {
            // Helper to check if a factor is present in an expression
            let contains_factor = |expr: &Expr, factor: &Expr| -> bool {
                match &expr.kind {
                    AstKind::Product(factors) => factors.iter().any(|f| **f == *factor),
                    _ => expr == factor,
                }
            };

            // Helper to check if expansion is useful
            let check_and_expand = |target: &Expr, other: &Expr| -> Option<Expr> {
                if let AstKind::Pow(base, exp) = &target.kind {
                    if let AstKind::Product(base_factors) = &base.kind {
                        // Check if any base factor is present in 'other'
                        let mut useful = false;
                        for factor in base_factors.iter() {
                            if contains_factor(other, factor) {
                                useful = true;
                                break;
                            }
                        }

                        if useful {
                            let pow_factors: Vec<Expr> = base_factors
                                .iter()
                                .map(|f| Expr::pow((**f).clone(), (**exp).clone()))
                                .collect();
                            return Some(Expr::product(pow_factors));
                        }
                    }
                }
                None
            };

            // Try expanding powers in numerator
            if let Some(expanded) = check_and_expand(num, den) {
                return Some(Expr::div_expr(expanded, (**den).clone()));
            }

            // Try expanding powers in denominator
            if let Some(expanded) = check_and_expand(den, num) {
                return Some(Expr::div_expr((**num).clone(), expanded));
            }
        }
        None
    }
);

rule!(
    PowerExpansionRule,
    "power_expansion",
    86,
    Algebraic,
    &[ExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Pow(base, exp) = &expr.kind {
            // Expand (a*b)^n -> a^n * b^n ONLY if expansion enables simplification
            if let AstKind::Product(base_factors) = &base.kind {
                if let AstKind::Number(n) = &exp.kind {
                    if *n > 1.0 && n.fract() == 0.0 && (*n as i64) < 10 {
                        // Check if expansion would enable simplification
                        let has_simplifiable = base_factors.iter().any(|f| match &f.kind {
                            AstKind::Pow(_, inner_exp) => {
                                if let AstKind::Number(inner_n) = &inner_exp.kind {
                                    (inner_n * n).fract().abs() < 1e-10
                                } else if let AstKind::Div(num, den) = &inner_exp.kind {
                                    if let (AstKind::Number(a), AstKind::Number(b)) =
                                        (&num.kind, &den.kind)
                                    {
                                        ((a * n) / b).fract().abs() < 1e-10
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            }
                            AstKind::FunctionCall { name, .. } => {
                                matches!(name.as_str(), "sqrt" | "cbrt") && *n >= 2.0
                            }
                            AstKind::Number(_) => true,
                            _ => false,
                        });

                        if has_simplifiable {
                            let factors: Vec<Expr> = base_factors
                                .iter()
                                .map(|f| Expr::pow((**f).clone(), (**exp).clone()))
                                .collect();
                            return Some(Expr::product(factors));
                        }
                    }
                }
            }

            // Expand (a/b)^n -> a^n / b^n ONLY if expansion enables simplification
            if let AstKind::Div(a, b) = &base.kind {
                if let AstKind::Number(n) = &exp.kind {
                    if *n > 1.0 && n.fract() == 0.0 && (*n as i64) < 10 {
                        // Helper to check if a term would simplify when raised to power n
                        let would_simplify = |term: &Expr| -> bool {
                            match &term.kind {
                                AstKind::Pow(_, inner_exp) => {
                                    if let AstKind::Number(inner_n) = &inner_exp.kind {
                                        (inner_n * n).fract().abs() < 1e-10
                                    } else if let AstKind::Div(num, den) = &inner_exp.kind {
                                        if let (AstKind::Number(a_val), AstKind::Number(b_val)) =
                                            (&num.kind, &den.kind)
                                        {
                                            ((a_val * n) / b_val).fract().abs() < 1e-10
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                }
                                AstKind::FunctionCall { name, .. } => {
                                    matches!(name.as_str(), "sqrt" | "cbrt") && *n >= 2.0
                                }
                                AstKind::Number(_) => true,
                                AstKind::Product(factors) => {
                                    factors.iter().any(|f| match &f.kind {
                                        AstKind::Number(_) => true,
                                        AstKind::FunctionCall { name, .. } => {
                                            matches!(name.as_str(), "sqrt" | "cbrt")
                                        }
                                        AstKind::Pow(_, inner_exp) => {
                                            if let AstKind::Number(inner_n) = &inner_exp.kind {
                                                (inner_n * n).fract().abs() < 1e-10
                                            } else {
                                                false
                                            }
                                        }
                                        _ => false,
                                    })
                                }
                                _ => false,
                            }
                        };

                        // Only expand if numerator or denominator would simplify
                        if would_simplify(a) || would_simplify(b) {
                            let a_pow = Expr::pow((**a).clone(), (**exp).clone());
                            let b_pow = Expr::pow((**b).clone(), (**exp).clone());
                            return Some(Expr::div_expr(a_pow, b_pow));
                        }
                    }
                }
            }
        }
        None
    }
);

rule!(
    PolynomialExpansionRule,
    "polynomial_expansion",
    89,
    Algebraic,
    &[ExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Pow(base, exp) = &expr.kind {
            // Expand (a + b)^n for small integer n - only on 2-term sums
            if let AstKind::Sum(terms) = &base.kind {
                if terms.len() == 2 {
                    if let AstKind::Number(n) = &exp.kind {
                        if *n >= 2.0 && *n <= 4.0 && n.fract() == 0.0 {
                            let a = &terms[0];
                            let b = &terms[1];

                            // CONSERVATIVE: Only expand if both terms are pure numbers
                            fn is_number(e: &Expr) -> bool {
                                matches!(e.kind, AstKind::Number(_))
                            }

                            if !(is_number(a) && is_number(b)) {
                                return None;
                            }

                            let n_int = *n as i64;
                            match n_int {
                                2 => {
                                    // (a + b)^2 = a^2 + 2*a*b + b^2
                                    let a2 = Expr::pow((**a).clone(), Expr::number(2.0));
                                    let b2 = Expr::pow((**b).clone(), Expr::number(2.0));
                                    let ab2 = Expr::product(vec![
                                        Expr::number(2.0),
                                        (**a).clone(),
                                        (**b).clone(),
                                    ]);
                                    return Some(Expr::sum(vec![a2, ab2, b2]));
                                }
                                3 => {
                                    // (a + b)^3 = a^3 + 3*a^2*b + 3*a*b^2 + b^3
                                    let a3 = Expr::pow((**a).clone(), Expr::number(3.0));
                                    let b3 = Expr::pow((**b).clone(), Expr::number(3.0));
                                    let a2b = Expr::product(vec![
                                        Expr::number(3.0),
                                        Expr::pow((**a).clone(), Expr::number(2.0)),
                                        (**b).clone(),
                                    ]);
                                    let ab2 = Expr::product(vec![
                                        Expr::number(3.0),
                                        (**a).clone(),
                                        Expr::pow((**b).clone(), Expr::number(2.0)),
                                    ]);
                                    return Some(Expr::sum(vec![a3, a2b, ab2, b3]));
                                }
                                4 => {
                                    // (a + b)^4 = a^4 + 4*a^3*b + 6*a^2*b^2 + 4*a*b^3 + b^4
                                    let a4 = Expr::pow((**a).clone(), Expr::number(4.0));
                                    let b4 = Expr::pow((**b).clone(), Expr::number(4.0));
                                    let a3b = Expr::product(vec![
                                        Expr::number(4.0),
                                        Expr::pow((**a).clone(), Expr::number(3.0)),
                                        (**b).clone(),
                                    ]);
                                    let a2b2 = Expr::product(vec![
                                        Expr::number(6.0),
                                        Expr::pow((**a).clone(), Expr::number(2.0)),
                                        Expr::pow((**b).clone(), Expr::number(2.0)),
                                    ]);
                                    let ab3 = Expr::product(vec![
                                        Expr::number(4.0),
                                        (**a).clone(),
                                        Expr::pow((**b).clone(), Expr::number(3.0)),
                                    ]);
                                    return Some(Expr::sum(vec![a4, a3b, a2b2, ab3, b4]));
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        None
    }
);

rule!(
    ExpandDifferenceOfSquaresProductRule,
    "expand_difference_of_squares_product",
    85,
    Algebraic,
    &[ExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Product(factors) = &expr.kind {
            if factors.len() == 2 {
                let a = &factors[0];
                let b = &factors[1];

                // Check for (x + y) * (x - y) pattern where second is Sum with negated second term
                // In n-ary, subtraction is represented as Sum([x, Product([-1, y])])
                let check_difference_of_squares = |left: &Expr, right: &Expr| -> Option<Expr> {
                    // left should be Sum([a1, a2])
                    // right should be Sum([s1, Product([-1, s2])]) where a1=s1 and a2=s2
                    if let (AstKind::Sum(left_terms), AstKind::Sum(right_terms)) =
                        (&left.kind, &right.kind)
                    {
                        if left_terms.len() == 2 && right_terms.len() == 2 {
                            let l1 = &left_terms[0];
                            let l2 = &left_terms[1];
                            let r1 = &right_terms[0];
                            let r2 = &right_terms[1];

                            // Check if r2 is -l2 (i.e., Product([-1, l2]))
                            if **l1 == **r1 {
                                if let AstKind::Product(r2_factors) = &r2.kind {
                                    if r2_factors.len() == 2 {
                                        if let AstKind::Number(n) = &r2_factors[0].kind {
                                            if (*n + 1.0).abs() < 1e-10 && *r2_factors[1] == **l2 {
                                                // (a + b)(a - b) = a^2 - b^2
                                                let a_squared =
                                                    Expr::pow((**l1).clone(), Expr::number(2.0));
                                                let b_squared =
                                                    Expr::pow((**l2).clone(), Expr::number(2.0));
                                                // a^2 - b^2 = Sum([a^2, Product([-1, b^2])])
                                                return Some(Expr::sum(vec![
                                                    a_squared,
                                                    Expr::product(vec![
                                                        Expr::number(-1.0),
                                                        b_squared,
                                                    ]),
                                                ]));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    None
                };

                if let Some(result) = check_difference_of_squares(a, b) {
                    return Some(result);
                }
                if let Some(result) = check_difference_of_squares(b, a) {
                    return Some(result);
                }
            }
        }
        None
    }
);
