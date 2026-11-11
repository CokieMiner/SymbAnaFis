//! Variable ordering, multi-prime CRT lifting, dense univariate arithmetic
//! modulo one prime, and deterministic residue sampling.

use core::{
    cmp::{Ordering, Reverse},
    mem::swap,
    num::NonZeroU64,
};

use alloc::{vec, vec::Vec};

use ntt_anafis::{CrtPair, PrimeField, Residue};
use num_anafis::NonZeroU64Ext;

use crate::errors::SimplifyError;

use super::{
    Divisor, ModularTerm, Monomial, PolynomialGcd, PolynomialWork, SparsePolynomial, Zippel,
};

/// Catalog primes tried by the modular reconstruction.
const PRIMES: usize = 4;

impl PolynomialGcd {
    /// Order the variables, reconstruct the GCD of the deflated pair, and map
    /// the certified divisor back to the original exponents.
    pub fn interpolated(
        inputs: [&SparsePolynomial; 2],
        degrees: &[Vec<u32>; 2],
        bounds: &[u32],
        strides: &[u32],
        sampler: &mut Sampler,
        work: &mut PolynomialWork,
    ) -> Result<Option<Divisor>, SimplifyError> {
        let dimensions = bounds.len();
        let order = Self::order(bounds, inputs, degrees);
        let essential = bounds.iter().filter(|bound| **bound != 0).count();
        let mut positions = vec![0; dimensions];
        for (position, &variable) in order.iter().enumerate() {
            if let Some(slot) = positions.get_mut(variable) {
                *slot = position;
            }
        }
        let permute = |powers: &Monomial| {
            Some(Monomial::from_fn(dimensions, |position| {
                order
                    .get(position)
                    .map_or(0, |&variable| powers.power(variable))
            }))
        };
        let permuted: Vec<u32> = order
            .iter()
            .map(|&variable| bounds.get(variable).copied().unwrap_or(0))
            .collect();
        let [left, right] = inputs;
        let (Some(left_permuted), Some(right_permuted)) =
            (left.mapped(permute), right.mapped(permute))
        else {
            return Err(SimplifyError::InvalidPolynomial);
        };
        let Some((common, left_cofactor, right_cofactor)) = Self::modular(
            [&left_permuted, &right_permuted],
            essential,
            &permuted,
            sampler,
            work,
        )?
        else {
            return Ok(None);
        };
        // Undo the permutation, then the deflation.
        let restore = |powers: &Monomial| {
            let mut valid = true;
            let restored = Monomial::from_fn(dimensions, |variable| {
                let power = positions
                    .get(variable)
                    .map_or(0, |&position| powers.power(position));
                let stride = strides.get(variable).copied().unwrap_or(0).max(1);
                power.checked_mul(stride).unwrap_or_else(|| {
                    valid = false;
                    0
                })
            });
            valid.then_some(restored)
        };
        let (Some(restored), Some(left_restored), Some(right_restored)) = (
            common.mapped(restore),
            left_cofactor.mapped(restore),
            right_cofactor.mapped(restore),
        ) else {
            return Err(SimplifyError::InvalidPolynomial);
        };
        Ok(Some((restored, left_restored, right_restored)))
    }

    /// Positions in the modular algorithm: the variable of positive bound
    /// minimizing its bound plus the term count of the shorter leading
    /// coefficient first, then the other positive bounds in descending
    /// order, then the remaining variables.
    fn order(
        bounds: &[u32],
        inputs: [&SparsePolynomial; 2],
        degrees: &[Vec<u32>; 2],
    ) -> Vec<usize> {
        let mut essential: Vec<usize> = (0..bounds.len())
            .filter(|&variable| bounds.get(variable).is_some_and(|bound| *bound != 0))
            .collect();
        let main = essential.iter().copied().min_by_key(|&variable| {
            let bound = bounds
                .get(variable)
                .and_then(|&bound| usize::try_from(bound).ok())
                .unwrap_or(usize::MAX);
            let terms = inputs
                .iter()
                .zip(degrees)
                .map(|(input, input_degrees)| {
                    let degree = input_degrees.get(variable).copied();
                    input
                        .terms()
                        .iter()
                        .filter(|(powers, _)| Some(powers.power(variable)) == degree)
                        .count()
                })
                .min()
                .unwrap_or(0);
            (bound.saturating_add(terms), variable)
        });
        essential.retain(|&variable| Some(variable) != main);
        essential.sort_by_key(|&variable| Reverse(bounds.get(variable).copied().unwrap_or(0)));
        main.into_iter()
            .chain(essential)
            .chain((0..bounds.len()).filter(|&variable| bounds.get(variable) == Some(&0)))
            .collect()
    }

