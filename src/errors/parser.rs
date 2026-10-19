//! Textual expression diagnostics and source locations.

use core::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
};

use alloc::{format, string::String};

use super::{FunctionError, SymbolError};

/// An expression cannot be parsed under its context's symbol and function bindings.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseError {
    /// No expression occurs in the source.
    EmptyInput,
    /// A character has no grammatical meaning.
    InvalidCharacter {
        /// The rejected character.
        character: char,
        /// The character's UTF-8 byte range.
        span: Span,
    },
    /// A literal is malformed, exceeds `i64`, or has a nonfinite `f64` value.
    InvalidNumber {
        /// The rejected literal's byte range.
        span: Span,
    },
    /// An operand is required at this location.
    ExpectedExpression {
        /// The offending token or the empty range at end of input.
        span: Span,
    },
    /// A token cannot follow the preceding expression.
    UnexpectedToken {
        /// The rejected token's byte range.
        span: Span,
    },
    /// An opening parenthesis has no matching closing parenthesis.
    UnclosedParenthesis {
        /// The opening parenthesis's byte range.
        span: Span,
    },
    /// A known function rejects the supplied argument count.
    Function {
        /// The function call's byte range.
        span: Span,
        /// The violated function contract.
        error: FunctionError,
    },
    /// A new scoped symbol cannot be allocated.
    Symbol {
        /// The symbol name's byte range.
        span: Span,
        /// The symbol allocation failure.
        error: SymbolError,
    },
}

/// A half-open range of byte offsets in a source string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Display for ParseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::EmptyInput => formatter.write_str("expected an expression"),
            Self::InvalidCharacter { character, span } => write!(
                formatter,
                "invalid character '{character}' at bytes {}..{}",
                span.start, span.end
            ),
            Self::InvalidNumber { span } => write!(
                formatter,
                "invalid or out-of-range number at bytes {}..{}",
                span.start, span.end
            ),
            Self::ExpectedExpression { span } => write!(
                formatter,
                "expected an expression at bytes {}..{}",
                span.start, span.end
            ),
            Self::UnexpectedToken { span } => write!(
                formatter,
                "unexpected token at bytes {}..{}",
                span.start, span.end
            ),
            Self::UnclosedParenthesis { span } => write!(
                formatter,
                "unclosed parenthesis at bytes {}..{}",
                span.start, span.end
            ),
            Self::Function { span, error } => {
                write!(formatter, "{error} at bytes {}..{}", span.start, span.end)
            }
            Self::Symbol { span, error } => {
                write!(formatter, "{error} at bytes {}..{}", span.start, span.end)
            }
        }
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Function { error, .. } => Some(error),
            Self::Symbol { error, .. } => Some(error),
            Self::EmptyInput
            | Self::InvalidCharacter { .. }
            | Self::InvalidNumber { .. }
            | Self::ExpectedExpression { .. }
            | Self::UnexpectedToken { .. }
            | Self::UnclosedParenthesis { .. } => None,
        }
    }
}

impl Span {
    /// Format nonempty spans with one-based byte positions.
    #[must_use]
    pub fn display(&self) -> String {
        if !self.is_valid() {
            String::new()
        } else if self.end.checked_sub(self.start) == Some(1) {
            // A valid span proves start < end, so the increment fits usize.
            format!(" at position {}", self.start.saturating_add(1))
        } else {
            format!(
                " at positions {}-{}",
                self.start.saturating_add(1),
                self.end
            )
        }
    }

    /// Create an ordered span, swapping the endpoints when needed.
    #[inline]
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        if end < start {
            Self {
                start: end,
                end: start,
            }
        } else {
            Self { start, end }
        }
    }

    /// Create a span covering one byte.
    ///
    /// # Panics
    /// Panics at `usize::MAX`, whose exclusive end is unrepresentable.
    #[inline]
    #[must_use]
    pub const fn at(pos: usize) -> Self {
        Self {
            start: pos,
            end: pos
                .checked_add(1)
                .expect("a single-position span needs a representable exclusive end"),
        }
    }

    /// Create an empty span without source information.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self { start: 0, end: 0 }
    }

    /// Get the inclusive start byte offset.
    #[inline]
    #[must_use]
    pub const fn start(&self) -> usize {
        self.start
    }

    /// Get the exclusive end byte offset.
    #[inline]
    #[must_use]
    pub const fn end(&self) -> usize {
        self.end
    }

    /// Test whether the span covers at least one byte.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.end > self.start
    }
}
