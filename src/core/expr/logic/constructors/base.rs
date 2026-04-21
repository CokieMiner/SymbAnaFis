//! Base expression constructors and accessors.

use std::mem::replace;
use std::sync::Arc;

use super::{
    CACHED_NEG_ONE, CACHED_TWO, CACHED_ZERO, EXPR_ONE, Expr, ExprKind, Polynomial,
    compute_expr_hash, compute_term_hash, next_id,
};
use crate::core::traits::{is_neg_one, is_one, is_zero};
use crate::core::{InternedSymbol, symb_interned};

impl Expr {
    /// Create a new expression with fresh ID
    #[inline]
    #[must_use]
    pub fn new(kind: ExprKind) -> Self {
        let hash = compute_expr_hash(&kind);
        let term_hash = compute_term_hash(&kind);
        Self {
            id: next_id(),
            hash,
            term_hash,
            kind,
        }
    }

    /// Get the unique ID of the expression
    #[inline]
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Get the structural hash of the expression
    #[inline]
    #[must_use]
    pub const fn structural_hash(&self) -> u64 {
        self.hash
    }

    /// Consume the expression and return its kind
    #[inline]
    #[must_use]
    pub fn into_kind(mut self) -> ExprKind {
        replace(&mut self.kind, ExprKind::Number(0.0))
    }

    // -------------------------------------------------------------------------
    // Accessor methods
    // -------------------------------------------------------------------------

    /// Check if expression is a constant number and return its value
    #[inline]
    #[must_use]
    pub const fn as_number(&self) -> Option<f64> {
        match &self.kind {
            ExprKind::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Check if this expression is the number zero (with tolerance)
    #[inline]
    pub fn is_zero_num(&self) -> bool {
        self.as_number().is_some_and(is_zero)
    }

    /// Check if this expression is the number one (with tolerance)
    #[inline]
    pub fn is_one_num(&self) -> bool {
        self.as_number().is_some_and(is_one)
    }

    /// Check if this expression is the number negative one (with tolerance)
    #[inline]
    pub fn is_neg_one_num(&self) -> bool {
        self.as_number().is_some_and(is_neg_one)
    }

    // -------------------------------------------------------------------------
    // Basic constructors
    // -------------------------------------------------------------------------

    /// Create a number expression
    ///
    /// Optimized: returns clones of cached constants for 0.0 and 1.0
    /// to avoid repeated `hash`/`term_hash` computation in hot paths
    /// (e.g., differentiation returns 0.0 for every constant node).
    #[inline]
    #[must_use]
    pub fn number(n: f64) -> Self {
        // Fast path for the most common values in differentiation and parsing
        if n == 0.0 {
            return Self::clone_cached_with_fresh_id(&CACHED_ZERO);
        }
        let bits = n.to_bits();
        if bits == 1.0_f64.to_bits() {
            return Self::clone_cached_with_fresh_id(&EXPR_ONE);
        }
        if bits == (-1.0_f64).to_bits() {
            return Self::clone_cached_with_fresh_id(&CACHED_NEG_ONE);
        }
        if bits == 2.0_f64.to_bits() {
            return Self::clone_cached_with_fresh_id(&CACHED_TWO);
        }
        Self::new(ExprKind::Number(n))
    }

    #[inline]
    fn clone_cached_with_fresh_id(template: &Self) -> Self {
        Self {
            id: next_id(),
            hash: template.hash,
            term_hash: template.term_hash,
            kind: template.kind.clone(),
        }
    }

    /// Create a symbol expression (auto-interned)
    pub fn symbol(s: impl AsRef<str>) -> Self {
        Self::new(ExprKind::Symbol(symb_interned(s.as_ref())))
    }

    /// Create from an already-interned symbol
    #[inline]
    pub(crate) fn from_interned(interned: InternedSymbol) -> Self {
        Self::new(ExprKind::Symbol(interned))
    }

    /// Create a function call from an already-interned symbol (single argument)
    pub(crate) fn func_symbol(name: InternedSymbol, arg: Self) -> Self {
        Self::new(ExprKind::FunctionCall {
            name,
            args: vec![Arc::new(arg)],
        })
    }

    /// Create a function call from an already-interned symbol (single Arc argument)
    /// Avoids deep cloning the argument if it's already an Arc.
    pub(crate) fn func_symbol_arc(name: InternedSymbol, arg: Arc<Self>) -> Self {
        Self::new(ExprKind::FunctionCall {
            name,
            args: vec![arg],
        })
    }

    /// Create a polynomial expression directly
    #[must_use]
    pub fn poly(p: Polynomial) -> Self {
        // Empty polynomial is 0
        if p.terms().is_empty() {
            return Self::number(0.0);
        }
        // Single constant term (pow=0) is just a number
        if p.terms().len() == 1 && p.terms()[0].0 == 0 {
            return Self::number(p.terms()[0].1);
        }
        Self::new(ExprKind::Poly(p))
    }
}
