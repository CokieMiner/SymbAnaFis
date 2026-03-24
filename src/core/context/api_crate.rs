//! Crate-internal API for the `context` module.
//!
//! `BodyFn` and `PartialFn` are the core function-pointer types used by the
//! simplification engine, evaluator, and diff subsystems internally.
//! They are also exposed publicly so users can construct `UserFunction` closures.

use std::sync::Arc;

use crate::Expr;

/// Thread-safe symbolic body function.
/// Takes argument expressions and returns the function body as an `Expr`.
pub type BodyFn = Arc<dyn Fn(&[Arc<Expr>]) -> Expr + Send + Sync>;

/// Thread-safe partial derivative function.
/// Takes argument expressions and returns the partial derivative as an `Expr`.
pub type PartialFn = Arc<dyn Fn(&[Arc<Expr>]) -> Expr + Send + Sync>;
