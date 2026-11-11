//! Canonical multibase polynomial selection and borrowed sparse storage.

#![expect(
    unsafe_code,
    reason = "term lengths partition the power list and base positions index the base table, both by construction"
)]

use core::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    iter::{FusedIterator, Zip},
    mem::size_of_val,
    slice::{Iter, from_raw_parts},
};

use alloc::{borrow::Cow, vec::Vec};

use super::{
    AdditiveOperands, BuiltinFunction, ExactIdentity, Expr, Node, NumericValue, Polynomial,
};

use candidate::Candidates;

mod candidate;
mod derivative;
mod logical;
mod rebuild;

pub use logical::Projection;

/// Outcome of attempting to pack an entire sum directly into compact storage.
#[derive(Clone)]
pub enum MonomialStatus {
    /// Every term joined one compact polynomial node.
    Packed(Expr),
    /// A subset of terms may still form compact storage after normalization.
    RetryAfterNormalize,
    /// Every term was inspected and no base appeared with distinct positive powers.
    NoDistinctPowers,
}

/// The terms of compact storage ordered lexicographically by their
/// `(base_position, exponent)` power lists (monomial power order): each
/// coefficient with its powers as base positions and exponents.
///
/// `Polynomial::projected()` presents the same terms as canonical products in
/// `Node::compare` structural order.
#[derive(Clone)]
pub struct Terms<'storage> {
    terms: Zip<Iter<'storage, Expr>, Iter<'storage, usize>>,
    /// The powers of the remaining terms, whose lengths sum to this length.
    powers: &'storage [(u32, u32)],
}

/// The monomials of compact storage ordered lexicographically by their
/// `(base_position, exponent)` power lists (monomial power order), with bases
/// resolved.
///
/// `Polynomial::projected()` presents the same terms as canonical products in
/// `Node::compare` structural order.
#[derive(Clone)]
pub struct Monomials<'storage> {
    terms: Terms<'storage>,
    bases: &'storage [Expr],
}

/// The bases and positive exponents of one monomial in compact storage.
#[derive(Clone)]
pub struct Powers<'storage> {
    powers: Iter<'storage, (u32, u32)>,
    bases: &'storage [Expr],
}

impl AdditiveOperands for Expr {
    /// The logical terms in canonical structural order, or `None` for a
    /// non-sum. Ordinary sums and compact storage borrow their terms; a sum
    /// with a compact group lists the group's projected terms among its own.
    /// Products and powers are not distributed.
    fn additive_operands(&self) -> Option<Cow<'_, [Self]>> {
        match self.node() {
            Node::Sum(terms) => Some(match Node::compact_group(terms) {
                None => Cow::Borrowed(terms),
                // The other operands and the projected terms are each in
                // structural order, so one merge orders them.
                Some((group, polynomial)) => Cow::Owned(Node::merge_operands(
                    terms.iter().filter(|term| *term != group),
                    polynomial.projected().iter(),
                )),
            }),
            Node::Poly(polynomial) => Some(Cow::Borrowed(polynomial.projected())),
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

    fn additive_order(&self, other: &Self) -> Ordering {
        Node::compare(self, other)
    }
}

impl Polynomial {
    /// Compact storage from distinct bases in strict structural order and from
    /// stored terms ordered lexicographically by their `(base_position,
    /// exponent)` power lists (monomial power order), whose `count` powers in
    /// all name those bases by position. `Polynomial::projected()` presents the
    /// terms in canonical `Node::compare` structural order.
    #[expect(
        clippy::as_conversions,
        reason = "the debug assertion widens a stored base position to a slice index"
    )]
    fn new<Term: ExactSizeIterator<Item = (u32, u32)>>(
        bases: Vec<Expr>,
        count: usize,
        terms: impl ExactSizeIterator<Item = (Expr, Term)>,
    ) -> Self {
        let simple_bases = bases
            .iter()
            .all(|base| matches!(base.node(), Node::Symbol(_) | Node::Pi | Node::Euler));
        let mut coefficients = Vec::with_capacity(terms.len());
        let mut lengths = Vec::with_capacity(terms.len());
        let mut powers = Vec::with_capacity(count);
        for (coefficient, term) in terms {
            coefficients.push(coefficient);
            lengths.push(term.len());
            powers.extend(term);
        }
        debug_assert!(
            powers.iter().all(|&(position, exponent)| exponent == 1
                || bases
                    .get(position as usize)
                    .is_some_and(|base| !matches!(base.node(), Node::Euler))),
            "a power of e above one is stored as exp"
        );
        Self {
            bases,
            coefficients,
            lengths,
            powers,
            simple_bases,
            projection: Projection::default(),
        }
    }

    /// The stored terms in monomial power order, with their powers as base positions.
    pub fn terms(&self) -> Terms<'_> {
        Terms {
            terms: self.coefficients.iter().zip(&self.lengths),
            powers: &self.powers,
        }
    }

    /// The stored monomials in monomial power order, with their bases resolved.
    pub fn monomials(&self) -> Monomials<'_> {
        Monomials {
            terms: self.terms(),
            bases: &self.bases,
        }
    }
}

