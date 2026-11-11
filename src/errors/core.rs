//! Validation errors for core identities and transformations.

use core::{
    error::Error,
    fmt::{Display, Error as FmtError, Formatter, Result as FmtResult, Write},
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
    /// The declared return properties contradict each other.
    ContradictoryProperties,
}

/// Failures while registering, looking up, or configuring symbols.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SymbolError {
    /// The global name is already bound.
    DuplicateName(String),
    /// The ID is absent from the requested owner or has been reclaimed.
    InvalidId(usize),
    /// Symbol IDs have exhausted the native `usize` range.
    IdExhausted,
    /// The declared symbol properties contradict each other.
    ContradictoryProperties,
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
            Self::ContradictoryProperties => {
                formatter.write_str("declared function properties are contradictory")
            }
        }
    }
}

impl Error for FunctionError {}

impl Display for SymbolError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::DuplicateName(name) => write!(formatter, "symbol '{name}' is already registered"),
            Self::InvalidId(id) => {
                write!(formatter, "symbol ID {id} is not available in this owner")
            }
            Self::IdExhausted => formatter.write_str("symbol IDs exhausted"),
            Self::ContradictoryProperties => {
                formatter.write_str("declared symbol properties are contradictory")
            }
        }
    }
}

impl Error for SymbolError {}

impl Display for SubstitutionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Conflict {
                from,
                first,
                second,
            } => {
                formatter.write_str("conflicting destinations for ")?;
                write_bounded(from, formatter)?;
                formatter.write_str(": ")?;
                write_bounded(first, formatter)?;
                formatter.write_str(" and ")?;
                write_bounded(second, formatter)
            }
        }
    }
}

impl Error for SubstitutionError {}

/// Byte budget for each expression rendered inside a conflict message.
const CONFLICT_EXPRESSION_BUDGET: usize = 96;

/// Appends at most its budget of rendered text and records any truncation.
struct BoundedSink<'formatter, 'output> {
    formatter: &'formatter mut Formatter<'output>,
    remaining: usize,
    truncated: bool,
}

impl Write for BoundedSink<'_, '_> {
    fn write_str(&mut self, text: &str) -> FmtResult {
        if self.truncated {
            return Err(FmtError);
        }
        if text.len() <= self.remaining {
            self.formatter.write_str(text)?;
            self.remaining = self.remaining.saturating_sub(text.len());
            return Ok(());
        }
        let mut end = self.remaining;
        while end > 0 && !text.is_char_boundary(end) {
            end = end.saturating_sub(1);
        }
        if let Some(prefix) = text.get(..end)
            && !prefix.is_empty()
        {
            self.formatter.write_str(prefix)?;
        }
        self.remaining = 0;
        self.truncated = true;
        Err(FmtError)
    }
}

/// Writes `expression` into `formatter`, truncated to [`CONFLICT_EXPRESSION_BUDGET`] bytes.
fn write_bounded(expression: &Expr, formatter: &mut Formatter<'_>) -> FmtResult {
    let mut sink = BoundedSink {
        formatter,
        remaining: CONFLICT_EXPRESSION_BUDGET,
        truncated: false,
    };
    match write!(sink, "{expression}") {
        Ok(()) => Ok(()),
        Err(_) if sink.truncated => sink.formatter.write_str("..."),
        Err(error) => Err(error),
    }
}
