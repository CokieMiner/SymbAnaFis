//! Function and calculus node constructors.

use std::sync::Arc;

use super::super::super::{Expr, ExprKind};
use crate::core::symbol::{InternedSymbol, symb_interned};

impl Expr {
    /// Create a function call expression (single argument)
    pub fn func(name: impl AsRef<str>, content: impl Into<Self>) -> Self {
        Self::new(ExprKind::FunctionCall {
            name: symb_interned(name.as_ref()),
            args: vec![Arc::new(content.into())],
        })
    }

    /// Create a multi-argument function call
    pub fn func_multi(name: impl AsRef<str>, args: Vec<Self>) -> Self {
        Self::new(ExprKind::FunctionCall {
            name: symb_interned(name.as_ref()),
            args: args.into_iter().map(Arc::new).collect(),
        })
    }

    /// Create a multi-argument function call using `InternedSymbol`
    pub(crate) fn func_multi_symbol(name: InternedSymbol, args: Vec<Self>) -> Self {
        Self::new(ExprKind::FunctionCall {
            name,
            args: args.into_iter().map(Arc::new).collect(),
        })
    }

    /// Create a function call from Arc arguments
    pub fn func_multi_from_arcs(name: impl AsRef<str>, args: Vec<Arc<Self>>) -> Self {
        Self::new(ExprKind::FunctionCall {
            name: symb_interned(name.as_ref()),
            args,
        })
    }

    /// Create a function call from Arc arguments using `InternedSymbol`
    pub(crate) fn func_multi_from_arcs_symbol(name: InternedSymbol, args: Vec<Arc<Self>>) -> Self {
        Self::new(ExprKind::FunctionCall { name, args })
    }

    /// Create a function call with explicit arguments using array syntax
    pub fn call<const N: usize>(name: impl AsRef<str>, args: [Self; N]) -> Self {
        Self::func_multi(name, args.into())
    }

    /// Create a partial derivative expression
    pub fn derivative(inner: Self, var: impl AsRef<str>, order: u32) -> Self {
        Self::new(ExprKind::Derivative {
            inner: Arc::new(inner),
            var: symb_interned(var.as_ref()),
            order,
        })
    }

    /// Create a partial derivative expression with an already-interned symbol
    pub(crate) fn derivative_interned(inner: Self, var: InternedSymbol, order: u32) -> Self {
        Self::new(ExprKind::Derivative {
            inner: Arc::new(inner),
            var,
            order,
        })
    }

    // -------------------------------------------------------------------------
    // Negation helper
    // -------------------------------------------------------------------------

    /// Negate this expression: -x = Product([-1, x])
    #[must_use]
    pub fn negate(self) -> Self {
        Self::product(vec![Self::number(-1.0), self])
    }
}
