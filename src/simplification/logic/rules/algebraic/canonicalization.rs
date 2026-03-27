use super::{Rule, RuleCategory, RuleContext, RuleExprKind, compare_expr, compare_mul_factors};
use crate::EPSILON;
use crate::core::{Expr, ExprKind};
use std::cmp::Ordering;
use std::sync::Arc;

// Note: With n-ary Sum and Product, canonicalization is simpler.
// Sum already flattens additions, Product already flattens multiplications.
// Subtraction is handled by adding negative terms to Sum.

rule!(
    CanonicalizeProductRule,
    "canonicalize_product",
    15,
    Algebraic,
    &[RuleExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::Product(factors) = &expr.kind {
            if factors.len() <= 1 {
                return None;
            }

            // Check if already sorted (compare on Arc contents)
            let is_sorted = factors
                .windows(2)
                .all(|w| compare_mul_factors(&w[0], &w[1]) != Ordering::Greater);

            if is_sorted {
                return None;
            }

            // Clone Arcs and sort (use unstable sort for performance)
            let mut sorted_factors: Vec<Arc<Expr>> = factors.clone();
            sorted_factors.sort_unstable_by(|a, b| compare_mul_factors(a, b));
            Some(Expr::product_from_arcs(sorted_factors))
        } else {
            None
        }
    }
);

rule!(
    CanonicalizeSumRule,
    "canonicalize_sum",
    15,
    Algebraic,
    &[RuleExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::Sum(terms) = &expr.kind {
            if terms.len() <= 1 {
                return None;
            }

            // Check if already sorted (compare on Arc contents)
            let is_sorted = terms
                .windows(2)
                .all(|w| compare_expr(&w[0], &w[1]) != Ordering::Greater);

            if is_sorted {
                return None;
            }

            // Clone Arcs and sort (use unstable sort for performance)
            let mut sorted_terms: Vec<Arc<Expr>> = terms.clone();
            sorted_terms.sort_unstable_by(|a, b| compare_expr(a, b));
            Some(Expr::sum_from_arcs(sorted_terms))
        } else {
            None
        }
    }
);

rule!(
    SimplifyNegativeProductRule,
    "simplify_negative_product",
    80,
    Algebraic,
    &[RuleExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::Product(factors) = &expr.kind {
            // Look for multiple (-1) factors and simplify
            let minus_one_count = factors
                .iter()
                .filter(|f| matches!(&f.kind, ExprKind::Number(n) if (n + 1.0).abs() < EPSILON))
                .count();

            if minus_one_count >= 2 {
                // (-1) * (-1) = 1, so pairs cancel out
                let remaining_minus_ones = minus_one_count % 2;
                let other_factors: Vec<Arc<Expr>> = factors
                    .iter()
                    .filter(
                        |f| !matches!(&f.kind, ExprKind::Number(n) if (n + 1.0).abs() < EPSILON),
                    )
                    .cloned()
                    .collect();

                let mut result_factors: Vec<Arc<Expr>> = Vec::new();
                if remaining_minus_ones == 1 {
                    result_factors.push(Arc::new(Expr::number(-1.0)));
                }
                result_factors.extend(other_factors);

                if result_factors.is_empty() {
                    return Some(Expr::number(1.0));
                }
                return Some(Expr::product_from_arcs(result_factors));
            }
        }
        None
    }
);
