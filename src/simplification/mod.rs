//! Simplification framework - reduces expressions
pub mod engine;
pub mod helpers;
mod patterns;
pub mod rules;

use crate::Expr;
use crate::core::unified_context::{BodyFn, Context};

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Type alias for custom body function map (symbolic expansion)
pub type CustomBodyMap = HashMap<u64, BodyFn>;

/// Simplify an expression with user-specified options
///
/// # Arguments
/// - `expr`: The expression to simplify
/// - `_known_symbols`: (Deprecated/Unused) Previously used for parsing hints.
///   Constants like 'e' are now always handled consistently via UIDs.
/// - `custom_bodies`: Map of custom function bodies for numeric evaluation
/// - `max_depth`: maximum tree depth during simplification (None = use default 50)
/// - `max_iterations`: maximum simplification iterations (None = use default 1000)
/// - `context`: optional unified Context (merges its function bodies)
/// - `domain_safe`: if true, avoids transformations that can change expression domain
pub fn simplify_expr(
    expr: Expr,
    _known_symbols: HashSet<String>,
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
        for id in ctx.fn_name_to_id().values() {
            if let Some(body) = ctx.get_body_by_id(*id) {
                custom_bodies.insert(*id, Arc::clone(body));
            }
        }
    }

    let mut simplifier = engine::Simplifier::new()
        .with_domain_safe(domain_safe)
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
