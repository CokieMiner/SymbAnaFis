//! Representation-aware literal identity and numeric folding policy.

#![expect(
    clippy::self_named_module_files,
    reason = "the type owner keeps fields private while its descendants implement operations; mod.rs files contain plumbing only"
)]

use num_anafis::{Complex, Number, Real};

use super::{BinaryRule, BuiltinFunction, UnaryRule};

static IMAGINARY_UNIT: NumericValue = NumericValue(Number::i());

/// A `NumAnaFis` value with reflexive, representation-preserving DAG identity.
#[derive(Clone, Debug)]
pub struct NumericValue(Number);

impl NumericValue {
    /// Retain the supplied numeric representation.
    pub const fn new(value: Number) -> Self {
        Self(value)
    }

    /// Borrow the stored value without projecting exact or complex components.
    pub const fn value(&self) -> &Number {
        &self.0
    }

    /// The exact scalar imaginary unit, shared by the dedicated DAG variant.
    pub const fn imaginary_unit() -> &'static Self {
        &IMAGINARY_UNIT
    }

    /// Recognize stored exact Gaussian integers, excluding integral floats.
    pub fn gaussian(&self) -> Option<(i64, i64)> {
        let parts = self.parts()?;
        if let (Real::Int(re), Real::Int(im)) = (parts.re, parts.im) {
            Some((re, im))
        } else {
            None
        }
    }

    pub fn has_float(&self) -> bool {
        self.parts()
            .is_some_and(|parts| parts.re.is_float() || parts.im.is_float())
    }

    /// Extract typed real components, without floating-point conversion.
    pub fn parts(&self) -> Option<Complex> {
        self.0.extract_complex().ok()
    }
}

mod approximate;
mod arithmetic;
mod display;
mod functions;
mod identity;
mod quotients;

#[cfg(test)]
mod tests;
