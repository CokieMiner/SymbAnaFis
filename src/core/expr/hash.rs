//! Structural hashing for expressions.
//!
//! Provides fast structural hash computation for O(1) equality rejection.

use super::ExprKind;
use rustc_hash::FxHasher;
use std::hash::{Hash, Hasher};

/// Compute structural hash for an `ExprKind` (Phase 7b optimization).
/// Unlike `get_term_hash` in helpers.rs (which ignores numeric coefficients for
/// like-term grouping), this hashes ALL content for true structural equality.
#[inline]
pub fn compute_expr_hash(kind: &ExprKind) -> u64 {
    let mut hasher = FxHasher::default();
    kind.hash(&mut hasher);
    hasher.finish()
}
