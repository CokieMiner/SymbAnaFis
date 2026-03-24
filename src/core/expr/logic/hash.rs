//! Structural and term hashing for expressions.
//!
//! Provides fast structural hash computation for O(1) equality rejection and
//! coefficient-insensitive term hashing used for like-term grouping.

use super::super::ExprKind;
use rustc_hash::FxHasher;
use std::hash::{Hash, Hasher};

/// Compute structural hash for an `ExprKind` (Phase 7b optimization).
/// Unlike `compute_term_hash` in helpers.rs (which ignores numeric coefficients for
/// like-term grouping), this hashes ALL content for true structural equality.
#[inline]
pub fn compute_expr_hash(kind: &ExprKind) -> u64 {
    let mut hasher = FxHasher::default();
    kind.hash(&mut hasher);
    hasher.finish()
}

const FNV_TERM_OFFSET: u64 = 14_695_981_039_346_656_037;
const FNV_TERM_PRIME: u64 = 1_099_511_628_211;

#[inline]
fn term_hash_u64(mut hash: u64, n: u64) -> u64 {
    for byte in n.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_TERM_PRIME);
    }
    hash
}

#[inline]
fn term_hash_f64(hash: u64, n: f64) -> u64 {
    term_hash_u64(hash, n.to_bits())
}

#[inline]
const fn term_hash_byte(mut hash: u64, b: u8) -> u64 {
    hash ^= b as u64;
    hash.wrapping_mul(FNV_TERM_PRIME)
}

fn hash_term_inner(hash: u64, kind: &ExprKind) -> u64 {
    match kind {
        ExprKind::Number(n) => term_hash_f64(term_hash_byte(hash, b'N'), *n),
        ExprKind::Symbol(s) => term_hash_u64(term_hash_byte(hash, b'S'), s.id()),
        ExprKind::Product(factors) => {
            let h = term_hash_byte(hash, b'P');
            let mut acc: u64 = 0;
            for f in factors {
                if !matches!(f.kind, ExprKind::Number(_)) {
                    acc = acc.wrapping_add(hash_term_inner(FNV_TERM_OFFSET, &f.kind));
                }
            }
            term_hash_u64(h, acc)
        }
        ExprKind::Pow(base, exp) => {
            let h = term_hash_byte(hash, b'^');
            let h = hash_term_inner(h, &base.kind);
            match &exp.kind {
                ExprKind::Number(n) => term_hash_f64(h, *n),
                ek => hash_term_inner(h, ek),
            }
        }
        ExprKind::FunctionCall { name, args } => {
            let h = term_hash_byte(hash, b'F');
            let h = term_hash_u64(h, name.id());
            args.iter().fold(h, |acc, a| hash_term_inner(acc, &a.kind))
        }
        ExprKind::Sum(terms) => {
            let h = term_hash_byte(hash, b'+');
            let mut acc: u64 = 0;
            for t in terms {
                acc = acc.wrapping_add(hash_term_inner(FNV_TERM_OFFSET, &t.kind));
            }
            term_hash_u64(h, acc)
        }
        ExprKind::Div(num, den) => {
            let h = term_hash_byte(hash, b'/');
            let h = hash_term_inner(h, &num.kind);
            hash_term_inner(h, &den.kind)
        }
        ExprKind::Derivative { inner, var, order } => {
            let h = term_hash_byte(hash, b'D');
            let h = term_hash_u64(h, var.id());
            let h = term_hash_u64(h, u64::from(*order));
            hash_term_inner(h, &inner.kind)
        }
        ExprKind::Poly(poly) => {
            let h = term_hash_byte(hash, b'Y');
            let h = hash_term_inner(h, &poly.base().kind);
            let mut acc: u64 = 0;
            for &(pow, coeff) in poly.terms() {
                let th = term_hash_u64(term_hash_f64(FNV_TERM_OFFSET, coeff), u64::from(pow));
                acc = acc.wrapping_add(th);
            }
            term_hash_u64(h, acc)
        }
    }
}

/// Compute the coefficient-insensitive term hash from an `ExprKind`.
#[inline]
pub fn compute_term_hash(kind: &ExprKind) -> u64 {
    match kind {
        ExprKind::Number(n) => term_hash_f64(term_hash_byte(FNV_TERM_OFFSET, b'N'), *n),
        ExprKind::Symbol(s) => term_hash_u64(term_hash_byte(FNV_TERM_OFFSET, b'S'), s.id()),
        ExprKind::Sum(terms) => {
            let h = term_hash_byte(FNV_TERM_OFFSET, b'+');
            let mut acc: u64 = 0;
            for t in terms {
                acc = acc.wrapping_add(t.term_hash);
            }
            term_hash_u64(h, acc)
        }
        ExprKind::Product(factors) => {
            let h = term_hash_byte(FNV_TERM_OFFSET, b'P');
            let mut acc: u64 = 0;
            for f in factors {
                if !matches!(f.kind, ExprKind::Number(_)) {
                    acc = acc.wrapping_add(f.term_hash);
                }
            }
            term_hash_u64(h, acc)
        }
        _ => hash_term_inner(FNV_TERM_OFFSET, kind),
    }
}
