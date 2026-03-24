//! Crate-internal API for the `expr` module.
//!
//! These items are used by other modules within this crate (diff, simplification,
//! evaluator, etc.) but are **not** part of the public library surface.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use super::api_user::{Expr, ExprKind};

pub use super::logic::hash::{compute_expr_hash, compute_term_hash};
pub use crate::EPSILON;

// ============================================================================
// Expression ID counter
// ============================================================================

static EXPR_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate the next unique expression ID.
pub fn next_id() -> u64 {
    EXPR_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

// ============================================================================
// Cached Expr constants (avoid allocation on hot paths)
// ============================================================================

pub(super) static EXPR_ONE: std::sync::LazyLock<Expr> = std::sync::LazyLock::new(|| {
    let kind = ExprKind::Number(1.0);
    Expr {
        id: 0,
        hash: compute_expr_hash(&kind),
        term_hash: compute_term_hash(&kind),
        kind,
    }
});

pub(super) static CACHED_ZERO: std::sync::LazyLock<Expr> = std::sync::LazyLock::new(|| {
    let kind = ExprKind::Number(0.0);
    Expr {
        id: 0,
        hash: compute_expr_hash(&kind),
        term_hash: compute_term_hash(&kind),
        kind,
    }
});

pub(super) static CACHED_NEG_ONE: std::sync::LazyLock<Expr> = std::sync::LazyLock::new(|| {
    let kind = ExprKind::Number(-1.0);
    Expr {
        id: 0,
        hash: compute_expr_hash(&kind),
        term_hash: compute_term_hash(&kind),
        kind,
    }
});

pub(super) static CACHED_TWO: std::sync::LazyLock<Expr> = std::sync::LazyLock::new(|| {
    let kind = ExprKind::Number(2.0);
    Expr {
        id: 0,
        hash: compute_expr_hash(&kind),
        term_hash: compute_term_hash(&kind),
        kind,
    }
});

/// Dummy Arc used during iterative Drop to swap out children without allocation.
pub(super) static DUMMY_ARC: std::sync::LazyLock<Arc<Expr>> = std::sync::LazyLock::new(|| {
    let kind = ExprKind::Number(0.0);
    Arc::new(Expr {
        id: 0,
        hash: compute_expr_hash(&kind),
        term_hash: compute_term_hash(&kind),
        kind,
    })
});

// ============================================================================
// Cached Arc<Expr> constants — O(1) refcount bump instead of allocation
// ============================================================================

fn make_arc_number(n: f64) -> Arc<Expr> {
    let kind = ExprKind::Number(n);
    Arc::new(Expr {
        id: 0,
        hash: compute_expr_hash(&kind),
        term_hash: compute_term_hash(&kind),
        kind,
    })
}

static ARC_ZERO: std::sync::LazyLock<Arc<Expr>> = std::sync::LazyLock::new(|| make_arc_number(0.0));
static ARC_ONE: std::sync::LazyLock<Arc<Expr>> = std::sync::LazyLock::new(|| make_arc_number(1.0));
static ARC_NEG_ONE: std::sync::LazyLock<Arc<Expr>> =
    std::sync::LazyLock::new(|| make_arc_number(-1.0));
static ARC_TWO: std::sync::LazyLock<Arc<Expr>> = std::sync::LazyLock::new(|| make_arc_number(2.0));

/// Return a cached `Arc<Expr>` for common constants (0, 1, -1, 2).
/// Falls back to allocation for other values.
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
