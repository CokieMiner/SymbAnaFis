//! Sparse exact rational polynomials over shared expression indeterminates.

use core::{
    iter::once,
    mem::take,
    num::NonZeroU32,
    ops::{Add, Div, Mul, Neg},
};

use alloc::{
    collections::{BTreeMap, btree_map::Entry},
    vec::Vec,
};

use num_anafis::{Number, RationalType};

use crate::{core::Expr, errors::SimplifyError};

use super::{Monomial, PolynomialWork};

/// Lexicographic monomials with exact backend-owned rational coefficients.
/// A converted pair shares one dimension and key representation. Every stored
/// coefficient is nonzero: accumulation removes zeros and coefficient extraction
/// only copies surviving terms. Thus a leading coefficient is a valid divisor.
#[derive(Clone, PartialEq, Eq)]
pub struct SparsePolynomial {
    terms: BTreeMap<Monomial, RationalType>,
    dimensions: usize,
}

impl SparsePolynomial {
    pub const fn zero(dimensions: usize) -> Self {
        Self {
            terms: BTreeMap::new(),
            dimensions,
        }
    }

    pub fn one(dimensions: usize, work: &mut PolynomialWork) -> Result<Self, SimplifyError> {
        Self::constant(RationalType::from_integer(1), dimensions, work)
    }

    pub fn constant(
        value: RationalType,
        dimensions: usize,
        work: &mut PolynomialWork,
    ) -> Result<Self, SimplifyError> {
        let mut result = Self::zero(dimensions);
        if *value.numer() != 0 {
            work.spend(dimensions.saturating_add(1))?;
            result.terms.insert(Monomial::zero(dimensions), value);
        }
        Ok(result)
    }

    pub fn variable(
        dimensions: usize,
        index: usize,
        work: &mut PolynomialWork,
    ) -> Result<Self, SimplifyError> {
        work.spend(dimensions)?;
        let powers = Monomial::variable(dimensions, index);
        Ok(Self {
            terms: BTreeMap::from([(powers, RationalType::from_integer(1))]),
            dimensions,
        })
    }

    pub fn divide_scalar(
        mut self,
        divisor: &RationalType,
        work: &mut PolynomialWork,
    ) -> Result<Self, SimplifyError> {
        if divisor.is_int() && *divisor.numer() == 1 {
            return Ok(self);
        }
        work.spend(self.terms.len())?;
        for coefficient in self.terms.values_mut() {
            *coefficient = Div::div(&*coefficient, divisor);
        }
        Ok(self)
    }

    pub const fn dimensions(&self) -> usize {
        self.dimensions
    }

    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    pub fn is_constant(&self) -> bool {
        // Unique monomial keys and nonzero coefficients imply that a constant
        // has at most one term, including the empty zero polynomial.
        self.terms.len() <= 1
            && self
                .terms
                .keys()
                .all(|powers| powers.as_slice().iter().all(|power| *power == 0))
    }

    pub fn monic(mut self, work: &mut PolynomialWork) -> Result<Self, SimplifyError> {
        let Some((_, leading)) = self.terms.last_key_value() else {
            return Ok(self);
        };
        if leading.is_int() && *leading.numer() == 1 {
            return Ok(self);
        }
        let divisor = leading.clone();
        work.spend(self.terms.len())?;
        for value in self.terms.values_mut() {
            *value = Div::div(&*value, &divisor);
        }
        Ok(self)
    }

