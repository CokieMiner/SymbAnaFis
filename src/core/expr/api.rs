//! Unified API for the `expr` module.
//!
//! Handled at the top `core` level for user-vs-crate visibility.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::mem::{discriminant, replace, take};
use std::ops::Deref;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};

use rustc_hash::FxHasher;

pub use super::logic::ArcExprExt;
pub use super::logic::Polynomial;
pub use super::logic::{compute_expr_hash, compute_term_hash};
pub use crate::EPSILON;
use crate::core::InternedSymbol;

// ============================================================================
// Type aliases
// ============================================================================

/// Map of custom evaluation functions (name → closure).
pub type CustomEvalMap = HashMap<String, Arc<dyn Fn(&[f64]) -> Option<f64> + Send + Sync>>;

// ============================================================================
// Expression ID counter
// ============================================================================

static EXPR_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate the next unique expression ID.
pub fn next_id() -> u64 {
    EXPR_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

// ============================================================================
// Expr
// ============================================================================

/// A symbolic mathematical expression.
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

impl Hash for Expr {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
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
    Poly(Polynomial),
}

// ============================================================================
// Drop — iterative to prevent stack overflow on deep trees
// ============================================================================

impl Drop for Expr {
    fn drop(&mut self) {
        fn drain_children(kind: &mut ExprKind, queue: &mut Vec<Arc<Expr>>) {
            match kind {
                ExprKind::FunctionCall { args, .. } => queue.extend(take(args)),
                ExprKind::Sum(terms) => queue.extend(take(terms)),
                ExprKind::Product(factors) => queue.extend(take(factors)),
                ExprKind::Div(left, right) => {
                    let d = Arc::clone(&DUMMY_ARC);
                    queue.push(replace(left, Arc::clone(&d)));
                    queue.push(replace(right, d));
                }
                ExprKind::Pow(base, exp) => {
                    let d = Arc::clone(&DUMMY_ARC);
                    queue.push(replace(base, Arc::clone(&d)));
                    queue.push(replace(exp, d));
                }
                ExprKind::Derivative { inner, .. } => {
                    queue.push(replace(inner, DUMMY_ARC.clone()));
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

impl Hash for ExprKind {
    fn hash<H: Hasher>(&self, state: &mut H) {
        discriminant(self).hash(state);
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

// ============================================================================
// Cached constants (from api_crate.rs)
// ============================================================================

pub(super) static EXPR_ONE: LazyLock<Expr> = LazyLock::new(|| {
    let kind = ExprKind::Number(1.0);
    Expr {
        id: 0,
        hash: compute_expr_hash(&kind),
        term_hash: compute_term_hash(&kind),
        kind,
    }
});

pub(super) static CACHED_ZERO: LazyLock<Expr> = LazyLock::new(|| {
    let kind = ExprKind::Number(0.0);
    Expr {
        id: 0,
        hash: compute_expr_hash(&kind),
        term_hash: compute_term_hash(&kind),
        kind,
    }
});

pub(super) static CACHED_NEG_ONE: LazyLock<Expr> = LazyLock::new(|| {
    let kind = ExprKind::Number(-1.0);
    Expr {
        id: 0,
        hash: compute_expr_hash(&kind),
        term_hash: compute_term_hash(&kind),
        kind,
    }
});

pub(super) static CACHED_TWO: LazyLock<Expr> = LazyLock::new(|| {
    let kind = ExprKind::Number(2.0);
    Expr {
        id: 0,
        hash: compute_expr_hash(&kind),
        term_hash: compute_term_hash(&kind),
        kind,
    }
});

/// Dummy Arc used during iterative Drop.
pub(super) static DUMMY_ARC: LazyLock<Arc<Expr>> = LazyLock::new(|| {
    let kind = ExprKind::Number(0.0);
    Arc::new(Expr {
        id: 0,
        hash: compute_expr_hash(&kind),
        term_hash: compute_term_hash(&kind),
        kind,
    })
});

fn make_arc_number(n: f64) -> Arc<Expr> {
    let kind = ExprKind::Number(n);
    Arc::new(Expr {
        id: 0,
        hash: compute_expr_hash(&kind),
        term_hash: compute_term_hash(&kind),
        kind,
    })
}

static ARC_ZERO: LazyLock<Arc<Expr>> = LazyLock::new(|| make_arc_number(0.0));
static ARC_ONE: LazyLock<Arc<Expr>> = LazyLock::new(|| make_arc_number(1.0));
static ARC_NEG_ONE: LazyLock<Arc<Expr>> = LazyLock::new(|| make_arc_number(-1.0));
static ARC_TWO: LazyLock<Arc<Expr>> = LazyLock::new(|| make_arc_number(2.0));

/// Return a cached `Arc<Expr>` for common constants (0, 1, -1, 2).
#[inline]
pub fn arc_number(n: f64) -> Arc<Expr> {
    if n.to_bits() == 1.0_f64.to_bits() {
        return Arc::clone(&*ARC_ONE);
    }
    if n.to_bits() == (-1.0_f64).to_bits() {
        return Arc::clone(&*ARC_NEG_ONE);
    }
    if n.to_bits() == 2.0_f64.to_bits() {
        return Arc::clone(&*ARC_TWO);
    }
    if n == 0.0 {
        return Arc::clone(&*ARC_ZERO);
    }
    Arc::new(Expr::number(n))
}
