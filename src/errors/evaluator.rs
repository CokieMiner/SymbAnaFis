//! Compilation and scalar input diagnostics.

use core::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
};

use alloc::string::String;

use num_anafis::Number;

/// A numerical program could not be compiled or supplied with complete inputs.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum EvalError {
    /// A symbol occurs twice in the ordered parameter list.
    DuplicateParameter {
        /// The repeated entry's zero-based index.
        index: usize,
    },
    /// A symbol in the expanded expression has no corresponding parameter.
    UnboundSymbol {
        /// The symbol's display name, or an anonymous identity label.
        name: String,
    },
    /// A custom function has no symbolic body for numerical compilation.
    MissingBody {
        /// The registered function name.
        name: String,
    },
    /// A symbolic body recursively invokes an active custom definition.
    RecursiveFunction {
        /// The registered function name.
        name: String,
    },
    /// An unevaluated derivative has no numerical implementation.
    UnevaluatedDerivative,
    /// Combined parameter, constant, and instruction counts exceed native capacity.
    ProgramTooLarge,
    /// The compiler produced a register layout that failed safety validation.
    InvalidProgram,
    /// A literal's domain or capacity is unavailable in the selected scalar.
    UnsupportedLiteral {
        /// The original literal.
        value: Number,
    },
    /// A selected scalar cannot construct pi or Euler's number.
    UnsupportedConstant {
        /// The mathematical constant name.
        name: &'static str,
    },
    /// Scalar evaluation requires exactly the compiled number of parameters.
    ParameterCount {
        /// The required parameter count.
        expected: usize,
        /// The supplied parameter count.
        actual: usize,
    },
    /// Each batch input column must contain one value per output entry.
    BatchLength {
        /// The incompatible input column's index.
        parameter: usize,
        /// The number of output entries.
        expected: usize,
        /// The number of input entries.
        actual: usize,
    },
}

impl Display for EvalError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::DuplicateParameter { index } => {
                write!(formatter, "parameter {index} repeats a symbol identity")
            }
            Self::UnboundSymbol { name } => {
                write!(formatter, "symbol {name} has no numerical input")
            }
            Self::MissingBody { name } => {
                write!(formatter, "function {name} has no numerical body")
            }
            Self::RecursiveFunction { name } => {
                write!(formatter, "function {name} has a recursive symbolic body")
            }
            Self::UnevaluatedDerivative => {
                formatter.write_str("an unevaluated derivative cannot be compiled numerically")
            }
            Self::ProgramTooLarge => {
                formatter.write_str("compiled register counts exceed native capacity")
            }
            Self::InvalidProgram => {
                formatter.write_str("compiled register layout failed safety validation")
            }
            Self::UnsupportedLiteral { value } => write!(
                formatter,
                "literal {value} is not representable in the evaluation scalar"
            ),
            Self::UnsupportedConstant { name } => write!(
                formatter,
                "constant {name} is not representable in the evaluation scalar"
            ),
            Self::ParameterCount { expected, actual } => write!(
                formatter,
                "expected {expected} parameters, received {actual}"
            ),
            Self::BatchLength {
                parameter,
                expected,
                actual,
            } => write!(
                formatter,
                "parameter column {parameter} has {actual} values; expected {expected}"
            ),
        }
    }
}

impl Error for EvalError {}
