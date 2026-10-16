//! Diagnostics consumed by the parked parsing, differentiation, and evaluation engines.

use core::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
};

use alloc::string::String;

use super::Span;

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

    // Compilation errors (for VmEvaluator)
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

    /// Register index or parameter index overflowed u32.
    RegisterOverflow,
}

impl DiffError {
    /// Create `InvalidSyntax` without a source span.
    pub fn invalid_syntax(msg: impl Into<String>) -> Self {
        Self::InvalidSyntax {
            msg: msg.into(),
            span: None,
        }
    }

    /// Create `InvalidSyntax` with a source span.
    pub fn invalid_syntax_at(msg: impl Into<String>, span: Span) -> Self {
        Self::InvalidSyntax {
            msg: msg.into(),
            span: Some(span),
        }
    }

    /// Create `InvalidNumber` without a source span.
    pub fn invalid_number(value: impl Into<String>) -> Self {
        Self::InvalidNumber {
            value: value.into(),
            span: None,
        }
    }

    /// Create `InvalidToken` without a source span.
    pub fn invalid_token(token: impl Into<String>) -> Self {
        Self::InvalidToken {
            token: token.into(),
            span: None,
        }
    }
}

impl Display for DiffError {
    #[expect(
        clippy::too_many_lines,
        reason = "the exhaustive formatter keeps each diagnostic beside its error variant"
    )]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
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
            Self::RegisterOverflow => {
                write!(f, "Register index or parameter index overflowed u32 limit")
            }
        }
    }
}

impl Error for DiffError {}
