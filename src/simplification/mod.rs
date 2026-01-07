//! Simplification framework - reduces expressions
pub mod engine;
pub mod helpers;
mod patterns;
mod rules;

use crate::Expr;
use crate::core::unified_context::{BodyFn, Context};

use std::collections::{HashMap, HashSet};

/// Type alias for custom body function map (symbolic expansion)
pub type CustomBodyMap = HashMap<String, BodyFn>;

/// Simplify an expression with user-specified options
///
/// # Arguments
/// - `expr`: The expression to simplify
/// - `known_symbols`: known symbol names (passed through for simplifier context)
/// - `custom_bodies`: custom function body definitions for symbolic expansion
/// - `max_depth`: maximum tree depth during simplification (None = use default 50)
/// - `max_iterations`: maximum simplification iterations (None = use default 1000)
/// - `context`: optional unified Context (merges its function bodies)
/// - `domain_safe`: if true, avoids transformations that can change expression domain
pub fn simplify_expr(
    expr: Expr,
    known_symbols: HashSet<String>,
    mut custom_bodies: CustomBodyMap,
    max_depth: Option<usize>,
    max_iterations: Option<usize>,
    context: Option<&Context>,
    domain_safe: bool,
) -> Expr {
    // Merge Context's values if provided
    if let Some(ctx) = context {
        // Context symbols are parsing hints, not simplification constants
        // But we still merge function bodies
        for name in ctx.function_names() {
            if let Some(body) = ctx.get_body(&name) {
                custom_bodies.insert(name, body.clone());
            }
        }
    }

    let variables = expr.variables();

    let mut simplifier = engine::Simplifier::new()
        .with_domain_safe(domain_safe)
        .with_variables(variables)
        .with_known_symbols(known_symbols)
        .with_custom_bodies(custom_bodies);

    if let Some(depth) = max_depth {
        simplifier = simplifier.with_max_depth(depth);
    }
    if let Some(iters) = max_iterations {
        simplifier = simplifier.with_max_iterations(iters);
    }

    let mut current = simplifier.simplify(expr);
    current = helpers::prettify_roots(current);
    current
}
