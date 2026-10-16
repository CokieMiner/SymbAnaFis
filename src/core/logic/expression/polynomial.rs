//! Canonical multibase polynomial selection and borrowed sparse storage.

use core::{cmp::Ordering, slice::from_ref};

use alloc::{borrow::Cow, vec::Vec};

use crate::collections::{HashMap, HashSet};

use super::{AdditiveOperands, Expr, Node, Polynomial, PolynomialTerm};

struct Candidate {
    source: usize,
    coefficient: Expr,
    powers: Vec<(Expr, u32)>,
}

impl AdditiveOperands for Expr {
    /// Read logical additive operands in canonical structural order.
    ///
    /// Ordinary sums borrow their terms. Compact monomials become canonical handles.
    /// Opaque bases remain factored; this does not distribute products or powers.
    /// Returned terms use the same structural order as general sums.
    ///
    /// # Panics
    /// Numeric folding propagates the backend's rational capacity panics.
    fn additive_operands(&self) -> Option<Cow<'_, [Self]>> {
        match self.node() {
            Node::Sum(terms) if Node::compact_group(terms).is_none() => Some(Cow::Borrowed(terms)),
            Node::Sum(terms) => {
                let mut logical = Vec::new();
                for term in terms {
                    if let Node::Poly(polynomial) = term.node() {
                        logical.extend(Node::mapped_polynomial(polynomial, Clone::clone));
                    } else {
                        logical.push(term.clone());
                    }
                }
                Node::sort_operands(&mut logical);
                Some(Cow::Owned(logical))
            }
            Node::Poly(polynomial) => Some(Cow::Owned(Node::mapped_polynomial(
                polynomial,
                Clone::clone,
            ))),
            Node::Number(_)
            | Node::ImaginaryUnit
            | Node::Pi
            | Node::Euler
            | Node::Symbol(_)
            | Node::Function { .. }
            | Node::Product(_)
            | Node::Div(..)
            | Node::Pow(..)
            | Node::Derivative { .. } => None,
        }
    }
}

impl Polynomial {
    /// Borrow stored DAG edges, including shared numeric coefficient handles.
    pub fn children(&self) -> impl Iterator<Item = &Expr> {
        self.bases
            .iter()
            .chain(self.terms.iter().map(|term| &term.coefficient))
            .chain(
                self.terms
                    .iter()
                    .flat_map(|term| term.powers.iter().map(|(base, _)| base)),
            )
    }
}

impl Node {
    /// Locate the unique compact group in structurally ordered sum operands.
    pub fn compact_group(terms: &[Expr]) -> Option<&Polynomial> {
        let position = terms.partition_point(|term| term.node().rank() < Self::POLYNOMIAL_RANK);
        if let Self::Poly(polynomial) = terms.get(position)?.node() {
            Some(polynomial)
        } else {
            None
        }
    }

    /// Reconstruct monomials after mapping shared bases and coefficients.
    pub fn mapped_polynomial(
        polynomial: &Polynomial,
        mut mapped: impl FnMut(&Expr) -> Expr,
    ) -> Vec<Expr> {
        let mut result: Vec<_> = polynomial
            .terms
            .iter()
            .map(|term| Self::mapped_monomial(term, &mut mapped))
            .collect();
        Self::sort_operands(&mut result);
        result
    }

    /// Reconstruct one monomial without materializing adjacent terms.
    pub fn mapped_monomial(term: &PolynomialTerm, mut mapped: impl FnMut(&Expr) -> Expr) -> Expr {
        let coefficient = mapped(&term.coefficient);
        let mut factors = Vec::new();
        if !coefficient.is_one_num() {
            factors.push(coefficient);
        }
        factors.extend(term.powers.iter().map(|(base, exponent)| {
            let value = mapped(base);
            if *exponent == 1 {
                value
            } else {
                Expr::pow_static(value, Expr::number(i64::from(*exponent)))
            }
        }));
        Expr::product(factors)
    }

