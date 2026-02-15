#![allow(
    clippy::print_stdout,
    clippy::panic,
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::uninlined_format_args,
    clippy::cast_lossless,
    clippy::manual_assert,
    clippy::redundant_clone,
    clippy::match_wildcard_for_single_variants,
    clippy::cast_possible_wrap,
    clippy::expect_used,
    reason = "Testing and fuzzing utilities require direct output and panic assertions"
)]

use crate::core::expr::ExprKind;
use crate::core::symbol::symb;
use crate::{Expr, Simplify};
use rand::{RngExt, SeedableRng, rngs::StdRng};
use std::collections::HashMap;

fn random_std_rng_with_seed() -> (StdRng, u64) {
    let seed: u64 = rand::random();
    (StdRng::seed_from_u64(seed), seed)
}

/// Generate a massive polynomial-like expression
///
/// Structure: Sum of many Products, where each Product is a mix of variables and constants.
/// This mimics the structure of f13 benchmarks.
fn generate_massive_poly(rng: &mut StdRng, num_terms: usize, num_vars: usize) -> Expr {
    let vars: Vec<String> = (0..num_vars).map(|i| format!("x{i}")).collect();
    let mut terms = Vec::with_capacity(num_terms);

    for _ in 0..num_terms {
        // Create a product term: c * x_i * x_j * ...
        let num_factors = rng.random_range(1..=5);
        let mut factors = Vec::with_capacity(num_factors + 1);

        // Coefficient (small integer to avoid float precision issues hiding logic bugs)
        let coeff = rng.random_range(-10..=10);
        if coeff != 0 {
            factors.push(Expr::number(f64::from(coeff)));
        }

        for _ in 0..num_factors {
            let var = &vars[rng.random_range(0..num_vars)];
            // symb returns Symbol, convert to Expr
            factors.push(symb(var).into());
        }

        if factors.is_empty() {
            factors.push(Expr::number(1.0)); // Should not happen with range 1..=5 but safe
        }

        terms.push(Expr::product(factors));
    }

    Expr::sum(terms)
}

/// Evaluation context with predictable values
fn make_eval_context(num_vars: usize) -> HashMap<String, f64> {
    (0..num_vars)
        .map(|i| (format!("x{i}"), (i + 1) as f64)) // 1, 2, 3... to avoid 0 cancellations
        .collect()
}

#[test]
fn fuzz_massive_polynomial_simplification() {
    for _ in 0..1000 {
        let (mut rng, seed) = random_std_rng_with_seed();

        // Progressive testing: start small, go big
        let cases = [
            (100, 5), // Small
            (1_000, 10), // Medium
                      //(5_000, 20), // Large
        ];

        for (num_terms, num_vars) in cases {
            let expr = generate_massive_poly(&mut rng, num_terms, num_vars);
            let context_map = make_eval_context(num_vars);
            let raw_val = eval_tree_manual(&expr, &context_map);

            let simplified = Simplify::new()
                .simplify(&expr)
                .expect("Simplification failed");

            let simp_val = eval_tree_manual(&simplified, &context_map);
            let diff = (raw_val - simp_val).abs();
            let tolerance = 1e-4 * (num_terms as f64).max(1.0);

            assert!(
                diff <= tolerance,
                "Mismatch! Seed: {seed}, Terms: {num_terms}, Vars: {num_vars}\nRaw: {raw_val}\nSimp: {simp_val}\nDiff: {diff}\nExpression: {expr}\nSimplified: {simplified}"
            );
        }
    }
}

#[test]
fn fuzz_cancellation_patterns() {
    for _ in 0..1000 {
        let (mut rng, seed) = random_std_rng_with_seed();

        let p = generate_massive_poly(&mut rng, 500, 10);
        let q = generate_massive_poly(&mut rng, 10, 5);
        let p_neg = p.clone().negate();
        let expr = Expr::sum(vec![p, q.clone(), p_neg]);

        let context_map = make_eval_context(10);
        let expected = eval_tree_manual(&q, &context_map);

        let simplified = Simplify::new()
            .simplify(&expr)
            .expect("Simplification failed");
        let actual = eval_tree_manual(&simplified, &context_map);

        let diff = (expected - actual).abs();

        assert!(
            diff <= 1e-5,
            "Cancellation failed! Seed: {seed}, Expected (approx): {expected}, Got: {actual}, Diff: {diff}\nOriginal: {expr}\nSimplified: {simplified}"
        );
    }
}

// Simple tree evaluator to serve as oracle
fn eval_tree_manual(expr: &Expr, vars: &HashMap<String, f64>) -> f64 {
    match &expr.kind {
        ExprKind::Number(n) => *n,
        ExprKind::Symbol(s) => *vars.get(s.as_str()).unwrap_or(&0.0),
        ExprKind::Sum(terms) => terms.iter().map(|t| eval_tree_manual(t, vars)).sum(),
        ExprKind::Product(factors) => factors.iter().map(|t| eval_tree_manual(t, vars)).product(),
        ExprKind::Div(n, d) => eval_tree_manual(n, vars) / eval_tree_manual(d, vars),
        ExprKind::Pow(b, e) => eval_tree_manual(b, vars).powf(eval_tree_manual(e, vars)),
        ExprKind::FunctionCall { name, args } => {
            let arg_vals: Vec<f64> = args.iter().map(|a| eval_tree_manual(a, vars)).collect();
            match name.as_str() {
                "sin" => arg_vals[0].sin(),
                "cos" => arg_vals[0].cos(),
                "exp" => arg_vals[0].exp(),
                _ => 1.0, // Mock others
            }
        }
        ExprKind::Poly(p) => {
            // Evaluate polynomial manually
            let base_val = eval_tree_manual(p.base(), vars);
            let mut sum = 0.0;
            for (exp, coeff) in p.terms() {
                sum += coeff * base_val.powi((*exp).cast_signed());
            }
            sum
        }
        ExprKind::Derivative { .. } => 0.0,
    }
}
