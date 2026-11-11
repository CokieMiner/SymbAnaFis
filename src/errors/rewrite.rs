//! Validation and work-limit errors for rewrite rules.

use core::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
};

use alloc::string::String;

/// A replacement template is invalid or a bounded match exhausts its work limit.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RewriteError {
    /// A named replacement capture is absent from the input pattern.
    UnboundCapture(String),
    /// Commutative matching requires more candidate attempts than permitted.
    MatchLimitExceeded {
        /// The maximum candidate attempts for this rewrite pass.
        limit: usize,
    },
}

impl Display for RewriteError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::UnboundCapture(name) => {
                write!(formatter, "replacement capture '{name}' is unbound")
            }
            Self::MatchLimitExceeded { limit } => {
                write!(
                    formatter,
                    "commutative matching exceeded {limit} candidate attempts"
                )
            }
        }
    }
}

impl Error for RewriteError {}
