//! Sparse exact rational polynomials over shared expression indeterminates,
//! stored as a rational content times a primitive integer polynomial.
//!
//! The child implements products and exact division.

#![expect(
    clippy::self_named_module_files,
    reason = "the polynomial owner keeps its invariants private while its child implements arithmetic"
)]

use core::{iter::once, mem::replace, num::NonZeroU64};

use alloc::{vec, vec::Vec};

use ntt_anafis::{PrimeField, Residue};
use num_anafis::{NonZeroU64Ext, Number, RationalType};

use crate::{
    core::{CompactStorage, Expr},
    errors::SimplifyError,
};

use super::{ModularTerm, Monomial, PolynomialWork};

mod arithmetic;

/// $c P$ for a nonzero rational content $c$ and a primitive integer
/// polynomial $P$, whose terms are strictly descending lexicographic
/// monomials with coprime nonzero coefficients of magnitude at most
/// `i64::MAX` and a positive leading coefficient. The zero polynomial has no
/// terms and content zero, so equal polynomials have equal representations.
/// A converted pair shares one dimension and key representation.
///
/// By Gauss's lemma a product of primitive polynomials is primitive, so a
/// product multiplies contents and integer coefficients and needs no
/// normalization; a sum brings its addends to a common denominator and
/// extracts the content once.
///
/// Checked arithmetic returns `None` when a content or a coefficient leaves
/// native capacity; the caller then retains the original expression instead
/// of aborting on a valid but coefficient-heavy input.
#[derive(Clone, PartialEq, Eq)]
pub struct SparsePolynomial {
    content: RationalType,
    terms: Vec<(Monomial, i64)>,
    dimensions: usize,
}

impl SparsePolynomial {
    pub const fn zero(dimensions: usize) -> Self {
        Self {
            content: RationalType::from_integer(0),
            terms: Vec::new(),
            dimensions,
        }
    }

    pub fn one(dimensions: usize) -> Self {
        Self {
            content: RationalType::from_integer(1),
            terms: vec![(Monomial::zero(dimensions), 1)],
            dimensions,
        }
    }

    pub fn constant(
        value: RationalType,
        dimensions: usize,
        work: &mut PolynomialWork,
    ) -> Result<Self, SimplifyError> {
        if *value.numer() == 0 {
            return Ok(Self::zero(dimensions));
        }
        work.spend(dimensions.saturating_add(1))?;
        Ok(Self {
            content: value,
            terms: vec![(Monomial::zero(dimensions), 1)],
            dimensions,
        })
    }

    pub fn variable(
        dimensions: usize,
        index: usize,
        work: &mut PolynomialWork,
    ) -> Result<Self, SimplifyError> {
        work.spend(dimensions)?;
        Ok(Self {
            content: RationalType::from_integer(1),
            terms: vec![(Monomial::variable(dimensions, index), 1)],
            dimensions,
        })
    }

    /// Collect rational terms in any order, or `None` when their common
    /// denominator, a coefficient sum, or the content leaves native capacity.
    pub fn from_terms(
        dimensions: usize,
        terms: Vec<(Monomial, RationalType)>,
        work: &mut PolynomialWork,
    ) -> Result<Option<Self>, SimplifyError> {
        work.spend(terms.len())?;
        let mut denominator = NonZeroU64::MIN;
        for (_, coefficient) in &terms {
            let Some(multiple) =
                NonZeroU64::new(*coefficient.denom()).and_then(|current| denominator.lcm(current))
            else {
                return Ok(None);
            };
            denominator = multiple;
        }
        let mut integral = Vec::with_capacity(terms.len());
        for (powers, coefficient) in terms {
            let Some(scaled) = denominator
                .get()
                .checked_div(*coefficient.denom())
                .and_then(|multiplier| {
                    i128::from(*coefficient.numer()).checked_mul(i128::from(multiplier))
                })
            else {
                return Ok(None);
            };
            integral.push((powers, scaled));
        }
        Ok(Self::from_integers(dimensions, integral, denominator))
    }

