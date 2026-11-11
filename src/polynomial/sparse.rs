//! Sparse exact rational polynomials over shared indeterminates, stored as a
//! rational content times a primitive integer polynomial.
//!
//! The children implement products, exact division, and Kronecker layouts.

#![expect(
    clippy::self_named_module_files,
    reason = "the polynomial owner keeps its invariants private while its children implement arithmetic"
)]

use core::{cmp::Ordering, mem::replace, num::NonZeroU64};

use alloc::{vec, vec::Vec};

use num_anafis::{NonZeroU64Ext, RationalType};

use super::{Monomial, PolynomialLimitExceeded, PolynomialWork};

use kronecker::Kronecker;

mod divide;
mod kronecker;
mod multiply;

type Terms = [(Monomial, i64)];

/// A max-heap entry ordered solely by its monomial key, avoiding tiebreaker
/// comparisons on the row and column indices.
struct HeapEntry<Key> {
    powers: Key,
    row: usize,
    column: usize,
}

impl<Key: PartialEq> PartialEq for HeapEntry<Key> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.powers == other.powers
    }
}

impl<Key: Eq> Eq for HeapEntry<Key> {}

impl<Key: Ord> PartialOrd for HeapEntry<Key> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Key: Ord> Ord for HeapEntry<Key> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.powers.cmp(&other.powers)
    }
}

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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SparsePolynomial {
    content: RationalType,
    terms: Vec<(Monomial, i64)>,
    dimensions: usize,
}

impl SparsePolynomial {
    /// The zero polynomial in `dimensions` variables.
    pub const fn zero(dimensions: usize) -> Self {
        Self {
            content: RationalType::from_integer(0),
            terms: Vec::new(),
            dimensions,
        }
    }

    /// The multiplicative identity $1$ in `dimensions` variables.
    pub fn one(dimensions: usize) -> Self {
        Self {
            content: RationalType::from_integer(1),
            terms: vec![(Monomial::zero(dimensions), 1)],
            dimensions,
        }
    }

