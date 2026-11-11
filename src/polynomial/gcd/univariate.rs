//! Dense univariate arithmetic modulo one prime and deterministic residue
//! sampling.

use core::mem::swap;

use alloc::{vec, vec::Vec};

use ntt_anafis::{PrimeField, Residue};

use super::{Monomial, PolynomialLimitExceeded, PolynomialWork};

/// A monomial with its nonzero coefficient modulo one prime.
pub type ModularTerm = (Monomial, Residue);

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
    ) -> Result<Vec<Residue>, PolynomialLimitExceeded> {
        while left.last() == Some(&Residue::ZERO) {
            left.pop();
        }
        while right.last() == Some(&Residue::ZERO) {
            right.pop();
        }
        if left.len() < right.len() {
            swap(&mut left, &mut right);
        }
        while let Some((&right_lead, right_lower)) = right.split_last() {
            if right_lower.is_empty() {
                return Ok(vec![Residue::ONE]);
            }
            let Some(inverse) = (if right_lead == Residue::ONE {
                Some(Residue::ONE)
            } else {
                field.inv(right_lead)
            }) else {
                break;
            };
            let steps = left.len().saturating_sub(right.len()).saturating_add(1);
            work.product(steps, right.len())?;
            // Each step pops the leading coefficient of `left` and eliminates
            // only against the lower terms of `right`.
            while left.len() >= right.len() {
                let Some(lead) = left.pop() else {
                    break;
                };
                if lead != Residue::ZERO {
                    let shift = left.len().saturating_sub(right_lower.len());
                    if let Some(window) = left.get_mut(shift..) {
                        let factor = if right_lead == Residue::ONE {
                            lead
                        } else {
                            field.mul(lead, inverse)
                        };
                        field.mul_sub(window, right_lower, factor);
                    }
                }
                while left.last() == Some(&Residue::ZERO) {
                    left.pop();
                }
            }
            swap(&mut left, &mut right);
        }
        if let Some(&lead) = left.last()
            && lead != Residue::ONE
            && let Some(inverse) = field.inv(lead)
        {
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
        let inverse = if lead == Residue::ONE {
            Residue::ONE
        } else {
            field.inv(lead)?
        };
        if dividend.is_empty() {
            return Some(Vec::new());
        }
        let mut remainder = dividend.to_vec();
        let mut quotient = vec![Residue::ZERO; remainder.len().checked_sub(lower.len())?];
        for (shift, slot) in quotient.iter_mut().enumerate().rev() {
            let top = *remainder.get(shift.checked_add(lower.len())?)?;
            if top == Residue::ZERO {
                continue;
            }
            let factor = if lead == Residue::ONE {
                top
            } else {
                field.mul(top, inverse)
            };
            *slot = factor;
            if !lower.is_empty() {
                field.mul_sub(remainder.get_mut(shift..)?, lower, factor);
            }
        }
        remainder
            .get(..lower.len())?
            .iter()
            .all(|&value| value == Residue::ZERO)
            .then_some(quotient)
    }
}

/// States of the `SplitMix64` generator (Steele, Lea, and Flood 2014). A fixed
/// seed makes every GCD reproducible; the modular algorithms certify their
/// results, so the points only need to avoid the few roots of the relevant
/// leading coefficients and resultants.
pub struct Sampler {
    state: u64,
}

impl Sampler {
    /// Creates the deterministic sampler at its fixed initial seed.
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
