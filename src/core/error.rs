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

    /// Create a span for a single position
    #[inline]
    #[must_use]
    pub const fn at(pos: usize) -> Self {
        Self {
            start: pos,
            end: pos + 1,
        }
    }

    /// Create an empty/unknown span
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self { start: 0, end: 0 }
    }

    /// Get the start position
    #[inline]
    #[must_use]
    pub const fn start(&self) -> usize {
        self.start
    }

    /// Get the end position
    #[inline]
    #[must_use]
    pub const fn end(&self) -> usize {
        self.end
    }

    /// Check if this span has valid location info
    ///
    /// A span is valid if it covers at least one character (end > start).
    /// An empty span (0..0 or N..N) is considered invalid.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.end > self.start
    }

    /// Format the span for display (1-indexed for users)
    #[must_use]
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

    // Compilation errors (for CompiledEvaluator)
    /// Expression contains unsupported constructs for numeric evaluation.
    UnsupportedExpression(String),
    /// Function not supported in compiled evaluation.
    UnsupportedFunction(String),
    /// Variable not found in parameter list during compilation.
    UnboundVariable(String),
    /// Expression requires too much stack depth.
    StackOverflow {
        /// Current stack depth.
        depth: usize,
        /// Maximum allowed stack depth.
        limit: usize,
    },

    // Evaluation errors (for batch evaluation)
    /// Column count doesn't match parameter count.
    EvalColumnMismatch {
        /// Expected number of columns.
        expected: usize,
        /// Got number of columns.
        got: usize,
    },
    /// Column lengths are not all equal.
    EvalColumnLengthMismatch,
    /// Output buffer is too small.
    EvalOutputTooSmall {
        /// Number of data points needed.
        needed: usize,
        /// Output buffer size.
        got: usize,
    },

    // UserFunction errors
    /// Partial derivative index exceeds function arity.
    InvalidPartialIndex {
        /// The invalid argument index.
        index: usize,
        /// Maximum allowed arity.
        max_arity: usize,
    },
}

impl DiffError {
    // Convenience constructors for backward compatibility

    /// Create `InvalidSyntax` without span (backward compatible)
    pub fn invalid_syntax(msg: impl Into<String>) -> Self {
        Self::InvalidSyntax {
            msg: msg.into(),
            span: None,
        }
    }

    /// Create `InvalidSyntax` with span
    pub fn invalid_syntax_at(msg: impl Into<String>, span: Span) -> Self {
        Self::InvalidSyntax {
            msg: msg.into(),
            span: Some(span),
        }
    }

    /// Create `InvalidNumber` without span (backward compatible)
    pub fn invalid_number(value: impl Into<String>) -> Self {
        Self::InvalidNumber {
            value: value.into(),
            span: None,
        }
    }

    /// Create `InvalidToken` without span (backward compatible)
    pub fn invalid_token(token: impl Into<String>) -> Self {
        Self::InvalidToken {
            token: token.into(),
            span: None,
        }
    }
}

