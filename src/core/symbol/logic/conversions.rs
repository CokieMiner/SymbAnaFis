//! Type conversions for Expressions and Symbols.

use std::sync::Arc;

use crate::core::Expr;
use crate::core::Symbol;

impl From<Symbol> for Expr {
    fn from(s: Symbol) -> Self {
        s.to_expr()
    }
}

impl From<f64> for Expr {
    fn from(n: f64) -> Self {
        Self::number(n)
    }
}

impl From<i32> for Expr {
    fn from(n: i32) -> Self {
        Self::number(f64::from(n))
    }
}

impl From<Arc<Self>> for Expr {
    fn from(arc: Arc<Self>) -> Self {
        Arc::try_unwrap(arc).unwrap_or_else(|arc| (*arc).clone())
    }
}

impl From<&Arc<Self>> for Expr {
    fn from(arc: &Arc<Self>) -> Self {
        (**arc).clone()
    }
}
