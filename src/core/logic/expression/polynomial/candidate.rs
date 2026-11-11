//! Monomial candidates and the construction of compact storage.

#![expect(
    unsafe_code,
    reason = "base ids are positions in the distinct-base list built with them, and power ids and ranks follow the candidates' powers in order"
)]

use core::{borrow::Borrow, cmp::Ordering, mem::take, slice::from_ref};

use alloc::{boxed::Box, vec, vec::Vec};

use crate::collections::FixedMap;

use super::{ExactIdentity, Expr, Monomial, Node, Polynomial};

/// One additive term represented by a finite coefficient and sorted powers,
/// whose bases are owned or borrowed handles.
#[derive(Debug)]
pub struct Candidate<Base> {
    source: usize,
    coefficient: Expr,
    powers: Vec<(Base, u32)>,
}

impl<Base: Borrow<Expr> + Into<Expr>> Candidate<Base> {
    /// Reconstruct the canonical factor slice for a failed direct construction.
    pub fn into_factors(self) -> Vec<Expr> {
        Node::monomial_factors(self.coefficient, self.powers)
    }

    /// Compact storage built from the candidates that contain a base with
    /// distinct powers, and the sources of those candidates in input order,
    /// recorded unless `all` requires every candidate to contain such a base.
    ///
    /// Each distinct base is hashed once per power, ranked once in structural
    /// order, and copied once into storage. Terms are ordered by their rank
    /// sequences, which is their structural order.
    ///
    /// # Errors
    /// Returns the monomials when no base has distinct positive powers, when
    /// `all` is set and a candidate lacks such a base, when two distinct bases
    /// compare equal, or when two selected candidates have identical powers.
    pub fn into_polynomial(
        candidates: Vec<Self>,
        all: bool,
    ) -> Result<(Expr, Vec<usize>), Vec<Monomial<Base>>> {
        let into_monomials = |rejected: Vec<Self>| {
            rejected
                .into_iter()
                .map(|candidate| (candidate.coefficient, candidate.powers))
                .collect()
        };
        if !candidates
            .iter()
            .any(|candidate| candidate.powers.iter().any(|(_, exponent)| *exponent > 1))
        {
            return Err(into_monomials(candidates));
        }
        let (distinct, ids) = Self::distinct_bases(&candidates);
        let mut selected = Vec::with_capacity(candidates.len());
        let mut used = vec![false; distinct.len()];
        let mut remaining = ids.as_slice();
        for candidate in &candidates {
            // SAFETY: `ids` holds one id per power, candidate after candidate.
            let (term, rest) = unsafe { remaining.split_at_unchecked(candidate.powers.len()) };
            remaining = rest;
            // SAFETY: every id is a position in `distinct` and in `used`.
            let keep = term
                .iter()
                .any(|&id| unsafe { distinct.get_unchecked(id) }.2);
            if keep {
                for &id in term {
                    // SAFETY: as above.
                    *unsafe { used.get_unchecked_mut(id) } = true;
                }
            }
            selected.push(keep);
        }
        if !selected.iter().any(|keep| *keep) || all && !selected.iter().all(|keep| *keep) {
            return Err(into_monomials(candidates));
        }
        let mut ranked_bases: Vec<(&Expr, usize)> = distinct
            .iter()
            .zip(&used)
            .enumerate()
            .filter_map(|(id, ((base, _, _), in_use))| in_use.then_some((*base, id)))
            .collect();
        ranked_bases.sort_unstable_by(|(left, _), (right, _)| Node::compare(left, right));
        if ranked_bases
            .iter()
            .zip(ranked_bases.iter().skip(1))
            .any(|((left, _), (right, _))| Node::compare(left, right) == Ordering::Equal)
        {
            return Err(into_monomials(candidates));
        }
        let mut rank_of = vec![0; distinct.len()];
        for (rank, (_, id)) in ranked_bases.iter().enumerate() {
            // SAFETY: every id is a position in `distinct`, and `rank_of` has
            // one entry per distinct base.
            *unsafe { rank_of.get_unchecked_mut(*id) } = rank;
        }
        let table: Vec<Expr> = ranked_bases
            .iter()
            .map(|(base, _)| (*base).clone())
            .collect();
        // Each candidate's powers are in structural order, so its ranks ascend.
        let mut ranked = Vec::with_capacity(ids.len());
        let mut remaining_ids = ids.as_slice();
        for (candidate, keep) in candidates.iter().zip(&selected) {
            // SAFETY: `ids` holds one id per power, candidate after candidate.
            let (term, rest) = unsafe { remaining_ids.split_at_unchecked(candidate.powers.len()) };
            remaining_ids = rest;
            if *keep {
                ranked.extend(
                    term.iter()
                        .zip(&candidate.powers)
                        .map(|(&id, (_, exponent))| {
                            // SAFETY: every id is a position in `rank_of`.
                            (*unsafe { rank_of.get_unchecked(id) }, *exponent)
                        }),
                );
            }
        }
        Self::assemble(candidates, selected, &ranked, table, all)
    }

