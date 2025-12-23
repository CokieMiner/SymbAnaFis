//! Simplification framework - reduces expressions
pub(crate) mod engine;
pub(crate) mod helpers;
mod patterns;
mod rules;

use crate::Expr;

use std::collections::HashSet;

/// Simplify an expression with user-specified fixed variables
/// Fixed variables are treated as constants (e.g., "e" as a variable, not Euler's constant)
pub(crate) fn simplify_expr(expr: Expr, fixed_vars: HashSet<String>) -> Expr {
    let mut current = expr;

    // Use the new rule-based simplification engine with fixed vars
    current = engine::simplify_expr_with_fixed_vars(current, fixed_vars);

    // Prettify roots (x^0.5 -> sqrt(x)) for display
    // This must be done AFTER simplification to avoid fighting with normalize_roots
    current = helpers::prettify_roots(current);

    current
}

/// Simplify an expression with domain safety and user-specified fixed variables
/// Fixed variables are treated as constants (e.g., "e" as a variable, not Euler's constant)
pub(crate) fn simplify_domain_safe(expr: Expr, fixed_vars: HashSet<String>) -> Expr {
    let mut current = expr;

    let mut simplifier = engine::Simplifier::new()
        .with_domain_safe(true)
        .with_fixed_vars(fixed_vars);
    current = simplifier.simplify(current);

    current = helpers::prettify_roots(current);
    current
}
