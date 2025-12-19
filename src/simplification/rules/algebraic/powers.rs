use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use crate::{Expr, ExprKind as AstKind};
use std::sync::Arc;

rule!(
    PowerZeroRule,
    "power_zero",
    80,
    Algebraic,
    &[ExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Pow(_u, v) = &expr.kind
            && matches!(v.kind, AstKind::Number(n) if n == 0.0)
        {
            return Some(Expr::number(1.0));
        }
        None
    }
);

rule!(
    PowerOneRule,
    "power_one",
    80,
    Algebraic,
    &[ExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Pow(u, v) = &expr.kind
            && matches!(v.kind, AstKind::Number(n) if n == 1.0)
        {
            return Some((**u).clone());
        }
        None
    }
);

rule!(
    PowerPowerRule,
    "power_power",
    75,
    Algebraic,
    &[ExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Pow(u, v) = &expr.kind
            && let AstKind::Pow(base, exp_inner) = &u.kind
        {
            // Check for special case: (x^even)^(1/even) where result would be x^1
            if let AstKind::Number(inner_n) = &exp_inner.kind {
                let inner_is_even =
                    *inner_n > 0.0 && inner_n.fract() == 0.0 && (*inner_n as i64) % 2 == 0;

                if inner_is_even {
                    if let AstKind::Div(num, den) = &v.kind
                        && let (AstKind::Number(num_val), AstKind::Number(den_val)) =
                            (&num.kind, &den.kind)
                        && *num_val == 1.0
                        && (*den_val - *inner_n).abs() < 1e-10
                    {
                        return Some(Expr::func_multi("abs", vec![(**base).clone()]));
                    }
                    if let AstKind::Number(outer_n) = &v.kind {
                        let product = inner_n * outer_n;
                        if (product - 1.0).abs() < 1e-10 {
                            return Some(Expr::func_multi(
                                "abs",
                                vec![(**base).clone()],
                            ));
                        }
                    }
                }
            }

            // Create new exponent: exp_inner * v using Product
            let new_exp = Expr::product(vec![(**exp_inner).clone(), (**v).clone()]);

            return Some(Expr::pow((**base).clone(), new_exp));
        }
        None
    }
);

rule!(
    PowerProductRule,
    "power_product",
    75,
    Algebraic,
    &[ExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Product(factors) = &expr.kind {
            // Look for pairs of powers with the same base
            for (i, f1) in factors.iter().enumerate() {
                for (j, f2) in factors.iter().enumerate() {
                    if i >= j {
                        continue;
                    }

                    // Helper to build result
                    let build_result = |combined: Expr| -> Option<Expr> {
                        let mut new_factors: Vec<Arc<Expr>> = factors
                            .iter()
                            .enumerate()
                            .filter(|(k, _)| *k != i && *k != j)
                            .map(|(_, f)| f.clone())
                            .collect();
                        new_factors.push(Arc::new(combined));

                        if new_factors.len() == 1 {
                            let arc = new_factors.into_iter().next().unwrap();
                            Some(match Arc::try_unwrap(arc) {
                                Ok(e) => e,
                                Err(a) => (*a).clone(),
                            })
                        } else {
                            Some(Expr::product_from_arcs(new_factors))
                        }
                    };

                    // Both are powers with the same base
                    if let (AstKind::Pow(base_1, exp_1), AstKind::Pow(base_2, exp_2)) =
                        (&f1.kind, &f2.kind)
                        && base_1 == base_2 {
                            // Combine: x^a * x^b = x^(a+b)
                            let new_exp = Expr::sum(vec![(**exp_1).clone(), (**exp_2).clone()]);
                            let combined = Expr::pow((**base_1).clone(), new_exp);
                            return build_result(combined);
                        }

                    // One is a power and the other is the same base
                    if let AstKind::Pow(base_1, exp_1) = &f1.kind
                        && **base_1 == **f2 {
                            let new_exp = Expr::sum(vec![(**exp_1).clone(), Expr::number(1.0)]);
                            let combined = Expr::pow((**base_1).clone(), new_exp);
                            return build_result(combined);
                        }

                    if let AstKind::Pow(base_2, exp_2) = &f2.kind
                        && **base_2 == **f1 {
                            let new_exp = Expr::sum(vec![Expr::number(1.0), (**exp_2).clone()]);
                            let combined = Expr::pow((**base_2).clone(), new_exp);
                            return build_result(combined);
                        }
                }
            }
        }
        None
    }
);

