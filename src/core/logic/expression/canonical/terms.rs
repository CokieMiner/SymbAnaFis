//! Like-term collection and compact-group preservation in canonical sums.

use core::{mem::take, slice::from_ref};

use alloc::{vec, vec::Vec};

use num_anafis::Number;

use crate::collections::{Entry, FixedMap, FixedState};

use super::{ExactIdentity, Expr, Node, NumericValue};

enum GroupTag {
    Single(usize),
    Repeated(usize),
}

impl Node {
    /// Combine like terms, which share the factors that follow their finite
    /// coefficient. Groups are keyed by those borrowed factors, so no base
    /// product is built for a term without like terms. Finite literals stay
    /// in place; overflowing coefficient sums fold as far as checked arithmetic
    /// permits and retain every unfolded term. Returns `true` if any terms
    /// were combined.
    pub fn collect_like_terms(normalized: &mut Vec<Expr>) -> bool {
        if normalized.len() <= 1 {
            return false;
        }
        if normalized.len() <= 8 {
            let mut seen_inline: [&[Expr]; 8] = [&[]; 8];
            let mut seen_len = 0_usize;
            let mut has_duplicate = false;
            for term in normalized.iter() {
                if term
                    .literal()
                    .is_some_and(|value| value.value().is_finite())
                {
                    continue;
                }
                let factors = Self::like_factors(term);
                if seen_inline
                    .get(..seen_len)
                    .is_some_and(|prefix| prefix.contains(&factors))
                {
                    has_duplicate = true;
                    break;
                }
                if let Some(slot) = seen_inline.get_mut(seen_len) {
                    *slot = factors;
                    seen_len = seen_len.saturating_add(1);
                }
            }
            if !has_duplicate {
                return false;
            }
        }
        let mut groups: FixedMap<&[Expr], GroupTag> =
            FixedMap::with_capacity_and_hasher(normalized.len(), FixedState::default());
        let mut duplicates: Vec<(usize, usize)> = Vec::new();
        let mut num_dup_groups = 0_usize;
        for (index, term) in normalized.iter().enumerate() {
            if term
                .literal()
                .is_some_and(|value| value.value().is_finite())
            {
                continue;
            }
            match groups.entry(Self::like_factors(term)) {
                Entry::Vacant(vacant) => {
                    vacant.insert(GroupTag::Single(index));
                }
                Entry::Occupied(mut known) => match *known.get() {
                    GroupTag::Single(first_index) => {
                        let dup_id = num_dup_groups;
                        num_dup_groups = num_dup_groups.saturating_add(1);
                        *known.get_mut() = GroupTag::Repeated(dup_id);
                        duplicates.push((first_index, dup_id));
                        duplicates.push((index, dup_id));
                    }
                    GroupTag::Repeated(dup_id) => {
                        duplicates.push((index, dup_id));
                    }
                },
            }
        }
        drop(groups);
        if duplicates.is_empty() {
            return false;
        }
        duplicates.sort_unstable_by_key(|&(index, _)| index);
        let mut repeated: Vec<Vec<Expr>> = vec![Vec::new(); num_dup_groups];
        let old = take(normalized);
        normalized.reserve(old.len());
        let mut dup_iter = duplicates.into_iter().peekable();
        for (index, term) in old.into_iter().enumerate() {
            if let Some((_, dup_id)) = dup_iter.next_if(|&(dup_idx, _)| dup_idx == index) {
                if let Some(bucket) = repeated.get_mut(dup_id) {
                    bucket.push(term);
                }
            } else {
                normalized.push(term);
            }
        }
        let unit = NumericValue::new(Number::one());
        let mut combined_any = false;
        for members in repeated {
            if Self::combine_repeated_group(members, &unit, normalized) {
                combined_any = true;
            }
        }
        combined_any
    }