/// Identity reads the storage alone: the base flag and the projection are
/// functions of it.
impl PartialEq for Polynomial {
    fn eq(&self, other: &Self) -> bool {
        self.bases == other.bases
            && self.coefficients == other.coefficients
            && self.lengths == other.lengths
            && self.powers == other.powers
    }
}

impl Eq for Polynomial {}

impl Hash for Polynomial {
    fn hash<HasherType: Hasher>(&self, state: &mut HasherType) {
        const {
            assert!(
                size_of::<(u32, u32)>() == 8,
                "`(u32, u32)` has no padding bytes"
            );
        }
        self.bases.hash(state);
        self.coefficients.hash(state);
        self.lengths.hash(state);
        state.write_usize(self.powers.len());
        // SAFETY: `(u32, u32)` is two adjacent `u32` values with no padding,
        // so `self.powers` is a contiguous initialized byte slice of length
        // `size_of_val(self.powers.as_slice())`.
        let bytes = unsafe {
            from_raw_parts(
                self.powers.as_ptr().cast::<u8>(),
                size_of_val(self.powers.as_slice()),
            )
        };
        state.write(bytes);
    }
}

impl<'storage> Iterator for Terms<'storage> {
    type Item = (&'storage Expr, &'storage [(u32, u32)]);

    fn next(&mut self) -> Option<Self::Item> {
        let (coefficient, &length) = self.terms.next()?;
        // SAFETY: the lengths of the remaining terms sum to the remaining
        // powers, so the first length is at most their count.
        let (term, rest) = unsafe { self.powers.split_at_unchecked(length) };
        self.powers = rest;
        Some((coefficient, term))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.terms.size_hint()
    }
}

impl DoubleEndedIterator for Terms<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let (coefficient, &length) = self.terms.next_back()?;
        // SAFETY: the lengths of the remaining terms sum to the remaining
        // powers, so the last length is at most their count.
        let (rest, term) = unsafe {
            self.powers
                .split_at_unchecked(self.powers.len().unchecked_sub(length))
        };
        self.powers = rest;
        Some((coefficient, term))
    }
}

impl ExactSizeIterator for Terms<'_> {}

impl FusedIterator for Terms<'_> {}

impl<'storage> Iterator for Monomials<'storage> {
    type Item = (&'storage Expr, Powers<'storage>);

    fn next(&mut self) -> Option<Self::Item> {
        let (coefficient, powers) = self.terms.next()?;
        Some((
            coefficient,
            Powers {
                powers: powers.iter(),
                bases: self.bases,
            },
        ))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.terms.size_hint()
    }
}

impl DoubleEndedIterator for Monomials<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let (coefficient, powers) = self.terms.next_back()?;
        Some((
            coefficient,
            Powers {
                powers: powers.iter(),
                bases: self.bases,
            },
        ))
    }
}

impl ExactSizeIterator for Monomials<'_> {}

impl FusedIterator for Monomials<'_> {}

impl<'storage> Iterator for Powers<'storage> {
    type Item = (&'storage Expr, u32);

    #[expect(
        clippy::as_conversions,
        reason = "a stored base position is a `u32` and widens losslessly to a slice index"
    )]
    fn next(&mut self) -> Option<Self::Item> {
        let &(position, exponent) = self.powers.next()?;
        // SAFETY: the base positions of a polynomial index its base table.
        Some((
            unsafe { self.bases.get_unchecked(position as usize) },
            exponent,
        ))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.powers.size_hint()
    }
}

impl DoubleEndedIterator for Powers<'_> {
    #[expect(
        clippy::as_conversions,
        reason = "a stored base position is a `u32` and widens losslessly to a slice index"
    )]
    fn next_back(&mut self) -> Option<Self::Item> {
        let &(position, exponent) = self.powers.next_back()?;
        // SAFETY: the base positions of a polynomial index its base table.
        Some((
            unsafe { self.bases.get_unchecked(position as usize) },
            exponent,
        ))
    }
}

impl ExactSizeIterator for Powers<'_> {}

impl FusedIterator for Powers<'_> {}

