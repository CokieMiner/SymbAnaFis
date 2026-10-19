//! Shape and scalar-domain diagnostics for first-order uncertainty propagation.

use core::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
};

/// Invalid covariance data or a disproved real, finite propagation condition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum UncertaintyError {
    /// A covariance row does not have the matrix's dimension.
    NonSquare {
        /// The offending row.
        row: usize,
        /// The matrix dimension.
        expected: usize,
        /// The row length.
        actual: usize,
    },
    /// Opposite covariance entries are neither identical nor numerically equal.
    Asymmetric {
        /// The upper-triangle row.
        row: usize,
        /// The upper-triangle column.
        column: usize,
    },
    /// An entry is proved nonfinite/nonreal, or a diagonal variance is negative.
    InvalidCovariance {
        /// The offending row.
        row: usize,
        /// The offending column.
        column: usize,
    },
    /// The supplied coordinate count differs from the covariance dimension.
    DimensionMismatch {
        /// The coordinate count.
        variables: usize,
        /// The covariance dimension.
        covariance: usize,
    },
    /// A coordinate identity occurs twice.
    DuplicateVariable {
        /// The first occurrence.
        first: usize,
        /// The repeated occurrence.
        duplicate: usize,
    },
    /// The model is proved nonreal or nonfinite.
    InvalidModel,
    /// A sensitivity coefficient is proved nonreal or nonfinite.
    InvalidSensitivity {
        /// Its coordinate index.
        index: usize,
    },
    /// The constructed variance is proved negative, nonreal, or nonfinite.
    InvalidVariance,
}

impl Display for UncertaintyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::NonSquare {
                row,
                expected,
                actual,
            } => write!(
                formatter,
                "covariance row {row} has {actual} entries; expected {expected}"
            ),
            Self::Asymmetric { row, column } => write!(
                formatter,
                "covariance entries ({row}, {column}) and ({column}, {row}) disagree"
            ),
            Self::InvalidCovariance { row, column } => write!(
                formatter,
                "covariance entry ({row}, {column}) violates its finite real variance contract"
            ),
            Self::DimensionMismatch {
                variables,
                covariance,
            } => write!(
                formatter,
                "{variables} variables supplied for covariance dimension {covariance}"
            ),
            Self::DuplicateVariable { first, duplicate } => write!(
                formatter,
                "coordinate {duplicate} repeats the identity at coordinate {first}"
            ),
            Self::InvalidModel => {
                formatter.write_str("uncertainty propagation requires a finite real model")
            }
            Self::InvalidSensitivity { index } => {
                write!(formatter, "sensitivity {index} is not finite and real")
            }
            Self::InvalidVariance => {
                formatter.write_str("propagated variance is not finite, real, and nonnegative")
            }
        }
    }
}

impl Error for UncertaintyError {}