    /// Select the primitive integer associate when its exact coefficients fit.
    pub fn primitive_integer(mut self, work: &mut PolynomialWork) -> Result<Self, SimplifyError> {
        let mut denominator = 1_u64;
        for value in self.terms.values() {
            let mut left = denominator;
            let mut right = *value.denom();
            while let Some(remainder) = left.checked_rem(right) {
                left = right;
                right = remainder;
            }
            // Both operands of the GCD are positive, so `left` is nonzero.
            let Some(common) = denominator
                .checked_div(left)
                .and_then(|scale| scale.checked_mul(*value.denom()))
            else {
                return Ok(self);
            };
            denominator = common;
        }
        if denominator == 1 {
            return Ok(self);
        }
        if self.terms.values().any(|value| {
            // Backend rationals have positive denominators. The product of an
            // i64 numerator and a u64 scale lies within the i128 range.
            denominator
                .checked_div(*value.denom())
                .and_then(|scale| i128::from(*value.numer()).checked_mul(i128::from(scale)))
                .is_none_or(|wide| i64::try_from(wide).is_err())
        }) {
            return Ok(self);
        }
        work.spend(self.terms.len())?;
        let divisor = RationalType::from_parts(1, denominator);
        for value in self.terms.values_mut() {
            *value = Div::div(&*value, &divisor);
        }
        Ok(self)
    }

    pub fn mul(&self, other: &Self, work: &mut PolynomialWork) -> Result<Self, SimplifyError> {
        work.product(self.terms.len(), other.terms.len())?;
        if self.is_one() {
            return Ok(other.clone());
        }
        if other.is_one() {
            return Ok(self.clone());
        }
        let mut product = Self::zero(self.dimensions);
        for (first, first_coefficient) in &self.terms {
            for (second, second_coefficient) in &other.terms {
                let powers = first.sum(second, work)?;
                product.accumulate(powers, Mul::mul(first_coefficient, second_coefficient));
            }
        }
        Ok(product)
    }

    fn is_one(&self) -> bool {
        self.terms.len() == 1
            && self.terms.first_key_value().is_some_and(|(powers, value)| {
                value.is_int()
                    && *value.numer() == 1
                    && powers.as_slice().iter().all(|power| *power == 0)
            })
    }

    pub fn pow(
        &self,
        exponent: NonZeroU32,
        work: &mut PolynomialWork,
    ) -> Result<Self, SimplifyError> {
        let mut count = exponent.get();
        let mut base = self.clone();
        // The exponent is nonzero. Initialize at its lowest set bit rather
        // than multiplying an allocated unit polynomial into the first factor.
        while count & 1 == 0 {
            base = base.mul(&base, work)?;
            count >>= 1;
        }
        let mut result = base.clone();
        count >>= 1;
        while count != 0 {
            base = base.mul(&base, work)?;
            if count & 1 != 0 {
                result = result.mul(&base, work)?;
            }
            count >>= 1;
        }
        Ok(result)
    }

    pub fn add_assign(
        &mut self,
        other: &Self,
        subtract: bool,
        work: &mut PolynomialWork,
    ) -> Result<(), SimplifyError> {
        work.spend(other.terms.len())?;
        for (powers, coefficient) in &other.terms {
            let value = if subtract {
                Neg::neg(coefficient)
            } else {
                coefficient.clone()
            };
            self.accumulate(powers.clone(), value);
        }
        Ok(())
    }

    fn accumulate(&mut self, powers: Monomial, coefficient: RationalType) {
        if *coefficient.numer() == 0 {
            return;
        }
        match self.terms.entry(powers) {
            Entry::Vacant(entry) => {
                entry.insert(coefficient);
            }
            Entry::Occupied(mut entry) => {
                let sum = Add::add(entry.get(), &coefficient);
                if *sum.numer() == 0 {
                    entry.remove_entry();
                } else {
                    *entry.get_mut() = sum;
                }
            }
        }
    }

