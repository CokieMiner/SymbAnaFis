//! DAG folding policy over `NumAnaFis`'s arithmetic and promotion rules.
//!
//! Native integer overflow and rational capacity failures belong to the backend.
//! These policies retain nonfinite results and approximate underflow as operations;
//! they neither replace backend arithmetic nor introduce another numeric type.

use core::ops::{Add, Div, Mul};

use num_anafis::{Number, Numeric};

use super::NumericValue;

impl NumericValue {
    /// Fold finite addition using the backend's representation promotion.
    pub fn add(&self, other: &Self) -> Option<Number> {
        let value = Add::add(&self.0, &other.0);
        value.is_finite().then_some(value)
    }

    /// Fold finite multiplication without erasing approximate underflow.
    pub fn mul(&self, other: &Self) -> Option<Number> {
        let value = Mul::mul(&self.0, &other.0);
        if value.is_zero()
            && !self.0.is_zero()
            && !other.0.is_zero()
            && (self.has_float() || other.has_float())
        {
            return None;
        }
        value.is_finite().then_some(value)
    }

    /// Fold finite division, preserving a zero denominator in the DAG.
    pub fn div(&self, other: &Self) -> Option<Number> {
        if other.0.is_zero() {
            return None;
        }
        let value = Div::div(&self.0, &other.0);
        if value.is_zero() && !self.0.is_zero() && (self.has_float() || other.has_float()) {
            return None;
        }
        value.is_finite().then_some(value)
    }

    /// Use `Numeric::pow` for integer exponents or approximate bases.
    /// Exact bases with general exponents remain symbolic.
    pub fn pow(&self, exponent: &Self) -> Option<Number> {
        if self.0.is_zero() && !exponent.0.is_positive() {
            return None;
        }
        if !self.has_float() && exponent.gaussian().is_none_or(|(_, im)| im != 0) {
            return None;
        }
        let value = Numeric::pow(&self.0, &exponent.0);
        if value.is_zero() && !self.0.is_zero() && self.has_float() {
            return None;
        }
        value.is_finite().then_some(value)
    }
}
