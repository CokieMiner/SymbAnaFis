//! Canonical multibase polynomial selection and borrowed sparse storage.

#![expect(
    unsafe_code,
    reason = "term lengths partition the power list and base positions index the base table, both by construction"
)]

use core::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    iter::{FusedIterator, Zip},
    slice::Iter,
};

use alloc::{borrow::Cow, vec::Vec};

use super::{AdditiveOperands, ExactIdentity, Expr, Node, NumericValue, Polynomial};

use candidate::Candidate;

mod candidate;
mod derivative;
mod logical;

pub use logical::Projection;

/// A coefficient and positive powers with distinct bases, owned or borrowed.
type Monomial<Base> = (Expr, Vec<(Base, u32)>);

/// The terms of compact storage in structural order: each coefficient with its
/// powers as base positions and exponents.
#[derive(Clone)]
pub struct Terms<'storage> {
    terms: Zip<Iter<'storage, Expr>, Iter<'storage, usize>>,
    /// The powers of the remaining terms, whose lengths sum to this length.
    powers: &'storage [(usize, u32)],
}

/// The monomials of compact storage in structural order, with bases resolved.
#[derive(Clone)]
pub struct Monomials<'storage> {
    terms: Terms<'storage>,
    bases: &'storage [Expr],
}

/// The bases and positive exponents of one monomial in compact storage.
#[derive(Clone)]
pub struct Powers<'storage> {
    powers: Iter<'storage, (usize, u32)>,
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
    /// terms in structural order, whose `count` powers in all name those bases
    /// by position.
    fn new<Term: ExactSizeIterator<Item = (usize, u32)>>(
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
                    .get(position)
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

    /// The terms with their powers as base positions.
    pub fn terms(&self) -> Terms<'_> {
        Terms {
            terms: self.coefficients.iter().zip(&self.lengths),
            powers: &self.powers,
        }
    }

    /// The monomials with their bases resolved.
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
        self.bases.hash(state);
        self.coefficients.hash(state);
        self.lengths.hash(state);
        self.powers.hash(state);
    }
}

impl<'storage> Iterator for Terms<'storage> {
    type Item = (&'storage Expr, &'storage [(usize, u32)]);

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

    fn next(&mut self) -> Option<Self::Item> {
        let &(position, exponent) = self.powers.next()?;
        // SAFETY: the base positions of a polynomial index its base table.
        Some((unsafe { self.bases.get_unchecked(position) }, exponent))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.powers.size_hint()
    }
}

impl DoubleEndedIterator for Powers<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let &(position, exponent) = self.powers.next_back()?;
        // SAFETY: the base positions of a polynomial index its base table.
        Some((unsafe { self.bases.get_unchecked(position) }, exponent))
    }
}

impl ExactSizeIterator for Powers<'_> {}

impl FusedIterator for Powers<'_> {}

/// Construction and differentiation of compact polynomial storage, for the
/// parser, differentiation, and sparse polynomial results. It stays out of the
/// public expression API.
pub trait CompactStorage: Sized {
    /// The sum of the products of each factor slice.
    ///
    /// Distinct monomials go directly to compact storage without intermediate
    /// product nodes. Other shapes, bare additive factors included, use
    /// canonical product and sum construction.
    fn sum_of_products(products: Vec<Vec<Self>>) -> Self;

    /// The partial derivative
    /// \(\partial_{x}\sum_{j} c_{j}\,x^{k_{j}}R_{j}=\sum_{k_{j}>0} c_{j}k_{j}\,x^{k_{j}-1}R_{j}\)
    /// of compact storage whose bases are symbols, \(\pi\), and \(e\), with
    /// respect to the symbol `variable`; zero when `variable` is not a base.
    /// `None` for any other expression.
    ///
    /// Equals the canonical sum of the monomials' products, where \(c_{j}k_{j}\)
    /// folds as product construction folds it.
    fn polynomial_derivative(&self, variable: &Self) -> Option<Self>;
}

