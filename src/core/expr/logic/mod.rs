//! Implementation details for the `expr` module.
//!
//! This folder contains all logic that is internal to the expression subsystem.
//! Nothing here is part of the public API — access goes through `expr::mod.rs`
//! and `expr::api`.

pub(super) mod analysis;
pub(super) mod constructors;
pub(super) mod hash;
pub(super) mod math_methods;
pub(super) mod operators;
pub(super) mod ordering;

// display is pub(in crate::core) so upper modules can wire the Display impl
pub(in crate::core) mod display;
pub(super) mod poly;

// Staircase re-exports — one hop up to api.rs
pub(super) use super::{
    CACHED_NEG_ONE, CACHED_TWO, CACHED_ZERO, EPSILON, EXPR_ONE, Expr, ExprKind, next_id,
};
pub use hash::{compute_expr_hash, compute_term_hash};
pub use math_methods::ArcExprExt;
pub(super) use ordering::expr_cmp;
pub use poly::Polynomial;

#[cfg(test)]
mod tests;