/// Construction and differentiation of compact polynomial storage, for the
/// parser, differentiation, and sparse polynomial results. It stays out of the
/// public expression API.
pub trait CompactStorage: Sized {
    /// The sum of the products partitioned in `factors` by ascending `ends`.
    ///
    /// Distinct monomials go directly to compact storage without intermediate
    /// product nodes. Other shapes, bare additive factors included, use
    /// canonical product and sum construction.
    fn sum_of_products(factors: Vec<Self>, ends: Vec<usize>) -> Self;

    /// The sum of non-constant monomials given as indexed `(base_position, exponent)`
    /// slices partitioned in `powers` by `lengths`, over `bases`.
    fn sum_of_indexed_monomials(
        bases: &[Self],
        coefficients: Vec<Self>,
        lengths: Vec<usize>,
        powers: Vec<(u32, u32)>,
    ) -> Self;

    /// The partial derivative
    /// $\partial_{x}\sum_{j} c_{j} x^{k_{j}}R_{j}=\sum_{k_{j}>0} c_{j}k_{j} x^{k_{j}-1}R_{j}$
    /// of compact storage whose bases are symbols, $\pi$, and $e$, with
    /// respect to the symbol `variable`; zero when `variable` is not a base.
    /// `None` for any other expression.
    ///
    /// Equals the canonical sum of the monomials' products, where $c_{j}k_{j}$
    /// folds as product construction folds it.
    fn polynomial_derivative(&self, variable: &Self) -> Option<Self>;
}

impl CompactStorage for Expr {
    fn sum_of_products(factors: Vec<Self>, ends: Vec<usize>) -> Self {
        if ends.is_empty() {
            return Self::from(0_i64);
        }
        if factors.iter().any(Node::has_positive_power) {
            let one = Self::from(1_i64);
            let mut candidates = Candidates::with_capacity(ends.len());
            let mut start = 0_usize;
            let mut all_ok = true;
            for (source, &end) in ends.iter().enumerate() {
                let Some(slice) = factors.get(start..end) else {
                    all_ok = false;
                    break;
                };
                start = end;
                if !candidates.push_factors(source, slice, &one) {
                    all_ok = false;
                    break;
                }
            }
            if all_ok
                && start == factors.len()
                && let Some((polynomial, _)) = candidates.into_polynomial(true)
            {
                return polynomial;
            }
        }
        let mut iter = factors.into_iter();
        let mut prev = 0_usize;
        let mut terms = Vec::with_capacity(ends.len());
        for end in ends {
            let len = end.saturating_sub(prev);
            prev = end;
            let mut slice = Vec::with_capacity(len);
            for _ in 0..len {
                if let Some(factor) = iter.next() {
                    slice.push(factor);
                }
            }
            terms.push(Self::product(slice));
        }
        Self::sum(terms)
    }

    #[expect(
        clippy::as_conversions,
        reason = "base positions are `u32` indices into `bases`"
    )]
    fn sum_of_indexed_monomials(
        bases: &[Self],
        coefficients: Vec<Self>,
        lengths: Vec<usize>,
        powers: Vec<(u32, u32)>,
    ) -> Self {
        if coefficients.is_empty() {
            return Self::from(0_i64);
        }
        let mut candidates = Candidates::with_capacity(coefficients.len());
        let mut rest = powers.as_slice();
        let mut all_ok = coefficients.len() == lengths.len();
        if all_ok {
            for (source, (coeff, &len)) in coefficients.iter().zip(&lengths).enumerate() {
                let Some((term_powers, tail)) = rest.split_at_checked(len) else {
                    all_ok = false;
                    break;
                };
                rest = tail;
                if !candidates.push_indexed(source, coeff, bases, term_powers) {
                    all_ok = false;
                    break;
                }
            }
        }
        if all_ok
            && rest.is_empty()
            && let Some((polynomial, _)) = candidates.into_polynomial(true)
        {
            return polynomial;
        }
        let mut power_rest = powers.as_slice();
        let mut terms = Vec::with_capacity(coefficients.len());
        for (coeff, len) in coefficients.into_iter().zip(lengths) {
            let Some((term_powers, tail)) = power_rest.split_at_checked(len) else {
                break;
            };
            power_rest = tail;
            let resolved = term_powers
                .iter()
                .filter_map(|&(pos, exp)| bases.get(pos as usize).map(|base| (base.clone(), exp)));
            terms.push(Self::product(Node::monomial_factors(coeff, resolved)));
        }
        Self::sum(terms)
    }

    fn polynomial_derivative(&self, variable: &Self) -> Option<Self> {
        let Node::Poly(polynomial) = self.node() else {
            return None;
        };
        if !polynomial.simple_bases {
            return None;
        }
        // Distinct scalar bases do not depend on one another.
        Some(
            polynomial
                .bases
                .iter()
                .position(|base| base == variable)
                .map_or_else(|| Self::from(0_i64), |target| polynomial.derivative(target)),
        )
    }
}

