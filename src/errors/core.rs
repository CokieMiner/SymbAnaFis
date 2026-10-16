//! Validation errors for core identities and transformations.

use core::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
};

use alloc::string::String;

use crate::core::Expr;

/// A function registration or call violates its name or argument contract.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FunctionError {
    /// A custom function name is already registered in this namespace.
    DuplicateName(String),
    /// The accepted argument range contains no possible argument count.
    InvalidArityRange {
        /// The rejected minimum argument count.
        minimum: usize,
        /// The rejected maximum argument count.
        maximum: usize,
    },
    /// The argument count lies outside the function's registered range.
    InvalidArity {
        /// The minimum accepted argument count.
        minimum: usize,
        /// The maximum accepted argument count.
        maximum: usize,
        /// The supplied argument count.
        actual: usize,
    },
    /// No accepted argument count contains this zero-based partial index.
    InvalidPartialIndex {
        /// The rejected argument index.
        index: usize,
        /// The maximum accepted number of arguments.
        max_arity: usize,
    },
}

/// Failures while registering or looking up symbols.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SymbolError {
    /// The global name is already bound.
    DuplicateName(String),
    /// The global name is absent.
    NotFound(String),
    /// The ID is absent from the requested owner or has been reclaimed.
    InvalidId(usize),
    /// Symbol IDs have exhausted the native `usize` range.
    IdExhausted,
}

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

/// One concrete source has two distinct canonical destinations.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SubstitutionError {
    /// Duplicate entries disagree on the destination for this source.
    Conflict {
        /// The shared canonical source expression.
        from: Expr,
        /// The destination already registered for the source.
        first: Expr,
        /// The different destination requested by the duplicate entry.
        second: Expr,
    },
}

impl Display for FunctionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::InvalidArityRange { minimum, maximum } => write!(
                formatter,
                "function argument range {minimum}..={maximum} is empty"
            ),
            Self::DuplicateName(name) => {
                write!(formatter, "function '{name}' is already registered")
            }
            Self::InvalidArity {
                minimum,
                maximum,
                actual,
            } => write!(
                formatter,
                "function accepts {minimum}..={maximum} arguments; received {actual}"
            ),
            Self::InvalidPartialIndex { index, max_arity } => write!(
                formatter,
                "partial index {index} is outside the accepted arity ending at {max_arity}"
            ),
        }
    }
}

impl Error for FunctionError {}

impl Display for SymbolError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::DuplicateName(name) => write!(formatter, "symbol '{name}' is already registered"),
            Self::NotFound(name) => write!(formatter, "symbol '{name}' is not registered"),
            Self::InvalidId(id) => {
                write!(formatter, "symbol ID {id} is not available in this owner")
            }
            Self::IdExhausted => formatter.write_str("symbol IDs exhausted"),
        }
    }
}

impl Error for SymbolError {}

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

impl Display for SubstitutionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Conflict {
                from,
                first,
                second,
            } => write!(
                formatter,
                "conflicting destinations for {from}: {first} and {second}"
            ),
        }
    }
}

impl Error for SubstitutionError {}
