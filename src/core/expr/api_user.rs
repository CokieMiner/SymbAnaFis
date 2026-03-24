//! Public API for end-users of the `expr` module.
//!
//! Defines the core public types:
//! - [`Expr`] — the symbolic expression type
//! - [`ExprKind`] — the AST node variants
//! - [`CustomEvalMap`] — type alias for custom evaluation functions

use rustc_hash::FxHasher;
use std::hash::Hasher;
use std::ops::Deref;
use std::sync::Arc;

use crate::core::symbol::InternedSymbol;

use super::api_crate::DUMMY_ARC;

// ============================================================================
// Type aliases
// ============================================================================

/// Map of custom evaluation functions (name → closure).
pub type CustomEvalMap =
    std::collections::HashMap<String, Arc<dyn Fn(&[f64]) -> Option<f64> + Send + Sync>>;

// ============================================================================
// Expr
// ============================================================================

/// A symbolic mathematical expression.
///
/// ```
/// use symb_anafis::{symb, Expr};
/// let x = symb("x");
/// let expr = x.pow(2.0) + x.sin();  // x² + sin(x)
/// ```
#[derive(Debug, Clone)]
pub struct Expr {
    pub(crate) id: u64,
    /// Structural hash for O(1) equality rejection.
    pub(crate) hash: u64,
    /// Coefficient-insensitive term hash for like-term grouping.
    pub(crate) term_hash: u64,
    pub(crate) kind: ExprKind,
}

impl Deref for Expr {
    type Target = ExprKind;
    fn deref(&self) -> &Self::Target {
        &self.kind
    }
}

impl PartialEq for Expr {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        if self.hash != other.hash {
            return false;
        }
        self.kind == other.kind
    }
}

impl Eq for Expr {}

impl std::hash::Hash for Expr {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

// ============================================================================
// ExprKind
// ============================================================================

/// The kind (structure) of an expression node.
#[derive(Debug, Clone, PartialEq)]
#[allow(
    private_interfaces,
    reason = "InternedSymbol is pub(crate) but exposed here for pattern matching"
)]
pub enum ExprKind {
    /// Constant number
    Number(f64),
    /// Variable or constant symbol
    Symbol(InternedSymbol),
    /// Function call
    FunctionCall {
        /// Function name (interned).
        name: InternedSymbol,
        /// Arguments.
        args: Vec<Arc<Expr>>,
    },
    /// N-ary sum: a + b + c + …
    Sum(Vec<Arc<Expr>>),
    /// N-ary product: a * b * c * …
    Product(Vec<Arc<Expr>>),
    /// Division (binary)
    Div(Arc<Expr>, Arc<Expr>),
    /// Exponentiation (binary)
    Pow(Arc<Expr>, Arc<Expr>),
    /// Symbolic partial derivative ∂^order / ∂var^order
    Derivative {
        /// Expression being differentiated.
        inner: Arc<Expr>,
        /// Differentiation variable (interned).
        var: InternedSymbol,
        /// Order of differentiation.
        order: u32,
    },
    /// Sparse polynomial (efficient for differentiation)
    Poly(crate::core::poly::Polynomial),
}

// ============================================================================
// Drop — iterative to prevent stack overflow on deep trees
// ============================================================================

impl Drop for Expr {
    fn drop(&mut self) {
        fn drain_children(kind: &mut ExprKind, queue: &mut Vec<Arc<Expr>>) {
            match kind {
                ExprKind::FunctionCall { args, .. } => queue.extend(std::mem::take(args)),
                ExprKind::Sum(terms) => queue.extend(std::mem::take(terms)),
                ExprKind::Product(factors) => queue.extend(std::mem::take(factors)),
                ExprKind::Div(left, right) => {
                    let d = Arc::clone(&DUMMY_ARC);
                    queue.push(std::mem::replace(left, Arc::clone(&d)));
                    queue.push(std::mem::replace(right, d));
                }
                ExprKind::Pow(base, exp) => {
                    let d = Arc::clone(&DUMMY_ARC);
                    queue.push(std::mem::replace(base, Arc::clone(&d)));
                    queue.push(std::mem::replace(exp, d));
                }
                ExprKind::Derivative { inner, .. } => {
                    queue.push(std::mem::replace(inner, DUMMY_ARC.clone()));
                }
                ExprKind::Poly(poly) => queue.push(poly.take_base()),
                ExprKind::Number(_) | ExprKind::Symbol(_) => {}
            }
        }

        let mut work_queue = Vec::new();
        drain_children(&mut self.kind, &mut work_queue);
        while let Some(child_arc) = work_queue.pop() {
            if let Ok(mut child_expr) = Arc::try_unwrap(child_arc) {
                drain_children(&mut child_expr.kind, &mut work_queue);
            }
        }
    }
}

// ============================================================================
// Hash for ExprKind
// ============================================================================

impl std::hash::Hash for ExprKind {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Number(n) => {
                let normalized = if *n == 0.0 { 0.0 } else { *n };
                normalized.to_bits().hash(state);
            }
            Self::Symbol(s) => s.hash(state),
            Self::FunctionCall { name, args } => {
                name.hash(state);
                args.hash(state);
            }
            Self::Sum(terms) => {
                let mut sum_hash: u64 = 0;
                for t in terms {
                    sum_hash = sum_hash.wrapping_add(t.hash);
                }
                sum_hash.hash(state);
            }
            Self::Product(factors) => {
                let mut prod_hash: u64 = 0;
                for f in factors {
                    prod_hash = prod_hash.wrapping_add(f.hash);
                }
                prod_hash.hash(state);
            }
            Self::Div(l, r) | Self::Pow(l, r) => {
                l.hash(state);
                r.hash(state);
            }
            Self::Derivative { inner, var, order } => {
                inner.hash(state);
                var.hash(state);
                order.hash(state);
            }
            Self::Poly(poly) => {
                poly.base().hash.hash(state);
                let mut terms_hash: u64 = 0;
                for &(pow, coeff) in poly.terms() {
                    let mut term_hasher = FxHasher::default();
                    coeff.to_bits().hash(&mut term_hasher);
                    pow.hash(&mut term_hasher);
                    terms_hash = terms_hash.wrapping_add(term_hasher.finish());
                }
                terms_hash.hash(state);
            }
        }
    }
}
