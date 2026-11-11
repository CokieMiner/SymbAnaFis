//! The derivative of compact storage over its own base table.

#![expect(
    unsafe_code,
    reason = "base positions index the base table and the per-base tables built beside it, and ascending term ends partition the lowered powers"
)]

use alloc::{boxed::Box, vec, vec::Vec};

use num_anafis::Number;

use super::{ExactIdentity, Expr, Node, NumericValue, Polynomial, Powers};

impl Polynomial {
    /// The derivative with respect to the base at `target`, on which no other
    /// base depends: each term \(c\,b^{k}R\) becomes \(c\,k\,b^{k-1}R\), and
    /// terms without \(b\) vanish.
    ///
    /// The result is the canonical sum of the lowered monomials' products.
    /// Lowering maps distinct terms to distinct terms, so no two monomials
    /// combine, and the surviving bases keep their strict structural order.
    /// When every monomial has a finite nonzero literal coefficient and a
    /// power, canonical construction stores those whose term holds a base with
    /// distinct powers and keeps the others as products beside them; that sum
    /// is built here directly, without hashing or comparing bases. Otherwise
    /// canonical sum construction builds it from the products.
    pub fn derivative(&self, target: usize) -> Expr {
        // The lowered powers of the surviving terms, term after term, beside
        // each scaled coefficient and the end of its term.
        let mut lowered = Vec::with_capacity(self.powers.len());
        let mut terms = Vec::new();
        let mut compact = true;
        for (coefficient, powers) in self.terms() {
            let Some(&(_, exponent)) = powers.iter().find(|(position, _)| *position == target)
            else {
                continue;
            };
            let start = lowered.len();
            lowered.extend(powers.iter().filter_map(|&(position, power)| {
                if position == target {
                    (power > 1).then(|| (position, power.saturating_sub(1)))
                } else {
                    Some((position, power))
                }
            }));
            compact &= lowered.len() > start;
            // Product construction omits a unit coefficient and folds the
            // remaining finite literals; a failed fold keeps both factors.
            let scale = i64::from(exponent);
            let scaled = if exponent == 1 {
                coefficient.clone()
            } else if coefficient.is_exact_one() {
                Expr::number(scale)
            } else if let Some(value) = coefficient.literal()
                && let Some(product) = value.mul(&NumericValue::new(Number::from(scale)))
            {
                compact &= product.is_finite() && !product.is_zero();
                Expr::number(product)
            } else {
                compact = false;
                Expr::product(vec![coefficient.clone(), Expr::number(scale)])
            };
            terms.push((scaled, lowered.len()));
        }
        if !compact {
            // A coefficient fold failed or a term lost its last power, so the
            // monomials are not all compact: canonical sum construction groups
            // their products.
            let mut begin = 0;
            return Expr::sum(
                terms
                    .into_iter()
                    .map(|(coefficient, end)| {
                        // SAFETY: term ends ascend from zero to the lowered length.
                        let term = unsafe { lowered.get_unchecked(begin..end) };
                        begin = end;
                        let powers = Powers {
                            powers: term.iter(),
                            bases: &self.bases,
                        };
                        Expr::product(Node::monomial_factors(coefficient, powers))
                    })
                    .collect(),
            );
        }
        // The first exponent of every base, zero when the base is unused, and
        // whether another of its exponents differs.
        let mut exponents = vec![(0_u32, false); self.bases.len()];
        for &(position, exponent) in &lowered {
            // SAFETY: lowered positions are stored base positions, which index
            // the base table that `exponents` mirrors.
            let first = unsafe { exponents.get_unchecked_mut(position) };
            if first.0 == 0 {
                first.0 = exponent;
            }
            first.1 |= first.0 != exponent;
        }
        // Canonical sum construction stores the monomials whose term holds a
        // base with distinct powers over the bases they use, and keeps every
        // other monomial as its product beside that storage.
        let mut selected = Vec::with_capacity(terms.len());
        let mut others = Vec::new();
        let mut start = 0;
        for (coefficient, end) in terms {
            // SAFETY: term ends ascend from zero to the lowered length.
            let term = unsafe { lowered.get_unchecked(start..end) };
            start = end;
            let distinct = term.iter().any(|&(position, _)| {
                // SAFETY: as above, positions index `exponents`.
                unsafe { exponents.get_unchecked(position) }.1
            });
            if distinct {
                selected.push((term, coefficient));
            } else {
                let powers = Powers {
                    powers: term.iter(),
                    bases: &self.bases,
                };
                others.push(Expr::product(Node::monomial_factors(coefficient, powers)));
            }
        }
        if selected.is_empty() {
            Node::sort_operands(&mut others);
            return Node::finish(others, true);
        }
        let used: Vec<bool> = if others.is_empty() {
            exponents.iter().map(|&(first, _)| first != 0).collect()
        } else {
            let mut marked = vec![false; self.bases.len()];
            for &(position, _) in selected.iter().flat_map(|(term, _)| *term) {
                // SAFETY: `marked` mirrors the base table, as `exponents` does.
                *unsafe { marked.get_unchecked_mut(position) } = true;
            }
            marked
        };
        others.push(self.stored(selected, &used));
        Node::sort_operands(&mut others);
        Node::finish(others, true)
    }

    /// Compact storage of the `selected` lowered terms over the bases that
    /// `used` marks, renumbered in their order. Renumbering preserves the order
    /// of terms, so they are ordered by their lowered powers.
    fn stored(&self, mut selected: Vec<(&[(usize, u32)], Expr)>, used: &[bool]) -> Expr {
        let mut table = Vec::with_capacity(self.bases.len());
        let mut renumbering = Vec::with_capacity(self.bases.len());
        for (base, in_use) in self.bases.iter().zip(used) {
            renumbering.push(table.len());
            if *in_use {
                table.push(base.clone());
            }
        }
        selected.sort_unstable_by_key(|(term, _)| *term);
        let count = selected.iter().map(|(term, _)| term.len()).sum();
        let renumbered = renumbering.as_slice();
        let polynomial = Self::new(
            table,
            count,
            selected.into_iter().map(|(term, coefficient)| {
                let powers = term.iter().map(|&(position, exponent)| {
                    // SAFETY: `renumbered` has one entry per base.
                    (*unsafe { renumbered.get_unchecked(position) }, exponent)
                });
                (coefficient, powers)
            }),
        );
        Expr::intern(Node::Poly(Box::new(polynomial)))
    }
}
