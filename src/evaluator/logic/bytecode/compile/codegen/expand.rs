use crate::core::Context;
use crate::core::{Expr, ExprKind};
use rustc_hash::FxHashSet;
use std::sync::Arc;

pub fn expand_user_functions(expr: &Expr, ctx: &Context) -> Expr {
    let mut expanding = FxHashSet::default();
    expand_user_functions_impl(expr, ctx, &mut expanding, 0)
}

fn expand_user_functions_impl(
    expr: &Expr,
    ctx: &Context,
    expanding: &mut FxHashSet<u64>,
    depth: usize,
) -> Expr {
    const MAX_EXPANSION_DEPTH: usize = 100;

    if depth > MAX_EXPANSION_DEPTH {
        return expr.clone();
    }

    if depth == 0 && !ctx.has_expandable_functions() {
        return expr.clone();
    }

    match &expr.kind {
        ExprKind::Number(_) | ExprKind::Symbol(_) => expr.clone(),
        ExprKind::Sum(terms) => {
            let expanded: Vec<Expr> = terms
                .iter()
                .map(|t| expand_user_functions_impl(t, ctx, expanding, depth + 1))
                .collect();
            Expr::sum(expanded)
        }
        ExprKind::Product(factors) => {
            let expanded: Vec<Expr> = factors
                .iter()
                .map(|f| expand_user_functions_impl(f, ctx, expanding, depth + 1))
                .collect();
            Expr::product(expanded)
        }
        ExprKind::Div(num, den) => {
            let num_exp = expand_user_functions_impl(num, ctx, expanding, depth + 1);
            let den_exp = expand_user_functions_impl(den, ctx, expanding, depth + 1);
            Expr::div_expr(num_exp, den_exp)
        }
        ExprKind::Pow(base, exp) => {
            let base_exp = expand_user_functions_impl(base, ctx, expanding, depth + 1);
            let exp_exp = expand_user_functions_impl(exp, ctx, expanding, depth + 1);
            Expr::pow_static(base_exp, exp_exp)
        }
        ExprKind::FunctionCall { name, args } => {
            let expanded_args: Vec<Expr> = args
                .iter()
                .map(|a| expand_user_functions_impl(a, ctx, expanding, depth + 1))
                .collect();

            let fn_id = name.id();
            if !expanding.contains(&fn_id)
                && let Some(user_fn) = ctx.get_user_fn_by_id(fn_id)
                && user_fn.accepts_arity(expanded_args.len())
                && let Some(body_fn) = &user_fn.body
            {
                expanding.insert(fn_id);
                let arc_args: Vec<Arc<Expr>> =
                    expanded_args.iter().map(|a| Arc::new(a.clone())).collect();
                let body_expr = body_fn(&arc_args);
                let result = expand_user_functions_impl(&body_expr, ctx, expanding, depth + 1);
                expanding.remove(&fn_id);
                return result;
            }

            Expr::func_multi_symbol(name.clone(), expanded_args)
        }
        ExprKind::Poly(poly) => {
            let expanded_base = expand_user_functions_impl(poly.base(), ctx, expanding, depth + 1);

            if expanded_base == **poly.base() {
                expr.clone()
            } else {
                Expr::poly(poly.with_base(Arc::new(expanded_base)))
            }
        }
        ExprKind::Derivative { inner, var, order } => {
            let expanded_inner = expand_user_functions_impl(inner, ctx, expanding, depth + 1);
            Expr::derivative_interned(expanded_inner, var.clone(), *order)
        }
    }
}
