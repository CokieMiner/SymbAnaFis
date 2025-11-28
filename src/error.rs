use std::fmt;

/// Errors that can occur during parsing and differentiation
#[derive(Debug, Clone, PartialEq)]
pub enum DiffError {
    // Input validation errors
    EmptyFormula,
    InvalidSyntax(String),

    // Parsing errors
    InvalidNumber(String),
    InvalidToken(String),
    UnexpectedToken { expected: String, got: String },
    UnexpectedEndOfInput,

    // Semantic errors
    VariableInBothFixedAndDiff { var: String },
    NameCollision { name: String },
    UnsupportedOperation(String),

    // Safety limits
    MaxDepthExceeded,
    MaxNodesExceeded,
}

impl fmt::Display for DiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiffError::EmptyFormula => write!(f, "Formula cannot be empty"),
            DiffError::InvalidSyntax(msg) => write!(f, "Invalid syntax: {}", msg),
            DiffError::InvalidNumber(s) => write!(f, "Invalid number format: '{}'", s),
            DiffError::InvalidToken(s) => write!(f, "Invalid token: '{}'", s),
            DiffError::UnexpectedToken { expected, got } => {
                write!(f, "Expected '{}', but got '{}'", expected, got)
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
