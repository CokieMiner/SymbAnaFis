use crate::core::Context;
use crate::core::{Expr, ExprKind, InternedSymbol};
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::Arc;

pub fn expand_user_functions(expr: &Expr, ctx: &Context) -> Expr {
    let mut expanding_fns = FxHashSet::default();
    let mut cache = FxHashMap::default();
    expand_user_functions_impl(
        expr,
        ctx,
        &mut expanding_fns,
        // The 'expanding_fns' parameter was missing in the function signature
        &mut cache,
        0,
    )
    .0
}

/// Expands user-defined functions in an expression tree.
///
/// Returns `(expanded_expr, is_pure)`, where `is_pure` is true if all user functions
/// encountered in the subtree were fully expanded (i.e., no recursion limit or cycle
/// detection was triggered).
fn expand_user_functions_impl(
    expr: &Expr,
    ctx: &Context,
    expanding: &mut FxHashSet<u64>, // Added this parameter
    cache: &mut FxHashMap<u64, (Expr, bool)>,
    depth: usize,
) -> (Expr, bool) {
    const MAX_EXPANSION_DEPTH: usize = 100;

    // We only use the cache for results that were previously "purely" expanded.
    // If a result was partial (skipped an expansion due to a cycle), we never
    // put it in this global cache.
    if let Some((cached_expr, is_pure)) = cache.get(&expr.id) {
        return (cached_expr.clone(), *is_pure);
    }

    if depth > MAX_EXPANSION_DEPTH {
        return (expr.clone(), false);
    }

    if depth == 0 && !ctx.has_expandable_functions() {
        return (expr.clone(), true);
    }

    let mut is_subtree_pure = true;
    let res = match &expr.kind {
        ExprKind::Number(_) | ExprKind::Symbol(_) => expr.clone(),
        ExprKind::Sum(terms) => {
            let (expanded, pure) = expand_children(terms, ctx, expanding, cache, depth); // Pass 'expanding'
            is_subtree_pure &= pure;
            Expr::sum(expanded)
        }
        ExprKind::Product(factors) => {
            let (expanded, pure) = expand_children(factors, ctx, expanding, cache, depth); // Pass 'expanding'
            is_subtree_pure &= pure;
            Expr::product(expanded)
        }
        ExprKind::Div(num, den) => {
            let ((n, d), pure) = expand_pair(num, den, ctx, expanding, cache, depth); // Pass 'expanding'
            is_subtree_pure &= pure;
            Expr::div_expr(n, d)
        }
        ExprKind::Pow(base, exp) => {
            let ((b, e), pure) = expand_pair(base, exp, ctx, expanding, cache, depth); // Pass 'expanding'
            is_subtree_pure &= pure;
            Expr::pow_static(b, e)
        }
        ExprKind::FunctionCall { name, args } => expand_function_call(
            expr,
            name,
            args,
            ctx,
            expanding, // Renamed to expanding_fns in the outer scope
            cache,
            depth,
            &mut is_subtree_pure,
        ),
        ExprKind::Poly(poly) => {
            let (expanded_base, pure) = expand_user_functions_impl(
                poly.base(),
                ctx,
                expanding, // Renamed to expanding_fns in the outer scope
                cache,
                depth + 1,
            );
            is_subtree_pure &= pure;
            if expanded_base == **poly.base() {
                expr.clone()
            } else {
                Expr::poly(poly.with_base(Arc::new(expanded_base)))
            }
        }
        ExprKind::Derivative { inner, var, order } => {
            let (expanded_inner, pure) = expand_user_functions_impl(
                inner,
                ctx,
                expanding, // Renamed to expanding_fns in the outer scope
                cache,
                depth + 1,
            );
            is_subtree_pure &= pure;
            Expr::derivative_interned(expanded_inner, var.clone(), *order)
        }
    };

    cache.insert(expr.id, (res.clone(), is_subtree_pure));
    (res, is_subtree_pure)
}

/// Expands all children of an N-ary node (Sum/Product).
fn expand_children(
    children: &[Arc<Expr>],
    ctx: &Context,
    expanding: &mut FxHashSet<u64>,
    cache: &mut FxHashMap<u64, (Expr, bool)>,
    depth: usize,
) -> (Vec<Expr>, bool) {
    let mut expanded = Vec::with_capacity(children.len());
    let mut all_pure = true;
    for child in children {
        let (exp, pure) = expand_user_functions_impl(child, ctx, expanding, cache, depth + 1);
        expanded.push(exp);
        all_pure &= pure;
    }
    (expanded, all_pure)
}

/// Expands two children of a binary node (Div/Pow).
fn expand_pair(
    lhs: &Expr,
    rhs: &Expr,
    ctx: &Context,
    expanding: &mut FxHashSet<u64>, // Added this parameter
    cache: &mut FxHashMap<u64, (Expr, bool)>,
    depth: usize,
) -> ((Expr, Expr), bool) {
    let (l, p1) = expand_user_functions_impl(lhs, ctx, expanding, cache, depth + 1); // Pass 'expanding'
    let (r, p2) = expand_user_functions_impl(rhs, ctx, expanding, cache, depth + 1); // Pass 'expanding'
    ((l, r), p1 && p2)
}

/// Handles function call expansion including user-defined function body substitution.
#[allow(
    clippy::too_many_arguments,
    reason = "Extracted helper sharing parent's mutable state; a context struct would add unnecessary indirection"
)]
fn expand_function_call(
    expr: &Expr,
    name: &InternedSymbol,
    args: &[Arc<Expr>],
    ctx: &Context,
    expanding: &mut FxHashSet<u64>, // Added this parameter
    cache: &mut FxHashMap<u64, (Expr, bool)>,
    depth: usize,
    is_subtree_pure: &mut bool,
) -> Expr {
    let (expanded_args, args_pure) = expand_children(args, ctx, expanding, cache, depth);
    *is_subtree_pure &= args_pure;

    let fn_id = name.id();
    let args_len = expanded_args.len();
    if !expanding.contains(&fn_id)
        && let Some(user_fn) = ctx.get_user_fn_by_id(fn_id)
        && user_fn.accepts_arity(args_len)
        && let Some(body_fn) = &user_fn.body
    {
        expanding.insert(fn_id);
        let arc_args: Vec<Arc<Expr>> = expanded_args.into_iter().map(Arc::new).collect();
        let body_expr = body_fn(&arc_args);
        let (result, pure) =
            expand_user_functions_impl(&body_expr, ctx, expanding, cache, depth + 1); // Pass 'expanding'
        expanding.remove(&fn_id);

        // Cache the result along with its purity status
        cache.insert(expr.id, (result.clone(), pure && *is_subtree_pure));
        return result;
    }

    // Cycle detected or depth limit — this expansion is "impure".
    *is_subtree_pure = false;
    Expr::func_multi_symbol(name.clone(), expanded_args)
}
