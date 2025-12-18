use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use crate::{Expr, ExprKind as AstKind};
use std::sync::Arc;

rule!(
    ProductDivCombinationRule,
    "product_div_combination",
    85,
    Algebraic,
    &[ExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Product(factors) = &expr.kind {
            // Look for Div among the factors: a * (b / c) * d -> (a * b * d) / c
            for (i, factor) in factors.iter().enumerate() {
                if let AstKind::Div(num, den) = &factor.kind {
                    // Collect all other factors into numerator
                    let mut new_numerator_factors: Vec<Arc<Expr>> = factors
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .map(|(_, f)| f.clone())
                        .collect();
                    new_numerator_factors.push(num.clone());

                    let new_num = if new_numerator_factors.len() == 1 {
                        let arc = new_numerator_factors.into_iter().next().unwrap();
                        match Arc::try_unwrap(arc) {
                            Ok(e) => e,
                            Err(a) => (*a).clone(),
                        }
                    } else {
                        Expr::product_from_arcs(new_numerator_factors)
                    };

                    return Some(Expr::new(AstKind::Div(Arc::new(new_num), den.clone())));
                }
            }
        }
        None
    }
);

rule!(
    CombineTermsRule,
    "combine_terms",
    50,
    Algebraic,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Sum(terms) = &expr.kind {
            if terms.len() < 2 {
                return None;
            }

            // Group terms by their base to find combinable terms
            let mut term_groups: std::collections::HashMap<Expr, f64> =
                std::collections::HashMap::new();
            for term in terms {
                let (coeff, base) = crate::simplification::helpers::extract_coeff(term);
                *term_groups.entry(base).or_insert(0.0) += coeff;
            }

            // If no terms were actually combined, don't change anything
            if term_groups.len() == terms.len() {
                return None;
            }

            // Build result from combined terms
            let mut result = Vec::new();
            for (base, coeff) in term_groups {
                if coeff == 0.0 {
                    // Drop zero terms
                } else if coeff == 1.0 {
                    result.push(base);
                } else if let AstKind::Number(n) = &base.kind {
                    if *n == 1.0 {
                        result.push(Expr::number(coeff));
                    } else {
                        result.push(Expr::product(vec![Expr::number(coeff), base]));
                    }
                } else {
                    result.push(Expr::product(vec![Expr::number(coeff), base]));
                }
            }

            if result.is_empty() {
                return Some(Expr::number(0.0));
            }

            // Sort terms for canonical ordering
            result.sort_by(crate::simplification::helpers::compare_expr);

            if result.len() == 1 {
                return Some(result.into_iter().next().unwrap());
            }

            Some(Expr::sum(result))
        } else {
            None
        }
    }
);

rule!(
    CombineFactorsRule,
    "combine_factors",
    58,
    Algebraic,
    &[ExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Product(factors) = &expr.kind {
            if factors.len() < 2 {
                return None;
            }

            let factors_len = factors.len();

            // Group factors by base and combine exponents
            let mut factor_groups: std::collections::HashMap<Expr, Vec<Expr>> =
                std::collections::HashMap::new();

            for factor in factors {
                match &factor.kind {
                    AstKind::Pow(base, exp) => {
                        factor_groups
                            .entry(base.as_ref().clone())
                            .or_default()
                            .push(exp.as_ref().clone());
                    }
                    _ => {
                        factor_groups
                            .entry((**factor).clone())
                            .or_default()
                            .push(Expr::number(1.0));
                    }
                }
            }

            // Combine exponents for each base
            let mut combined_factors = Vec::new();
            for (base, exponents) in factor_groups {
                if exponents.len() == 1 {
                    if exponents[0] == Expr::number(1.0) {
                        combined_factors.push(base);
                    } else {
                        combined_factors.push(Expr::pow(base, exponents[0].clone()));
                    }
                } else {
                    // Sum all exponents using n-ary Sum
                    let total_exp = Expr::sum(exponents);
                    combined_factors.push(Expr::pow(base, total_exp));
                }
            }

            if combined_factors.len() != factors_len {
                if combined_factors.len() == 1 {
                    Some(combined_factors.into_iter().next().unwrap())
                } else {
                    Some(Expr::product(combined_factors))
                }
            } else {
                None
            }
        } else {
            None
        }
    }
);

