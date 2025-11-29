// Simplification framework - reduces expressions
mod algebraic;
mod hyperbolic;
mod log_exp;
mod numeric;
mod roots;
pub mod trig;

use crate::Expr;
use std::collections::HashSet;

/// Simplify an expression by applying rules until fixed point
pub fn simplify(expr: Expr) -> Expr {
    let mut current = expr;
    let mut seen = HashSet::new();
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 1000;

    loop {
        if iterations >= MAX_ITERATIONS {
            eprintln!("Warning: Simplification iteration limit reached");
            break;
        }

        // Cycle detection
        let expr_str = format!("{:?}", current);
        if seen.contains(&expr_str) {
            break;
        }
        seen.insert(expr_str);

        let original = current.clone();
        current = apply_rules(current);

        if current == original {
            break;
        }

        iterations += 1;
    }

    current
}

/// Apply all simplification rules recursively (bottom-up)
fn apply_rules(expr: Expr) -> Expr {
    match expr {
        Expr::Add(u, v) => apply_single_rule(Expr::Add(
            Box::new(apply_rules(*u)),
            Box::new(apply_rules(*v)),
        )),
        Expr::Sub(u, v) => apply_single_rule(Expr::Sub(
            Box::new(apply_rules(*u)),
            Box::new(apply_rules(*v)),
        )),
        Expr::Mul(u, v) => apply_single_rule(Expr::Mul(
            Box::new(apply_rules(*u)),
            Box::new(apply_rules(*v)),
        )),
        Expr::Div(u, v) => apply_single_rule(Expr::Div(
            Box::new(apply_rules(*u)),
            Box::new(apply_rules(*v)),
        )),
        Expr::Pow(u, v) => apply_single_rule(Expr::Pow(
            Box::new(apply_rules(*u)),
            Box::new(apply_rules(*v)),
        )),
        Expr::FunctionCall { name, args } => apply_single_rule(Expr::FunctionCall {
            name,
            args: args.into_iter().map(apply_rules).collect(),
        }),
        other => apply_single_rule(other),
    }
}

/// Apply simple simplification rule to a single node
fn apply_single_rule(expr: Expr) -> Expr {
    let mut current = expr;

    // Apply rules in sequence
    current = numeric::apply_numeric_rules(current);
    current = algebraic::apply_algebraic_rules(current);
    current = trig::apply_trig_rules(current);
    current = hyperbolic::apply_hyperbolic_rules(current);
    current = log_exp::apply_log_exp_rules(current);
    current = roots::apply_root_rules(current);

    current
}
