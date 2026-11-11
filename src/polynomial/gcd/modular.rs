//! Variable ordering, univariate and multivariate multi-prime CRT lifting, and
//! exact division certification.

use core::{
    cmp::{Ordering, Reverse},
    num::NonZeroU64,
};

use alloc::{vec, vec::Vec};

use ntt_anafis::{CrtPair, PrimeField, Residue};
use num_anafis::NonZeroU64Ext;

use super::{
    Dense, ModularTerm, Monomial, PolynomialLimitExceeded, PolynomialWork, Sampler,
    SparsePolynomial, Zippel,
};

/// Catalog primes tried by the modular reconstruction.
const PRIMES: usize = 4;

/// A common divisor with both exact cofactors.
pub type Divisor = (SparsePolynomial, SparsePolynomial, SparsePolynomial);

/// Namespace for prime-field reduction, modular lifting, and certification.
pub struct Modular;

impl Modular {
    /// The nonzero images of the primitive terms modulo the prime of `field`.
    pub fn reduce(polynomial: &SparsePolynomial, field: PrimeField) -> Vec<ModularTerm> {
        let mut terms = Vec::with_capacity(polynomial.terms().len());
        for (powers, value) in polynomial.terms() {
            let residue = field.reduce(*value);
            if residue != Residue::ZERO {
                terms.push((powers.clone(), residue));
            }
        }
        terms
    }

