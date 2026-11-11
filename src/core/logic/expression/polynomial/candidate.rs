//! Monomial candidates and the construction of compact storage.

#![expect(
    unsafe_code,
    reason = "base ids are positions in the distinct-base list built with them, and term lengths partition the flat power list in candidate order"
)]

use core::{
    cmp::Ordering,
    ptr::{eq, null},
    slice::from_ref,
};

use alloc::{boxed::Box, vec, vec::Vec};

use crate::collections::{Entry, FixedMap};

use super::{BuiltinFunction, ExactIdentity, Expr, Node, Polynomial};

/// Number of slots in the direct-mapped base pointer cache.
const PTR_CACHE_SLOTS: usize = 32;

/// Mask for indexing the direct-mapped base pointer cache.
const PTR_CACHE_MASK: usize = PTR_CACHE_SLOTS - 1;

/// Flat accumulator of additive monomial candidates borrowing their bases and
/// coefficients from the caller's terms or factor slices.
pub struct Candidates<'term> {
    /// Per-candidate metadata: `(source, coefficient, power_count)`.
    terms: Vec<(usize, &'term Expr, usize)>,
    /// Flat power list across all accepted candidates: `(base_id_or_rank, exponent)`.
    powers: Vec<(u32, u32)>,
    /// Distinct bases in first-occurrence order: `(base, first_exponent, has_distinct_exponents)`.
    distinct: Vec<(&'term Expr, u32, bool)>,
    /// Map from `base.pointer()` to its index in `distinct`, populated lazily past 8 bases.
    ids_of: FixedMap<*const Node, usize>,
    /// Direct-mapped pointer cache for `ids_of` lookups.
    ptr_cache: [(*const Node, u32); PTR_CACHE_SLOTS],
    /// Whether any accepted candidate power has `exponent > 1`.
    has_higher_power: bool,
    /// Whether any distinct base has appeared with at least two different exponents.
    has_distinct_powers: bool,
}

impl<'term> Candidates<'term> {
    /// Allocate a flat candidate accumulator for up to `term_count` terms.
    pub fn with_capacity(term_count: usize) -> Self {
        Self {
            terms: Vec::with_capacity(term_count),
            powers: Vec::with_capacity(term_count.saturating_mul(2)),
            distinct: Vec::new(),
            ids_of: FixedMap::default(),
            ptr_cache: [(null(), 0); PTR_CACHE_SLOTS],
            has_higher_power: false,
            has_distinct_powers: false,
        }
    }

    /// Whether any accepted base has been observed with at least two distinct exponents.
    pub const fn has_distinct_powers(&self) -> bool {
        self.has_distinct_powers
    }

    /// Add a candidate from a canonical term, borrowing its coefficient and
    /// bases; `one` is the unit coefficient shared by terms without a literal.
    pub fn push_term(&mut self, source: usize, term: &'term Expr, one: &'term Expr) -> bool {
        let (coefficient, factors): (&'term Expr, &'term [Expr]) =
            if let Node::Product(factors) = term.node() {
                let Some((first, rest)) = factors.split_first() else {
                    return false;
                };
                if let Some(value) = first.literal() {
                    if !value.value().is_finite() || value.value().is_zero() {
                        return false;
                    }
                    (first, rest)
                } else {
                    (one, factors)
                }
            } else {
                (one, from_ref(term))
            };
        self.push_slice(source, coefficient, factors, true)
    }

    /// Add a candidate from a raw factor slice without cloning handles.
    ///
    /// Bare additive factors and nested products are rejected so the canonical
    /// fallback flattens and groups them.
    pub fn push_factors(
        &mut self,
        source: usize,
        factors: &'term [Expr],
        one: &'term Expr,
    ) -> bool {
        let Some((first, rest)) = factors.split_first() else {
            return false;
        };
        let (coefficient, slice) = if let Some(value) = first.literal() {
            if !value.value().is_finite() || value.value().is_zero() {
                return false;
            }
            (first, rest)
        } else {
            (one, factors)
        };
        self.push_slice(source, coefficient, slice, false)
    }

    /// Add a candidate directly from an indexed `(base_position, exponent)` list
    /// referencing `bases` without constructing intermediate `Pow` or `Product` nodes.
    #[expect(
        clippy::as_conversions,
        reason = "base positions are `u32` and widen losslessly to slice indices"
    )]
    pub fn push_indexed(
        &mut self,
        source: usize,
        coefficient: &'term Expr,
        bases: &'term [Expr],
        indexed_powers: &[(u32, u32)],
    ) -> bool {
        if indexed_powers.is_empty()
            || !coefficient
                .literal()
                .is_some_and(|value| value.value().is_finite() && !value.value().is_zero())
        {
            return false;
        }
        let start = self.powers.len();
        for &(position, exponent) in indexed_powers {
            if exponent == 0 {
                self.powers.truncate(start);
                return false;
            }
            let Some(raw_base) = bases.get(position as usize) else {
                self.powers.truncate(start);
                return false;
            };
            let (base, actual_exponent) = match raw_base.node() {
                Node::Symbol(_)
                | Node::Pi
                | Node::Sum(_)
                | Node::Poly(_)
                | Node::Derivative { .. } => (raw_base, exponent),
                Node::Function { name, .. } if *name != BuiltinFunction::Sqrt.function() => {
                    (raw_base, exponent)
                }
                Node::Euler | Node::Div(..) if exponent == 1 => (raw_base, 1),
                Node::Number(_)
                | Node::ImaginaryUnit
                | Node::Euler
                | Node::Function { .. }
                | Node::Product(_)
                | Node::Div(..)
                | Node::Pow(..) => {
                    if exponent == 1
                        && indexed_powers.len() == 1
                        && let Some(pair) = Self::factor(raw_base)
                    {
                        pair
                    } else {
                        self.powers.truncate(start);
                        return false;
                    }
                }
            };
            let Some(id_u32) = self.base_id(base) else {
                self.powers.truncate(start);
                return false;
            };
            // SAFETY: `start` is at most `self.powers.len()`.
            if unsafe { self.powers.get_unchecked(start..) }
                .iter()
                .any(|&(previous, _)| previous == id_u32)
            {
                self.powers.truncate(start);
                return false;
            }
            self.powers.push((id_u32, actual_exponent));
        }
        self.record_term_powers(source, coefficient, start);
        true
    }

    fn push_slice(
        &mut self,
        source: usize,
        coefficient: &'term Expr,
        factors: &'term [Expr],
        already_normalized: bool,
    ) -> bool {
        if factors.is_empty() {
            return false;
        }
        let start = self.powers.len();
        for factor in factors {
            if !already_normalized {
                match factor.node() {
                    Node::Product(_) | Node::Sum(_) | Node::Poly(_) => {
                        self.powers.truncate(start);
                        return false;
                    }
                    Node::Function { name, .. }
                        if factors.len() > 1 && *name == BuiltinFunction::Sqrt.function() =>
                    {
                        self.powers.truncate(start);
                        return false;
                    }
                    Node::Pow(..) if factors.len() > 1 && factor.positive_factor().is_none() => {
                        self.powers.truncate(start);
                        return false;
                    }
                    Node::Number(_)
                    | Node::ImaginaryUnit
                    | Node::Pi
                    | Node::Euler
                    | Node::Symbol(_)
                    | Node::Function { .. }
                    | Node::Div(..)
                    | Node::Pow(..)
                    | Node::Derivative { .. } => {}
                }
            }
            let Some((base, exponent)) = Self::factor(factor) else {
                self.powers.truncate(start);
                return false;
            };
            let Some(id_u32) = self.base_id(base) else {
                self.powers.truncate(start);
                return false;
            };
            // SAFETY: `start` is at most `self.powers.len()`.
            if unsafe { self.powers.get_unchecked(start..) }
                .iter()
                .any(|&(previous, _)| previous == id_u32)
            {
                self.powers.truncate(start);
                return false;
            }
            self.powers.push((id_u32, exponent));
        }
        self.record_term_powers(source, coefficient, start);
        true
    }

    fn base_id(&mut self, base: &'term Expr) -> Option<u32> {
        let ptr = base.pointer();
        if self.ids_of.is_empty() {
            if let Some(pos) = self
                .distinct
                .iter()
                .position(|&(existing, _, _)| eq(existing.pointer(), ptr))
            {
                return u32::try_from(pos).ok();
            }
            if self.distinct.len() < 8 {
                let id_u32 = u32::try_from(self.distinct.len()).ok()?;
                self.distinct.push((base, 0, false));
                return Some(id_u32);
            }
            self.ids_of.reserve(16);
            for (index, &(existing, _, _)) in self.distinct.iter().enumerate() {
                let existing_ptr = existing.pointer();
                self.ids_of.insert(existing_ptr, index);
                if let Ok(existing_u32) = u32::try_from(index) {
                    let existing_slot = existing_ptr.addr().wrapping_shr(4) & PTR_CACHE_MASK;
                    if let Some(entry) = self.ptr_cache.get_mut(existing_slot) {
                        *entry = (existing_ptr, existing_u32);
                    }
                }
            }
        }
        let slot = ptr.addr().wrapping_shr(4) & PTR_CACHE_MASK;
        if let Some(&(cached_ptr, cached_id)) = self.ptr_cache.get(slot)
            && eq(cached_ptr, ptr)
        {
            return Some(cached_id);
        }
        let id_u32 = match self.ids_of.entry(ptr) {
            Entry::Occupied(known) => u32::try_from(*known.get()).ok()?,
            Entry::Vacant(vacant) => {
                let next = self.distinct.len();
                let next_u32 = u32::try_from(next).ok()?;
                self.distinct.push((base, 0, false));
                vacant.insert(next);
                next_u32
            }
        };
        if let Some(entry) = self.ptr_cache.get_mut(slot) {
            *entry = (ptr, id_u32);
        }
        Some(id_u32)
    }

    #[expect(
        clippy::as_conversions,
        reason = "every `id_u32` pushed since `start` is a valid index into `self.distinct`"
    )]
    fn record_term_powers(&mut self, source: usize, coefficient: &'term Expr, start: usize) {
        // SAFETY: `start` was `self.powers.len()` before pushing non-empty term powers.
        for &(id_u32, exponent) in unsafe { self.powers.get_unchecked(start..) } {
            // SAFETY: every `id_u32` is an index into `self.distinct`.
            let known = unsafe { self.distinct.get_unchecked_mut(id_u32 as usize) };
            if known.1 == 0 {
                known.1 = exponent;
            } else {
                let distinct_exp = known.1 != exponent;
                known.2 |= distinct_exp;
                self.has_distinct_powers |= distinct_exp;
            }
            self.has_higher_power |= exponent > 1;
        }
        self.terms
            .push((source, coefficient, self.powers.len().saturating_sub(start)));
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

    #[expect(
        clippy::as_conversions,
        reason = "base ids in `self.powers` are valid `u32` indices into `self.distinct` and `used`"
    )]
    fn select_candidates(&self, all: bool) -> Option<(Vec<bool>, Vec<bool>)> {
        let mut selected = Vec::with_capacity(self.terms.len());
        let mut used = vec![false; self.distinct.len()];
        let mut remaining = self.powers.as_slice();
        let mut any_selected = false;
        for &(_, _, length) in &self.terms {
            // SAFETY: the term lengths sum to `self.powers.len()`.
            let (term, rest) = unsafe { remaining.split_at_unchecked(length) };
            remaining = rest;
            // SAFETY: every base id is a position in `self.distinct` and in `used`.
            let keep = term
                .iter()
                .any(|&(id, _)| unsafe { self.distinct.get_unchecked(id as usize) }.2);
            if !keep && all {
                return None;
            }
            if keep {
                any_selected = true;
                for &(id, _) in term {
                    // SAFETY: as above.
                    *unsafe { used.get_unchecked_mut(id as usize) } = true;
                }
            }
            selected.push(keep);
        }
        any_selected.then_some((selected, used))
    }

    /// Compact storage built from the candidates that contain a base with
    /// distinct powers, and the sources of those candidates in input order,
    /// recorded unless `all` requires every candidate to contain such a base.
    ///
    /// Each distinct base is hashed once per power, ranked once in structural
    /// order, and copied once into storage. Terms are ordered by their rank
    /// sequences, which is their structural order.
    #[expect(
        clippy::as_conversions,
        reason = "base ids in `self.powers` are valid `u32` indices into `rank_of`, and `first_key` packs two `u32` values"
    )]
    pub fn into_polynomial(mut self, all: bool) -> Option<(Expr, Vec<usize>)> {
        if !self.has_higher_power || !self.has_distinct_powers || self.terms.is_empty() {
            return None;
        }
        let all_bases_distinct = self
            .distinct
            .iter()
            .all(|&(_, _, has_distinct)| has_distinct);
        let (selected, mut ranked_bases): (Vec<bool>, Vec<(&Expr, usize)>) = if all_bases_distinct {
            (
                Vec::new(),
                self.distinct
                    .iter()
                    .enumerate()
                    .map(|(id, &(base, _, _))| (base, id))
                    .collect(),
            )
        } else {
            let (sel, used) = self.select_candidates(all)?;
            let ranked = self
                .distinct
                .iter()
                .zip(&used)
                .enumerate()
                .filter_map(|(id, ((base, _, _), in_use))| in_use.then_some((*base, id)))
                .collect();
            (sel, ranked)
        };
        ranked_bases.sort_unstable_by(|(left, _), (right, _)| Node::compare(left, right));
        if ranked_bases
            .iter()
            .zip(ranked_bases.iter().skip(1))
            .any(|((left, _), (right, _))| Node::compare(left, right) == Ordering::Equal)
        {
            return None;
        }
        let mut rank_of = vec![0_u32; self.distinct.len()];
        let mut rank = 0_u32;
        for (_, id) in &ranked_bases {
            // SAFETY: every id is a position in `self.distinct`.
            *unsafe { rank_of.get_unchecked_mut(*id) } = rank;
            // The table fits in a `u32`, so the last rank is below `u32::MAX`.
            rank = rank.saturating_add(1);
        }
        let table: Vec<Expr> = ranked_bases
            .iter()
            .map(|(base, _)| (*base).clone())
            .collect();
        let mut ordered_terms = Vec::with_capacity(self.terms.len());
        let mut sources = Vec::new();
        let mut total_powers = 0_usize;
        let mut rest = self.powers.as_mut_slice();
        for (term_index, (source, coefficient, length)) in self.terms.into_iter().enumerate() {
            // SAFETY: the term lengths sum to `self.powers.len()`.
            let (term, tail) = unsafe { rest.split_at_mut_unchecked(length) };
            rest = tail;
            if all_bases_distinct || selected.get(term_index).copied().unwrap_or(false) {
                let mut previous = None;
                let mut sorted = true;
                for (id_or_rank, _) in term.iter_mut() {
                    // SAFETY: every id is a position in `rank_of`.
                    let base_rank = *unsafe { rank_of.get_unchecked(*id_or_rank as usize) };
                    if previous.is_some_and(|prev| base_rank < prev) {
                        sorted = false;
                    }
                    previous = Some(base_rank);
                    *id_or_rank = base_rank;
                }
                if !sorted {
                    term.sort_unstable_by_key(|&(base_rank, _)| base_rank);
                }
                let &(first_rank, first_exp) = term.first()?;
                let first_key = u64::from(first_rank).wrapping_shl(32) | u64::from(first_exp);
                total_powers = total_powers.saturating_add(term.len());
                ordered_terms.push((first_key, &*term, coefficient));
                if !all {
                    sources.push(source);
                }
            }
        }
        ordered_terms.sort_unstable_by(|(left_key, left_term, _), (right_key, right_term, _)| {
            left_key.cmp(right_key).then_with(|| {
                // SAFETY: every pushed `term` is non-empty, verified by `term.first()?`.
                let left_tail = unsafe { left_term.get_unchecked(1..) };
                // SAFETY: as above.
                let right_tail = unsafe { right_term.get_unchecked(1..) };
                left_tail.cmp(right_tail)
            })
        });
        if ordered_terms
            .iter()
            .zip(ordered_terms.iter().skip(1))
            .any(|((lk, lt, _), (rk, rt, _))| lk == rk && lt == rt)
        {
            return None;
        }
        let polynomial = Polynomial::new(
            table,
            total_powers,
            ordered_terms
                .into_iter()
                .map(|(_, term, coefficient)| (coefficient.clone(), term.iter().copied())),
        );
        Some((Expr::intern(Node::Poly(Box::new(polynomial))), sources))
    }
}
