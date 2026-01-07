use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use crate::{Expr, ExprKind as AstKind};
use std::sync::Arc;

// Note: With n-ary Sum and Product, canonicalization is simpler.
// Sum already flattens additions, Product already flattens multiplications.
// Subtraction is handled by adding negative terms to Sum.

rule!(
    CanonicalizeProductRule,
    "canonicalize_product",
    15,
    Algebraic,
    &[ExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Product(factors) = &expr.kind {
            if factors.len() <= 1 {
                return None;
            }

            // Check if already sorted (compare on Arc contents)
            let is_sorted = factors.windows(2).all(|w| {
                crate::simplification::helpers::compare_mul_factors(&w[0], &w[1])
                    != std::cmp::Ordering::Greater
            });

            if is_sorted {
                return None;
            }

            // Clone Arcs and sort
            let mut sorted_factors: Vec<Arc<Expr>> = factors.clone();
            sorted_factors
                .sort_by(|a, b| crate::simplification::helpers::compare_mul_factors(a, b));
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
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Sum(terms) = &expr.kind {
            if terms.len() <= 1 {
                return None;
            }

            // Check if already sorted (compare on Arc contents)
            let is_sorted = terms.windows(2).all(|w| {
                crate::simplification::helpers::compare_expr(&w[0], &w[1])
                    != std::cmp::Ordering::Greater
            });

            if is_sorted {
                return None;
            }

            // Clone Arcs and sort
            let mut sorted_terms: Vec<Arc<Expr>> = terms.clone();
            sorted_terms.sort_by(|a, b| crate::simplification::helpers::compare_expr(a, b));
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
    &[ExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Product(factors) = &expr.kind {
            // Look for multiple (-1) factors and simplify
            let minus_one_count = factors
                .iter()
                .filter(|f| matches!(&f.kind, AstKind::Number(n) if (*n + 1.0).abs() < 1e-10))
                .count();

            if minus_one_count >= 2 {
                // (-1) * (-1) = 1, so pairs cancel out
                let remaining_minus_ones = minus_one_count % 2;
                let other_factors: Vec<Arc<Expr>> = factors
                    .iter()
                    .filter(|f| !matches!(&f.kind, AstKind::Number(n) if (*n + 1.0).abs() < 1e-10))
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
