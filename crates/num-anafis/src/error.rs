//! Error types for `num-anafis`.

use core::fmt;

/// Error type for invalid `num-anafis` operations and constructors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumAnafisError {
    /// Active generators exceed the signature capacity.
    ActiveGeneratorsExceedSignature {
        /// Requested active generator count.
        active: u8,
        /// Available generators in the signature.
        available: u8,
    },
    /// Generator index is out of bounds for the active algebra.
    GeneratorIndexOutOfRange {
        /// Requested generator index.
        index: u8,
        /// Active generator count.
        active: u8,
    },
    /// Inline constructor was used beyond the supported inline threshold.
    InlineCoefficientsRequireAtMostFourGenerators {
        /// Requested active generator count.
        active: u8,
    },
    /// Dense coefficients length does not match `2^n`.
    DenseCoefficientLengthMismatch {
        /// Expected coefficient count (`2^n`).
        expected: usize,
        /// Provided coefficient count.
        found: usize,
    },
    /// Active generators exceed platform indexable width for `usize` bitmasks.
    ActiveGeneratorsTooLargeForPlatform {
        /// Requested active generator count.
        active: u8,
    },
}

impl fmt::Display for NumAnafisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ActiveGeneratorsExceedSignature { active, available } => {
                write!(
                    f,
                    "active generators ({active}) exceed signature generators ({available})"
                )
            }
            Self::GeneratorIndexOutOfRange { index, active } => {
                write!(f, "generator index {index} out of range for n={active}")
            }
            Self::InlineCoefficientsRequireAtMostFourGenerators { active } => {
                write!(f, "inline coefficients require n <= 4, got n={active}")
            }
            Self::DenseCoefficientLengthMismatch { expected, found } => {
                write!(f, "dense coefficient length mismatch: expected {expected}, got {found}")
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

impl std::error::Error for NumAnafisError {}