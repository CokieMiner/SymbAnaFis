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

use super::{ExactIdentity, Expr, Node, Polynomial};

/// The logical terms of compact storage, built on first use and kept with it.
///
/// Racing first readers may each build the terms; the first to publish its
/// list wins and the others drop theirs, so every reader borrows one list. A
/// published list is removed only through exclusive access.
#[derive(Debug, Default)]
pub struct Projection(AtomicPtr<Vec<Expr>>);

/// A stored monomial with the facts that order its product.
struct Key<'storage> {
    kind: u8,
    count: usize,
    /// Whether the literal is a factor: it is not the unit, or it is the whole term.
    literal: bool,
    coefficient: &'storage Expr,
    /// The leaves to the first power, then the raised leaves, each as
    /// whether it is raised, its base position, and its exponent.
    factors: &'storage [(bool, usize, u32)],
}

impl Polynomial {
    /// The logical terms in structural order, each monomial as a canonical
    /// product, built on first use and then borrowed.
    pub fn projected(&self) -> &[Expr] {
        let mut published = self.projection.0.load(Acquire);
        if published.is_null() {
            let terms = if self.simple_bases {
                self.scalar_terms()
            } else {
                let mut terms: Vec<_> = self
                    .monomials()
                    .map(|(coefficient, powers)| {
                        Expr::product(Node::monomial_factors(coefficient.clone(), powers))
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

    /// The canonical products of storage over symbols, \(\pi\), and \(e\),
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
    fn scalar_terms(&self) -> Vec<Expr> {
        let mut order = Vec::with_capacity(self.powers.len());
        for (_, powers) in self.terms() {
            order.extend(
                powers
                    .iter()
                    .filter(|power| power.1 == 1)
                    .chain(powers.iter().filter(|power| power.1 > 1))
                    .map(|&(position, exponent)| (exponent > 1, position, exponent)),
            );
        }
        let mut rest = order.as_slice();
        let mut keys: Vec<_> = self
            .terms()
            .map(|(coefficient, powers)| {
                // SAFETY: `order` holds one entry per stored power, term after
                // term, so the remaining entries begin with this term's.
                let (factors, tail) = unsafe { rest.split_at_unchecked(powers.len()) };
                rest = tail;
                let unit = coefficient.is_exact_one();
                let kind = match (unit, powers) {
                    (_, []) => coefficient.node().rank(),
                    (true, [(position, 1)]) => {
                        // SAFETY: the base positions of a polynomial index its
                        // base table.
                        unsafe { self.bases.get_unchecked(*position) }.node().rank()
                    }
                    (true, [_]) => Node::POWER_RANK,
                    _ => Node::PRODUCT_RANK,
                };
                let literal = !unit || powers.is_empty();
                Key {
                    kind,
                    count: usize::from(literal).saturating_add(powers.len()),
                    literal,
                    coefficient,
                    factors,
                }
            })
            .collect();
        keys.sort_unstable_by(|left, right| {
            left.kind.cmp(&right.kind).then_with(|| {
                if left.kind != Node::PRODUCT_RANK {
                    // One factor: a literal, a leaf, or a raised leaf.
                    return if left.factors.is_empty() {
                        Node::compare(left.coefficient, right.coefficient)
                    } else {
                        left.factors.cmp(right.factors)
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
                    .then_with(|| left.factors.cmp(right.factors))
                })
            })
        });
        keys.iter()
            .map(|key| {
                let mut factors = Vec::with_capacity(key.count);
                if key.literal {
                    factors.push(key.coefficient.clone());
                }
                factors.extend(key.factors.iter().map(|&(raised, position, exponent)| {
                    // SAFETY: the base positions of a polynomial index its base
                    // table.
                    let base = unsafe { self.bases.get_unchecked(position) }.clone();
                    if raised {
                        Node::normalize_power(base, Expr::number(i64::from(exponent)))
                    } else {
                        base
                    }
                }));
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
