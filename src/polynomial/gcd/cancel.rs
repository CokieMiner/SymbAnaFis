//! Exact $\mathbb{Z}$-reduction, deflation, degree bounds, and top-level GCD cancellation.

use alloc::{vec, vec::Vec};

use ntt_anafis::{PrimeField, Residue};
use num_anafis::RationalType;

use crate::errors::SimplifyError;

use super::{Dense, Monomial, PolynomialWork, Sampler, SparsePolynomial};

/// Dense univariate images allocate one residue per degree; beyond this
/// degree only trial division is attempted.
const DENSE_DEGREE_LIMIT: u32 = 1 << 12;
/// Random points tried for the degree bounds.
const ATTEMPTS: usize = 4;

/// Namespace for exact polynomial GCD cancellation.
pub struct PolynomialGcd;

/// $P/Q$ = `numerator/denominator` with $P = C N s$ and $Q = C D s$ for the
/// removed factor $C$ = `common` and a nonzero rational $s$.
pub struct Cancellation {
    pub common: SparsePolynomial,
    pub numerator: SparsePolynomial,
    pub denominator: SparsePolynomial,
}

/// A common divisor with both exact cofactors.
pub type Divisor = (SparsePolynomial, SparsePolynomial, SparsePolynomial);

impl PolynomialGcd {
    /// Remove the GCD of two nonzero polynomials of one dimension. The
    /// rational contents cancel to coprime integers, so both results have
    /// integer coefficients. `None` when an intermediate leaves native
    /// capacity or no candidate is certified.
    pub fn cancel(
        left: &SparsePolynomial,
        right: &SparsePolynomial,
        work: &mut PolynomialWork,
    ) -> Result<Option<Cancellation>, SimplifyError> {
        let (left_shift, left_rest) = left.split_monomial(work)?;
        let (right_shift, right_rest) = right.split_monomial(work)?;
        let shared = left_shift.minimum(&right_shift);
        let (Some(left_excess), Some(right_excess)) = (
            left_shift.difference(&shared),
            right_shift.difference(&shared),
        ) else {
            return Err(SimplifyError::InvalidPolynomial);
        };
        let (left_scale, left_primitive) = left_rest.into_primitive();
        let (right_scale, right_primitive) = right_rest.into_primitive();
        let Some((divisor, left_cofactor, right_cofactor)) =
            Self::divisor(&left_primitive, &right_primitive, work)?
        else {
            return Ok(None);
        };
        let Some(ratio) = left_scale.checked_div(&right_scale) else {
            return Ok(None);
        };
        let (Some(numerator), Some(denominator)) = (
            left_cofactor.checked_scale(&RationalType::from_integer(*ratio.numer())),
            i64::try_from(*ratio.denom())
                .ok()
                .and_then(|scale| right_cofactor.checked_scale(&RationalType::from_integer(scale))),
        ) else {
            return Ok(None);
        };
        Ok(Some(Cancellation {
            common: divisor.multiply_monomial(&shared, work)?,
            numerator: numerator.multiply_monomial(&left_excess, work)?,
            denominator: denominator.multiply_monomial(&right_excess, work)?,
        }))
    }

