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
    pub start: usize,
    /// End position (exclusive, 0-indexed byte offset)
    pub end: usize,
}

impl Span {
    /// Create a new span
    #[inline]
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
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
    EmptyFormula,
    InvalidSyntax {
        msg: String,
        span: Option<Span>,
    },

    // Parsing errors
    InvalidNumber {
        value: String,
        span: Option<Span>,
    },
    InvalidToken {
        token: String,
        span: Option<Span>,
    },
    UnexpectedToken {
        expected: String,
        got: String,
        span: Option<Span>,
    },
    UnexpectedEndOfInput,

    // Semantic errors
    VariableInBothFixedAndDiff {
        var: String,
    },
    NameCollision {
        name: String,
    },
    UnsupportedOperation(String),
    AmbiguousSequence {
        sequence: String,
        suggestion: String,
        span: Option<Span>,
    },

    // Safety limits
    MaxDepthExceeded,
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