    /// The sum of `addends` over their least common denominator, or `None`
    /// beyond native capacity.
    pub fn sum(
        dimensions: usize,
        addends: &[&Self],
        work: &mut PolynomialWork,
    ) -> Result<Option<Self>, SimplifyError> {
        let mut denominator = NonZeroU64::MIN;
        let mut count = 0_usize;
        for addend in addends {
            let Some(multiple) = NonZeroU64::new(*addend.content.denom())
                .and_then(|current| denominator.lcm(current))
            else {
                return Ok(None);
            };
            denominator = multiple;
            count = count.saturating_add(addend.terms.len());
        }
        work.spend(count)?;
        let mut terms = Vec::with_capacity(count);
        for addend in addends {
            // The content $n/d$ scales each coefficient by $n D / d$.
            let Some(multiplier) = denominator
                .get()
                .checked_div(*addend.content.denom())
                .and_then(|quotient| {
                    i128::from(*addend.content.numer()).checked_mul(i128::from(quotient))
                })
            else {
                return Ok(None);
            };
            for (powers, value) in &addend.terms {
                let Some(scaled) = multiplier.checked_mul(i128::from(*value)) else {
                    return Ok(None);
                };
                terms.push((powers.clone(), scaled));
            }
        }
        Ok(Self::from_integers(dimensions, terms, denominator))
    }

    /// $\frac{1}{d}\sum_{i} a_{i} m_{i}$ for integer terms in any order, or
    /// `None` when a coefficient sum, the primitive part, or the content
    /// leaves native capacity.
    ///
    /// A stable sort adds equal monomials in input order and merges presorted
    /// runs, such as the terms of several addends; presorted input costs one
    /// comparison per term. The content is the gcd of the sums, signed so
    /// that the leading coefficient is positive.
    pub fn from_integers(
        dimensions: usize,
        mut terms: Vec<(Monomial, i128)>,
        denominator: NonZeroU64,
    ) -> Option<Self> {
        if !terms.is_sorted_by(|(left, _), (right, _)| left > right) {
            terms.sort_by(|(left, _), (right, _)| right.cmp(left));
        }
        let mut fits = true;
        terms.dedup_by(|(powers, value), (kept, sum)| {
            if powers != kept {
                return false;
            }
            match sum.checked_add(*value) {
                Some(total) => *sum = total,
                None => fits = false,
            }
            true
        });
        if !fits {
            return None;
        }
        terms.retain(|(_, value)| *value != 0);
        let Some(&(_, lead)) = terms.first() else {
            return Some(Self::zero(dimensions));
        };
        let mut common = 0_u128;
        for (_, value) in &terms {
            let mut remainder = value.unsigned_abs();
            while let Some(next) = common.checked_rem(remainder) {
                common = remainder;
                remainder = next;
            }
        }
        let magnitude = i128::try_from(common).ok()?;
        let divisor = if lead < 0 {
            magnitude.checked_neg()?
        } else {
            magnitude
        };
        let mut primitive = Vec::with_capacity(terms.len());
        for (powers, value) in terms {
            primitive.push((powers, Self::native(value.checked_div(divisor)?)?));
        }
        let (numerator, reduced) = denominator.reduce(divisor);
        Some(Self {
            content: RationalType::from_parts(i64::try_from(numerator).ok()?, reduced.get()),
            terms: primitive,
            dimensions,
        })
    }

    /// The primitive terms in strictly descending lexicographic order.
    pub fn terms(&self) -> &[(Monomial, i64)] {
        &self.terms
    }

    pub const fn dimensions(&self) -> usize {
        self.dimensions
    }

    pub const fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    /// Whether the polynomial is a constant, zero included; the primitive
    /// part of a nonzero constant is one.
    pub fn is_constant(&self) -> bool {
        match self.terms.as_slice() {
            [] => true,
            [(powers, _)] => powers.as_slice().iter().all(|power| *power == 0),
            _ => false,
        }
    }

    /// Split a nonzero polynomial into its content and its primitive part,
    /// which has content one.
    pub const fn into_primitive(mut self) -> (RationalType, Self) {
        if self.terms.is_empty() {
            return (RationalType::from_integer(0), self);
        }
        let content = replace(&mut self.content, RationalType::from_integer(1));
        (content, self)
    }

    /// The polynomial times a rational, or `None` beyond native capacity.
    pub fn checked_scale(mut self, factor: &RationalType) -> Option<Self> {
        if *factor.numer() == 0 {
            return Some(Self::zero(self.dimensions));
        }
        self.content = self.content.checked_mul(factor)?;
        Some(self)
    }

