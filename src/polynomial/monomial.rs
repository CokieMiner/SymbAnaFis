//! Allocation-free exponent vectors for polynomials in at most four variables
//! and shared work accounting.

use core::cmp::Ordering;

use alloc::{vec, vec::Vec};

use crate::errors::SimplifyError;

/// All keys in one polynomial have the same dimension and representation.
/// Inline vectors pad unused variables with zeros, preserving lexicographic
/// order and componentwise arithmetic without a separate length or allocation.
///
/// Monomials are ordered lexicographically, inline before heap vectors.
#[derive(Clone, PartialEq, Eq)]
pub struct Monomial {
    powers: Powers,
}

#[derive(Clone, PartialEq, Eq)]
enum Powers {
    Inline([u32; 4]),
    Heap(Vec<u32>),
}

impl Ord for Monomial {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.powers, &other.powers) {
            (Powers::Inline(left), Powers::Inline(right)) => {
                // Four exponents in lexicographic order are the digits, most
                // significant first, of one integer in base $2^{32}$.
                let packed = |&[first, second, third, fourth]: &[u32; 4]| {
                    (u128::from(first) << 96)
                        | (u128::from(second) << 64)
                        | (u128::from(third) << 32)
                        | u128::from(fourth)
                };
                packed(left).cmp(&packed(right))
            }
            (Powers::Heap(left), Powers::Heap(right)) => left.cmp(right),
            (Powers::Inline(_), Powers::Heap(_)) => Ordering::Less,
            (Powers::Heap(_), Powers::Inline(_)) => Ordering::Greater,
        }
    }
}

impl PartialOrd for Monomial {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Monomial {
    pub fn zero(dimensions: usize) -> Self {
        Self {
            powers: if dimensions <= 4 {
                Powers::Inline([0; 4])
            } else {
                Powers::Heap(vec![0; dimensions])
            },
        }
    }

    pub fn variable(dimensions: usize, variable: usize) -> Self {
        Self::from_fn(dimensions, |index| u32::from(index == variable))
    }

    /// The exponent vector whose power at each of the first `dimensions`
    /// variables is `power(variable)`; padding variables stay zero.
    pub fn from_fn(dimensions: usize, mut power: impl FnMut(usize) -> u32) -> Self {
        let mut result = Self::zero(dimensions);
        for (index, slot) in result
            .as_mut_slice()
            .iter_mut()
            .enumerate()
            .take(dimensions)
        {
            *slot = power(index);
        }
        result
    }

    pub fn from_indexed(dimensions: usize, powers: &[(usize, u32)]) -> Option<Self> {
        let mut result = Self::zero(dimensions);
        for (index, power) in powers {
            let slot = result.as_mut_slice().get_mut(*index)?;
            if *slot != 0 || *power == 0 {
                return None;
            }
            *slot = *power;
        }
        Some(result)
    }

    #[inline]
    pub fn sum(
        &self,
        other: &Self,
        dimensions: usize,
        work: &PolynomialWork,
    ) -> Result<Self, SimplifyError> {
        let powers =
            if let (Powers::Inline(left), Powers::Inline(right)) = (&self.powers, &other.powers) {
                let mut sum = [0; 4];
                for ((output, l), r) in sum.iter_mut().zip(left).zip(right).take(dimensions) {
                    *output = work.exponent_sum(*l, *r)?;
                }
                Powers::Inline(sum)
            } else {
                let mut sum = Vec::with_capacity(self.as_slice().len());
                for (left, right) in self.as_slice().iter().zip(other.as_slice()) {
                    sum.push(work.exponent_sum(*left, *right)?);
                }
                Powers::Heap(sum)
            };
        Ok(Self { powers })
    }

    pub fn difference(&self, other: &Self) -> Option<Self> {
        let powers =
            if let (Powers::Inline(left), Powers::Inline(right)) = (&self.powers, &other.powers) {
                Powers::Inline([
                    left[0].checked_sub(right[0])?,
                    left[1].checked_sub(right[1])?,
                    left[2].checked_sub(right[2])?,
                    left[3].checked_sub(right[3])?,
                ])
            } else {
                let mut difference = Vec::with_capacity(self.as_slice().len());
                for (left, right) in self.as_slice().iter().zip(other.as_slice()) {
                    difference.push(left.checked_sub(*right)?);
                }
                Powers::Heap(difference)
            };
        Some(Self { powers })
    }

    /// Return the componentwise minimum exponent vector.
    ///
    /// This is the largest monomial dividing both inputs. It is used to remove
    /// common monomial factors before the polynomial remainder sequence.
    pub fn minimum(&self, other: &Self) -> Self {
        let mut result = Self::zero(self.as_slice().len());
        for ((power, left), right) in result
            .as_mut_slice()
            .iter_mut()
            .zip(self.as_slice())
            .zip(other.as_slice())
        {
            *power = (*left).min(*right);
        }
        result
    }

    /// Separate one variable from its coefficient monomial. Absent variables
    /// have degree zero; the GCD driver selects axes below the shared dimension.
    pub fn separate(&self, variable: usize) -> (u32, Self) {
        let mut reduced = self.clone();
        let mut degree = 0;
        for (index, power) in reduced.as_mut_slice().iter_mut().enumerate() {
            if index == variable {
                degree = *power;
                *power = 0;
                break;
            }
        }
        (degree, reduced)
    }

    /// Shift a validated basis variable, checking native exponent capacity.
    pub fn shift(
        &mut self,
        variable: usize,
        count: u32,
        work: &PolynomialWork,
    ) -> Result<(), SimplifyError> {
        for (index, power) in self.as_mut_slice().iter_mut().enumerate() {
            if index == variable {
                *power = work.exponent_sum(*power, count)?;
                break;
            }
        }
        Ok(())
    }

    /// The exponent of `variable`, zero beyond the stored variables.
    #[inline]
    pub fn power(&self, variable: usize) -> u32 {
        self.as_slice().get(variable).copied().unwrap_or(0)
    }

    #[inline]
    pub fn as_slice(&self) -> &[u32] {
        match &self.powers {
            Powers::Inline(powers) => powers,
            Powers::Heap(powers) => powers,
        }
    }

    fn as_mut_slice(&mut self) -> &mut [u32] {
        match &mut self.powers {
            Powers::Inline(powers) => powers,
            Powers::Heap(powers) => powers,
        }
    }
}

/// Shared work accounting for sparse polynomial conversion and arithmetic.
pub struct PolynomialWork {
    remaining: usize,
    limit: usize,
}

impl PolynomialWork {
    pub const fn new(limit: usize) -> Self {
        Self {
            remaining: limit,
            limit,
        }
    }

    #[inline]
    pub fn spend(&mut self, count: usize) -> Result<(), SimplifyError> {
        self.remaining = self
            .remaining
            .checked_sub(count)
            .ok_or(SimplifyError::PolynomialLimitExceeded { limit: self.limit })?;
        Ok(())
    }

    #[inline]
    pub fn product(&mut self, left: usize, right: usize) -> Result<(), SimplifyError> {
        let count = left
            .checked_mul(right)
            .ok_or(SimplifyError::PolynomialLimitExceeded { limit: self.limit })?;
        self.spend(count)
    }

    #[inline]
    pub fn exponent_sum(&self, left: u32, right: u32) -> Result<u32, SimplifyError> {
        left.checked_add(right)
            .ok_or(SimplifyError::PolynomialLimitExceeded { limit: self.limit })
    }
}
