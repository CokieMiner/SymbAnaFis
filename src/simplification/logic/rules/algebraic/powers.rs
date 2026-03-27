use super::{
    Rule, RuleCategory, RuleContext, RuleExprKind, is_fractional_root_exponent,
    is_known_non_negative,
};
use crate::EPSILON;
use crate::core::arc_number;
use crate::core::known_symbols::{KS, get_symbol};
use crate::core::{Expr, ExprKind};
use std::sync::Arc;

rule!(
    PowerZeroRule,
    "power_zero",
    80,
    Algebraic,
    &[RuleExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::Pow(_u, v) = &expr.kind
            && matches!(v.kind, ExprKind::Number(n) if n == 0.0)
        {
            return Some(Expr::number(1.0));
        }
        None
    }
);

rule_arc!(
    PowerOneRule,
    "power_one",
    80,
    Algebraic,
    &[RuleExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::Pow(u, v) = &expr.kind {
            // Exact check for exponent == 1.0
            #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
            let is_one = matches!(v.kind, ExprKind::Number(n) if n == 1.0);
            if is_one {
                return Some(Arc::clone(u));
            }
        }
        None
    }
);

rule!(
    PowerPowerRule,
    "power_power",
    75,
    Algebraic,
    &[RuleExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::Pow(u, v) = &expr.kind
            && let ExprKind::Pow(base, exp_inner) = &u.kind
        {
            // Check for special case: (x^even)^(1/even) where result would be x^1
            if let ExprKind::Number(inner_n) = &exp_inner.kind {
                // Exact comparison for integer check, cast for modulo
                #[allow(
                    clippy::float_cmp,
                    clippy::cast_possible_truncation,
                    reason = "Exact check for integer, cast for modulo"
                )]
                // Exact check for integer, cast for modulo
                let inner_is_even =
                    *inner_n > 0.0 && inner_n.fract() == 0.0 && (*inner_n as i64) % 2 == 0;

                if inner_is_even {
                    if let ExprKind::Div(num, den) = &v.kind
                        && let (ExprKind::Number(num_val), ExprKind::Number(den_val)) =
                            (&num.kind, &den.kind)
                        && {
                            // Exact check for 1.0 numerator in exponent fraction
                            #[allow(
                                clippy::float_cmp,
                                reason = "Comparing against exact constant 1.0"
                            )]
                            let is_one = *num_val == 1.0;
                            is_one
                        }
                        && (*den_val - *inner_n).abs() < EPSILON
                    {
                        return Some(Expr::func_multi_from_arcs_symbol(
                            get_symbol(KS.abs),
                            vec![Arc::clone(base)],
                        ));
                    }
                    if let ExprKind::Number(outer_n) = &v.kind {
                        let product = inner_n * outer_n;
                        if (product - 1.0).abs() < EPSILON {
                            return Some(Expr::func_multi_from_arcs_symbol(
                                get_symbol(KS.abs),
                                vec![Arc::clone(base)],
                            ));
                        }
                    }
                }
            }

            // Create new exponent: exp_inner * v using Product
            let new_exp = Expr::product_from_arcs(vec![Arc::clone(exp_inner), Arc::clone(v)]);

            return Some(Expr::pow_from_arcs(Arc::clone(base), Arc::new(new_exp)));
        }
        None
    }
);