    /// Split the largest common monomial factor from every stored term.
    ///
    /// The returned exponent vector is componentwise minimal across the terms.
    /// Removing it leaves a polynomial with at least one zero exponent in every
    /// variable, so a subsequent polynomial GCD cannot recover a hidden
    /// monomial factor. The primitive part is exact because the minimum
    /// exponent divides every stored monomial, and it keeps the term order
    /// because lexicographic order is invariant under division by a common
    /// monomial.
    pub fn split_monomial(
        &self,
        work: &mut PolynomialWork,
    ) -> Result<(Monomial, Self), SimplifyError> {
        let Some(((first, _), rest)) = self.terms.split_first() else {
            return Ok((Monomial::zero(self.dimensions), Self::zero(self.dimensions)));
        };
        let mut content = first.clone();
        for (powers, _) in rest {
            content = content.minimum(powers);
        }
        work.spend(self.terms.len())?;
        let mut primitive = self.clone();
        if content.as_slice().iter().any(|power| *power != 0) {
            for (powers, _) in &mut primitive.terms {
                *powers = powers
                    .difference(&content)
                    .ok_or(SimplifyError::InvalidPolynomial)?;
            }
        }
        Ok((content, primitive))
    }

    /// Multiply every stored term by one monomial factor; lexicographic order
    /// is invariant under this multiplication, so the terms stay in order.
    pub fn multiply_monomial(
        mut self,
        factor: &Monomial,
        work: &mut PolynomialWork,
    ) -> Result<Self, SimplifyError> {
        if factor.as_slice().iter().all(|power| *power == 0) {
            return Ok(self);
        }
        work.spend(self.terms.len())?;
        for (powers, _) in &mut self.terms {
            *powers = powers.sum(factor, self.dimensions, work)?;
        }
        Ok(self)
    }

    /// The degree in each variable.
    pub fn degrees(&self) -> Vec<u32> {
        let mut degrees = vec![0; self.dimensions];
        for (powers, _) in &self.terms {
            for (degree, power) in degrees.iter_mut().zip(powers.as_slice()) {
                *degree = (*degree).max(*power);
            }
        }
        degrees
    }

    /// Apply an injective monomial map, restore the descending order, and
    /// move the sign of a negative leading coefficient into the content.
    /// `None` when the map declines or the content has no negation.
    pub fn mapped(&self, mut map: impl FnMut(&Monomial) -> Option<Monomial>) -> Option<Self> {
        let mut terms = Vec::with_capacity(self.terms.len());
        for (powers, value) in &self.terms {
            terms.push((map(powers)?, *value));
        }
        terms.sort_unstable_by(|left, right| right.0.cmp(&left.0));
        let mut content = self.content.clone();
        if terms.first().is_some_and(|(_, lead)| *lead < 0) {
            content = content.checked_neg()?;
            for (_, value) in &mut terms {
                // Every coefficient is at least `-i64::MAX`.
                *value = value.checked_neg()?;
            }
        }
        Some(Self {
            content,
            terms,
            dimensions: self.dimensions,
        })
    }

    /// The nonzero images of the primitive terms modulo the prime of `field`.
    pub fn reduce(&self, field: PrimeField) -> Vec<ModularTerm> {
        self.terms
            .iter()
            .filter_map(|(powers, value)| {
                let residue = field.reduce(*value);
                (residue != Residue::ZERO).then(|| (powers.clone(), residue))
            })
            .collect()
    }

    /// The canonical sum of the terms over `bases`, or `None` when a reduced
    /// coefficient leaves native rational capacity. Distinct monomials in
    /// scalar bases go directly to compact storage.
    pub fn to_expr(&self, bases: &[Expr]) -> Option<Expr> {
        let mut products = Vec::with_capacity(self.terms.len());
        // The canonical construction sorts ascending input fastest.
        for (powers, value) in self.terms.iter().rev() {
            let coefficient = self
                .content
                .checked_mul(&RationalType::from_integer(*value))?;
            let scalar = if coefficient.is_int() {
                Expr::number(*coefficient.numer())
            } else {
                Expr::number(Number::from_rational(coefficient))
            };
            products.push(
                once(scalar)
                    .chain(
                        powers
                            .as_slice()
                            .iter()
                            .zip(bases)
                            .filter(|(count, _)| **count != 0)
                            .map(|(count, base)| base.pow(i64::from(*count))),
                    )
                    .collect(),
            );
        }
        Some(Expr::sum_of_products(products))
    }

    /// A primitive coefficient, or `None` beyond `i64::MAX` in magnitude, so
    /// that every coefficient has a negation.
    fn native(value: i128) -> Option<i64> {
        i64::try_from(value)
            .ok()
            .filter(|coefficient| *coefficient != i64::MIN)
    }
}
