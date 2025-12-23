//! Centralized mathematical function registry
//!
//! This module provides a single source of truth for all mathematical functions,
//! including their derivative formulas.

use crate::{Expr, ExprKind};
use std::sync::Arc;

pub mod context;
pub(crate) mod definitions;
pub mod registry;

pub use self::context::FunctionContext;
pub use registry::FunctionDefinition;

// ===== Helper functions for building derivative expressions =====

/// Create a function call expression from Expr
#[inline]
pub(crate) fn func(name: &str, arg: Expr) -> Expr {
    Expr::func(name, arg)
}

/// Create a function call expression from Arc<Expr> - cheap, avoids deep clone
#[inline]
pub(crate) fn func_arc(name: &str, arg: Arc<Expr>) -> Expr {
    Expr::func_multi_from_arcs(name, vec![arg])
}

/// Multiply, optimizing for common cases (0 and 1)
pub(crate) fn mul_opt(a: Expr, b: Expr) -> Expr {
    match (&a.kind, &b.kind) {
        (ExprKind::Number(x), _) if *x == 0.0 => Expr::number(0.0),
        (_, ExprKind::Number(x)) if *x == 0.0 => Expr::number(0.0),
        (ExprKind::Number(x), _) if *x == 1.0 => b,
        (_, ExprKind::Number(x)) if *x == 1.0 => a,
        _ => Expr::mul_expr(a, b),
    }
}

/// Multiply Arc<Expr> and Expr, optimizing for common cases (0 and 1) without deep cloning if possible
pub(crate) fn mul_opt_arc(a: Arc<Expr>, b: Expr) -> Expr {
    // Check 'a' (Arc) for 0 or 1
    if let ExprKind::Number(x) = a.kind {
        if x == 0.0 {
            return Expr::number(0.0);
        }
        if x == 1.0 {
            return b;
        }
    }
    // Check 'b' (Expr) for 0 or 1
    if let ExprKind::Number(x) = b.kind {
        if x == 0.0 {
            return Expr::number(0.0);
        }
        if x == 1.0 {
            // Return 'a', cloning underlying Expr if needed
            return match Arc::try_unwrap(a) {
                Ok(expr) => expr,
                Err(arc) => (*arc).clone(),
            };
        }
    }
    // Fallback: create product with Arc (cheap)
    Expr::mul_from_arcs(vec![a, Arc::new(b)])
}

/// Negate an expression
#[inline]
pub(crate) fn neg(e: Expr) -> Expr {
    Expr::mul_expr(Expr::number(-1.0), e)
}