    /// Exact multivariate long division under lexicographic monomial order.
    pub fn exact_div(
        &self,
        divisor: &Self,
        work: &mut PolynomialWork,
    ) -> Result<Option<Self>, SimplifyError> {
        let Some((divisor_powers, divisor_coefficient)) = divisor.terms.last_key_value() else {
            return Ok(None);
        };
        if divisor.is_constant() {
            return Ok(Some(self.clone().divide_scalar(divisor_coefficient, work)?));
        }
        let mut remainder = self.clone();
        let mut quotient = Self::zero(self.dimensions);
        while let Some((powers, coefficient)) = remainder.terms.last_key_value() {
            work.spend(1)?;
            let Some(difference) = powers.difference(divisor_powers) else {
                return Ok(None);
            };
            let scale = Div::div(coefficient, divisor_coefficient);
            quotient.accumulate(difference.clone(), scale.clone());
            work.spend(divisor.terms.len())?;
            for (term, value) in &divisor.terms {
                let shifted = term.sum(&difference, work)?;
                remainder.accumulate(shifted, Neg::neg(Mul::mul(value, &scale)));
            }
        }
        Ok(Some(quotient))
    }

    pub fn coefficients(
        &self,
        variable: usize,
        work: &mut PolynomialWork,
    ) -> Result<Vec<Self>, SimplifyError> {
        work.spend(self.terms.len())?;
        let mut groups = BTreeMap::<u32, Self>::new();
        for (powers, coefficient) in &self.terms {
            let (degree, reduced) = powers.separate(variable);
            groups
                .entry(degree)
                .or_insert_with(|| Self::zero(self.dimensions))
                .terms
                .insert(reduced, coefficient.clone());
        }
        Ok(groups.into_values().collect())
    }

    /// Pseudo-division in one variable over the remaining coefficient ring.
    /// Each step uses \(R\leftarrow\operatorname{lc}(B)R-\operatorname{lc}(R)x^d B\).
    pub fn pseudo_remainder(
        mut self,
        divisor: &Self,
        variable: usize,
        work: &mut PolynomialWork,
    ) -> Result<Self, SimplifyError> {
        let degree = divisor.degree(variable);
        let leading = divisor.coefficient(variable, degree, work)?;
        while !self.is_zero() {
            let current_degree = self.degree(variable);
            let Some(shift) = current_degree.checked_sub(degree) else {
                break;
            };
            work.spend(1)?;
            let coefficient = self.coefficient(variable, current_degree, work)?;
            let mut subtrahend = divisor.mul(&coefficient, work)?;
            subtrahend.shift(variable, shift, work)?;
            let mut next = self.mul(&leading, work)?;
            next.add_assign(&subtrahend, true, work)?;
            self = next;
        }
        Ok(self)
    }

    fn degree(&self, variable: usize) -> u32 {
        self.terms
            .keys()
            .map(|powers| powers.as_slice().get(variable).copied().unwrap_or(0))
            .max()
            .unwrap_or(0)
    }

    fn coefficient(
        &self,
        variable: usize,
        degree: u32,
        work: &mut PolynomialWork,
    ) -> Result<Self, SimplifyError> {
        work.spend(self.terms.len())?;
        let mut result = Self::zero(self.dimensions);
        for (powers, value) in &self.terms {
            if powers.as_slice().get(variable).copied() != Some(degree) {
                continue;
            }
            let (_, reduced) = powers.separate(variable);
            result.terms.insert(reduced, value.clone());
        }
        Ok(result)
    }

    fn shift(
        &mut self,
        variable: usize,
        count: u32,
        work: &mut PolynomialWork,
    ) -> Result<(), SimplifyError> {
        if count == 0 {
            return Ok(());
        }
        work.spend(self.terms.len())?;
        let mut shifted = BTreeMap::new();
        for (mut powers, coefficient) in take(&mut self.terms) {
            powers.shift(variable, count, work)?;
            shifted.insert(powers, coefficient);
        }
        self.terms = shifted;
        Ok(())
    }

    pub fn to_expr(&self, bases: &[Expr]) -> Expr {
        Expr::sum(
            self.terms
                .iter()
                .map(|(powers, coefficient)| {
                    let scalar = if coefficient.is_int() {
                        Expr::number(*coefficient.numer())
                    } else {
                        Expr::number(Number::from_rational(coefficient.clone()))
                    };
                    Expr::product(
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
                    )
                })
                .collect(),
        )
    }
}