    /// Every distinct base in first-occurrence order with its first power and
    /// whether another power of it occurs, and the id of every power.
    fn distinct_bases(candidates: &[Self]) -> (Vec<(&Expr, u32, bool)>, Vec<usize>) {
        let mut ids_of = FixedMap::default();
        let mut distinct: Vec<(&Expr, u32, bool)> = Vec::new();
        let mut ids = Vec::new();
        for (base, power) in candidates.iter().flat_map(|candidate| &candidate.powers) {
            let handle: &Expr = base.borrow();
            let id = *ids_of.entry(handle.pointer()).or_insert_with(|| {
                let id = distinct.len();
                distinct.push((handle, *power, false));
                id
            });
            // SAFETY: an id is the position at which its base was pushed.
            let known = unsafe { distinct.get_unchecked_mut(id) };
            known.2 |= known.1 != *power;
            ids.push(id);
        }
        (distinct, ids)
    }

    /// Order the selected candidates by their rank sequences in `ranked` and
    /// store them over `table`; repeated rank sequences return the monomials.
    fn assemble(
        candidates: Vec<Self>,
        selected: Vec<bool>,
        ranked: &[(usize, u32)],
        table: Vec<Expr>,
        all: bool,
    ) -> Result<(Expr, Vec<usize>), Vec<Monomial<Base>>> {
        let mut terms = Vec::new();
        let mut powers = Vec::new();
        let mut sources = Vec::new();
        let mut others = Vec::new();
        let mut rest = ranked;
        for (candidate, keep) in candidates.into_iter().zip(selected) {
            if keep {
                // SAFETY: `ranked` holds the powers of the selected candidates
                // in order, so the remaining part starts with this term.
                let (term, tail) = unsafe { rest.split_at_unchecked(candidate.powers.len()) };
                rest = tail;
                terms.push((term, candidate.coefficient, powers.len()));
                powers.push(candidate.powers);
                if !all {
                    sources.push(candidate.source);
                }
            } else {
                others.push((candidate.coefficient, candidate.powers));
            }
        }
        terms.sort_unstable_by_key(|(term, ..)| *term);
        if terms
            .iter()
            .zip(terms.iter().skip(1))
            .any(|((left, ..), (right, ..))| left == right)
        {
            // Failed finite coefficient folding can leave repeated monomials.
            others.extend(terms.into_iter().map(|(_, coefficient, index)| {
                let original = powers.get_mut(index).map_or_else(Vec::new, take);
                (coefficient, original)
            }));
            return Err(others);
        }
        let polynomial = Polynomial::new(
            table,
            ranked.len(),
            terms
                .into_iter()
                .map(|(term, coefficient, _)| (coefficient, term.iter().copied())),
        );
        Ok((Expr::intern(Node::Poly(Box::new(polynomial))), sources))
    }
}

impl<'term> Candidate<&'term Expr> {
    /// Build a candidate from a canonical term, borrowing its bases; `one` is
    /// the unit coefficient shared by terms without a literal.
    pub fn new(source: usize, term: &'term Expr, one: &Expr) -> Option<Self> {
        let (coefficient, factors): (Expr, &[Expr]) = if let Node::Product(factors) = term.node() {
            let (first, rest) = factors.split_first()?;
            if let Some(value) = first.literal() {
                if !value.value().is_finite() || value.value().is_zero() {
                    return None;
                }
                (first.clone(), rest)
            } else {
                (one.clone(), factors)
            }
        } else {
            (one.clone(), from_ref(term))
        };
        let mut powers: Vec<_> = factors.iter().map(Self::factor).collect::<Option<_>>()?;
        if powers.is_empty() {
            return None;
        }
        powers.sort_unstable_by(|(left, _), (right, _)| Node::compare(left, right));
        if powers
            .iter()
            .zip(powers.iter().skip(1))
            .any(|(left, right)| left.0 == right.0)
        {
            return None;
        }
        Some(Self {
            source,
            coefficient,
            powers,
        })
    }

