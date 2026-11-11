//! The derivative of compact storage over its own base table.

#![expect(
    unsafe_code,
    reason = "base positions index the base table and the per-base tables built beside it, and ascending term ends partition the lowered powers"
)]

use core::ptr::null;

use alloc::{boxed::Box, vec, vec::Vec};

use num_anafis::Number;

use super::{Expr, Node, NumericValue, Polynomial, Powers, Projection};

/// Direct-mapped stack cache slots for scaled literal coefficients `(coeff, exponent)`.
const SCALE_CACHE_SLOTS: usize = 64;

/// A stack cache of scaled coefficients keyed by `(coefficient_ptr, exponent)`.
type ScaleCache = [(*const Node, u32, Option<Expr>); SCALE_CACHE_SLOTS];

/// A sorted run of lowered monomial power slices and their scaled coefficients.
type LoweredRun<'powers> = Vec<(&'powers [(u32, u32)], Expr)>;

impl Polynomial {
    /// The derivative with respect to the base at `target`, on which no other
    /// base depends: each term $c b^{k}R$ becomes $c k b^{k-1}R$, and
    /// terms without $b$ vanish.
    ///
    /// The result is the canonical sum of the lowered monomials' products.
    /// Lowering maps distinct terms to distinct terms, so no two monomials
    /// combine, and the surviving bases keep their strict structural order.
    /// When every surviving monomial has a finite nonzero literal coefficient,
    /// canonical construction stores those whose term holds a base with
    /// distinct powers and keeps the others as products beside them; that sum
    /// is built here directly, without hashing or comparing bases. Otherwise
    /// canonical sum construction builds it from the products.
    #[expect(
        clippy::as_conversions,
        reason = "stored base positions are `u32` and widen losslessly to indices of the base table and the per-base tables beside it"
    )]
    pub fn derivative(&self, target: usize) -> Expr {
        let Ok(target_pos) = u32::try_from(target) else {
            return Expr::number(0);
        };
        let mut lowered = Vec::with_capacity(self.powers.len().min(1024));
        let mut terms = Vec::with_capacity(self.coefficients.len().min(256));
        let mut bucket_counts = [0_usize; 3];
        let mut compact = u32::try_from(self.bases.len()).is_ok();
        let mut scale_cache: ScaleCache = [const { (null(), 0, None) }; SCALE_CACHE_SLOTS];
        for (coefficient, powers) in self.terms() {
            let Some(index) = powers
                .iter()
                .take_while(|&&(position, _)| position <= target_pos)
                .position(|&(position, _)| position == target_pos)
            else {
                continue;
            };
            // SAFETY: `index` was recorded from `powers.iter().enumerate()`, so `index < powers.len()`.
            let &(_, exponent) = unsafe { powers.get_unchecked(index) };
            // SAFETY: `index < powers.len()`.
            lowered.extend_from_slice(unsafe { powers.get_unchecked(..index) });
            if exponent > 1 {
                lowered.push((target_pos, exponent.saturating_sub(1)));
            }
            let next_index = index.saturating_add(1);
            // SAFETY: `index + 1 <= powers.len()`.
            lowered.extend_from_slice(unsafe { powers.get_unchecked(next_index..) });
            let scaled =
                Self::scale_coefficient(coefficient, exponent, &mut compact, &mut scale_cache);
            let bucket = u8::from(exponent == 1)
                .saturating_add(u8::from(exponent == 1 && next_index == powers.len()));
            // SAFETY: `bucket` is 0, 1, or 2.
            let count = unsafe { bucket_counts.get_unchecked_mut(bucket as usize) };
            *count = count.saturating_add(1);
            terms.push((scaled, lowered.len(), bucket));
        }
        if !compact {
            let mut begin = 0;
            return Expr::sum(
                terms
                    .into_iter()
                    .map(|(coefficient, end, _)| {
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
        let mut exponents = vec![(0_u32, false); self.bases.len()];
        for &(position, exponent) in &lowered {
            // SAFETY: lowered positions index the base table that `exponents` mirrors.
            let first = unsafe { exponents.get_unchecked_mut(position as usize) };
            if first.0 == 0 {
                first.0 = exponent;
            }
            first.1 |= first.0 != exponent;
        }
        let all_distinct = exponents
            .iter()
            .all(|&(first, distinct)| first == 0 || distinct);
        // Surviving terms partition into three runs:
        // - Run 0 (`exponent > 1`): `target` remains in place with `exponent - 1`,
        //   preserving strict lexicographical order.
        // - Run 1 (`exponent == 1` with trailing powers `position > target`):
        //   `target` is replaced by a position `> target`, also preserving order.
        // - Run 2 (`exponent == 1` as the last power in the term): removing
        //   `target` shortens the power slice, so only this run is sorted before merging.
        let mut runs: [LoweredRun<'_>; 3] = bucket_counts.map(Vec::with_capacity);
        let mut others = Vec::new();
        let mut start = 0;
        for (coefficient, end, bucket) in terms {
            // SAFETY: term ends ascend from zero to the lowered length.
            let term = unsafe { lowered.get_unchecked(start..end) };
            start = end;
            if term.is_empty() {
                others.push(coefficient);
                continue;
            }
            let distinct = all_distinct
                || term.iter().any(|&(position, _)| {
                    // SAFETY: lowered positions index `exponents`.
                    unsafe { exponents.get_unchecked(position as usize) }.1
                });
            if distinct {
                // SAFETY: `bucket` is 0, 1, or 2.
                unsafe { runs.get_unchecked_mut(bucket as usize) }.push((term, coefficient));
            } else {
                let powers = Powers {
                    powers: term.iter(),
                    bases: &self.bases,
                };
                others.push(Expr::product(Node::monomial_factors(coefficient, powers)));
            }
        }
        if runs.iter().all(Vec::is_empty) {
            Node::sort_operands(&mut others);
            return Node::finish(others, true);
        }
        let used: Vec<bool> = if others.is_empty() {
            exponents.iter().map(|&(first, _)| first != 0).collect()
        } else {
            let mut marked = vec![false; self.bases.len()];
            for &(position, _) in runs.iter().flatten().flat_map(|(term, _)| *term) {
                // SAFETY: `marked` mirrors the base table, as `exponents` does.
                *unsafe { marked.get_unchecked_mut(position as usize) } = true;
            }
            marked
        };
        others.push(self.stored(runs, &used));
        Node::sort_operands(&mut others);
        Node::finish(others, true)
    }

    /// Multiply `coefficient` by `exponent`, folding finite literals and
    /// caching repeated `(coefficient, exponent)` pairs on the stack.
    #[expect(
        clippy::as_conversions,
        reason = "the exponent widens to `usize` for cache slot hashing"
    )]
    fn scale_coefficient(
        coefficient: &Expr,
        exponent: u32,
        compact: &mut bool,
        cache: &mut ScaleCache,
    ) -> Expr {
        if exponent == 1 {
            return coefficient.clone();
        }
        let key_ptr = coefficient.pointer();
        let slot = ((key_ptr.addr() >> 4) ^ (exponent as usize).wrapping_mul(31))
            & (SCALE_CACHE_SLOTS - 1);
        // SAFETY: `slot` is masked below `SCALE_CACHE_SLOTS`.
        let entry = unsafe { cache.get_unchecked_mut(slot) };
        if entry.0 == key_ptr
            && entry.1 == exponent
            && let Some(cached) = &entry.2
        {
            return cached.clone();
        }
        let scale = i64::from(exponent);
        let (computed, cacheable) = if coefficient.is_exact_one() {
            (Expr::number(scale), true)
        } else if let Some(value) = coefficient.literal()
            && let Some(product) = value.mul(&NumericValue::new(Number::from(scale)))
        {
            let valid = product.is_finite() && !product.is_zero();
            *compact &= valid;
            (Expr::number(product), valid)
        } else {
            *compact = false;
            (
                Expr::product(vec![coefficient.clone(), Expr::number(scale)]),
                false,
            )
        };
        if cacheable {
            *entry = (key_ptr, exponent, Some(computed.clone()));
        }
        computed
    }

    /// Compact storage of the three sorted lowered runs over the bases that
    /// `used` marks, renumbered in their order. Renumbering is strictly
    /// order-preserving on used bases, so merging by lowered powers produces
    /// the canonical monomial power order in linear time.
    #[expect(
        clippy::as_conversions,
        reason = "a stored base position is a `u32` and widens losslessly to an index of the renumbering table"
    )]
    fn stored(
        &self,
        [kept_target, dropped_mid, mut dropped_end]: [LoweredRun<'_>; 3],
        used: &[bool],
    ) -> Expr {
        let mut table = Vec::with_capacity(self.bases.len());
        let mut renumbering: Vec<u32> = Vec::with_capacity(self.bases.len());
        let mut next = 0_u32;
        for (base, in_use) in self.bases.iter().zip(used) {
            renumbering.push(next);
            if *in_use {
                table.push(base.clone());
                // The table fits in a `u32`, so the last position is below it.
                next = next.saturating_add(1);
            }
        }
        dropped_end.sort_unstable_by_key(|(term, _)| *term);
        debug_assert!(
            kept_target.is_sorted_by(|(a, _), (b, _)| a < b)
                && dropped_mid.is_sorted_by(|(a, _), (b, _)| a < b),
            "runs 0 and 1 are strictly sorted"
        );
        let runs = [kept_target, dropped_mid, dropped_end];
        let term_count = runs.iter().map(Vec::len).sum();
        let power_count = runs.iter().flatten().map(|(term, _)| term.len()).sum();
        let all_bases_used = table.len() == self.bases.len();
        let simple_bases = self.simple_bases
            || table
                .iter()
                .all(|base| matches!(base.node(), Node::Symbol(_) | Node::Pi | Node::Euler));
        let mut coefficients = Vec::with_capacity(term_count);
        let mut lengths = Vec::with_capacity(term_count);
        let mut powers = Vec::with_capacity(power_count);
        let renumbered = renumbering.as_slice();
        Self::merge_runs(runs, |(term, coefficient)| {
            coefficients.push(coefficient);
            lengths.push(term.len());
            if all_bases_used {
                powers.extend_from_slice(term);
            } else {
                powers.extend(term.iter().map(|&(position, exponent)| {
                    // SAFETY: `renumbered` has one entry per base.
                    (
                        *unsafe { renumbered.get_unchecked(position as usize) },
                        exponent,
                    )
                }));
            }
        });
        let polynomial = Self {
            bases: table,
            coefficients,
            lengths,
            powers,
            simple_bases,
            projection: Projection::default(),
        };
        debug_assert!(
            polynomial
                .terms()
                .is_sorted_by(|(_, left), (_, right)| left < right),
            "merged derivative terms are strictly ordered"
        );
        Expr::intern(Node::Poly(Box::new(polynomial)))
    }

    /// Merge three sorted lowered-monomial runs in ascending power order.
    fn merge_runs<'powers>(
        [first, second, third]: [LoweredRun<'powers>; 3],
        mut push_term: impl FnMut((&'powers [(u32, u32)], Expr)),
    ) {
        let mut a = first.into_iter().peekable();
        let mut b = second.into_iter().peekable();
        let mut c = third.into_iter().peekable();
        loop {
            let pick = match (a.peek(), b.peek(), c.peek()) {
                (Some(_), None, None) => {
                    a.for_each(&mut push_term);
                    break;
                }
                (None, Some(_), None) => {
                    b.for_each(&mut push_term);
                    break;
                }
                (None, None, Some(_)) => {
                    c.for_each(&mut push_term);
                    break;
                }
                (None, None, None) => break,
                (Some((ta, _)), Some((tb, _)), Some((tc, _))) => {
                    if ta <= tb && ta <= tc {
                        0
                    } else if tb <= tc {
                        1
                    } else {
                        2
                    }
                }
                (Some((ta, _)), Some((tb, _)), None) => usize::from(tb < ta),
                (Some((ta, _)), None, Some((tc, _))) => {
                    if ta <= tc {
                        0
                    } else {
                        2
                    }
                }
                (None, Some((tb, _)), Some((tc, _))) => {
                    if tb <= tc {
                        1
                    } else {
                        2
                    }
                }
            };
            if let Some(item) = match pick {
                0 => a.next(),
                1 => b.next(),
                _ => c.next(),
            } {
                push_term(item);
            }
        }
    }
}
