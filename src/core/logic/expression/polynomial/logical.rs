//! The logical terms of compact storage, built once and kept with it.

#![expect(
    unsafe_code,
    reason = "base positions index the base table and term powers partition the power list, both by construction; a published projection is a boxed list freed only through exclusive access"
)]

use core::{
    cmp::Ordering,
    mem::replace,
    ptr::null_mut,
    sync::atomic::{
        AtomicPtr,
        Ordering::{Acquire, Release},
    },
};

use alloc::{boxed::Box, vec::Vec};

use super::{Expr, Node, Polynomial};

/// Direct-mapped stack cache slots for raised base powers `(position, exponent)`.
const POWER_CACHE_SLOTS: usize = 64;

/// Bitmask for indexing [`PowerCache`].
const POWER_CACHE_MASK: usize = 63;

/// Stack cache of materialized `Node::Pow` handles keyed by `(position, exponent)`.
type PowerCache = [(u32, u32, Option<Expr>); POWER_CACHE_SLOTS];

/// The logical terms of compact storage, built on first use and kept with it.
///
/// Racing first readers may each build the terms; the first to publish its
/// list wins and the others drop theirs, so every reader borrows one list. A
/// published list is removed only through exclusive access.
#[derive(Debug, Default)]
pub struct Projection(AtomicPtr<Vec<Expr>>);

/// Factor representation for ordering monomials in `scalar_terms`.
trait ScalarFactor: Copy + Ord {
    fn encode(position: u32, exponent: u32) -> Self;
    fn decode(self) -> (u32, u32);
}

impl ScalarFactor for u64 {
    #[inline]
    fn encode(position: u32, exponent: u32) -> Self {
        if exponent == 1 {
            Self::from(position)
        } else {
            (1_u64 << 63) | (Self::from(position) << 32) | Self::from(exponent)
        }
    }

    #[inline]
    fn decode(self) -> (u32, u32) {
        if (self >> 63) == 0 {
            (u32::try_from(self & 0x7FFF_FFFF).unwrap_or(0), 1)
        } else {
            (
                u32::try_from((self >> 32) & 0x7FFF_FFFF).unwrap_or(0),
                u32::try_from(self & 0xFFFF_FFFF).unwrap_or(1),
            )
        }
    }
}

impl ScalarFactor for (bool, u32, u32) {
    #[inline]
    fn encode(position: u32, exponent: u32) -> Self {
        (exponent > 1, position, exponent)
    }

    #[inline]
    fn decode(self) -> (u32, u32) {
        (self.1, self.2)
    }
}

/// Materialize `base^exponent` for `position`, caching `exponent > 1` on the stack.
#[inline]
#[expect(
    clippy::as_conversions,
    reason = "stored base positions and exponents widen losslessly to `usize` for indexing and hashing"
)]
fn cached_power(bases: &[Expr], position: u32, exponent: u32, cache: &mut PowerCache) -> Expr {
    if exponent == 1 {
        // SAFETY: `position` is a stored base position indexing `bases`.
        return unsafe { bases.get_unchecked(position as usize) }.clone();
    }
    let slot = ((position as usize).wrapping_mul(31) ^ (exponent as usize).wrapping_mul(13))
        & POWER_CACHE_MASK;
    // SAFETY: `slot` is masked by `POWER_CACHE_MASK` below `POWER_CACHE_SLOTS`.
    let entry = unsafe { cache.get_unchecked_mut(slot) };
    if entry.0 == position
        && entry.1 == exponent
        && let Some(cached) = &entry.2
    {
        return cached.clone();
    }
    // SAFETY: `position` is a stored base position indexing `bases`.
    let base = unsafe { bases.get_unchecked(position as usize) }.clone();
    let built = Node::normalize_power(base, Expr::number(i64::from(exponent)));
    *entry = (position, exponent, Some(built.clone()));
    built
}

/// A stored monomial with the facts that order its product.
struct Key<'storage, Factor> {
    kind: u8,
    /// Whether the literal is a factor: it is not the unit, or it is the whole term.
    literal: bool,
    count: usize,
    coefficient: &'storage Expr,
    /// The first factor (valid whenever `count > usize::from(literal)`).
    first_factor: Factor,
    /// The remaining factors after `first_factor`, leaves to the first power
    /// followed by raised leaves.
    rest_factors: &'storage [Factor],
}

fn compare_keys<Factor: ScalarFactor>(left: &Key<'_, Factor>, right: &Key<'_, Factor>) -> Ordering {
    left.kind.cmp(&right.kind).then_with(|| {
        if left.kind != Node::PRODUCT_RANK {
            // One factor: a literal, a leaf, or a raised leaf.
            return if left.count == usize::from(left.literal) {
                Node::compare(left.coefficient, right.coefficient)
            } else {
                left.first_factor.cmp(&right.first_factor)
            };
        }
        left.count.cmp(&right.count).then_with(|| {
            // Equal literals share one interned node.
            match (left.literal, right.literal) {
                (true, true) if left.coefficient != right.coefficient => {
                    Node::compare(left.coefficient, right.coefficient)
                }
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                (true | false, _) => Ordering::Equal,
            }
            .then_with(|| {
                left.first_factor
                    .cmp(&right.first_factor)
                    .then_with(|| left.rest_factors.cmp(right.rest_factors))
            })
        })
    })
}