// Helper: Extract factor and addends from Product containing Sum
fn extract_product_with_sum(expr: &Expr) -> Option<(Expr, Vec<Expr>)> {
    if let AstKind::Product(factors) = &expr.kind {
        // Look for a Sum among the factors
        for (i, factor) in factors.iter().enumerate() {
            if let AstKind::Sum(addends) = &factor.kind {
                // Collect all other factors
                let other_factors: Vec<Expr> = factors
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .map(|(_, f)| (**f).clone())
                    .collect();

                let combined_factor = if other_factors.is_empty() {
                    Expr::number(1.0)
                } else if other_factors.len() == 1 {
                    other_factors.into_iter().next().unwrap()
                } else {
                    Expr::product(other_factors)
                };

                let addends_vec: Vec<Expr> = addends.iter().map(|a| (**a).clone()).collect();
                return Some((combined_factor, addends_vec));
            }
        }
    }
    None
}

// Helper: Check if expression contains a variable (not just numbers)
fn contains_variable(expr: &Expr) -> bool {
    match &expr.kind {
        AstKind::Symbol(_) => true,
        AstKind::Number(_) => false,
        AstKind::Sum(terms) => terms.iter().any(|t| contains_variable(t)),
        AstKind::Product(factors) => factors.iter().any(|f| contains_variable(f)),
        AstKind::Div(a, b) | AstKind::Pow(a, b) => contains_variable(a) || contains_variable(b),
        AstKind::FunctionCall { args, .. } => args.iter().any(contains_variable),
        AstKind::Derivative { inner, .. } => contains_variable(inner),
    }
}

// Helper: Extract base and numeric exponent from an expression
// x -> (x, 1.0), x^n -> (x, n)
fn extract_base_and_exp(expr: &Expr) -> Option<(Expr, f64)> {
    match &expr.kind {
        AstKind::Symbol(_) => Some((expr.clone(), 1.0)),
        AstKind::Pow(base, exp) => {
            if let AstKind::Number(n) = &exp.kind {
                Some((base.as_ref().clone(), *n))
            } else {
                None // Non-numeric exponent, can't combine
            }
        }
        _ => None,
    }
}

// Helper: Combine variable expressions, handling x*x*x...->x^n
fn combine_var_parts(a: Expr, b: Expr) -> Expr {
    // Try to extract base and exponent from both
    if let (Some((base_a, exp_a)), Some((base_b, exp_b))) =
        (extract_base_and_exp(&a), extract_base_and_exp(&b))
    {
        // If same base, combine exponents
        if base_a == base_b {
            let total_exp = exp_a + exp_b;
            if (total_exp - 1.0).abs() < 1e-10 {
                return base_a;
            }
            return Expr::pow(base_a, Expr::number(total_exp));
        }
    }

    // Default: just multiply
    Expr::product(vec![a, b])
}

// Helper: Distribute a factor over addends and build canonical terms
fn distribute_factor(factor: &Expr, addends: &[Expr]) -> Vec<Expr> {
    addends
        .iter()
        .map(|addend| {
            // Extract coefficients from both factor and addend
            let (factor_coeff, factor_var) = crate::simplification::helpers::extract_coeff(factor);
            let (addend_coeff, addend_var) = crate::simplification::helpers::extract_coeff(addend);

            let total_coeff = factor_coeff * addend_coeff;

            // Combine variable parts, converting x*x to x^2 etc.
            let combined_var = if matches!(factor_var.kind, AstKind::Number(n) if n == 1.0) {
                addend_var
            } else if matches!(addend_var.kind, AstKind::Number(n) if n == 1.0) {
                factor_var
            } else {
                combine_var_parts(factor_var, addend_var)
            };

            // Build canonical result (coefficient first)
            if (total_coeff - 1.0).abs() < 1e-10 {
                combined_var
            } else {
                Expr::product(vec![Expr::number(total_coeff), combined_var])
            }
        })
        .collect()
}