    /// Reconstruct and certify the GCD of two permuted, deflated primitive
    /// polynomials whose first `essential` positions carry the GCD.
    fn modular(
        inputs: [&SparsePolynomial; 2],
        essential: usize,
        bounds: &[u32],
        sampler: &mut Sampler,
        work: &mut PolynomialWork,
    ) -> Result<Option<Divisor>, SimplifyError> {
        let dimensions = bounds.len();
        let [left, right] = inputs;
        let (Some(&(_, left_lead)), Some(&(_, right_lead))) =
            (left.terms().first(), right.terms().first())
        else {
            return Ok(None);
        };
        // The leading coefficient of the GCD divides both input leads.
        let Some(gamma) = NonZeroU64::new(left_lead.unsigned_abs())
            .map(|lead| lead.gcd(right_lead.unsigned_abs()))
        else {
            return Ok(None);
        };
        // A lifted candidate is the primitive part of its integer terms.
        let certify = |lifted: Vec<(Monomial, i128)>,
                       budget: &mut PolynomialWork|
         -> Result<Option<Divisor>, SimplifyError> {
            let Some((_, candidate)) =
                SparsePolynomial::from_integers(dimensions, lifted, NonZeroU64::MIN)
                    .map(SparsePolynomial::into_primitive)
            else {
                return Ok(None);
            };
            let Some(left_cofactor) = left.divide(&candidate, budget)? else {
                return Ok(None);
            };
            let Some(right_cofactor) = right.divide(&candidate, budget)? else {
                return Ok(None);
            };
            Ok(Some((candidate, left_cofactor, right_cofactor)))
        };
        let mut previous: Option<(PrimeField, Vec<ModularTerm>)> = None;
        for field in PrimeField::catalog().take(PRIMES) {
            if field.reduce(left_lead) == Residue::ZERO || field.reduce(right_lead) == Residue::ZERO
            {
                continue;
            }
            let mut zippel = Zippel::new(field, dimensions, bounds, sampler, work);
            let left_image = zippel.specialize(&left.reduce(field), essential)?;
            let right_image = zippel.specialize(&right.reduce(field), essential)?;
            let Some(mut image) = zippel.gcd(&left_image, &right_image, essential)? else {
                continue;
            };
            let Some(scale) = image
                .first()
                .and_then(|(_, lead)| field.inv(*lead))
                .map(|inverse| field.mul(field.reduce_u64(gamma.get()), inverse))
            else {
                continue;
            };
            for (_, value) in &mut image {
                *value = field.mul(*value, scale);
            }
            let lifted = image
                .iter()
                .map(|(powers, value)| (powers.clone(), i128::from(field.signed(*value))))
                .collect();
            if let Some(result) = certify(lifted, work)? {
                return Ok(Some(result));
            }
            // Images of one GCD share their leading monomial; otherwise one
            // prime was unlucky and the later image replaces the earlier.
            if let Some((earlier_field, earlier)) = &previous
                && earlier.first().map(|(powers, _)| powers)
                    == image.first().map(|(powers, _)| powers)
                && let Some(pair) = CrtPair::new(*earlier_field, field)
            {
                let mut combined = Vec::with_capacity(image.len().max(earlier.len()));
                let mut old = earlier.iter().peekable();
                let mut new = image.iter().peekable();
                loop {
                    // Merge the descending supports; an absent term is zero.
                    let step = match (old.peek(), new.peek()) {
                        (None, None) => break,
                        (Some((older, _)), Some((newer, _))) => older.cmp(newer),
                        (Some(_), None) => Ordering::Greater,
                        (None, Some(_)) => Ordering::Less,
                    };
                    let older = old.next_if(|_| step != Ordering::Less);
                    let newer = new.next_if(|_| step != Ordering::Greater);
                    let Some((powers, _)) = older.or(newer) else {
                        break;
                    };
                    let value = |term: Option<&ModularTerm>| {
                        term.map_or(Residue::ZERO, |(_, residue)| *residue)
                    };
                    combined.push((powers.clone(), pair.combine(value(older), value(newer))));
                }
                if let Some(result) = certify(combined, work)? {
                    return Ok(Some(result));
                }
            }
            previous = Some((field, image));
        }
        Ok(None)
    }
}

