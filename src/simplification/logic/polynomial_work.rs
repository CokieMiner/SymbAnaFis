//! Shared work accounting for sparse polynomial conversion and arithmetic.

use crate::errors::SimplifyError;

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

    pub fn spend(&mut self, count: usize) -> Result<(), SimplifyError> {
        self.remaining = self
            .remaining
            .checked_sub(count)
            .ok_or(SimplifyError::PolynomialLimitExceeded { limit: self.limit })?;
        Ok(())
    }

    pub fn product(&mut self, left: usize, right: usize) -> Result<(), SimplifyError> {
        let count = left
            .checked_mul(right)
            .ok_or(SimplifyError::PolynomialLimitExceeded { limit: self.limit })?;
        self.spend(count)
    }

    pub fn exponent_sum(&self, left: u32, right: u32) -> Result<u32, SimplifyError> {
        left.checked_add(right)
            .ok_or(SimplifyError::PolynomialLimitExceeded { limit: self.limit })
    }
}