rule!(
    CombineLikeTermsInSumRule,
    "combine_like_terms_in_sum",
    52,
    Algebraic,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Sum(terms) = &expr.kind {
            if terms.len() < 2 {
                return None;
            }

            let terms_vec: Vec<Expr> = terms.iter().map(|t| (**t).clone()).collect();

            // First pass: collect hashes of non-product-with-sum terms
            let mut existing_hashes: std::collections::HashSet<u64> =
                std::collections::HashSet::new();
            for term in &terms_vec {
                if extract_product_with_sum(term).is_none() {
                    let (_, base) = crate::simplification::helpers::extract_coeff(term);
                    existing_hashes.insert(crate::simplification::helpers::get_term_hash(&base));
                }
            }

            // Second pass: expand products that would create combinable terms
            let mut expanded_terms: Vec<Expr> = Vec::new();
            let mut did_expand = false;

            for term in &terms_vec {
                if let Some((factor, addends)) = extract_product_with_sum(term) {
                    // Only expand if factor contains a variable
                    if contains_variable(&factor) {
                        let distributed = distribute_factor(&factor, &addends);

                        // Check if any distributed term would combine with existing terms
                        let would_combine = distributed.iter().any(|dt| {
                            let (_, base) = crate::simplification::helpers::extract_coeff(dt);
                            let hash = crate::simplification::helpers::get_term_hash(&base);
                            existing_hashes.contains(&hash)
                        });

                        if would_combine {
                            expanded_terms.extend(distributed);
                            did_expand = true;
                            continue;
                        }
                    }
                }
                expanded_terms.push(term.clone());
            }

            // If we expanded anything, return the expanded form (will be re-simplified)
            if did_expand {
                if expanded_terms.len() == 1 {
                    return Some(expanded_terms.into_iter().next().unwrap());
                }
                return Some(Expr::sum(expanded_terms));
            }

            // Original like-term combination logic
            // Group terms by their structure (ignoring coefficients)
            let mut like_terms: std::collections::HashMap<u64, Vec<Expr>> =
                std::collections::HashMap::new();

            for term in &terms_vec {
                let key = crate::simplification::helpers::get_term_hash(term);
                like_terms.entry(key).or_default().push(term.clone());
            }

            // Combine like terms
            let mut combined_terms = Vec::new();
            for (_signature, group_terms) in like_terms {
                if group_terms.len() == 1 {
                    combined_terms.push(group_terms[0].clone());
                } else {
                    // Sum the coefficients of like terms
                    let mut total_coeff = 0.0;
                    let mut base_term = None;

                    for term in group_terms {
                        let (coeff, var_part) =
                            crate::simplification::helpers::extract_coeff(&term);
                        total_coeff += coeff;
                        if base_term.is_none() {
                            base_term = Some(var_part);
                        }
                    }

                    if (total_coeff - 0.0).abs() < 1e-10 {
                        // Terms cancel out
                        continue;
                    }

                    let base_term = base_term.unwrap();
                    if (total_coeff - 1.0).abs() < 1e-10 {
                        combined_terms.push(base_term);
                    } else {
                        combined_terms
                            .push(Expr::product(vec![Expr::number(total_coeff), base_term]));
                    }
                }
            }

            if combined_terms.len() != terms.len() {
                if combined_terms.is_empty() {
                    Some(Expr::number(0.0))
                } else if combined_terms.len() == 1 {
                    Some(combined_terms.into_iter().next().unwrap())
                } else {
                    Some(Expr::sum(combined_terms))
                }
            } else {
                None
            }
        } else {
            None
        }
    }
);

rule!(
    DistributeNegationRule,
    "distribute_negation",
    50,
    Algebraic,
    &[ExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Product(factors) = &expr.kind {
            // Check if first factor is -1
            if let Some(first) = factors.first() {
                if matches!(&first.kind, AstKind::Number(n) if (*n + 1.0).abs() < 1e-10) {
                    // Get the remaining factors
                    let rest: Vec<_> = factors.iter().skip(1).map(|f| (**f).clone()).collect();
                    if rest.len() == 1 {
                        let inner = &rest[0];
                        // -1 * Sum -> negate all terms
                        if let AstKind::Sum(terms) = &inner.kind {
                            let negated: Vec<Expr> = terms
                                .iter()
                                .map(|t| Expr::product(vec![Expr::number(-1.0), (**t).clone()]))
                                .collect();
                            return Some(Expr::sum(negated));
                        }
                    }
                }
            }
        }
        None
    }
);
