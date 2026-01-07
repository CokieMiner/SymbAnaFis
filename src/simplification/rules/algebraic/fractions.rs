use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use crate::{Expr, ExprKind as AstKind};
use std::sync::Arc;

rule_arc!(DivSelfRule, "div_self", 78, Algebraic, &[ExprKind::Div], alters_domain: true, |expr: &Expr, _context: &RuleContext| {
    if let AstKind::Div(u, v) = &expr.kind
        && u == v
    {
        return Some(Arc::new(Expr::number(1.0)));
    }
    None
});

rule_arc!(
    DivDivRule,
    "div_div_flatten",
    92,
    Algebraic,
    &[ExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(num, den) = &expr.kind {
            // Case 1: (a/b)/(c/d) -> (a*d)/(b*c)
            if let (AstKind::Div(a, b), AstKind::Div(c, d)) = (&num.kind, &den.kind) {
                return Some(Arc::new(Expr::div_from_arcs(
                    Arc::new(Expr::product_from_arcs(vec![a.clone(), d.clone()])),
                    Arc::new(Expr::product_from_arcs(vec![b.clone(), c.clone()])),
                )));
            }
            // Case 2: x/(c/d) -> (x*d)/c
            if let AstKind::Div(c, d) = &den.kind {
                return Some(Arc::new(Expr::div_from_arcs(
                    Arc::new(Expr::product_from_arcs(vec![num.clone(), d.clone()])),
                    c.clone(),
                )));
            }
            // Case 3: (a/b)/y -> a/(b*y)
            if let AstKind::Div(a, b) = &num.kind {
                return Some(Arc::new(Expr::div_from_arcs(
                    a.clone(),
                    Arc::new(Expr::product_from_arcs(vec![b.clone(), den.clone()])),
                )));
            }
        }
        None
    }
);

rule_arc!(
    CombineNestedFractionRule,
    "combine_nested_fraction",
    91,
    Algebraic,
    &[ExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(num, outer_den) = &expr.kind {
            // Check if num is a Sum containing a Div
            if let AstKind::Sum(terms) = &num.kind
                && terms.len() == 2
            {
                // Look for pattern: Sum([a, Div(b, c)]) / d -> Sum([a*c, b]) / (c*d)
                // Note: terms contains Arcs.
                for (i, term) in terms.iter().enumerate() {
                    if let AstKind::Div(b, c) = &term.kind {
                        let other_term = if i == 0 { &terms[1] } else { &terms[0] };
                        // (a*c + b) / (c*d)
                        let new_num = Arc::new(Expr::sum_from_arcs(vec![
                            Arc::new(Expr::product_from_arcs(vec![other_term.clone(), c.clone()])),
                            b.clone(),
                        ]));
                        let new_den =
                            Arc::new(Expr::product_from_arcs(vec![c.clone(), outer_den.clone()]));
                        return Some(Arc::new(Expr::div_from_arcs(new_num, new_den)));
                    }
                }
            }
        }
        None
    }
);

rule_arc!(
    AddFractionRule,
    "add_fraction",
    45,
    Algebraic,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Sum(terms) = &expr.kind
            && terms.len() == 2
        {
            let u = &terms[0];
            let v = &terms[1];

            // Case 1: a/b + c/d
            if let (AstKind::Div(n1, d1), AstKind::Div(n2, d2)) = (&u.kind, &v.kind) {
                // Check for common denominator
                if d1 == d2 {
                    return Some(Arc::new(Expr::div_from_arcs(
                        Arc::new(Expr::sum_from_arcs(vec![n1.clone(), n2.clone()])),
                        d1.clone(),
                    )));
                }
                // (n1*d2 + n2*d1) / (d1*d2)
                let new_num = Arc::new(Expr::sum_from_arcs(vec![
                    Arc::new(Expr::product_from_arcs(vec![n1.clone(), d2.clone()])),
                    Arc::new(Expr::product_from_arcs(vec![n2.clone(), d1.clone()])),
                ]));
                let new_den = Arc::new(Expr::product_from_arcs(vec![d1.clone(), d2.clone()]));
                return Some(Arc::new(Expr::div_from_arcs(new_num, new_den)));
            }

            // Case 2: a + b/c (v is the fraction)
            if let AstKind::Div(n, d) = &v.kind {
                let u_times_d = if matches!(&u.kind, AstKind::Number(x) if (*x - 1.0).abs() < 1e-10)
                {
                    d.clone()
                } else {
                    Arc::new(Expr::product_from_arcs(vec![u.clone(), d.clone()]))
                };
                let new_num = Arc::new(Expr::sum_from_arcs(vec![u_times_d, n.clone()]));
                return Some(Arc::new(Expr::div_from_arcs(new_num, d.clone())));
            }

            // Case 3: a/b + c (u is the fraction)
            if let AstKind::Div(n, d) = &u.kind {
                let v_times_d = if matches!(&v.kind, AstKind::Number(x) if (*x - 1.0).abs() < 1e-10)
                {
                    d.clone()
                } else {
                    Arc::new(Expr::product_from_arcs(vec![v.clone(), d.clone()]))
                };
                let new_num = Arc::new(Expr::sum_from_arcs(vec![n.clone(), v_times_d]));
                return Some(Arc::new(Expr::div_from_arcs(new_num, d.clone())));
            }
        }
        None
    }
);

