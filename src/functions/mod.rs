//! Centralized mathematical function registry
//!
//! This module provides a single source of truth for all mathematical functions,
//! including their derivative formulas.

use crate::{Expr, ExprKind};

pub(crate) mod definitions;
pub(crate) mod registry;

// ===== Helper functions for building derivative expressions =====

/// Create a function call expression
pub(crate) fn func(name: &str, arg: Expr) -> Expr {
    Expr::func(name, arg)
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

/// Negate an expression
pub(crate) fn neg(e: Expr) -> Expr {
    Expr::mul_expr(Expr::number(-1.0), e)
}
