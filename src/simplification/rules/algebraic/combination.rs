// Algebraic combination operations on validated expressions

use crate::core::traits::EPSILON;
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use crate::{Expr, ExprKind as AstKind};
use std::sync::Arc;

rule_arc!(
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
                        .map(|(_, f)| Arc::clone(f))
                        .collect();
                    new_numerator_factors.push(Arc::clone(num));

                    let new_num = if new_numerator_factors.len() == 1 {
                        Arc::clone(&new_numerator_factors[0])
                    } else {
                        Arc::new(Expr::product_from_arcs(new_numerator_factors))
                    };

                    return Some(Arc::new(Expr::div_from_arcs(new_num, Arc::clone(den))));
                }
            }
        }
        None
    }
);

rule_arc!(
    CombineTermsRule,
    "combine_terms",
    50,
    Algebraic,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        // Optimized combine_terms using Arcs and HashMap<Arc<Expr>, f64>
        if let AstKind::Sum(terms) = &expr.kind {
            if terms.len() < 2 {
                return None;
            }

            // Group terms by their base to find combinable terms
            // Use HashMap with Arc<Expr> key to avoid cloning
            let mut term_groups: std::collections::HashMap<Arc<Expr>, f64> =
                std::collections::HashMap::new();

            for term in terms {
                // Use new extract_coeff_arc helper
                let (coeff, base) =
                    crate::simplification::helpers::extract_coeff_arc(term.as_ref());
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
                } else if (coeff - 1.0).abs() < EPSILON {
                    // 1*base = base
                    result.push(base);
                } else if let AstKind::Number(n) = &base.kind {
                    if (*n - 1.0).abs() < EPSILON {
                        result.push(Arc::new(Expr::number(coeff)));
                    } else {
                        // number * number -> combined number
                        result.push(Arc::new(Expr::number(coeff * n)));
                    }
                } else {
                    result.push(Arc::new(Expr::product_from_arcs(vec![
                        Arc::new(Expr::number(coeff)),
                        base,
                    ])));
                }
            }

            if result.is_empty() {
                return Some(Arc::new(Expr::number(0.0)));
            }

            // Sort terms for canonical ordering (use unstable sort for performance)
            if result.len() > 1 {
                result.sort_unstable_by(|a, b| crate::simplification::helpers::compare_expr(a, b));
            }

            if result.len() == 1 {
                return Some(
                    result
                        .into_iter()
                        .next()
                        .expect("Iterator guaranteed to have one element"),
                );
            }

            Some(Arc::new(Expr::sum_from_arcs(result)))
        } else {
            None
        }
    }
);

rule_arc!(
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
            // Use HashMap<Arc<Expr>, ...> to avoid cloning keys
            let mut factor_groups: std::collections::HashMap<Arc<Expr>, Vec<Arc<Expr>>> =
                std::collections::HashMap::new();

            for factor in factors {
                match &factor.kind {
                    AstKind::Pow(base, exp) => {
                        factor_groups
                            .entry(Arc::clone(base))
                            .or_default()
                            .push(Arc::clone(exp));
                    }
                    _ => {
                        factor_groups
                            .entry(Arc::clone(factor))
                            .or_default()
                            .push(Arc::new(Expr::number(1.0)));
                    }
                }
            }

            // Combine exponents for each base
            let mut combined_factors = Vec::new();
            for (base, exponents) in factor_groups {
                if exponents.len() == 1 {
                    if let AstKind::Number(n) = &exponents[0].kind
                        && (*n - 1.0).abs() < EPSILON
                    {
                        combined_factors.push(base);
                    } else {
                        combined_factors.push(Arc::new(Expr::pow_from_arcs(
                            base,
                            Arc::clone(&exponents[0]),
                        )));
                    }
                } else {
                    // Sum all exponents using n-ary Sum
                    // Need to optimize sum construction? Expr::sum_from_arcs
                    let total_exp = Arc::new(Expr::sum_from_arcs(exponents));
                    combined_factors.push(Arc::new(Expr::pow_from_arcs(base, total_exp)));
                }
            }

            if combined_factors.len() == factors_len {
                None
            } else if combined_factors.len() == 1 {
                Some(
                    combined_factors
                        .into_iter()
                        .next()
                        .expect("Iterator guaranteed to have one element"),
                )
            } else {
                Some(Arc::new(Expr::product_from_arcs(combined_factors)))
            }
        } else {
            None
        }
    }
);