impl CompactStorage for Expr {
    fn sum_of_products(products: Vec<Vec<Self>>) -> Self {
        match Node::try_monomial_products_owned(products) {
            Ok(polynomial) => polynomial,
            Err(fallback) => Self::sum(fallback.into_iter().map(Self::product).collect()),
        }
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
                .map_or_else(|| Self::number(0), |target| polynomial.derivative(target)),
        )
    }
}

impl Node {
    fn try_monomial_products_owned(products: Vec<Vec<Expr>>) -> Result<Expr, Vec<Vec<Expr>>> {
        if products.iter().any(|factors| {
            matches!(factors.as_slice(), [term]
                if matches!(term.node(), Self::Sum(_) | Self::Poly(_)))
        }) || !products.iter().flatten().any(Self::has_positive_power)
        {
            return Err(products);
        }
        let mut candidates = Vec::with_capacity(products.len());
        let mut source_products = products.into_iter().enumerate();
        let one = Expr::number(1);
        while let Some((source, factors)) = source_products.next() {
            match Candidate::from_factors_owned(source, factors, &one) {
                Ok(candidate) => candidates.push(candidate),
                Err(rejected) => {
                    let mut restored: Vec<_> = candidates
                        .into_iter()
                        .map(Candidate::into_factors)
                        .collect();
                    restored.push(rejected);
                    restored.extend(source_products.map(|(_, rest)| rest));
                    return Err(restored);
                }
            }
        }
        Candidate::into_polynomial(candidates, true)
            .map(|(polynomial, _)| polynomial)
            .map_err(|monomials| {
                monomials
                    .into_iter()
                    .map(|(coefficient, powers)| Self::monomial_factors(coefficient, powers))
                    .collect()
            })
    }

    /// One compact polynomial when every term is a distinct monomial, the usual
    /// shape of a parsed expanded sum. `None` leaves numeric terms, repeated
    /// monomials, opaque sums, and terms unfit for polynomial storage to the
    /// general grouping path.
    pub fn try_monomial_polynomial(terms: &[Expr]) -> Option<Expr> {
        if terms
            .iter()
            .any(|term| matches!(term.node(), Self::Sum(_) | Self::Poly(_)))
            || !terms.iter().any(Self::has_positive_power)
        {
            return None;
        }
        let mut candidates = Vec::with_capacity(terms.len());
        let one = Expr::number(1);
        for (source, term) in terms.iter().enumerate() {
            candidates.push(Candidate::new(source, term, &one)?);
        }
        Candidate::into_polynomial(candidates, true)
            .ok()
            .map(|(polynomial, _)| polynomial)
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
        let candidates: Vec<_> = terms
            .iter()
            .enumerate()
            .filter_map(|(index, term)| Candidate::new(index, term, &one))
            .collect();
        // Failed finite coefficient folding can leave repeated monomials.
        // Do not replace those explicit operations with a numeric result.
        let Ok((polynomial, sources)) = Candidate::into_polynomial(candidates, false) else {
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

    /// Reconstruct the monomials from mapped bases and coefficients, in
    /// structural order. `mapped` receives each child's ordinal among the
    /// stored edges, which list every base and then every coefficient.
    pub fn mapped_polynomial(
        polynomial: &Polynomial,
        mut mapped: impl FnMut(usize, &Expr) -> Expr,
    ) -> Vec<Expr> {
        let bases: Vec<_> = polynomial
            .bases
            .iter()
            .enumerate()
            .map(|(ordinal, base)| mapped(ordinal, base))
            .collect();
        let mut result: Vec<_> = polynomial
            .terms()
            .zip(bases.len()..)
            .map(|((coefficient, powers), ordinal)| {
                let factors = Self::monomial_factors(
                    mapped(ordinal, coefficient),
                    powers.iter().map(|&(position, exponent)| {
                        // SAFETY: `bases` maps the base table one to one, and
                        // the base positions of a polynomial index that table.
                        (unsafe { bases.get_unchecked(position) }, exponent)
                    }),
                );
                Expr::product(factors)
            })
            .collect();
        Self::sort_operands(&mut result);
        result
    }

    /// The factors of \(c \prod_{i} b_{i}^{k_{i}}\): the coefficient unless it is
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