impl fmt::Display for DiffError {
    // Complex error display logic with many variants
    #[allow(
        clippy::too_many_lines,
        reason = "Complex error display logic with many variants"
    )]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyFormula => write!(f, "Formula cannot be empty"),
            Self::InvalidSyntax { msg, span } => {
                write!(
                    f,
                    "Invalid syntax: {}{}",
                    msg,
                    span.map_or(String::new(), |s| s.display())
                )
            }
            Self::InvalidNumber { value, span } => {
                write!(
                    f,
                    "Invalid number format: '{}'{}",
                    value,
                    span.map_or(String::new(), |s| s.display())
                )
            }
            Self::InvalidToken { token, span } => {
                write!(
                    f,
                    "Invalid token: '{}'{}",
                    token,
                    span.map_or(String::new(), |s| s.display())
                )
            }
            Self::UnexpectedToken {
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
            Self::UnexpectedEndOfInput => write!(f, "Unexpected end of input"),
            Self::InvalidFunctionCall {
                name,
                expected,
                got,
            } => {
                write!(
                    f,
                    "Function '{name}' requires at least {expected} argument(s), but got {got}"
                )
            }
            Self::VariableInBothFixedAndDiff { var } => {
                write!(
                    f,
                    "Variable '{var}' cannot be both the differentiation variable and a fixed constant"
                )
            }
            Self::NameCollision { name } => {
                write!(
                    f,
                    "Name '{name}' appears in both fixed_vars and custom_functions"
                )
            }
            Self::UnsupportedOperation(msg) => {
                write!(f, "Unsupported operation: {msg}")
            }
            Self::AmbiguousSequence {
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
            Self::MaxDepthExceeded => {
                write!(f, "Expression nesting depth exceeds maximum limit")
            }
            Self::MaxNodesExceeded => {
                write!(f, "Expression size exceeds maximum node count limit")
            }
            // Compile errors
            Self::UnsupportedExpression(msg) => {
                write!(f, "Unsupported expression: {msg}")
            }
            Self::UnsupportedFunction(name) => {
                write!(f, "Unsupported function for evaluation: {name}")
            }
            Self::UnboundVariable(name) => {
                write!(f, "Unbound variable: {name}")
            }
            Self::StackOverflow { depth, limit } => {
                write!(
                    f,
                    "Expression requires stack depth {depth} which exceeds limit {limit}"
                )
            }
            // Evaluation errors
            Self::EvalColumnMismatch { expected, got } => {
                write!(
                    f,
                    "Column count mismatch: expected {expected} columns, got {got}"
                )
            }
            Self::EvalColumnLengthMismatch => {
                write!(f, "All columns must have the same length")
            }
            Self::EvalOutputTooSmall { needed, got } => {
                write!(
                    f,
                    "Output buffer too small: need {needed} elements, got {got}"
                )
            }
            Self::InvalidPartialIndex { index, max_arity } => {
                write!(
                    f,
                    "Partial derivative index {index} exceeds maximum arity {max_arity}"
                )
            }
        }
    }
}

impl std::error::Error for DiffError {}

#[cfg(test)]
// Standard test relaxations: unwrap/panic for assertions, precision loss for math
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::let_underscore_must_use,
    clippy::no_effect_underscore_binding,
    reason = "Standard test relaxations"
)]
mod tests {
    use super::*;

    #[test]
    fn test_span_creation() {
        let span = Span::new(5, 10);
        assert_eq!(span.start(), 5);
        assert_eq!(span.end(), 10);
        assert!(span.is_valid());
    }

    #[test]
    fn test_span_swap() {
        let span = Span::new(10, 5);
        assert_eq!(span.start(), 5);
        assert_eq!(span.end(), 10);
        assert!(span.is_valid());
    }

    #[test]
    fn test_span_at() {
        let span = Span::at(7);
        assert_eq!(span.start(), 7);
        assert_eq!(span.end(), 8);
        assert!(span.is_valid());
    }

    #[test]
    fn test_span_empty() {
        let span = Span::empty();
        assert_eq!(span.start(), 0);
        assert_eq!(span.end(), 0);
        assert!(!span.is_valid());
    }

    #[test]
    fn test_span_display() {
        let span1 = Span::new(4, 8);
        assert_eq!(span1.display(), " at positions 5-8");

        let span2 = Span::at(9);
        assert_eq!(span2.display(), " at position 10");

        let span3 = Span::empty();
        assert_eq!(span3.display(), "");
    }

    #[test]
    fn test_diff_error_display() {
        let err1 = DiffError::EmptyFormula;
        assert_eq!(format!("{err1}"), "Formula cannot be empty");

        let err2 = DiffError::invalid_syntax("test message");
        assert_eq!(format!("{err2}"), "Invalid syntax: test message");

        let err3 = DiffError::invalid_syntax_at("spanned message", Span::new(1, 3));
        assert_eq!(
            format!("{err3}"),
            "Invalid syntax: spanned message at positions 2-3"
        );

        let err4 = DiffError::MaxDepthExceeded;
        assert_eq!(
            format!("{err4}"),
            "Expression nesting depth exceeds maximum limit"
        );
    }

    #[test]
    fn test_diff_error_constructors() {
        let err5 = DiffError::invalid_syntax("msg");
        match err5 {
            DiffError::InvalidSyntax { msg, span: None } => assert_eq!(msg, "msg"),
            _ => panic!("Wrong error type"),
        }

        let err6 = DiffError::invalid_syntax_at("msg", Span::at(5));
        match err6 {
            DiffError::InvalidSyntax {
                msg,
                span: Some(span),
            } => {
                assert_eq!(msg, "msg");
                assert_eq!(span.start(), 5);
            }
            _ => panic!("Wrong error type"),
        }
    }
}
