//! Like-term collection and compact-group preservation in canonical sums.

#![expect(
    unsafe_code,
    reason = "like-term group indices are assigned by the vectors they read, so they need no bounds checks"
)]

use core::slice::from_ref;

use alloc::{vec, vec::Vec};

use num_anafis::Number;

use crate::collections::{Entry, FixedMap};

use super::{ExactIdentity, Expr, Node, NumericValue};

impl Node {
    /// Combine like terms, which share the factors that follow their finite
    /// coefficient. Groups are keyed by those borrowed factors, so no base
    /// product is built for a term without like terms. Finite literals stay
    /// in place; failed coefficient folds keep every original term.
    pub fn collect_like_terms(normalized: &mut Vec<Expr>) {
        let extracted: Vec<Expr> = normalized
            .extract_if(.., |term| {
                term.literal()
                    .is_none_or(|value| !value.value().is_finite())
            })
            .collect();
        let mut group_of = Vec::with_capacity(extracted.len());
        let mut sizes: Vec<usize> = Vec::new();
        let mut groups: FixedMap<&[Expr], usize> = FixedMap::default();
        for term in &extracted {
            let group = match groups.entry(Self::like_factors(term)) {
                Entry::Occupied(known) => *known.get(),
                Entry::Vacant(vacant) => {
                    let fresh = sizes.len();
                    vacant.insert(fresh);
                    sizes.push(0);
                    fresh
                }
            };
            // SAFETY: `group` is a recorded group index, below `sizes.len()`.
            let size = unsafe { sizes.get_unchecked_mut(group) };
            *size = size.saturating_add(1);
            group_of.push(group);
        }
        drop(groups);
        let mut repeated: Vec<Vec<Expr>> = sizes
            .iter()
            .map(|&size| {
                if size > 1 {
                    Vec::with_capacity(size)
                } else {
                    Vec::new()
                }
            })
            .collect();
        for (term, group) in extracted.into_iter().zip(group_of) {
            // SAFETY: `group` is below `sizes.len()`, and `repeated` has one
            // vector per group.
            let (size, members) = unsafe {
                (
                    *sizes.get_unchecked(group),
                    repeated.get_unchecked_mut(group),
                )
            };
            if size > 1 {
                members.push(term);
            } else {
                normalized.push(term);
            }
        }
        for members in repeated.into_iter().filter(|members| !members.is_empty()) {
            let unit = NumericValue::new(Number::one());
            let mut coefficients: Vec<_> = members
                .iter()
                .map(|term| Self::coefficient_parts(term).0.unwrap_or(&unit))
                .collect();
            coefficients.sort_unstable_by(|left, right| left.compare(right));
            let folded = Self::finite_fold(coefficients.into_iter(), true);
            if let Some(value) = folded
                && let Some(first) = members.first()
            {
                let factors = Self::like_factors(first);
                let mut collected = Vec::with_capacity(factors.len().saturating_add(1));
                collected.push(Expr::number(value));
                collected.extend_from_slice(factors);
                normalized.push(Self::normalize_product(collected));
            } else {
                normalized.extend(members);
            }
        }
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
    fn like_factors(term: &Expr) -> &[Expr] {
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