impl Node {
    /// Inspect `terms` once: pack directly into compact storage when every term
    /// is a distinct monomial containing a distinct-power base, or report
    /// whether no base had distinct positive powers so `normalize_sum` can skip
    /// a second `Candidates` pass when no like terms combined.
    pub fn try_monomial_polynomial(terms: &[Expr]) -> MonomialStatus {
        if terms
            .iter()
            .any(|term| matches!(term.node(), Self::Sum(_) | Self::Poly(_)))
        {
            return MonomialStatus::RetryAfterNormalize;
        }
        if !terms.iter().any(Self::has_positive_power) {
            return MonomialStatus::NoDistinctPowers;
        }
        let one = Expr::from(1_i64);
        let mut candidates = Candidates::with_capacity(terms.len());
        let mut all_valid = true;
        for (source, term) in terms.iter().enumerate() {
            if !candidates.push_term(source, term, &one) {
                all_valid = false;
            }
        }
        if !candidates.has_distinct_powers() {
            return MonomialStatus::NoDistinctPowers;
        }
        if all_valid && let Some((polynomial, _)) = candidates.into_polynomial(true) {
            return MonomialStatus::Packed(polynomial);
        }
        MonomialStatus::RetryAfterNormalize
    }

    /// Select all monomials involving a repeated positive-power base.
    /// Other terms, including every standalone numeric literal, stay outside.
    pub fn collect_polynomials(terms: Vec<Expr>) -> Vec<Expr> {
        // Distinct powers require at least one explicit power above one.
        // Plain unit-coefficient sums retain their vector without scratch tables.
        if !terms.iter().any(Self::has_positive_power) {
            return terms;
        }
        let one = Expr::number(1);
        let mut candidates = Candidates::with_capacity(terms.len());
        for (index, term) in terms.iter().enumerate() {
            let _ = candidates.push_term(index, term, &one);
        }
        // Failed finite coefficient folding can leave repeated monomials.
        // Do not replace those explicit operations with a numeric result.
        let Some((polynomial, sources)) = candidates.into_polynomial(false) else {
            return terms;
        };
        let mut skipped = sources.into_iter().peekable();
        let mut result = Vec::with_capacity(terms.len());
        for (index, term) in terms.into_iter().enumerate() {
            if skipped.next_if_eq(&index).is_none() {
                result.push(term);
            }
        }
        result.push(polynomial);
        result
    }

    /// The factors of $c \prod_{i} b_{i}^{k_{i}}$: the coefficient unless it is
    /// the exact unit, then each base raised to its positive exponent.
    pub fn monomial_factors<Base: Into<Expr>>(
        coefficient: Expr,
        powers: impl IntoIterator<Item = (Base, u32)>,
    ) -> Vec<Expr> {
        let pairs = powers.into_iter();
        let mut factors = Vec::with_capacity(pairs.size_hint().0.saturating_add(1));
        if !coefficient.is_exact_one() {
            factors.push(coefficient);
        }
        factors.extend(pairs.map(|(base, exponent)| {
            let handle = base.into();
            if exponent == 1 {
                handle
            } else {
                Self::normalize_power(handle, Expr::number(i64::from(exponent)))
            }
        }));
        factors
    }

    /// Return whether the term contains a positive integer power above one.
    pub fn has_positive_power(term: &Expr) -> bool {
        let is_power = |factor: &Expr| {
            matches!(factor.node(), Self::Pow(..))
                && factor
                    .positive_factor()
                    .is_some_and(|(_, exponent)| exponent > 1 && u32::try_from(exponent).is_ok())
        };
        if let Self::Product(factors) = term.node() {
            factors.iter().any(is_power)
        } else {
            is_power(term)
        }
    }

    /// Locate the unique compact group in structurally ordered sum operands,
    /// as the operand and its storage. Structural order places it after the
    /// lower-ranked operands, so a binary search finds it.
    pub fn compact_group(terms: &[Expr]) -> Option<(&Expr, &Polynomial)> {
        let position = terms.partition_point(|term| term.node().rank() < Self::POLYNOMIAL_RANK);
        let group = terms.get(position)?;
        if let Self::Poly(polynomial) = group.node() {
            Some((group, polynomial))
        } else {
            None
        }
    }
}
