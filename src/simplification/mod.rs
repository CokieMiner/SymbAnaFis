// Simplification framework - reduces expressions
pub(crate) mod engine;
mod helpers;
mod patterns;
mod rules;

use crate::Expr;

use std::collections::HashSet;

/// Simplify an expression using the new rule-based engine
pub fn simplify(expr: Expr) -> Expr {
    simplify_with_fixed_vars(expr, HashSet::new())
}

/// Simplify an expression with user-specified fixed variables
/// Fixed variables are treated as constants (e.g., "e" as a variable, not Euler's constant)
pub fn simplify_with_fixed_vars(expr: Expr, fixed_vars: HashSet<String>) -> Expr {
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
pub fn simplify_domain_safe_with_fixed_vars(expr: Expr, fixed_vars: HashSet<String>) -> Expr {
    let mut current = expr;

    // Use the new rule-based simplification engine with domain safety and fixed vars
    let variables = current.variables();
    current = engine::simplify_expr_with_verification_and_fixed_vars(
        current.clone(),
        variables,
        fixed_vars.clone(),
        true,
    )
    .unwrap_or_else(|_| {
        // Fallback if verification fails
        let mut simplifier = engine::Simplifier::new()
            .with_domain_safe(true)
            .with_fixed_vars(fixed_vars);
        simplifier.simplify(current)
    });

    // Prettify roots (x^0.5 -> sqrt(x)) for display
    // This must be done AFTER simplification to avoid fighting with normalize_roots
    current = helpers::prettify_roots(current);

    // Final step: Evaluate numeric functions like sqrt(4) -> 2
    // This happens at the very end so algebraic simplification works on powers
    current = evaluate_numeric_functions(current);

    current
}

/// Evaluate numeric functions like sqrt(4) -> 2, cbrt(27) -> 3
/// This runs at the very end after prettification
fn evaluate_numeric_functions(expr: Expr) -> Expr {
    match expr {
        // Recursively process subexpressions first
        Expr::Add(u, v) => Expr::Add(
            Box::new(evaluate_numeric_functions(*u)),
            Box::new(evaluate_numeric_functions(*v)),
        ),
        Expr::Sub(u, v) => Expr::Sub(
            Box::new(evaluate_numeric_functions(*u)),
            Box::new(evaluate_numeric_functions(*v)),
        ),
        Expr::Mul(u, v) => {
            let u = evaluate_numeric_functions(*u);
            let v = evaluate_numeric_functions(*v);

            // Canonical form: 0.5 * expr -> expr / 2 (for fractional coefficients)
            // This makes log2(x^0.5) -> log2(x)/2 instead of 0.5*log2(x)
            if let Expr::Number(n) = &u
                && *n == 0.5
            {
                return Expr::Div(Box::new(v), Box::new(Expr::Number(2.0)));
            }
            if let Expr::Number(n) = &v
                && *n == 0.5
            {
                return Expr::Div(Box::new(u), Box::new(Expr::Number(2.0)));
            }

            Expr::Mul(Box::new(u), Box::new(v))
        }
        Expr::Div(u, v) => {
            let u = evaluate_numeric_functions(*u);
            let v = evaluate_numeric_functions(*v);

            if let (Expr::Number(n1), Expr::Number(n2)) = (&u, &v)
                && *n2 != 0.0
            {
                let result = n1 / n2;
                if (result - result.round()).abs() < 1e-10 {
                    return Expr::Number(result.round());
                }
            }

            Expr::Div(Box::new(u), Box::new(v))
        }
        Expr::Pow(u, v) => {
            let u = evaluate_numeric_functions(*u);
            let v = evaluate_numeric_functions(*v);

            // Evaluate Number^Number if result is clean
            if let (Expr::Number(base), Expr::Number(exp)) = (&u, &v) {
                let result = base.powf(*exp);
                if (result - result.round()).abs() < 1e-10 {
                    return Expr::Number(result.round());
                }
            }

            Expr::Pow(Box::new(u), Box::new(v))
        }
        Expr::FunctionCall { name, args } => {
            let args: Vec<Expr> = args.into_iter().map(evaluate_numeric_functions).collect();

            // Evaluate sqrt(n) if n is a perfect square
            if name == "sqrt"
                && args.len() == 1
                && let Expr::Number(n) = &args[0]
            {
                let sqrt_n = n.sqrt();
                if (sqrt_n - sqrt_n.round()).abs() < 1e-10 {
                    return Expr::Number(sqrt_n.round());
                }
            }

            // Evaluate cbrt(n) if n is a perfect cube
            if name == "cbrt"
                && args.len() == 1
                && let Expr::Number(n) = &args[0]
            {
                let cbrt_n = n.cbrt();
                if (cbrt_n - cbrt_n.round()).abs() < 1e-10 {
                    return Expr::Number(cbrt_n.round());
                }
            }

            Expr::FunctionCall { name, args }
        }
        other => other,
    }
}
