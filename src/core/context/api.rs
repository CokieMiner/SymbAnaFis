//! Unified API for the `context` module.
//!
//! Handled at the top `core` level for user-vs-crate visibility.

use crate::core::Expr;
use std::sync::Arc;

pub use super::logic::{Context, UserFunction};

/// Thread-safe symbolic body function.
/// Takes argument expressions and returns the function body as an `Expr`.
pub type BodyFn = Arc<dyn Fn(&[Arc<Expr>]) -> Expr + Send + Sync>;

/// Thread-safe partial derivative function.
/// Takes argument expressions and returns the partial derivative as an `Expr`.
pub type PartialFn = Arc<dyn Fn(&[Arc<Expr>]) -> Expr + Send + Sync>;