impl Polynomial {
    /// The logical terms in structural order, each monomial as a canonical
    /// product, built on first use and then borrowed.
    pub fn projected(&self) -> &[Expr] {
        let mut published = self.projection.0.load(Acquire);
        if published.is_null() {
            let terms = if self.simple_bases {
                if u32::try_from(self.bases.len()).is_ok_and(|len| len <= 0x8000_0000) {
                    self.scalar_terms::<u64>()
                } else {
                    self.scalar_terms::<(bool, u32, u32)>()
                }
            } else {
                let mut power_cache: PowerCache = [const { (0, 0, None) }; POWER_CACHE_SLOTS];
                let mut terms: Vec<_> = self
                    .terms()
                    .map(|(coefficient, powers)| {
                        let mut factors = Vec::with_capacity(powers.len().saturating_add(1));
                        if !coefficient.is_exact_one() {
                            factors.push(coefficient.clone());
                        }
                        factors.extend(powers.iter().map(|&(position, exponent)| {
                            cached_power(&self.bases, position, exponent, &mut power_cache)
                        }));
                        Expr::product(factors)
                    })
                    .collect();
                Node::sort_operands(&mut terms);
                terms
            };
            debug_assert!(
                terms.is_sorted_by(|left, right| Node::compare(left, right).is_lt()),
                "projected terms are distinct and in structural order"
            );
            let fresh = Box::into_raw(Box::new(terms));
            published =
                match self
                    .projection
                    .0
                    .compare_exchange(null_mut(), fresh, Release, Acquire)
                {
                    Ok(_) => fresh,
                    Err(winner) => {
                        // SAFETY: `fresh` came from `Box::into_raw` and was never
                        // published, so this is its only owner.
                        drop(unsafe { Box::from_raw(fresh) });
                        winner
                    }
                };
        }
        // SAFETY: a published pointer came from `Box::into_raw` and is never
        // replaced while shared; it is freed only through exclusive access,
        // which this borrow of `self` excludes until the slice is released.
        unsafe { &*published }
    }

    /// The canonical products of storage over symbols, $\pi$, and $e$,
    /// in structural order, built directly and ordered from storage.
    ///
    /// Such a monomial is a literal times distinct leaves raised to positive
    /// integers. Its canonical product lists the literal unless it is the
    /// exact unit, then the leaves to the first power, then the raised leaves,
    /// each group in base order: literals rank first, a power ranks after every
    /// leaf, powers compare base first, and the base table is in structural
    /// order. A product of one factor is that factor. The terms are ordered as
    /// [`Node::compare`] orders these products: by node kind, then factor
    /// count, then factor by factor.
    #[expect(
        clippy::as_conversions,
        reason = "stored base positions are `u32` and widen losslessly to indices of the base table"
    )]
    fn scalar_terms<Factor: ScalarFactor>(&self) -> Vec<Expr> {
        let mut order = Vec::with_capacity(self.powers.len());
        for (_, powers) in self.terms() {
            order.extend(
                powers
                    .iter()
                    .filter(|power| power.1 == 1)
                    .chain(powers.iter().filter(|power| power.1 > 1))
                    .map(|&(position, exponent)| Factor::encode(position, exponent)),
            );
        }
        let mut rest = order.as_slice();
        let dummy = Factor::encode(0, 1);
        let mut keys: Vec<_> = self
            .terms()
            .map(|(coefficient, powers)| {
                // SAFETY: `order` holds one entry per stored power, term after
                // term, so the remaining entries begin with this term's.
                let (factors, tail) = unsafe { rest.split_at_unchecked(powers.len()) };
                rest = tail;
                let (first_factor, rest_factors) = factors
                    .split_first()
                    .map_or((dummy, [].as_slice()), |(&first, tail_factors)| {
                        (first, tail_factors)
                    });
                let unit = coefficient.is_exact_one();
                let kind = match (unit, powers) {
                    (_, []) => coefficient.node().rank(),
                    (true, [(position, 1)]) => {
                        // SAFETY: the base positions of a polynomial index its
                        // base table.
                        unsafe { self.bases.get_unchecked(*position as usize) }
                            .node()
                            .rank()
                    }
                    (true, [_]) => Node::POWER_RANK,
                    _ => Node::PRODUCT_RANK,
                };
                let literal = !unit || powers.is_empty();
                Key {
                    kind,
                    literal,
                    count: usize::from(literal).saturating_add(powers.len()),
                    coefficient,
                    first_factor,
                    rest_factors,
                }
            })
            .collect();
        keys.sort_unstable_by(compare_keys);
        let mut power_cache: PowerCache = [const { (0, 0, None) }; POWER_CACHE_SLOTS];
        keys.iter()
            .map(|key| {
                let mut factors = Vec::with_capacity(key.count);
                if key.literal {
                    factors.push(key.coefficient.clone());
                }
                if key.count > usize::from(key.literal) {
                    let (first_pos, first_exp) = key.first_factor.decode();
                    factors.push(cached_power(
                        &self.bases,
                        first_pos,
                        first_exp,
                        &mut power_cache,
                    ));
                    for &factor in key.rest_factors {
                        let (position, exponent) = factor.decode();
                        factors.push(cached_power(
                            &self.bases,
                            position,
                            exponent,
                            &mut power_cache,
                        ));
                    }
                }
                Node::finish(factors, false)
            })
            .collect()
    }
}

impl Projection {
    /// Remove the published terms, if any.
    pub fn take(&mut self) -> Option<Vec<Expr>> {
        let published = replace(self.0.get_mut(), null_mut());
        // SAFETY: a published pointer came from `Box::into_raw`, and exclusive
        // access replaced it with null, so ownership moves here exactly once.
        (!published.is_null()).then(|| *unsafe { Box::from_raw(published) })
    }
}

impl Drop for Projection {
    fn drop(&mut self) {
        drop(self.take());
    }
}