    /// Select all monomials involving a repeated positive-power base.
    /// Other terms, including every standalone numeric literal, stay outside.
    pub fn collect_polynomials(terms: Vec<Expr>) -> Vec<Expr> {
        // Distinct powers require at least one explicit power above one.
        // Plain unit-coefficient sums retain their vector without scratch tables.
        if !terms.iter().any(Candidate::has_positive_power) {
            return terms;
        }
        let candidates: Vec<_> = terms
            .iter()
            .enumerate()
            .filter_map(|(index, term)| Candidate::new(index, term))
            .collect();
        let repeated = Candidate::repeated_bases(&candidates);
        if repeated.is_empty() {
            return terms;
        }
        let selected: Vec<_> = candidates
            .into_iter()
            .filter(|candidate| {
                candidate
                    .powers
                    .iter()
                    .any(|(base, _)| repeated.contains(base))
            })
            .collect();
        let mut bases: Vec<_> = selected
            .iter()
            .flat_map(|candidate| candidate.powers.iter().map(|(base, _)| base.clone()))
            .collect();
        Self::sort_operands(&mut bases);
        bases.dedup();
        let mut sources = HashSet::new();
        let mut monomials = Vec::with_capacity(selected.len());
        for candidate in selected {
            sources.insert(candidate.source);
            monomials.push(PolynomialTerm {
                coefficient: candidate.coefficient,
                powers: candidate.powers,
            });
        }
        monomials.sort_unstable_by(|left, right| Self::compare_powers(&left.powers, &right.powers));
        if monomials.windows(2).any(|pair| {
            let [left, right] = pair else {
                return false;
            };
            left.powers == right.powers
        }) {
            // Failed finite coefficient folding can leave repeated monomials.
            // Do not replace those explicit operations with a numeric result.
            return terms;
        }
        let polynomial = Expr::intern(Self::Poly(Polynomial {
            bases,
            terms: monomials,
        }));
        let mut result: Vec<_> = terms
            .into_iter()
            .enumerate()
            .filter_map(|(index, term)| (!sources.contains(&index)).then_some(term))
            .collect();
        result.push(polynomial);
        result
    }

    fn compare_powers(left: &[(Expr, u32)], right: &[(Expr, u32)]) -> Ordering {
        for ((first, first_exponent), (second, second_exponent)) in left.iter().zip(right) {
            let order =
                Self::compare(first, second).then_with(|| first_exponent.cmp(second_exponent));
            if order != Ordering::Equal {
                return order;
            }
        }
        left.len().cmp(&right.len())
    }
}

impl Candidate {
    fn new(source: usize, term: &Expr) -> Option<Self> {
        let (coefficient, factors): (Expr, &[Expr]) = if let Node::Product(factors) = term.node() {
            let (first, rest) = factors.split_first()?;
            if let Some(value) = first.literal() {
                if !value.value().is_finite() || value.value().is_zero() {
                    return None;
                }
                (first.clone(), rest)
            } else {
                (Expr::number(1), factors)
            }
        } else {
            (Expr::number(1), from_ref(term))
        };
        let mut powers: Vec<_> = factors.iter().map(Self::factor).collect::<Option<_>>()?;
        if powers.is_empty() {
            return None;
        }
        powers.sort_unstable_by(|(left, _), (right, _)| Node::compare(left, right));
        if powers.windows(2).any(|pair| {
            let [left, right] = pair else {
                return false;
            };
            left.0.same_node(&right.0)
        }) {
            return None;
        }
        Some(Self {
            source,
            coefficient,
            powers,
        })
    }

    fn factor(factor: &Expr) -> Option<(Expr, u32)> {
        if let Some((base, power)) = Node::positive_factor(factor) {
            if base.literal().is_some() {
                return None;
            }
            return u32::try_from(power)
                .ok()
                .map(|exponent| (base.clone(), exponent));
        }
        // A general power may itself be an opaque base, e.g. a + a^2 where
        // a = x^(1/2). Its inner exponent and branch are not combined or erased.
        matches!(factor.node(), Node::Pow(..)).then(|| (factor.clone(), 1))
    }

    fn has_positive_power(term: &Expr) -> bool {
        let is_power = |factor: &Expr| {
            matches!(factor.node(), Node::Pow(..))
                && Node::positive_factor(factor)
                    .is_some_and(|(_, exponent)| exponent > 1 && u32::try_from(exponent).is_ok())
        };
        if let Node::Product(factors) = term.node() {
            factors.iter().any(is_power)
        } else {
            is_power(term)
        }
    }

    fn repeated_bases(candidates: &[Self]) -> HashSet<Expr> {
        let mut first_powers = HashMap::new();
        let mut repeated = HashSet::new();
        for candidate in candidates {
            for (base, power) in &candidate.powers {
                if let Some(first) = first_powers.insert(base.clone(), *power)
                    && first != *power
                {
                    repeated.insert(base.clone());
                }
            }
        }
        repeated
    }
}
