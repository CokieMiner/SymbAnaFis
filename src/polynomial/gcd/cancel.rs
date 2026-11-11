//! Exact $\mathbb{Z}$-reduction, deflation, degree bounds, and top-level GCD cancellation.

use core::iter::successors;

use alloc::{vec, vec::Vec};

use ntt_anafis::{PrimeField, Residue};
use num_anafis::RationalType;

use super::{
    Dense, Divisor, Modular, Monomial, PolynomialLimitExceeded, PolynomialWork, Sampler,
    SparsePolynomial,
};

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

impl PolynomialGcd {
    /// Remove the GCD of two nonzero polynomials of one dimension. The
    /// rational contents cancel to coprime integers, so both results have
    /// integer coefficients. `None` when an intermediate leaves native
    /// capacity or no candidate is certified.
    pub fn cancel(
        left: &SparsePolynomial,
        right: &SparsePolynomial,
        work: &mut PolynomialWork,
    ) -> Result<Option<Cancellation>, PolynomialLimitExceeded> {
        let (left_shift, left_rest) = left.split_monomial(work)?;
        let (right_shift, right_rest) = right.split_monomial(work)?;
        let shared = left_shift.minimum(&right_shift);
        let (Some(left_excess), Some(right_excess)) = (
            left_shift.difference(&shared),
            right_shift.difference(&shared),
        ) else {
            return Ok(None);
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
    ) -> Result<Option<Divisor>, PolynomialLimitExceeded> {
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
        let strides = Self::strides(left, right, dimensions);
        // Unit and zero strides leave every exponent, so the pair is its own
        // deflation and is not copied.
        let deflated = if strides.iter().any(|&stride| stride > 1) {
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
                return Ok(None);
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
        let mut shared_count = 0_usize;
        let mut any_count = 0_usize;
        let mut unique_var = 0_usize;
        for (variable, (&first, &second)) in left_degrees.iter().zip(right_degrees).enumerate() {
            if first != 0 || second != 0 {
                any_count = any_count.saturating_add(1);
                unique_var = variable;
            }
            if first != 0 && second != 0 {
                shared_count = shared_count.saturating_add(1);
            }
        }
        if shared_count == 0 {
            return Ok(Some((one, left.clone(), right.clone())));
        }
        let within_dense = !degrees
            .iter()
            .flatten()
            .any(|&degree| degree > DENSE_DEGREE_LIMIT);
        // Pure univariate pairs lift straight from dense modular GCDs without
        // probing degree bounds or running `Zippel`.
        if any_count == 1 && within_dense {
            let pair_degrees = [
                left_degrees.get(unique_var).copied().unwrap_or(0),
                right_degrees.get(unique_var).copied().unwrap_or(0),
            ];
            let stride = strides.get(unique_var).copied().unwrap_or(1);
            return Modular::univariate(inputs, pair_degrees, unique_var, stride, work);
        }
        let mut sampler = Sampler::new();
        let bounds = if within_dense {
            Self::bounds(inputs, &degrees, &mut sampler, work)?
        } else {
            None
        };
        if bounds
            .as_ref()
            .is_some_and(|limits| limits.iter().all(|&bound| bound == 0))
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
        Modular::interpolated(inputs, &degrees, &limits, &strides, &mut sampler, work)
    }

    /// Componentwise exponent GCD across both inputs, stopping early once
    /// every coordinate reaches stride one.
    fn strides(left: &SparsePolynomial, right: &SparsePolynomial, dimensions: usize) -> Vec<u32> {
        let mut strides = vec![0_u32; dimensions];
        let mut remaining = dimensions;
        for (powers, _) in left.terms().iter().chain(right.terms()) {
            for (stride, &power) in strides.iter_mut().zip(powers.as_slice()) {
                if *stride == 1 || power == 0 {
                    continue;
                }
                let mut remainder = power;
                while let Some(next) = stride.checked_rem(remainder) {
                    *stride = remainder;
                    remainder = next;
                }
                if *stride == 1 {
                    remaining = remaining.saturating_sub(1);
                }
            }
            if remaining == 0 {
                break;
            }
        }
        strides
    }

    /// Upper bounds on the degree of the GCD in each variable from one
    /// univariate image per variable. A point that keeps both input degrees
    /// makes each image GCD a multiple of the image of the GCD.
    ///
    /// Precomputing each term's full evaluation $\tau_{t} = c_{t}\prod_{j}
    /// \alpha_{j}^{e_{t,j}}$ and per-variable inverse power tables
    /// $\alpha_{v}^{-e}$ evaluates every variable's univariate image in
    /// $O(d \cdot T)$ field multiplications instead of $O(d^2 \cdot T)$.
    fn bounds(
        inputs: [&SparsePolynomial; 2],
        [left_degrees, right_degrees]: &[Vec<u32>; 2],
        sampler: &mut Sampler,
        work: &mut PolynomialWork,
    ) -> Result<Option<Vec<u32>>, PolynomialLimitExceeded> {
        let dimensions = left_degrees.len();
        let mut bounds = vec![0; dimensions];
        let Some(field) = PrimeField::catalog().next() else {
            return Ok(None);
        };
        let reduced = inputs.map(|input| Modular::reduce(input, field));
        let mut inv_table = Vec::new();
        'attempt: for _ in 0..ATTEMPTS {
            let point: Vec<_> = (0..dimensions).map(|_| sampler.nonzero(field)).collect();
            let tables: Vec<Vec<Residue>> = point
                .iter()
                .zip(left_degrees.iter().zip(right_degrees))
                .map(|(&base, (&first, &second))| {
                    let length = usize::try_from(first.max(second))
                        .map_or(0, |degree| degree.saturating_add(1));
                    let mut table = Vec::with_capacity(length);
                    table.extend(
                        successors(Some(Residue::ONE), |&p| Some(field.mul(p, base))).take(length),
                    );
                    table
                })
                .collect();
            // Full evaluation of every term at `point`, one residue per term.
            let [left_red, right_red] = &reduced;
            work.product(left_red.len().saturating_add(right_red.len()), dimensions)?;
            let mut totals: [Vec<Residue>; 2] = [Vec::new(), Vec::new()];
            for (values, terms) in totals.iter_mut().zip(&reduced) {
                values.reserve(terms.len());
                for (monomial, coefficient) in terms {
                    let mut product = *coefficient;
                    for (&exponent, table) in monomial.as_slice().iter().zip(&tables) {
                        if exponent != 0 {
                            let Some(&power) =
                                usize::try_from(exponent).ok().and_then(|i| table.get(i))
                            else {
                                return Ok(None);
                            };
                            product = field.mul(product, power);
                        }
                    }
                    values.push(product);
                }
            }
            for (variable, ((bound, &base), (&first, &second))) in bounds
                .iter_mut()
                .zip(&point)
                .zip(left_degrees.iter().zip(right_degrees))
                .enumerate()
            {
                if first == 0 || second == 0 {
                    continue;
                }
                let Some(inverse) = field.inv(base) else {
                    continue 'attempt;
                };
                inv_table.clear();
                inv_table.extend(
                    successors(Some(Residue::ONE), |&p| Some(field.mul(p, inverse))).take(
                        usize::try_from(first.max(second)).map_or(0, |d| d.saturating_add(1)),
                    ),
                );
                let mut images = [first, second].map(|d| {
                    vec![Residue::ZERO; usize::try_from(d).map_or(0, |n| n.saturating_add(1))]
                });
                for (image, (terms, values)) in images.iter_mut().zip(reduced.iter().zip(&totals)) {
                    work.spend(terms.len())?;
                    for ((monomial, _), &total) in terms.iter().zip(values) {
                        let Some(index) = usize::try_from(monomial.power(variable)).ok() else {
                            return Ok(None);
                        };
                        let Some(&inv_power) = inv_table.get(index) else {
                            return Ok(None);
                        };
                        if let Some(slot) = image.get_mut(index) {
                            let term = if index == 0 {
                                total
                            } else {
                                field.mul(total, inv_power)
                            };
                            *slot = field.add(*slot, term);
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
            return Ok(Some(bounds));
        }
        for (bound, (&first, &second)) in bounds
            .iter_mut()
            .zip(left_degrees.iter().zip(right_degrees))
        {
            *bound = first.min(second);
        }
        Ok(Some(bounds))
    }
}