    /// The GCD of two primitive polynomials with its cofactors, after the
    /// shortcuts that need no interpolation.
    fn divisor(
        left: &SparsePolynomial,
        right: &SparsePolynomial,
        work: &mut PolynomialWork,
    ) -> Result<Option<Divisor>, SimplifyError> {
        let dimensions = left.dimensions();
        let one = SparsePolynomial::one(dimensions);
        if left.is_constant() || right.is_constant() {
            return Ok(Some((one, left.clone(), right.clone())));
        }
        if left == right {
            return Ok(Some((left.clone(), one.clone(), one)));
        }
        // A pair in $x^s$ has its GCD in $x^s$; the substitution is a ring
        // homomorphism, so certified results map back exactly.
        let mut strides = vec![0_u32; dimensions];
        for (powers, _) in left.terms().iter().chain(right.terms()) {
            for (stride, &power) in strides.iter_mut().zip(powers.as_slice()) {
                let mut remainder = power;
                while let Some(next) = stride.checked_rem(remainder) {
                    *stride = remainder;
                    remainder = next;
                }
            }
        }
        // Unit and zero strides leave every exponent, so the pair is its own
        // deflation and is not copied.
        let deflated = if strides.iter().any(|stride| *stride > 1) {
            let deflate = |powers: &Monomial| {
                Some(Monomial::from_fn(dimensions, |index| {
                    let power = powers.power(index);
                    strides
                        .get(index)
                        .and_then(|&stride| power.checked_div(stride))
                        .unwrap_or(power)
                }))
            };
            let (Some(left_deflated), Some(right_deflated)) =
                (left.mapped(deflate), right.mapped(deflate))
            else {
                return Err(SimplifyError::InvalidPolynomial);
            };
            Some([left_deflated, right_deflated])
        } else {
            None
        };
        let inputs = deflated
            .as_ref()
            .map_or([left, right], |[first, second]| [first, second]);
        let degrees = inputs.map(SparsePolynomial::degrees);
        let [left_degrees, right_degrees] = &degrees;
        if !left_degrees
            .iter()
            .zip(right_degrees)
            .any(|(first, second)| *first != 0 && *second != 0)
        {
            return Ok(Some((one, left.clone(), right.clone())));
        }
        let mut sampler = Sampler::new();
        let bounds = if degrees
            .iter()
            .flatten()
            .any(|degree| *degree > DENSE_DEGREE_LIMIT)
        {
            None
        } else {
            Some(Self::bounds(inputs, &degrees, &mut sampler, work)?)
        };
        if bounds
            .as_ref()
            .is_some_and(|limits| limits.iter().all(|bound| *bound == 0))
        {
            return Ok(Some((one, left.clone(), right.clone())));
        }
        // Bounds equal to the degrees of one input leave that input as the
        // only candidate.
        for (divisor, dividend, input_degrees, swapped) in [
            (right, left, right_degrees, false),
            (left, right, left_degrees, true),
        ] {
            if bounds
                .as_ref()
                .is_some_and(|limits| limits != input_degrees)
            {
                continue;
            }
            if let Some(quotient) = dividend.divide(divisor, work)? {
                let unit = SparsePolynomial::one(dimensions);
                return Ok(Some(if swapped {
                    (divisor.clone(), unit, quotient)
                } else {
                    (divisor.clone(), quotient, unit)
                }));
            }
        }
        let Some(limits) = bounds else {
            return Ok(None);
        };
        Self::interpolated(inputs, &degrees, &limits, &strides, &mut sampler, work)
    }

    /// Upper bounds on the degree of the GCD in each variable from one
    /// univariate image per variable. A point that keeps both input degrees
    /// makes each image GCD a multiple of the image of the GCD.
    fn bounds(
        inputs: [&SparsePolynomial; 2],
        degrees: &[Vec<u32>; 2],
        sampler: &mut Sampler,
        work: &mut PolynomialWork,
    ) -> Result<Vec<u32>, SimplifyError> {
        let [left_degrees, right_degrees] = degrees;
        let dimensions = left_degrees.len();
        let mut bounds = vec![0; dimensions];
        let Some(field) = PrimeField::catalog().next() else {
            return Err(SimplifyError::InvalidPolynomial);
        };
        let reduced = inputs.map(|input| input.reduce(field));
        'attempt: for _ in 0..ATTEMPTS {
            let point: Vec<_> = (0..dimensions).map(|_| sampler.nonzero(field)).collect();
            // Each term with the power of every coordinate at the point.
            let powers = reduced.each_ref().map(|terms| {
                terms
                    .iter()
                    .map(|(monomial, value)| {
                        let factors: Vec<_> = monomial
                            .as_slice()
                            .iter()
                            .zip(&point)
                            .map(|(&exponent, &base)| field.pow(base, u64::from(exponent)))
                            .collect();
                        (monomial, *value, factors)
                    })
                    .collect::<Vec<_>>()
            });
            for (variable, (bound, (first, second))) in bounds
                .iter_mut()
                .zip(left_degrees.iter().zip(right_degrees))
                .enumerate()
            {
                if *first == 0 || *second == 0 {
                    continue;
                }
                let mut images = [first, second].map(|degree| {
                    let length =
                        usize::try_from(*degree).map_or(0, |count| count.saturating_add(1));
                    vec![Residue::ZERO; length]
                });
                for (image, terms) in images.iter_mut().zip(&powers) {
                    work.spend(terms.len())?;
                    for (monomial, value, factors) in terms {
                        let product = factors
                            .iter()
                            .enumerate()
                            .filter(|(index, _)| *index != variable)
                            .fold(*value, |product, (_, &factor)| field.mul(product, factor));
                        let exponent = monomial.power(variable);
                        if let Some(slot) = usize::try_from(exponent)
                            .ok()
                            .and_then(|index| image.get_mut(index))
                        {
                            *slot = field.add(*slot, product);
                        }
                    }
                }
                let [image_left, image_right] = images;
                if image_left.last() == Some(&Residue::ZERO)
                    || image_right.last() == Some(&Residue::ZERO)
                {
                    continue 'attempt;
                }
                let common = Dense::gcd(field, image_left, image_right, work)?;
                *bound = u32::try_from(common.len().saturating_sub(1)).unwrap_or(u32::MAX);
            }
            return Ok(bounds);
        }
        for (bound, (first, second)) in bounds
            .iter_mut()
            .zip(left_degrees.iter().zip(right_degrees))
        {
            *bound = (*first).min(*second);
        }
        Ok(bounds)
    }
}