rule!(
    PowerDivRule,
    "power_div",
    75,
    Algebraic,
    &[ExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(u, v) = &expr.kind {
            // Check if both numerator and denominator are powers with the same base
            if let (AstKind::Pow(base_u, exp_u), AstKind::Pow(base_v, exp_v)) = (&u.kind, &v.kind)
                && base_u == base_v
            {
                // x^a / x^b = x^(a-b) = x^(a + (-1)*b)
                let neg_exp_v = Expr::product(vec![Expr::number(-1.0), (**exp_v).clone()]);
                let new_exp = Expr::sum(vec![(**exp_u).clone(), neg_exp_v]);
                return Some(Expr::pow((**base_u).clone(), new_exp));
            }
            // Check if numerator is a power and denominator is the same base
            if let AstKind::Pow(base_u, exp_u) = &u.kind
                && base_u == v
            {
                let new_exp = Expr::sum(vec![(**exp_u).clone(), Expr::number(-1.0)]);
                return Some(Expr::pow((**base_u).clone(), new_exp));
            }
            // Check if denominator is a power and numerator is the same base
            if let AstKind::Pow(base_v, exp_v) = &v.kind
                && base_v == u
            {
                let neg_exp = Expr::product(vec![Expr::number(-1.0), (**exp_v).clone()]);
                let new_exp = Expr::sum(vec![Expr::number(1.0), neg_exp]);
                return Some(Expr::pow((**base_v).clone(), new_exp));
            }
        }
        None
    }
);

rule!(
    PowerCollectionRule,
    "power_collection",
    60,
    Algebraic,
    &[ExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Product(factors) = &expr.kind {
            // Group by base
            use std::collections::HashMap;
            let mut base_to_exponents: HashMap<Expr, Vec<Expr>> = HashMap::new();

            for factor in factors.iter() {
                if let AstKind::Pow(base, exp) = &factor.kind {
                    base_to_exponents
                        .entry((**base).clone())
                        .or_default()
                        .push((**exp).clone());
                } else {
                    // Non-power factor, treat as base^1
                    base_to_exponents
                        .entry((**factor).clone())
                        .or_default()
                        .push(Expr::number(1.0));
                }
            }

            // Check if any base has multiple occurrences
            let has_combination = base_to_exponents.values().any(|v| v.len() > 1);
            if !has_combination {
                return None;
            }

            // Combine exponents for each base
            let mut result_factors = Vec::new();
            for (base, exponents) in base_to_exponents {
                if exponents.len() == 1 {
                    if exponents[0] == Expr::number(1.0) {
                        result_factors.push(base);
                    } else {
                        result_factors.push(Expr::pow(base, exponents[0].clone()));
                    }
                } else {
                    // Sum all exponents using n-ary Sum
                    let sum = Expr::sum(exponents);
                    result_factors.push(Expr::pow(base, sum));
                }
            }

            // Rebuild the expression
            if result_factors.len() == 1 {
                Some(result_factors.into_iter().next().unwrap())
            } else {
                Some(Expr::product(result_factors))
            }
        } else {
            None
        }
    }
);

rule!(
    CommonExponentDivRule,
    "common_exponent_div",
    55,
    Algebraic,
    &[ExprKind::Div],
    |expr: &Expr, context: &RuleContext| {
        if let AstKind::Div(num, den) = &expr.kind
            && let (AstKind::Pow(base_num, exp_num), AstKind::Pow(base_den, exp_den)) =
                (&num.kind, &den.kind)
            && exp_num == exp_den
        {
            if context.domain_safe
                && crate::simplification::helpers::is_fractional_root_exponent(exp_num)
            {
                let num_non_neg = crate::simplification::helpers::is_known_non_negative(base_num);
                let den_non_neg = crate::simplification::helpers::is_known_non_negative(base_den);
                if !(num_non_neg && den_non_neg) {
                    return None;
                }
            }

            return Some(Expr::pow(
                Expr::div_expr((**base_num).clone(), (**base_den).clone()),
                (**exp_num).clone(),
            ));
        }
        None
    }
);