rule_arc!(
    PowerProductRule,
    "power_product",
    75,
    Algebraic,
    &[RuleExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::Product(factors) = &expr.kind {
            // Look for pairs of powers with the same base
            for (i, f1) in factors.iter().enumerate() {
                for (j, f2) in factors.iter().enumerate() {
                    if i >= j {
                        continue;
                    }

                    // Helper to build result
                    let build_result = |combined: Expr| -> Option<Arc<Expr>> {
                        let mut new_factors: Vec<Arc<Expr>> = factors
                            .iter()
                            .enumerate()
                            .filter(|(k, _)| *k != i && *k != j)
                            .map(|(_, f)| Arc::clone(f))
                            .collect();
                        new_factors.push(Arc::new(combined));

                        if new_factors.len() == 1 {
                            let arc = new_factors
                                .into_iter()
                                .next()
                                .expect("New factors guaranteed to have one element");
                            Some(arc)
                        } else {
                            Some(Arc::new(Expr::product_from_arcs(new_factors)))
                        }
                    };

                    // Both are powers with the same base
                    if let (ExprKind::Pow(base_1, exp_1), ExprKind::Pow(base_2, exp_2)) =
                        (&f1.kind, &f2.kind)
                        && base_1 == base_2
                    {
                        // Combine: x^a * x^b = x^(a+b)
                        let new_exp =
                            Expr::sum_from_arcs(vec![Arc::clone(exp_1), Arc::clone(exp_2)]);
                        let combined = Expr::pow_from_arcs(Arc::clone(base_1), Arc::new(new_exp));
                        return build_result(combined);
                    }

                    // One is a power and the other is the same base
                    if let ExprKind::Pow(base_1, exp_1) = &f1.kind
                        && **base_1 == **f2
                    {
                        let new_exp = Expr::sum_from_arcs(vec![Arc::clone(exp_1), arc_number(1.0)]);
                        let combined = Expr::pow_from_arcs(Arc::clone(base_1), Arc::new(new_exp));
                        return build_result(combined);
                    }

                    if let ExprKind::Pow(base_2, exp_2) = &f2.kind
                        && **base_2 == **f1
                    {
                        let new_exp = Expr::sum_from_arcs(vec![arc_number(1.0), Arc::clone(exp_2)]);
                        let combined = Expr::pow_from_arcs(Arc::clone(base_2), Arc::new(new_exp));
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
    &[RuleExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::Div(u, v) = &expr.kind {
            // Check if both numerator and denominator are powers with the same base
            if let (ExprKind::Pow(base_u, exp_u), ExprKind::Pow(base_v, exp_v)) = (&u.kind, &v.kind)
                && base_u == base_v
            {
                // x^a / x^b = x^(a-b) = x^(a + (-1)*b)
                let neg_exp_v = Expr::product_from_arcs(vec![arc_number(-1.0), Arc::clone(exp_v)]);
                let new_exp = Expr::sum_from_arcs(vec![Arc::clone(exp_u), Arc::new(neg_exp_v)]);
                return Some(Expr::pow_from_arcs(Arc::clone(base_u), Arc::new(new_exp)));
            }
            // Check if numerator is a power and denominator is the same base
            if let ExprKind::Pow(base_u, exp_u) = &u.kind
                && base_u == v
            {
                let new_exp = Expr::sum_from_arcs(vec![Arc::clone(exp_u), arc_number(-1.0)]);
                return Some(Expr::pow_from_arcs(Arc::clone(base_u), Arc::new(new_exp)));
            }
            // Check if denominator is a power and numerator is the same base
            if let ExprKind::Pow(base_v, exp_v) = &v.kind
                && base_v == u
            {
                let neg_exp = Expr::product_from_arcs(vec![arc_number(-1.0), Arc::clone(exp_v)]);
                let combined_exp = Expr::sum_from_arcs(vec![arc_number(1.0), Arc::new(neg_exp)]);
                return Some(Expr::pow_from_arcs(
                    Arc::clone(base_v),
                    Arc::new(combined_exp),
                ));
            }
        }
        None
    }
);

rule_arc!(
    PowerCollectionRule,
    "power_collection",
    60,
    Algebraic,
    &[RuleExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::Product(factors) = &expr.kind {
            // Group by base
            use rustc_hash::FxHashMap;
            let mut base_to_exponents: FxHashMap<Arc<Expr>, Vec<Arc<Expr>>> = FxHashMap::default();

            for factor in factors {
                if let ExprKind::Pow(base, exp) = &factor.kind {
                    base_to_exponents
                        .entry(Arc::clone(base))
                        .or_default()
                        .push(Arc::clone(exp));
                } else {
                    // Non-power factor, treat as base^1
                    base_to_exponents
                        .entry(Arc::clone(factor))
                        .or_default()
                        .push(arc_number(1.0));
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
                    // Exact check for exponent == 1.0
                    #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
                    let is_one_exp = matches!(exponents[0].kind, ExprKind::Number(n) if n == 1.0);
                    if is_one_exp {
                        result_factors.push(base);
                    } else {
                        result_factors.push(Arc::new(Expr::pow_from_arcs(
                            base,
                            Arc::clone(&exponents[0]),
                        )));
                    }
                } else {
                    // Sum all exponents using n-ary Sum
                    let sum = Expr::sum_from_arcs(exponents);
                    result_factors.push(Arc::new(Expr::pow_from_arcs(base, Arc::new(sum))));
                }
            }

            // Rebuild the expression
            if result_factors.len() == 1 {
                Some(
                    result_factors
                        .into_iter()
                        .next()
                        .expect("Result factors guaranteed to have one element"),
                )
            } else {
                Some(Arc::new(Expr::product_from_arcs(result_factors)))
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
    &[RuleExprKind::Div],
    |expr: &Expr, context: &RuleContext| {
        if let ExprKind::Div(num, den) = &expr.kind
            && let (ExprKind::Pow(base_num, exp_num), ExprKind::Pow(base_den, exp_den)) =
                (&num.kind, &den.kind)
            && exp_num == exp_den
        {
            if context.domain_safe && is_fractional_root_exponent(exp_num) {
                let num_non_neg = is_known_non_negative(base_num);
                let den_non_neg = is_known_non_negative(base_den);
                if !(num_non_neg && den_non_neg) {
                    return None;
                }
            }

            let inner = Expr::div_from_arcs(Arc::clone(base_num), Arc::clone(base_den));
            return Some(Expr::pow_from_arcs(Arc::new(inner), Arc::clone(exp_num)));
        }
        None
    }
);

rule_arc!(
    CommonExponentProductRule,
    "common_exponent_product",
    55,
    Algebraic,
    &[RuleExprKind::Product],
    |expr: &Expr, context: &RuleContext| {
        if let ExprKind::Product(factors) = &expr.kind {
            // Look for pairs with same exponent
            for (i, f1) in factors.iter().enumerate() {
                for (j, f2) in factors.iter().enumerate() {
                    if i >= j {
                        continue;
                    }

                    if let (ExprKind::Pow(base_1, exp_1), ExprKind::Pow(base_2, exp_2)) =
                        (&f1.kind, &f2.kind)
                        && exp_1 == exp_2
                    {
                        // Skip if either base^exp would result in an integer - we prefer expanded numeric coefficients
                        // (e.g., 9*y^2 not (3y)^2, but allow sqrt(2)*sqrt(pi) → sqrt(2pi))
                        let would_simplify_to_int = |base: &Expr, exp: &Expr| -> bool {
                            if let (ExprKind::Number(base_val), ExprKind::Number(exp_val)) =
                                (&base.kind, &exp.kind)
                            {
                                let result = base_val.powf(*exp_val);
                                result.is_finite() && (result - result.round()).abs() < EPSILON
                            } else {
                                false
                            }
                        };

                        if would_simplify_to_int(base_1, exp_1)
                            || would_simplify_to_int(base_2, exp_2)
                        {
                            continue;
                        }

                        if context.domain_safe && is_fractional_root_exponent(exp_1) {
                            let left_non_neg = is_known_non_negative(base_1);
                            let right_non_neg = is_known_non_negative(base_2);
                            if !(left_non_neg && right_non_neg) {
                                continue;
                            }
                        }

                        // Combine: x^a * y^a = (x*y)^a
                        let combined_base =
                            Expr::product_from_arcs(vec![Arc::clone(base_1), Arc::clone(base_2)]);
                        let combined =
                            Expr::pow_from_arcs(Arc::new(combined_base), Arc::clone(exp_1));

                        let mut new_factors: Vec<Arc<Expr>> = factors
                            .iter()
                            .enumerate()
                            .filter(|(k, _)| *k != i && *k != j)
                            .map(|(_, f)| Arc::clone(f))
                            .collect();
                        new_factors.push(Arc::new(combined));

                        if new_factors.len() == 1 {
                            return Some(
                                new_factors
                                    .into_iter()
                                    .next()
                                    .expect("new_factors has exactly one element"),
                            );
                        }
                        return Some(Arc::new(Expr::product_from_arcs(new_factors)));
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
    &[RuleExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::Pow(base, exp) = &expr.kind {
            // Handle negative number exponent: x^-n -> 1/x^n
            if let ExprKind::Number(n) = exp.kind
                && n < 0.0
            {
                let positive_exp = Arc::new(Expr::number(-n));
                let denominator = Expr::pow_from_arcs(Arc::clone(base), positive_exp);
                return Some(Expr::div_expr(Expr::number(1.0), denominator));
            }
            // Handle negative fraction exponent: x^(-a/b) -> 1/x^(a/b)
            if let ExprKind::Div(num, den) = &exp.kind
                && let ExprKind::Number(n) = num.kind
                && n < 0.0
            {
                let positive_num = Expr::number(-n);
                let positive_exp = Expr::div_from_arcs(Arc::new(positive_num), Arc::clone(den));
                let denominator = Expr::pow_from_arcs(Arc::clone(base), Arc::new(positive_exp));
                return Some(Expr::div_expr(Expr::number(1.0), denominator));
            }
            // Handle Product with leading -1: x^(-1 * a * b * ...) -> 1/x^(a*b*...)
            // This handles products with any number of factors, not just 2
            if let ExprKind::Product(factors) = &exp.kind
                && !factors.is_empty()
                && let ExprKind::Number(n) = &factors[0].kind
            {
                // Exact check for -1.0 coefficient
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant -1.0")]
                let is_neg_one = *n == -1.0;
                if is_neg_one {
                    // Create a new exponent with the remaining factors (excluding -1)
                    let remaining_factors: Vec<Arc<Expr>> = factors[1..].to_vec();
                    let positive_exp = if remaining_factors.len() == 1 {
                        Arc::clone(&remaining_factors[0])
                    } else {
                        Arc::new(Expr::product_from_arcs(remaining_factors))
                    };
                    let denominator = Expr::pow_from_arcs(Arc::clone(base), positive_exp);
                    return Some(Expr::div_expr(Expr::number(1.0), denominator));
                }
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
    &[RuleExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::Pow(base, exp) = &expr.kind
            && let ExprKind::Div(num, den) = &base.kind
        {
            let is_root_exponent = match &exp.kind {
                ExprKind::Div(n, d) => {
                    // Exact check for 1/n root exponent
                    #[allow(
                        clippy::float_cmp,
                        reason = "Comparing against exact constants for root exponent"
                    )]
                    // Comparing against exact constants for root exponent
                    let res = matches!((&n.kind, &d.kind), (ExprKind::Number(num_val), ExprKind::Number(den_val))
                        if *num_val == 1.0 && *den_val >= 2.0);
                    res
                }
                ExprKind::Number(n) => *n > 0.0 && *n < 1.0,
                _ => false,
            };

            let den_would_simplify = match &den.kind {
                ExprKind::Pow(_, inner_exp) => {
                    if let (ExprKind::Number(m), ExprKind::Div(one, n_rc)) =
                        (&inner_exp.kind, &exp.kind)
                    {
                        if let (ExprKind::Number(one_val), ExprKind::Number(n_val)) =
                            (&one.kind, &n_rc.kind)
                        {
                            // Exact check for 1/n exponent form
                            #[allow(
                                clippy::float_cmp,
                                reason = "Comparing against exact constants for 1/n exponent"
                            )]
                            // Comparing against exact constants for 1/n exponent
                            let matches = *one_val == 1.0 && (m / n_val).fract().abs() < EPSILON;
                            matches
                        } else {
                            false
                        }
                    } else if let (ExprKind::Number(m), ExprKind::Number(exp_val)) =
                        (&inner_exp.kind, &exp.kind)
                    {
                        (m * exp_val).fract().abs() < EPSILON
                    } else {
                        false
                    }
                }
                ExprKind::Symbol(_) | ExprKind::Number(_) => is_root_exponent,
                _ => false,
            };

            if is_root_exponent || den_would_simplify {
                let num_pow = Expr::pow_from_arcs(Arc::clone(num), Arc::clone(exp));
                let den_pow = Expr::pow_from_arcs(Arc::clone(den), Arc::clone(exp));
                return Some(Expr::div_expr(num_pow, den_pow));
            }
        }
        None
    }
);
