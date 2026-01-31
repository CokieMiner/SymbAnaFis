//! Error conversion and domain validation for Python bindings

use crate::{DiffError, SymbolError};
use pyo3::PyErr;

// =============================================================================
// Error Conversion: DiffError -> PyErr
// =============================================================================
// Maps Rust errors to appropriate Python exception types:
// - ValueError: semantic/validation errors
// - SyntaxError: parsing errors
// - RuntimeError: compilation/evaluation errors

impl From<DiffError> for PyErr {
    fn from(err: DiffError) -> Self {
        match &err {
            // Semantic/validation errors → ValueError
            DiffError::EmptyFormula
            | DiffError::InvalidSyntax { .. }
            | DiffError::InvalidNumber { .. }
            | DiffError::InvalidFunctionCall { .. }
            | DiffError::VariableInBothFixedAndDiff { .. }
            | DiffError::MaxDepthExceeded
            | DiffError::MaxNodesExceeded
            | DiffError::EvalColumnMismatch { .. }
            | DiffError::EvalColumnLengthMismatch
            | DiffError::EvalOutputTooSmall { .. }
            | DiffError::InvalidPartialIndex { .. } => {
                Self::new::<pyo3::exceptions::PyValueError, _>(err.to_string())
            }
            // Parse errors → SyntaxError
            DiffError::InvalidToken { .. }
            | DiffError::UnexpectedToken { .. }
            | DiffError::UnexpectedEndOfInput
            | DiffError::AmbiguousSequence { .. } => {
                Self::new::<pyo3::exceptions::PySyntaxError, _>(err.to_string())
            }
            // Compile/runtime errors → RuntimeError
            DiffError::UnsupportedOperation(_)
            | DiffError::UnsupportedExpression(_)
            | DiffError::UnsupportedFunction(_)
            | DiffError::UnboundVariable(_)
            | DiffError::StackOverflow { .. }
            | DiffError::NameCollision { .. } => {
                Self::new::<pyo3::exceptions::PyRuntimeError, _>(err.to_string())
            }
        }
    }
}

impl From<SymbolError> for PyErr {
    fn from(err: SymbolError) -> Self {
        Self::new::<pyo3::exceptions::PyValueError, _>(err.to_string())
    }
}