/// Namespace for dense univariate arithmetic modulo one prime.
///
/// Coefficients are stored by ascending degree without trailing zeros, so the
/// zero polynomial is empty and the last coefficient leads.
pub struct Dense;

impl Dense {
    /// The monic GCD by Euclid's algorithm; zero when both inputs vanish.
    pub fn gcd(
        field: PrimeField,
        mut left: Vec<Residue>,
        mut right: Vec<Residue>,
        work: &mut PolynomialWork,
    ) -> Result<Vec<Residue>, SimplifyError> {
        while left.last() == Some(&Residue::ZERO) {
            left.pop();
        }
        while right.last() == Some(&Residue::ZERO) {
            right.pop();
        }
        if left.len() < right.len() {
            swap(&mut left, &mut right);
        }
        while let Some(inverse) = right.last().and_then(|&lead| field.inv(lead)) {
            work.product(left.len(), right.len())?;
            // Each step cancels the leading coefficient of `left` exactly.
            while let Some(&lead) = left.last() {
                let Some(window) = left
                    .len()
                    .checked_sub(right.len())
                    .and_then(|shift| left.get_mut(shift..))
                else {
                    break;
                };
                field.mul_sub(window, &right, field.mul(lead, inverse));
                left.pop();
                while left.last() == Some(&Residue::ZERO) {
                    left.pop();
                }
            }
            swap(&mut left, &mut right);
        }
        if let Some(inverse) = left.last().and_then(|&lead| field.inv(lead)) {
            field.scale(&mut left, inverse);
        }
        Ok(left)
    }

    /// The quotient of an exact division, or `None` for a zero divisor or a
    /// nonzero remainder.
    pub fn quotient(
        field: PrimeField,
        dividend: &[Residue],
        divisor: &[Residue],
    ) -> Option<Vec<Residue>> {
        let (&lead, lower) = divisor.split_last()?;
        let inverse = field.inv(lead)?;
        if dividend.is_empty() {
            return Some(Vec::new());
        }
        let mut remainder = dividend.to_vec();
        let mut quotient = vec![Residue::ZERO; remainder.len().checked_sub(lower.len())?];
        for (shift, slot) in quotient.iter_mut().enumerate().rev() {
            let window = remainder.get_mut(shift..)?;
            let factor = field.mul(*window.get(lower.len())?, inverse);
            field.mul_sub(window, divisor, factor);
            *slot = factor;
        }
        remainder
            .iter()
            .all(|&value| value == Residue::ZERO)
            .then_some(quotient)
    }

    /// The product of two polynomials. Leading coefficients of a field
    /// product cannot cancel, so the result has no trailing zeros.
    pub fn product(field: PrimeField, left: &[Residue], right: &[Residue]) -> Vec<Residue> {
        if left.is_empty() || right.is_empty() {
            return Vec::new();
        }
        let mut result =
            vec![Residue::ZERO; left.len().saturating_add(right.len()).saturating_sub(1)];
        for (shift, &value) in left.iter().enumerate() {
            if let Some(window) = result.get_mut(shift..) {
                field.mul_sub(window, right, field.neg(value));
            }
        }
        result
    }
}

/// States of the `SplitMix64` generator (Steele, Lea, and Flood 2014). A fixed seed makes every
/// GCD reproducible; the modular algorithms certify their results, so the
/// points only need to avoid the few roots of the relevant leading
/// coefficients and resultants.
pub struct Sampler {
    state: u64,
}

impl Sampler {
    pub const fn new() -> Self {
        Self {
            state: 0x243F_6A88_85A3_08D3,
        }
    }

    /// A uniform nonzero residue of `field`.
    pub fn nonzero(&mut self, field: PrimeField) -> Residue {
        loop {
            self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut mixed = self.state;
            mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            let residue = field.reduce_u64(mixed ^ (mixed >> 31));
            if residue != Residue::ZERO {
                return residue;
            }
        }
    }
}
