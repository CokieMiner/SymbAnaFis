//! Error types for parsing and differentiation
//!
//! This module provides:
//! - `DiffError` - The main error enum for all parsing/differentiation failures
//! - `Span` - Source location tracking for precise error messages

use std::fmt;

/// Source location span for error reporting
/// Represents a range of characters in the input string
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// Start position (0-indexed byte offset)
    start: usize,
    /// End position (exclusive, 0-indexed byte offset)
    end: usize,
}

impl Span {
    /// Create a new span. If end < start, they will be swapped.
    #[inline]
    pub fn new(start: usize, end: usize) -> Self {
        if end < start {
            Span {
                start: end,
                end: start,
            }
        } else {
            Span { start, end }
        }
    }

    /// Create a span for a single position
    #[inline]
    pub fn at(pos: usize) -> Self {
        Span {
            start: pos,
            end: pos + 1,
        }
    }

    /// Create an empty/unknown span
    #[inline]
    pub fn empty() -> Self {
        Span { start: 0, end: 0 }
    }

    /// Get the start position
    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }

    /// Get the end position
    #[inline]
    pub fn end(&self) -> usize {
        self.end
    }

    /// Check if this span has valid location info
    ///
    /// A span is valid if it covers at least one character (end > start).
    /// An empty span (0..0 or N..N) is considered invalid.
    pub fn is_valid(&self) -> bool {
        self.end > self.start
    }

    /// Format the span for display (1-indexed for users)
    pub fn display(&self) -> String {
        if !self.is_valid() {
            String::new()
        } else if self.end - self.start == 1 {
            format!(" at position {}", self.start + 1)
        } else {
            format!(" at positions {}-{}", self.start + 1, self.end)
        }
    }
}

/// Errors that can occur during parsing and differentiation
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiffError {
    // Input validation errors
    /// The input formula was empty or contained only whitespace.
    EmptyFormula,
    /// The input has invalid syntax.
    InvalidSyntax {
        /// Description of the syntax error.
        msg: String,
        /// Location of the error in the source.
        span: Option<Span>,
    },

    // Parsing errors
    /// A numeric literal could not be parsed.
    InvalidNumber {
        /// The invalid number string.
        value: String,
        /// Location of the error in the source.
        span: Option<Span>,
    },
    /// An unrecognized token was encountered.
    InvalidToken {
        /// The invalid token.
        token: String,
        /// Location of the error in the source.
        span: Option<Span>,
    },
    /// A different token was expected at this position.
    UnexpectedToken {
        /// What was expected.
        expected: String,
        /// What was found.
        got: String,
        /// Location of the error in the source.
        span: Option<Span>,
    },
    /// The input ended unexpectedly while parsing.
    UnexpectedEndOfInput,
    /// A function was called with the wrong number of arguments.
    InvalidFunctionCall {
        /// The function name.
        name: String,
        /// Minimum number of arguments expected.
        expected: usize,
        /// Number of arguments provided.
        got: usize,
    },

    // Semantic errors
    /// A variable appears in both fixed variables and as differentiation target.
    VariableInBothFixedAndDiff {
        /// The conflicting variable name.
        var: String,
    },
    /// A name is used for both a variable and a function.
    NameCollision {
        /// The conflicting name.
        name: String,
    },
    /// An operation is not supported (e.g., unsupported function).
    UnsupportedOperation(String),
    /// An ambiguous token sequence was found.
    AmbiguousSequence {
        /// The ambiguous sequence.
        sequence: String,
        /// Suggested resolution.
        suggestion: String,
        /// Location of the error in the source.
        span: Option<Span>,
    },

    // Safety limits
    /// The expression exceeded the maximum allowed AST depth.
    MaxDepthExceeded,
    /// The expression exceeded the maximum allowed node count.
    MaxNodesExceeded,
}

impl DiffError {
    // Convenience constructors for backward compatibility

    /// Create InvalidSyntax without span (backward compatible)
    pub fn invalid_syntax(msg: impl Into<String>) -> Self {
        DiffError::InvalidSyntax {
            msg: msg.into(),
            span: None,
        }
    }

    /// Create InvalidSyntax with span
    pub fn invalid_syntax_at(msg: impl Into<String>, span: Span) -> Self {
        DiffError::InvalidSyntax {
            msg: msg.into(),
            span: Some(span),
        }
    }

    /// Create InvalidNumber without span (backward compatible)
    pub fn invalid_number(value: impl Into<String>) -> Self {
        DiffError::InvalidNumber {
            value: value.into(),
            span: None,
        }
    }

    /// Create InvalidToken without span (backward compatible)
    pub fn invalid_token(token: impl Into<String>) -> Self {
        DiffError::InvalidToken {
            token: token.into(),
            span: None,
        }
    }
}

impl fmt::Display for DiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiffError::EmptyFormula => write!(f, "Formula cannot be empty"),
            DiffError::InvalidSyntax { msg, span } => {
                write!(
                    f,
                    "Invalid syntax: {}{}",
                    msg,
                    span.map_or(String::new(), |s| s.display())
                )
            }
            DiffError::InvalidNumber { value, span } => {
                write!(
                    f,
                    "Invalid number format: '{}'{}",
                    value,
                    span.map_or(String::new(), |s| s.display())
                )
            }
            DiffError::InvalidToken { token, span } => {
                write!(
                    f,
                    "Invalid token: '{}'{}",
                    token,
                    span.map_or(String::new(), |s| s.display())
                )
            }
            DiffError::UnexpectedToken {
                expected,
                got,
                span,
            } => {
                write!(
                    f,
                    "Expected '{}', but got '{}'{}",
                    expected,
                    got,
                    span.map_or(String::new(), |s| s.display())
                )
            }
            DiffError::UnexpectedEndOfInput => write!(f, "Unexpected end of input"),
            DiffError::InvalidFunctionCall {
                name,
                expected,
                got,
            } => {
                write!(
                    f,
                    "Function '{}' requires at least {} argument(s), but got {}",
                    name, expected, got
                )
            }
            DiffError::VariableInBothFixedAndDiff { var } => {
                write!(
                    f,
                    "Variable '{}' cannot be both the differentiation variable and a fixed constant",
                    var
                )
            }
            DiffError::NameCollision { name } => {
                write!(
                    f,
                    "Name '{}' appears in both fixed_vars and custom_functions",
                    name
                )
            }
            DiffError::UnsupportedOperation(msg) => {
                write!(f, "Unsupported operation: {}", msg)
            }
            DiffError::AmbiguousSequence {
                sequence,
                suggestion,
                span,
            } => {
                write!(
                    f,
                    "Ambiguous identifier sequence '{}': {}.{} \
                     Consider using explicit multiplication (e.g., 'x*sin(y)') or \
                     declaring multi-character variables in fixed_vars.",
                    sequence,
                    suggestion,
                    span.map_or(String::new(), |s| s.display())
                )
            }
            DiffError::MaxDepthExceeded => {
                write!(f, "Expression nesting depth exceeds maximum limit")
            }
            DiffError::MaxNodesExceeded => {
                write!(f, "Expression size exceeds maximum node count limit")
            }
        }
    }
}

impl std::error::Error for DiffError {}
