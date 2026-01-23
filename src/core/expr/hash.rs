//! Structural hashing for expressions.
//!
//! Provides fast structural hash computation for O(1) equality rejection.

use super::ExprKind;

/// Compute structural hash for an `ExprKind` (Phase 7b optimization).
/// Unlike `get_term_hash` in helpers.rs (which ignores numeric coefficients for
/// like-term grouping), this hashes ALL content for true structural equality.
pub fn compute_expr_hash(kind: &ExprKind) -> u64 {
    // FNV-1a constants
    const FNV_OFFSET: u64 = 14_695_981_039_346_656_037;
    const FNV_PRIME: u64 = 1_099_511_628_211;

    #[inline]
    fn hash_u64(mut hash: u64, n: u64) -> u64 {
        for byte in n.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }

    #[inline]
    fn hash_f64(hash: u64, n: f64) -> u64 {
        hash_u64(hash, n.to_bits())
    }

    #[inline]
    const fn hash_byte(mut hash: u64, b: u8) -> u64 {
        hash ^= b as u64;
        hash.wrapping_mul(FNV_PRIME)
    }

    fn hash_kind(hash: u64, kind: &ExprKind) -> u64 {
        match kind {
            ExprKind::Number(n) => {
                let h = hash_byte(hash, b'N');
                hash_f64(h, *n)
            }

            ExprKind::Symbol(s) => {
                let h = hash_byte(hash, b'S');
                hash_u64(h, s.id())
            }

            // Sum: Use commutative (order-independent) hashing
            ExprKind::Sum(terms) => {
                let h = hash_byte(hash, b'+');
                // Commutative: sum of individual hashes
                let mut acc: u64 = 0;
                for t in terms {
                    acc = acc.wrapping_add(t.hash);
                }
                hash_u64(h, acc)
            }

            // Product: Use commutative (order-independent) hashing
            ExprKind::Product(factors) => {
                let h = hash_byte(hash, b'*');
                // Commutative: sum of individual hashes
                let mut acc: u64 = 0;
                for f in factors {
                    acc = acc.wrapping_add(f.hash);
                }
                hash_u64(h, acc)
            }

            // Div: Non-commutative, ordered
            ExprKind::Div(num, den) => {
                let h = hash_byte(hash, b'/');
                let h = hash_u64(h, num.hash);
                hash_u64(h, den.hash)
            }

            // Pow: Non-commutative, ordered
            ExprKind::Pow(base, exp) => {
                let h = hash_byte(hash, b'^');
                let h = hash_u64(h, base.hash);
                hash_u64(h, exp.hash)
            }

            // FunctionCall: Name + ordered args
            ExprKind::FunctionCall { name, args } => {
                let h = hash_byte(hash, b'F');
                let h = hash_u64(h, name.id());
                args.iter().fold(h, |acc, arg| hash_u64(acc, arg.hash))
            }

            // Derivative: var symbol ID + order + inner
            ExprKind::Derivative { inner, var, order } => {
                let h = hash_byte(hash, b'D');
                let h = hash_u64(h, var.id()); // Use symbol ID directly
                let h = hash_u64(h, u64::from(*order));
                hash_u64(h, inner.hash)
            }

            // Polynomial: hash based on terms and base
            ExprKind::Poly(poly) => {
                let h = hash_byte(hash, b'P');
                // Hash base expression
                let h = hash_u64(h, poly.base().hash);
                // Hash each term (power, coeff) - commutative, so sum hashes
                let mut acc: u64 = 0;
                for &(pow, coeff) in poly.terms() {
                    let term_hash = hash_u64(hash_f64(0, coeff), u64::from(pow));
                    acc = acc.wrapping_add(term_hash);
                }
                hash_u64(h, acc)
            }
        }
    }

    hash_kind(FNV_OFFSET, kind)
}