// Helper: Extract factor and addends from Product containing Sum
/// Extracts a factor and addends from a product containing a sum.
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
                    other_factors
                        .into_iter()
                        .next()
                        .expect("Iterator guaranteed to have one element")
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
/// Checks if the expression contains any variables (not just numbers).
fn contains_variable(expr: &Expr) -> bool {
    match &expr.kind {
        AstKind::Symbol(_) => true,
        AstKind::Number(_) => false,
        AstKind::Sum(terms) => terms.iter().any(|t| contains_variable(t)),
        AstKind::Product(factors) => factors.iter().any(|f| contains_variable(f)),
        AstKind::Div(a, b) | AstKind::Pow(a, b) => contains_variable(a) || contains_variable(b),
        AstKind::FunctionCall { args, .. } => args.iter().any(|a| contains_variable(a)),
        AstKind::Derivative { inner, .. } => contains_variable(inner),
        AstKind::Poly(poly) => {
            // Check if base contains variables (non-constant polynomial)
            contains_variable(poly.base())
        }
    }
}

// Helper: Extract base and numeric exponent from an expression
// x -> (x, 1.0), x^n -> (x, n)
/// Extracts the base and numeric exponent from an expression.
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
/// Combines variable expressions, handling x*x*x...->x^n.
fn combine_var_parts(a: Expr, b: Expr) -> Expr {
    // Try to extract base and exponent from both
    if let (Some((base_a, exp_a)), Some((base_b, exp_b))) =
        (extract_base_and_exp(&a), extract_base_and_exp(&b))
    {
        // If same base, combine exponents
        if base_a == base_b {
            let total_exp = exp_a + exp_b;
            if (total_exp - 1.0).abs() < EPSILON {
                return base_a;
            }
            return Expr::pow(base_a, Expr::number(total_exp));
        }
    }

    // Default: just multiply
    Expr::product(vec![a, b])
}

// Helper: Distribute a factor over addends and build canonical terms
/// Distributes a factor over addends and builds canonical terms.
fn distribute_factor(factor: &Expr, addends: &[Expr]) -> Vec<Expr> {
    addends
        .iter()
        .map(|addend| {
            // Extract coefficients from both factor and addend
            let (factor_coeff, factor_var) = crate::simplification::helpers::extract_coeff(factor);
            let (addend_coeff, addend_var) = crate::simplification::helpers::extract_coeff(addend);

            let total_coeff = factor_coeff * addend_coeff;

            // Combine variable parts, converting x*x to x^2 etc.
            let combined_var = {
                // Exact check for 1.0 to avoid redundant multiplication
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
                let factor_var_is_one = matches!(factor_var.kind, AstKind::Number(n) if n == 1.0);
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
                let addend_var_is_one = matches!(addend_var.kind, AstKind::Number(n) if n == 1.0);

                if factor_var_is_one {
                    addend_var
                } else if addend_var_is_one {
                    factor_var
                } else {
                    combine_var_parts(factor_var, addend_var)
                }
            };

            // Build canonical result (coefficient first)
            if (total_coeff - 1.0).abs() < EPSILON {
                combined_var
            } else {
                Expr::product(vec![Expr::number(total_coeff), combined_var])
            }
        })
        .collect()
}