    /// Lift and certify the GCD of a pure univariate deflated pair in
    /// `variable`, bypassing degree-bound probes, permutations, and `Zippel`.
    ///
    /// A degree-zero modular GCD modulo a prime not dividing either leading
    /// coefficient proves a unit GCD over $\mathbb{Z}$ immediately; an image
    /// matching one input's degree reduces to one trial division.
    pub fn univariate(
        [left, right]: [&SparsePolynomial; 2],
        [left_degree, right_degree]: [u32; 2],
        variable: usize,
        stride: u32,
        work: &mut PolynomialWork,
    ) -> Result<Option<Divisor>, PolynomialLimitExceeded> {
        let dimensions = left.dimensions();
        let Some((left_lead, right_lead, gamma, left_len, right_len)) = (|| {
            let (&(_, l_lead), &(_, r_lead)) = (left.terms().first()?, right.terms().first()?);
            let gcd = NonZeroU64::new(l_lead.unsigned_abs())?.gcd(r_lead.unsigned_abs());
            let l_len = usize::try_from(left_degree).ok()?.checked_add(1)?;
            let r_len = usize::try_from(right_degree).ok()?.checked_add(1)?;
            Some((l_lead, r_lead, gcd, l_len, r_len))
        })() else {
            return Ok(None);
        };
        let mut previous: Option<(PrimeField, Vec<Residue>)> = None;
        let mut tried = false;
        let Some((common, left_co, right_co)) = ('prime: {
            for field in PrimeField::catalog().take(PRIMES) {
                if field.reduce(left_lead) == Residue::ZERO
                    || field.reduce(right_lead) == Residue::ZERO
                {
                    continue;
                }
                work.spend(left.terms().len().saturating_add(right.terms().len()))?;
                let [left_dense, right_dense] =
                    [(left, left_len), (right, right_len)].map(|(p, len)| {
                        let mut dense = vec![Residue::ZERO; len];
                        for (powers, value) in p.terms() {
                            if let Ok(idx) = usize::try_from(powers.power(variable))
                                && let Some(slot) = dense.get_mut(idx)
                            {
                                *slot = field.reduce(*value);
                            }
                        }
                        dense
                    });
                let mut common = Dense::gcd(field, left_dense, right_dense, work)?;
                if previous
                    .as_ref()
                    .is_some_and(|(_, p)| common.len() > p.len())
                {
                    continue;
                }
                if common.len() <= 1 {
                    break 'prime Some((
                        SparsePolynomial::one(dimensions),
                        left.clone(),
                        right.clone(),
                    ));
                }
                if !tried && left_len != right_len && common.len() == left_len.min(right_len) {
                    tried = true;
                    let unit = SparsePolynomial::one(dimensions);
                    let trial = if right_len < left_len {
                        left.divide(right, work)?.map(|q| (right.clone(), q, unit))
                    } else {
                        right.divide(left, work)?.map(|q| (left.clone(), unit, q))
                    };
                    if let Some(div) = trial {
                        break 'prime Some(div);
                    }
                }
                field.scale(&mut common, field.reduce_u64(gamma.get()));
                let earlier = previous.as_ref().map(|(f, p)| (*f, p.as_slice()));
                if let Some(result) =
                    Self::lift_univariate([left, right], variable, field, &common, earlier, work)?
                {
                    break 'prime Some(result);
                }
                previous = Some((field, common));
            }
            None
        }) else {
            return Ok(None);
        };
        if stride <= 1 {
            return Ok(Some((common, left_co, right_co)));
        }
        let inflate = |powers: &Monomial| powers.scale(stride, work).ok();
        Ok(common
            .mapped(inflate)
            .and_then(|gcd| Some((gcd, left_co.mapped(inflate)?, right_co.mapped(inflate)?))))
    }

    /// Order the variables, reconstruct the GCD of the deflated pair, and map
    /// the certified divisor back to the original exponents.
    pub fn interpolated(
        inputs: [&SparsePolynomial; 2],
        degrees: &[Vec<u32>; 2],
        bounds: &[u32],
        strides: &[u32],
        sampler: &mut Sampler,
        work: &mut PolynomialWork,
    ) -> Result<Option<Divisor>, PolynomialLimitExceeded> {
        let dimensions = bounds.len();
        let order = Self::order(bounds, inputs, degrees);
        let essential = bounds.iter().filter(|bound| **bound != 0).count();
        let is_identity = order
            .iter()
            .enumerate()
            .all(|(position, &variable)| position == variable);
        let needs_inflate = strides.iter().any(|&stride| stride > 1);
        let [left, right] = inputs;
        let permuted_pair = if is_identity {
            None
        } else {
            let permute = |powers: &Monomial| {
                Some(Monomial::from_fn(dimensions, |position| {
                    order
                        .get(position)
                        .map_or(0, |&variable| powers.power(variable))
                }))
            };
            let (Some(left_permuted), Some(right_permuted)) =
                (left.mapped(permute), right.mapped(permute))
            else {
                return Ok(None);
            };
            Some([left_permuted, right_permuted])
        };
        let permuted_bounds: Vec<u32> = order
            .iter()
            .map(|&variable| bounds.get(variable).copied().unwrap_or(0))
            .collect();
        let permuted_degrees = degrees.each_ref().map(|input_degrees| {
            order
                .iter()
                .map(|&variable| input_degrees.get(variable).copied().unwrap_or(0))
                .collect::<Vec<_>>()
        });
        let modular_inputs = permuted_pair
            .as_ref()
            .map_or([left, right], |[first, second]| [first, second]);
        let Some((common, left_cofactor, right_cofactor)) = Self::modular(
            modular_inputs,
            &permuted_degrees,
            essential,
            &permuted_bounds,
            sampler,
            work,
        )?
        else {
            return Ok(None);
        };
        if is_identity && !needs_inflate {
            return Ok(Some((common, left_cofactor, right_cofactor)));
        }
        let mut positions = vec![0; dimensions];
        for (position, &variable) in order.iter().enumerate() {
            if let Some(slot) = positions.get_mut(variable) {
                *slot = position;
            }
        }
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
            return Ok(None);
        };
        Ok(Some((restored, left_restored, right_restored)))
    }

    /// Positions in the modular algorithm: the variable of positive bound
    /// preferring a monomial leading coefficient (so sparse images have fixed
    /// scales) and minimizing its bound plus leading-coefficient term count
    /// first, then the other positive bounds in descending order, then the
    /// remaining variables.
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
            (terms > 1, bound.saturating_add(terms), variable)
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
        degrees: &[Vec<u32>; 2],
        essential: usize,
        bounds: &[u32],
        sampler: &mut Sampler,
        work: &mut PolynomialWork,
    ) -> Result<Option<Divisor>, PolynomialLimitExceeded> {
        let dimensions = bounds.len();
        let [left, right] = inputs;
        let [left_degrees, right_degrees] = degrees;
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
        let mut previous: Option<(PrimeField, Vec<ModularTerm>)> = None;
        for field in PrimeField::catalog().take(PRIMES) {
            if field.reduce(left_lead) == Residue::ZERO || field.reduce(right_lead) == Residue::ZERO
            {
                continue;
            }
            let mut zippel = Zippel::new(field, dimensions, bounds, sampler, work);
            let left_image = zippel.specialize(left, left_degrees, essential)?;
            let right_image = zippel.specialize(right, right_degrees, essential)?;
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
            if scale != Residue::ONE {
                for (_, value) in &mut image {
                    *value = field.mul(*value, scale);
                }
            }
            let lifted = image
                .iter()
                .map(|(powers, value)| (powers.clone(), i128::from(field.signed(*value))))
                .collect();
            if let Some(result) = Self::certify(left, right, dimensions, lifted, work)? {
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
                if let Some(result) = Self::certify(left, right, dimensions, combined, work)? {
                    return Ok(Some(result));
                }
            }
            previous = Some((field, image));
        }
        Ok(None)
    }

    /// Lift a gamma-scaled univariate image (and optional two-prime CRT pair) to $\mathbb{Z}$
    /// and certify by exact division.
    fn lift_univariate(
        [left, right]: [&SparsePolynomial; 2],
        variable: usize,
        field: PrimeField,
        common: &[Residue],
        previous: Option<(PrimeField, &[Residue])>,
        work: &mut PolynomialWork,
    ) -> Result<Option<Divisor>, PolynomialLimitExceeded> {
        let dimensions = left.dimensions();
        let unit_pow = |power| {
            if dimensions == 1 {
                Monomial::univariate(power)
            } else {
                Monomial::from_fn(dimensions, |i| if i == variable { power } else { 0 })
            }
        };
        let candidate: Vec<_> = common
            .iter()
            .enumerate()
            .rev()
            .filter_map(|(deg, &res)| {
                (res != Residue::ZERO)
                    .then(|| {
                        Some((
                            unit_pow(u32::try_from(deg).ok()?),
                            i128::from(field.signed(res)),
                        ))
                    })
                    .flatten()
            })
            .collect();
        if let Some(result) = Self::certify(left, right, dimensions, candidate, work)? {
            return Ok(Some(result));
        }
        if let Some((earlier_field, earlier)) = previous
            && earlier.len() == common.len()
            && let Some(pair) = CrtPair::new(earlier_field, field)
        {
            let combined: Vec<_> = earlier
                .iter()
                .zip(common)
                .enumerate()
                .rev()
                .filter_map(|(deg, (&a, &b))| {
                    (a != Residue::ZERO || b != Residue::ZERO)
                        .then(|| Some((unit_pow(u32::try_from(deg).ok()?), pair.combine(a, b))))
                        .flatten()
                })
                .collect();
            if let Some(result) = Self::certify(left, right, dimensions, combined, work)? {
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    /// Certify a lifted integer candidate by exact trial division of both inputs.
    fn certify(
        left: &SparsePolynomial,
        right: &SparsePolynomial,
        dimensions: usize,
        lifted: Vec<(Monomial, i128)>,
        work: &mut PolynomialWork,
    ) -> Result<Option<Divisor>, PolynomialLimitExceeded> {
        let Some((_, candidate)) =
            SparsePolynomial::from_integers(dimensions, lifted, NonZeroU64::MIN)
                .map(SparsePolynomial::into_primitive)
        else {
            return Ok(None);
        };
        let Some(left_cofactor) = left.divide(&candidate, work)? else {
            return Ok(None);
        };
        let Some(right_cofactor) = right.divide(&candidate, work)? else {
            return Ok(None);
        };
        Ok(Some((candidate, left_cofactor, right_cofactor)))
    }
}
