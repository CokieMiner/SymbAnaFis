//! Simplification framework - reduces expressions
pub(crate) mod engine;
pub(crate) mod helpers;
mod patterns;
mod rules;

use crate::{Expr, ExprKind};

use std::collections::HashSet;

/// Simplify an expression with user-specified fixed variables
/// Fixed variables are treated as constants (e.g., "e" as a variable, not Euler's constant)
pub fn simplify_expr(expr: Expr, fixed_vars: HashSet<String>) -> Expr {
    let mut current = expr;

    // Use the new rule-based simplification engine with fixed vars
    current = engine::simplify_expr_with_fixed_vars(current, fixed_vars);

    // Prettify roots (x^0.5 -> sqrt(x)) for display
    // This must be done AFTER simplification to avoid fighting with normalize_roots
    current = helpers::prettify_roots(current);

    // Final step: Evaluate numeric functions like sqrt(4) -> 2
    // This happens at the very end so algebraic simplification works on powers
    current = evaluate_numeric_functions(current);

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
    current = evaluate_numeric_functions(current);
    current
}

/// Evaluate numeric functions like sqrt(4) -> 2, cbrt(27) -> 3
/// This runs at the very end after prettification
fn evaluate_numeric_functions(expr: Expr) -> Expr {
    match expr.kind {
        // N-ary Sum - recursively process all terms
        ExprKind::Sum(terms) => {
            let processed: Vec<Expr> = terms
                .into_iter()
                .map(|t| {
                    evaluate_numeric_functions(
                        std::sync::Arc::try_unwrap(t).unwrap_or_else(|arc| (*arc).clone()),
                    )
                })
                .collect();
            Expr::sum(processed)
        }

        // N-ary Product - recursively process all factors
        ExprKind::Product(factors) => {
            let processed: Vec<Expr> = factors
                .into_iter()
                .map(|f| {
                    evaluate_numeric_functions(
                        std::sync::Arc::try_unwrap(f).unwrap_or_else(|arc| (*arc).clone()),
                    )
                })
                .collect();

            // Check for 0.5 coefficient: 0.5 * expr -> expr / 2
            if processed.len() == 2 {
                if let ExprKind::Number(n) = &processed[0].kind
                    && *n == 0.5 {
                        return Expr::div_expr(processed[1].clone(), Expr::number(2.0));
                    }
                if let ExprKind::Number(n) = &processed[1].kind
                    && *n == 0.5 {
                        return Expr::div_expr(processed[0].clone(), Expr::number(2.0));
                    }
            }

            Expr::product(processed)
        }
        ExprKind::Div(u, v) => {
            let u = evaluate_numeric_functions(u.as_ref().clone());
            let v = evaluate_numeric_functions(v.as_ref().clone());

            if let (ExprKind::Number(n1), ExprKind::Number(n2)) = (&u.kind, &v.kind)
                && *n2 != 0.0
            {
                let result = n1 / n2;
                if (result - result.round()).abs() < 1e-10 {
                    return Expr::number(result.round());
                }
            }

            Expr::div_expr(u, v)
        }
        ExprKind::Pow(u, v) => {
            let u = evaluate_numeric_functions(u.as_ref().clone());
            let v = evaluate_numeric_functions(v.as_ref().clone());

            // Evaluate Number^Number if result is clean
            if let (ExprKind::Number(base), ExprKind::Number(exp)) = (&u.kind, &v.kind) {
                let result = base.powf(*exp);
                if (result - result.round()).abs() < 1e-10 {
                    return Expr::number(result.round());
                }
            }

            Expr::pow(u, v)
        }
        ExprKind::FunctionCall { name, args } => {
            let args: Vec<Expr> = args.into_iter().map(evaluate_numeric_functions).collect();

            // Evaluate sqrt(n) if n is a perfect square
            if name == "sqrt"
                && args.len() == 1
                && let ExprKind::Number(n) = &args[0].kind
            {
                let sqrt_n = n.sqrt();
                if (sqrt_n - sqrt_n.round()).abs() < 1e-10 {
                    return Expr::number(sqrt_n.round());
                }
            }

            // Evaluate cbrt(n) if n is a perfect cube
            if name == "cbrt"
                && args.len() == 1
                && let ExprKind::Number(n) = &args[0].kind
            {
                let cbrt_n = n.cbrt();
                if (cbrt_n - cbrt_n.round()).abs() < 1e-10 {
                    return Expr::number(cbrt_n.round());
                }
            }

            Expr::func_multi(name, args)
        }
        _ => Expr::new(expr.kind),
    }
}