rule_arc!(
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

            // We can work with Arcs directly
            let terms_vec = terms;

            // First pass: collect hashes of non-product-with-sum terms
            let mut existing_hashes: std::collections::HashSet<u64> =
                std::collections::HashSet::new();
            for term in terms_vec {
                if extract_product_with_sum(term).is_none() {
                    let (_, base) =
                        crate::simplification::helpers::extract_coeff_arc(term.as_ref());
                    existing_hashes
                        .insert(crate::simplification::helpers::get_term_hash(base.as_ref()));
                }
            }

            // Second pass: expand products that would create combinable terms
            let mut expanded_terms: Vec<Arc<Expr>> = Vec::new();
            let mut did_expand = false;

            for term in terms_vec {
                if let Some((factor, addends)) = extract_product_with_sum(term) {
                    // factor and addends are Expr (deep copy from helper)
                    // We can't easily optimize extract_product_with_sum without changing helper signature
                    // But assume we accept the clone there for now.
                    if contains_variable(&factor) {
                        let distributed = distribute_factor(&factor, &addends);

                        // Check if any distributed term would combine with existing terms
                        let would_combine = distributed.iter().any(|dt| {
                            let (_, base) = crate::simplification::helpers::extract_coeff(dt);
                            let hash = crate::simplification::helpers::get_term_hash(&base);
                            existing_hashes.contains(&hash)
                        });

                        if would_combine {
                            expanded_terms.extend(distributed.into_iter().map(Arc::new));
                            did_expand = true;
                            continue;
                        }
                    }
                }
                expanded_terms.push(Arc::clone(term));
            }

            // If we expanded anything, return the expanded form (will be re-simplified)
            if did_expand {
                if expanded_terms.len() == 1 {
                    return Some(
                        expanded_terms
                            .into_iter()
                            .next()
                            .expect("Iterator guaranteed to have one element"),
                    );
                }
                return Some(Arc::new(Expr::sum_from_arcs(expanded_terms)));
            }

            // Original like-term combination logic
            // Group terms by their structure (ignoring coefficients)
            // Use HashMap<u64, Vec<Arc<Expr>>> for initial grouping by hash
            let mut like_terms: std::collections::HashMap<u64, Vec<Arc<Expr>>> =
                std::collections::HashMap::new();

            for term in terms_vec {
                let key = crate::simplification::helpers::get_term_hash(term.as_ref());
                like_terms.entry(key).or_default().push(Arc::clone(term));
            }

            // Combine like terms, with structural verification to handle hash collisions
            let mut combined_terms = Vec::new();
            for (_signature, group_terms) in like_terms {
                if group_terms.len() == 1 {
                    combined_terms.push(Arc::clone(&group_terms[0]));
                } else {
                    // Sub-group by actual structural base using HashMap (O(n) amortized).
                    // Expr's Hash+PartialEq handles hash collisions via structural equality.
                    let mut sub_groups: std::collections::HashMap<Arc<Expr>, f64> =
                        std::collections::HashMap::new();
                    for term in &group_terms {
                        let (coeff, var_part) =
                            crate::simplification::helpers::extract_coeff_arc(term.as_ref());
                        *sub_groups.entry(var_part).or_insert(0.0) += coeff;
                    }

                    for (base_term, total_coeff) in sub_groups {
                        if total_coeff.abs() < EPSILON {
                            // Terms cancel out
                            continue;
                        }

                        if (total_coeff - 1.0).abs() < EPSILON {
                            combined_terms.push(base_term);
                        } else if let AstKind::Number(n) = &base_term.kind {
                            if (*n - 1.0).abs() < EPSILON {
                                combined_terms.push(Arc::new(Expr::number(total_coeff)));
                            } else {
                                combined_terms.push(Arc::new(Expr::number(total_coeff * n)));
                            }
                        } else {
                            combined_terms.push(Arc::new(Expr::product_from_arcs(vec![
                                Arc::new(Expr::number(total_coeff)),
                                base_term,
                            ])));
                        }
                    }
                }
            }

            if combined_terms.len() == terms.len() {
                None
            } else if combined_terms.is_empty() {
                Some(Arc::new(Expr::number(0.0)))
            } else if combined_terms.len() == 1 {
                Some(
                    combined_terms
                        .into_iter()
                        .next()
                        .expect("Iterator guaranteed to have one element"),
                )
            } else {
                Some(Arc::new(Expr::sum_from_arcs(combined_terms)))
            }
        } else {
            None
        }
    }
);
