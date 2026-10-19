//! Resource and rewrite errors from a simplification sequence.

use core::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
};

use super::RewriteError;

/// A simplification candidate exceeded an explicit resource bound.
/// No partial expression is returned on failure.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SimplifyError {
    /// An input or completed candidate has too many stored DAG nodes.
    NodeLimitExceeded {
        /// The maximum permitted distinct stored nodes.
        limit: usize,
    },
    /// An input or completed candidate has excessive stored DAG depth.
    DepthLimitExceeded {
        /// The maximum permitted stored depth, counting the root.
        limit: usize,
    },
    /// Expansion requires more generated terms than allowed in one pass.
    ExpansionLimitExceeded {
        /// The maximum permitted cumulative generated terms in one pass.
        limit: usize,
    },
    /// Sparse polynomial conversion or GCD work exceeded its per-pass bound.
    PolynomialLimitExceeded {
        /// The maximum permitted sparse polynomial work in one algebraic pass.
        limit: usize,
    },
    /// A user or builtin rewrite exhausted its commutative search bound.
    Rewrite(RewriteError),
}

impl Display for SimplifyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::NodeLimitExceeded { limit } => {
                write!(formatter, "simplification exceeded {limit} stored nodes")
            }
            Self::DepthLimitExceeded { limit } => {
                write!(formatter, "simplification exceeded stored depth {limit}")
            }
            Self::ExpansionLimitExceeded { limit } => {
                write!(formatter, "expansion exceeded {limit} generated terms")
            }
            Self::Rewrite(error) => Display::fmt(error, formatter),
            Self::PolynomialLimitExceeded { limit } => {
                write!(
                    formatter,
                    "polynomial simplification exceeded {limit} work units"
                )
            }
        }
    }
}

impl Error for SimplifyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Rewrite(error) => Some(error),
            Self::NodeLimitExceeded { .. }
            | Self::DepthLimitExceeded { .. }
            | Self::ExpansionLimitExceeded { .. }
            | Self::PolynomialLimitExceeded { .. } => None,
        }
    }
}

impl From<RewriteError> for SimplifyError {
    fn from(error: RewriteError) -> Self {
        Self::Rewrite(error)
    }
}
