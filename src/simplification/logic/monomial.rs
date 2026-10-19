//! Allocation-free exponent vectors for polynomials in at most four variables.

use alloc::vec::Vec;

use crate::errors::SimplifyError;

use super::PolynomialWork;

/// All keys in one polynomial have the same dimension and representation.
/// Inline vectors pad unused variables with zeros, preserving lexicographic
/// order and componentwise arithmetic without a separate length or allocation.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Monomial {
    powers: Powers,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Powers {
    Inline([u32; 4]),
    Heap(Vec<u32>),
}

impl Monomial {
    pub fn zero(dimensions: usize) -> Self {
        Self {
            powers: if dimensions <= 4 {
                Powers::Inline([0; 4])
            } else {
                Powers::Heap(alloc::vec![0; dimensions])
            },
        }
    }

    pub fn variable(dimensions: usize, variable: usize) -> Self {
        let mut result = Self::zero(dimensions);
        for (index, power) in result.as_mut_slice().iter_mut().enumerate() {
            *power = u32::from(index == variable);
        }
        result
    }

    pub fn sum(&self, other: &Self, work: &PolynomialWork) -> Result<Self, SimplifyError> {
        let powers =
            if let (Powers::Inline(left), Powers::Inline(right)) = (&self.powers, &other.powers) {
                Powers::Inline([
                    work.exponent_sum(left[0], right[0])?,
                    work.exponent_sum(left[1], right[1])?,
                    work.exponent_sum(left[2], right[2])?,
                    work.exponent_sum(left[3], right[3])?,
                ])
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