    /// A rational constant polynomial in `dimensions` variables.
    pub fn constant(
        value: RationalType,
        dimensions: usize,
        work: &mut PolynomialWork,
    ) -> Result<Self, PolynomialLimitExceeded> {
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

    /// The generator $x_{\text{index}}$ in `dimensions` variables.
    pub fn variable(
        dimensions: usize,
        index: usize,
        work: &mut PolynomialWork,
    ) -> Result<Self, PolynomialLimitExceeded> {
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
    ) -> Result<Option<Self>, PolynomialLimitExceeded> {
        work.spend(terms.len())?;
        Ok((|| {
            let mut denominator = NonZeroU64::MIN;
            for (_, coefficient) in &terms {
                let denom = *coefficient.denom();
                if denom != 1 {
                    denominator = denominator.lcm(NonZeroU64::new(denom)?)?;
                }
            }
            let mut integral = Vec::with_capacity(terms.len());
            for (powers, coefficient) in terms {
                let numer = i128::from(*coefficient.numer());
                let denom = *coefficient.denom();
                let scaled = if denominator.get() == denom {
                    numer
                } else {
                    numer.checked_mul(i128::from(denominator.get().checked_div(denom)?))?
                };
                integral.push((powers, scaled));
            }
            Self::from_integers(dimensions, integral, denominator)
        })())
    }

    /// The sum of `addends` over their least common denominator, or `None`
    /// beyond native capacity.
    pub fn sum(
        dimensions: usize,
        addends: &[&Self],
        work: &mut PolynomialWork,
    ) -> Result<Option<Self>, PolynomialLimitExceeded> {
        match addends {
            [] => return Ok(Some(Self::zero(dimensions))),
            [single] => {
                work.spend(single.terms.len())?;
                return Ok(Some((*single).clone()));
            }
            [first, second] => {
                work.spend(first.terms.len().saturating_add(second.terms.len()))?;
                return Ok(Self::merge_pair(dimensions, first, second));
            }
            _ => {}
        }
        let count = addends
            .iter()
            .fold(0_usize, |total, a| total.saturating_add(a.terms.len()));
        work.spend(count)?;
        Ok((|| {
            let mut denominator = NonZeroU64::MIN;
            for addend in addends {
                let denom = *addend.content.denom();
                if denom != 1 {
                    denominator = denominator.lcm(NonZeroU64::new(denom)?)?;
                }
            }
            let mut terms = Vec::with_capacity(count);
            for addend in addends {
                let numer = i128::from(*addend.content.numer());
                if numer == 0 {
                    continue;
                }
                let denom = *addend.content.denom();
                let multiplier = if denominator.get() == denom {
                    numer
                } else {
                    numer.checked_mul(i128::from(denominator.get().checked_div(denom)?))?
                };
                for (powers, value) in &addend.terms {
                    terms.push((powers.clone(), multiplier.checked_mul(i128::from(*value))?));
                }
            }
            Self::from_integers(dimensions, terms, denominator)
        })())
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
        Self::normalize_sorted(dimensions, terms, denominator)
    }

    /// The rational content multiplying the primitive integer part.
    #[inline]
    pub const fn content(&self) -> &RationalType {
        &self.content
    }

    /// The primitive terms in strictly descending lexicographic order.
    #[inline]
    pub fn terms(&self) -> &[(Monomial, i64)] {
        &self.terms
    }

    /// Shared number of variables of every monomial key.
    #[inline]
    pub const fn dimensions(&self) -> usize {
        self.dimensions
    }

    /// Whether the polynomial is identically zero.
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    /// Whether the polynomial is a constant, zero included; the primitive
    /// part of a nonzero constant is one.
    pub fn is_constant(&self) -> bool {
        match self.terms.as_slice() {
            [] => true,
            [(powers, _)] => powers.is_zero(),
            _ => false,
        }
    }

    /// The degree in each variable.
    pub fn degrees(&self) -> Vec<u32> {
        let mut degrees = vec![0; self.dimensions];
        for (powers, _) in &self.terms {
            for (degree, &power) in degrees.iter_mut().zip(powers.as_slice()) {
                *degree = (*degree).max(power);
            }
        }
        degrees
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

    /// The polynomial divided by a nonzero rational, or `None` for zero or
    /// beyond native capacity.
    pub fn checked_div_scalar(mut self, divisor: &RationalType) -> Option<Self> {
        if *divisor.numer() == 0 {
            return None;
        }
        if self.terms.is_empty() {
            return Some(self);
        }
        self.content = self.content.checked_div(divisor)?;
        Some(self)
    }

    /// Split the largest common monomial factor from every stored term.
    pub fn split_monomial(
        &self,
        work: &mut PolynomialWork,
    ) -> Result<(Monomial, Self), PolynomialLimitExceeded> {
        let Some(((first, _), rest)) = self.terms.split_first() else {
            return Ok((Monomial::zero(self.dimensions), Self::zero(self.dimensions)));
        };
        work.spend(self.terms.len())?;
        let mut content = first.clone();
        if !content.is_zero() && rest.iter().any(|(powers, _)| content.min_assign(powers)) {
            return Ok((content, self.clone()));
        }
        let mut primitive = self.clone();
        if !content.is_zero() {
            for (powers, _) in &mut primitive.terms {
                powers.remove_factor(&content);
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
    ) -> Result<Self, PolynomialLimitExceeded> {
        if factor.is_zero() {
            return Ok(self);
        }
        work.spend(self.terms.len())?;
        for (powers, _) in &mut self.terms {
            powers.add_assign(factor, work)?;
        }
        Ok(self)
    }

    /// Apply an injective monomial map, restore the descending order, and
    /// move the sign of a negative leading coefficient into the content.
    /// `None` when the map declines or the content has no negation.
    pub fn mapped(&self, mut map: impl FnMut(&Monomial) -> Option<Monomial>) -> Option<Self> {
        let mut terms = Vec::with_capacity(self.terms.len());
        for (powers, value) in &self.terms {
            terms.push((map(powers)?, *value));
        }
        if !terms.is_sorted_by(|(left, _), (right, _)| left > right) {
            terms.sort_unstable_by(|left, right| right.0.cmp(&left.0));
        }
        let mut content = self.content.clone();
        if terms.first().is_some_and(|(_, lead)| *lead < 0) {
            content = content.checked_neg()?;
            for (_, value) in &mut terms {
                *value = value.checked_neg()?;
            }
        }
        Some(Self {
            content,
            terms,
            dimensions: self.dimensions,
        })
    }

    /// Direct linear merge of two strictly descending polynomials without sorting.
    fn merge_pair(dimensions: usize, first: &Self, second: &Self) -> Option<Self> {
        if first.terms.is_empty() {
            return Some(second.clone());
        }
        if second.terms.is_empty() {
            return Some(first.clone());
        }
        let d1 = NonZeroU64::new(*first.content.denom())?;
        let d2 = NonZeroU64::new(*second.content.denom())?;
        let denominator = if d1 == d2 { d1 } else { d1.lcm(d2)? };
        let m1 = i128::from(*first.content.numer())
            .checked_mul(i128::from(denominator.get().checked_div(d1.get())?))?;
        let m2 = i128::from(*second.content.numer())
            .checked_mul(i128::from(denominator.get().checked_div(d2.get())?))?;
        let mut merged = Vec::with_capacity(first.terms.len().saturating_add(second.terms.len()));
        let mut left_iter = first.terms.iter().peekable();
        let mut right_iter = second.terms.iter().peekable();
        loop {
            let step = match (left_iter.peek(), right_iter.peek()) {
                (None, None) => break,
                (Some((l_pow, _)), Some((r_pow, _))) => l_pow.cmp(r_pow),
                (Some(_), None) => Ordering::Greater,
                (None, Some(_)) => Ordering::Less,
            };
            let left_term = left_iter.next_if(|_| step != Ordering::Less);
            let right_term = right_iter.next_if(|_| step != Ordering::Greater);
            let Some((powers, _)) = left_term.or(right_term) else {
                break;
            };
            let l_scaled = match left_term {
                Some((_, val)) => m1.checked_mul(i128::from(*val))?,
                None => 0,
            };
            let r_scaled = match right_term {
                Some((_, val)) => m2.checked_mul(i128::from(*val))?,
                None => 0,
            };
            let sum = l_scaled.checked_add(r_scaled)?;
            if sum != 0 {
                merged.push((powers.clone(), sum));
            }
        }
        Self::normalize_sorted(dimensions, merged, denominator)
    }

    /// Extract the signed integer content from strictly descending, nonzero terms.
    fn normalize_sorted(
        dimensions: usize,
        terms: Vec<(Monomial, i128)>,
        denominator: NonZeroU64,
    ) -> Option<Self> {
        let Some(&(_, lead)) = terms.first() else {
            return Some(Self::zero(dimensions));
        };
        let negative = lead < 0;
        // Narrowing first keeps the content and division in native width; once
        // the running gcd hits one, total content is one and no division runs.
        let mut common: Option<NonZeroU64> = None;
        let mut split = terms.len();
        for (index, (_, value)) in terms.iter().enumerate() {
            let Ok(native) = i64::try_from(*value) else {
                split = index;
                break;
            };
            let magnitude = NonZeroU64::new(native.unsigned_abs())?;
            let next = common.map_or(magnitude, |current| current.gcd(magnitude.get()));
            common = Some(next);
            if next == NonZeroU64::MIN {
                break;
            }
        }
        let mut primitive = Vec::with_capacity(terms.len());
        let divisor = if common == Some(NonZeroU64::MIN) {
            for (powers, value) in terms {
                let signed = if negative {
                    value.checked_neg()?
                } else {
                    value
                };
                primitive.push((powers, Self::native(signed)?));
            }
            if negative { -1 } else { 1 }
        } else if split == terms.len()
            && let Some(narrow) = common
            && let Ok(magnitude) = i64::try_from(narrow.get())
        {
            let div = if negative {
                magnitude.checked_neg()?
            } else {
                magnitude
            };
            for (powers, value) in terms {
                let quotient = i64::try_from(value).ok()?.checked_div(div)?;
                primitive.push((powers, (quotient != i64::MIN).then_some(quotient)?));
            }
            i128::from(div)
        } else {
            // A content of $2^{63}$ or a coefficient beyond `i64` takes `i128`.
            let mut common_wide = common.map_or(0_u128, |narrow| u128::from(narrow.get()));
            for (_, value) in terms.get(split..).unwrap_or_default() {
                let mut remainder = value.unsigned_abs();
                while let Some(next) = common_wide.checked_rem(remainder) {
                    common_wide = remainder;
                    remainder = next;
                }
                if common_wide == 1 {
                    return None;
                }
            }
            let magnitude = i128::try_from(common_wide).ok()?;
            let div = if negative {
                magnitude.checked_neg()?
            } else {
                magnitude
            };
            for (powers, value) in terms {
                primitive.push((powers, Self::native(value.checked_div(div)?)?));
            }
            div
        };
        let (numerator, reduced) = denominator.reduce(divisor);
        Some(Self {
            content: RationalType::from_parts(i64::try_from(numerator).ok()?, reduced.get()),
            terms: primitive,
            dimensions,
        })
    }

    /// A primitive coefficient, or `None` beyond `i64::MAX` in magnitude, so
    /// that every coefficient has a negation.
    fn native(value: i128) -> Option<i64> {
        i64::try_from(value).ok().filter(|&c| c != i64::MIN)
    }
}