rule_with_helpers_arc!(FractionToEndRule, "fraction_to_end", 50, Algebraic, &[ExprKind::Div, ExprKind::Product],
    helpers: {
        // Helper to check if expression contains any Div inside Product
        fn product_contains_div(e: &Expr) -> bool {
            match &e.kind {
                AstKind::Div(_, _) => true,
                AstKind::Product(factors) => factors.iter().any(|f| product_contains_div(f)),
                _ => false,
            }
        }

        // Helper to extract all factors from a multiplication, separating numerators and denominators
        fn extract_factors(e: &Expr, numerators: &mut Vec<Arc<Expr>>, denominators: &mut Vec<Arc<Expr>>) {
            match &e.kind {
                AstKind::Product(factors) => {
                    for f in factors {
                        extract_factors(f, numerators, denominators);
                    }
                }
                AstKind::Div(num, den) => {
                    extract_factors(num, numerators, denominators);
                    denominators.push(den.clone());
                }
                _ => {
                    numerators.push(Arc::new(e.clone()));
                }
            }
        }
    },
    |expr: &Expr, _context: &RuleContext| {
        // Case 1: Div where numerator is a Product containing Divs
        if let AstKind::Div(num, den) = &expr.kind
            && product_contains_div(num) {
                let mut numerators = Vec::new();
                let mut denominators = Vec::new();
                extract_factors(num, &mut numerators, &mut denominators);

                // Add the outer denominator
                denominators.push(den.clone());

                // Filter out 1s from numerators
                let filtered_nums: Vec<Arc<Expr>> = numerators
                    .into_iter()
                    .filter(|e| !matches!(e.kind, AstKind::Number(n) if (n - 1.0).abs() < 1e-10))
                    .collect();

                let num_expr = if filtered_nums.is_empty() {
                    Arc::new(Expr::number(1.0))
                } else if filtered_nums.len() == 1 {
                    filtered_nums.into_iter().next().unwrap()
                } else {
                    Arc::new(Expr::product_from_arcs(filtered_nums))
                };

                let den_expr = if denominators.len() == 1 {
                    denominators.into_iter().next().unwrap()
                } else {
                    Arc::new(Expr::product_from_arcs(denominators))
                };

                let result = Arc::new(Expr::div_from_arcs(num_expr, den_expr));

                if result.as_ref() != expr {
                    return Some(result);
                }
            }

        // Case 2: Product containing at least one Div
        if let AstKind::Product(_) = &expr.kind {
            if !product_contains_div(expr) {
                return None;
            }

            let mut numerators = Vec::new();
            let mut denominators = Vec::new();
            extract_factors(expr, &mut numerators, &mut denominators);

            if denominators.is_empty() {
                return None;
            }

            // Filter out 1s from numerators
            let filtered_nums: Vec<Arc<Expr>> = numerators
                .into_iter()
                .filter(|e| !matches!(e.kind, AstKind::Number(n) if (n - 1.0).abs() < 1e-10))
                .collect();

            let num_expr = if filtered_nums.is_empty() {
                Arc::new(Expr::number(1.0))
            } else if filtered_nums.len() == 1 {
                filtered_nums.into_iter().next().unwrap()
            } else {
                Arc::new(Expr::product_from_arcs(filtered_nums))
            };

            let den_expr = if denominators.len() == 1 {
                denominators.into_iter().next().unwrap()
            } else {
                Arc::new(Expr::product_from_arcs(denominators))
            };

            let result = Arc::new(Expr::div_from_arcs(num_expr, den_expr));

            if result.as_ref() != expr {
                return Some(result);
            }
        }

        None
    }
);