    fn combine_repeated_group(
        members: Vec<Expr>,
        unit: &NumericValue,
        normalized: &mut Vec<Expr>,
    ) -> bool {
        if members.is_empty() {
            return false;
        }
        let mut coefficients: Vec<_> = members
            .iter()
            .map(|term| Self::coefficient_parts(term).0.unwrap_or(unit))
            .collect();
        coefficients.sort_unstable_by(|left, right| left.compare(right));
        if let Some(value) = Self::finite_fold(coefficients.iter().copied(), true)
            && let Some(first) = members.first()
        {
            let factors = Self::like_factors(first);
            let mut collected = Vec::with_capacity(factors.len().saturating_add(1));
            collected.push(Expr::number(value));
            collected.extend_from_slice(factors);
            normalized.push(Self::normalize_product(collected));
            return true;
        }
        let buckets = Self::fold_partial_literals(coefficients.into_iter(), true);
        if buckets.len() < members.len()
            && let Some(first) = members.first()
        {
            let factors = Self::like_factors(first);
            for value in buckets {
                let mut collected = Vec::with_capacity(factors.len().saturating_add(1));
                collected.push(Expr::number(value));
                collected.extend_from_slice(factors);
                normalized.push(Self::normalize_product(collected));
            }
            return true;
        }
        normalized.extend(members);
        false
    }

    /// Keep the unique compact group of a sum compact when no other operand
    /// shares one of its bases and no base of the other operands occurs with
    /// distinct powers. Then no like term combines with a monomial and no
    /// monomial joins the group, so the sum is the group beside the normalized
    /// other operands. Otherwise the flattened operands are returned.
    pub fn keep_compact_group(terms: Vec<Expr>) -> Result<Expr, Vec<Expr>> {
        if !terms.iter().any(|term| match term.node() {
            Self::Poly(_) => true,
            Self::Sum(children) => Self::compact_group(children).is_some(),
            Self::Number(_)
            | Self::ImaginaryUnit
            | Self::Pi
            | Self::Euler
            | Self::Symbol(_)
            | Self::Function { .. }
            | Self::Product(_)
            | Self::Div(..)
            | Self::Pow(..)
            | Self::Derivative { .. } => false,
        }) {
            return Err(terms);
        }
        let mut others = Vec::with_capacity(terms.len());
        let mut found = None;
        let mut groups = 0_usize;
        for term in &terms {
            let operands = if let Self::Sum(children) = term.node() {
                children.as_slice()
            } else {
                from_ref(term)
            };
            for operand in operands {
                if matches!(operand.node(), Self::Poly(_)) {
                    groups = groups.saturating_add(1);
                    found = Some(operand);
                } else {
                    others.push(operand.clone());
                }
            }
        }
        let (Some(group), 1) = (found, groups) else {
            return Err(terms);
        };
        let Self::Poly(polynomial) = group.node() else {
            return Err(terms);
        };
        // Bases of the other operands with their powers, keyed by identity; an
        // opaque factor is its own base with power one. Each factor occurrence
        // and each base of the group costs one lookup.
        let mut bases: FixedMap<&Expr, i64> = FixedMap::default();
        for term in &others {
            for factor in Self::like_factors(term) {
                let (base, power) = match factor.positive_factor() {
                    Some(power) => power,
                    None if matches!(factor.node(), Self::Pow(..)) => (factor, 1),
                    None => continue,
                };
                if *bases.entry(base).or_insert(power) != power {
                    return Err(terms);
                }
            }
        }
        if !bases.is_empty() && polynomial.bases.iter().any(|base| bases.contains_key(base)) {
            return Err(terms);
        }
        drop(bases);
        let compact = group.clone();
        drop(terms);
        let rest = Self::normalize_sum(others);
        let mut operands = if let Self::Sum(children) = rest.node() {
            children.clone()
        } else {
            vec![rest]
        };
        operands.push(compact);
        if operands.len() > 1 {
            operands.retain(|term| !term.is_exact_zero());
        }
        Self::sort_operands(&mut operands);
        Ok(Self::finish(operands, true))
    }

    /// The factors that like terms share: a product's factors after a leading
    /// finite coefficient, or the term itself.
    pub fn like_factors(term: &Expr) -> &[Expr] {
        match term.node() {
            Self::Product(factors) => match factors.split_first() {
                Some((first, rest))
                    if first
                        .literal()
                        .is_some_and(|value| value.value().is_finite()) =>
                {
                    rest
                }
                _ => factors,
            },
            Self::Number(_)
            | Self::ImaginaryUnit
            | Self::Pi
            | Self::Euler
            | Self::Symbol(_)
            | Self::Function { .. }
            | Self::Sum(_)
            | Self::Poly(_)
            | Self::Div(..)
            | Self::Pow(..)
            | Self::Derivative { .. } => from_ref(term),
        }
    }
}
