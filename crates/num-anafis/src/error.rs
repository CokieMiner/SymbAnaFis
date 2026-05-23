//! Error types for `num-anafis`.

use alloc::boxed::Box;
#[cfg(feature = "std")]
use core::error::Error;
use core::fmt::{Display, Formatter, Result};

/// Error type for invalid `num-anafis` operations and constructors.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NumAnafisError {
    /// Active generators exceed the signature capacity.
    ActiveGeneratorsExceedSignature(Box<SignatureMismatchError>),
    /// Generator index is out of bounds for the active algebra.
    GeneratorIndexOutOfRange(Box<IndexOutOfRangeError>),
    /// Inline constructor was used beyond the supported inline threshold.
    InlineCoefficientsRequireAtMostFourGenerators {
        /// Requested active generator count.
        active: u8,
    },
    /// Dense coefficients length does not match `2^n`.
    DenseCoefficientLengthMismatch(Box<DenseLengthError>),
    /// Active generators exceed platform indexable width for `usize` bitmasks.
    ActiveGeneratorsTooLargeForPlatform {
        /// Requested active generator count.
        active: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignatureMismatchError {
    pub active: u8,
    pub available: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexOutOfRangeError {
    pub index: u8,
    pub active: u8,
}

/// Error details for length mismatches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DenseLengthError {
    /// Expected coefficient count.
    pub expected: usize,
    /// Provided coefficient count.
    pub found: usize,
}

impl Display for NumAnafisError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match *self {
            Self::ActiveGeneratorsExceedSignature(ref e) => {
                let (active, available) = (e.active, e.available);
                write!(
                    f,
                    "active generators ({active}) exceed signature generators ({available})"
                )
            }
            Self::GeneratorIndexOutOfRange(ref e) => {
                let (index, active) = (e.index, e.active);
                write!(f, "generator index {index} out of range for n={active}")
            }
            Self::InlineCoefficientsRequireAtMostFourGenerators { active } => {
                write!(f, "inline coefficients require n <= 4, got n={active}")
            }
            Self::DenseCoefficientLengthMismatch(ref e) => {
                let (expected, found) = (e.expected, e.found);
                write!(
                    f,
                    "dense coefficient length mismatch: expected {expected}, got {found}"
                )
            }
            Self::ActiveGeneratorsTooLargeForPlatform { active } => {
                write!(
                    f,
                    "active generators ({active}) exceed platform bit width for indexing"
                )
            }
        }
    }
}

#[cfg(feature = "std")]
impl Error for NumAnafisError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