    fn factor(factor: &'term Expr) -> Option<(&'term Expr, u32)> {
        if let Some((base, power)) = factor.positive_factor() {
            if base.literal().is_some() {
                return None;
            }
            return u32::try_from(power).ok().map(|exponent| (base, exponent));
        }
        // A general power may itself be an opaque base, e.g. a + a^2 where
        // a = x^(1/2). Its inner exponent and branch are not combined or erased.
        matches!(factor.node(), Node::Pow(..)).then_some((factor, 1))
    }
}

impl Candidate<Expr> {
    /// Consume one parser factor chain into a candidate without cloning handles.
    ///
    /// The first finite literal is the scalar coefficient; `one` is the unit
    /// coefficient of a chain without one. Rejected shapes are
    /// reconstructed canonically so the caller can retain its general product
    /// fallback without retaining a second copy of every factor.
    ///
    /// # Errors
    /// Returns the original factors when a factor is not an opaque monomial
    /// base, the coefficient is zero or non-finite, or a base repeats.
    pub fn from_factors_owned(
        source: usize,
        factors: Vec<Expr>,
        one: &Expr,
    ) -> Result<Self, Vec<Expr>> {
        let factor_count = factors.len();
        if matches!(factors.as_slice(), [only]
            if matches!(only.node(), Node::Sum(_) | Node::Poly(_)))
        {
            return Err(factors);
        }
        if let [first, only] = factors.as_slice()
            && first.is_exact_one()
            && matches!(only.node(), Node::Sum(_) | Node::Poly(_))
        {
            return Err(factors);
        }
        let mut remaining = factors.into_iter();
        let Some(first) = remaining.next() else {
            return Err(Vec::new());
        };
        let (coefficient, mut powers) = if let Some(value) = first.literal() {
            if !value.value().is_finite() || value.value().is_zero() {
                let mut restored = Vec::with_capacity(factor_count);
                restored.push(first);
                restored.extend(remaining);
                return Err(restored);
            }
            (first, Vec::with_capacity(factor_count.saturating_sub(1)))
        } else {
            let mut values = Vec::with_capacity(factor_count);
            match Self::factor_owned(first) {
                Ok(power) => values.push(power),
                Err(rejected) => {
                    let mut restored = Vec::with_capacity(factor_count);
                    restored.push(rejected);
                    restored.extend(remaining);
                    return Err(restored);
                }
            }
            (one.clone(), values)
        };
        while let Some(next) = remaining.next() {
            match Self::factor_owned(next) {
                Ok(power) => powers.push(power),
                Err(rejected) => {
                    let mut restored = Node::monomial_factors(coefficient, powers);
                    restored.push(rejected);
                    restored.extend(remaining);
                    return Err(restored);
                }
            }
        }
        if powers.is_empty() {
            return Err(Node::monomial_factors(coefficient, powers));
        }
        powers.sort_unstable_by(|(left, _), (right, _)| Node::compare(left, right));
        if powers
            .iter()
            .zip(powers.iter().skip(1))
            .any(|(left, right)| left.0 == right.0)
        {
            return Err(Node::monomial_factors(coefficient, powers));
        }
        Ok(Self {
            source,
            coefficient,
            powers,
        })
    }

    /// The power of [`Candidate::factor`], moving the factor when it is its
    /// own base. Bare additive factors and products are not opaque monomial
    /// bases here: only an explicit power of an additive base enters compact
    /// storage directly, and the canonical fallback groups the rest.
    fn factor_owned(factor: Expr) -> Result<(Expr, u32), Expr> {
        if matches!(
            factor.node(),
            Node::Product(_) | Node::Sum(_) | Node::Poly(_)
        ) {
            return Err(factor);
        }
        let Some((base, exponent)) = Candidate::factor(&factor) else {
            return Err(factor);
        };
        if base.pointer() == factor.pointer() {
            Ok((factor, exponent))
        } else {
            Ok((base.clone(), exponent))
        }
    }
}