rule!(
    CommonExponentProductRule,
    "common_exponent_product",
    55,
    Algebraic,
    &[ExprKind::Product],
    |expr: &Expr, context: &RuleContext| {
        if let AstKind::Product(factors) = &expr.kind {
            // Look for pairs with same exponent
            for (i, f1) in factors.iter().enumerate() {
                for (j, f2) in factors.iter().enumerate() {
                    if i >= j {
                        continue;
                    }

                    if let (AstKind::Pow(base_1, exp_1), AstKind::Pow(base_2, exp_2)) =
                        (&f1.kind, &f2.kind)
                        && exp_1 == exp_2 {
                            // Skip if either base^exp would result in an integer - we prefer expanded numeric coefficients
                            // (e.g., 9*y^2 not (3y)^2, but allow sqrt(2)*sqrt(pi) → sqrt(2pi))
                            let would_simplify_to_int = |base: &Expr, exp: &Expr| -> bool {
                                if let (AstKind::Number(base_val), AstKind::Number(exp_val)) =
                                    (&base.kind, &exp.kind)
                                {
                                    let result = base_val.powf(*exp_val);
                                    result.is_finite() && (result - result.round()).abs() < 1e-10
                                } else {
                                    false
                                }
                            };

                            if would_simplify_to_int(base_1, exp_1)
                                || would_simplify_to_int(base_2, exp_2)
                            {
                                continue;
                            }

                            if context.domain_safe
                                && crate::simplification::helpers::is_fractional_root_exponent(
                                    exp_1,
                                )
                            {
                                let left_non_neg =
                                    crate::simplification::helpers::is_known_non_negative(base_1);
                                let right_non_neg =
                                    crate::simplification::helpers::is_known_non_negative(base_2);
                                if !(left_non_neg && right_non_neg) {
                                    continue;
                                }
                            }

                            // Combine: x^a * y^a = (x*y)^a
                            let combined_base =
                                Expr::product(vec![(**base_1).clone(), (**base_2).clone()]);
                            let combined = Expr::pow(combined_base, (**exp_1).clone());

                            let mut new_factors: Vec<Expr> = factors
                                .iter()
                                .enumerate()
                                .filter(|(k, _)| *k != i && *k != j)
                                .map(|(_, f)| (**f).clone())
                                .collect();
                            new_factors.push(combined);

                            if new_factors.len() == 1 {
                                return Some(new_factors.into_iter().next().unwrap());
                            } else {
                                return Some(Expr::product(new_factors));
                            }
                        }
                }
            }
        }
        None
    }
);

rule!(
    NegativeExponentToFractionRule,
    "negative_exponent_to_fraction",
    90,
    Algebraic,
    &[ExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Pow(base, exp) = &expr.kind {
            // Handle negative number exponent: x^-n -> 1/x^n
            if let AstKind::Number(n) = exp.kind
                && n < 0.0
            {
                let positive_exp = Expr::number(-n);
                let denominator = Expr::pow((**base).clone(), positive_exp);
                return Some(Expr::div_expr(Expr::number(1.0), denominator));
            }
            // Handle negative fraction exponent: x^(-a/b) -> 1/x^(a/b)
            if let AstKind::Div(num, den) = &exp.kind
                && let AstKind::Number(n) = num.kind
                && n < 0.0
            {
                let positive_num = Expr::number(-n);
                let positive_exp = Expr::div_expr(positive_num, (**den).clone());
                let denominator = Expr::pow((**base).clone(), positive_exp);
                return Some(Expr::div_expr(Expr::number(1.0), denominator));
            }
            // Handle Product([-1, exp]): x^(-1 * a) -> 1/x^a
            if let AstKind::Product(factors) = &exp.kind
                && factors.len() == 2
                    && let AstKind::Number(n) = &factors[0].kind
                        && *n == -1.0 {
                            let denominator = Expr::pow((**base).clone(), (*factors[1]).clone());
                            return Some(Expr::div_expr(Expr::number(1.0), denominator));
                        }
        }
        None
    }
);

rule!(
    PowerOfQuotientRule,
    "power_of_quotient",
    88,
    Algebraic,
    &[ExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Pow(base, exp) = &expr.kind
            && let AstKind::Div(num, den) = &base.kind
        {
            let is_root_exponent = match &exp.kind {
                AstKind::Div(n, d) => {
                    matches!((&n.kind, &d.kind), (AstKind::Number(num_val), AstKind::Number(den_val))
                    if *num_val == 1.0 && *den_val >= 2.0)
                }
                AstKind::Number(n) => *n > 0.0 && *n < 1.0,
                _ => false,
            };

            let den_would_simplify = match &den.kind {
                AstKind::Pow(_, inner_exp) => {
                    if let (AstKind::Number(m), AstKind::Div(one, n_rc)) =
                        (&inner_exp.kind, &exp.kind)
                    {
                        if let (AstKind::Number(one_val), AstKind::Number(n_val)) =
                            (&one.kind, &n_rc.kind)
                        {
                            *one_val == 1.0 && (m / n_val).fract().abs() < 1e-10
                        } else {
                            false
                        }
                    } else if let (AstKind::Number(m), AstKind::Number(exp_val)) =
                        (&inner_exp.kind, &exp.kind)
                    {
                        (m * exp_val).fract().abs() < 1e-10
                    } else {
                        false
                    }
                }
                AstKind::Symbol(_) => is_root_exponent,
                AstKind::Number(_) => is_root_exponent,
                _ => false,
            };

            if is_root_exponent || den_would_simplify {
                let num_pow = Expr::pow((**num).clone(), (**exp).clone());
                let den_pow = Expr::pow((**den).clone(), (**exp).clone());
                return Some(Expr::div_expr(num_pow, den_pow));
            }
        }
        None
    }
);
