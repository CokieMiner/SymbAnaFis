//! Python exception identities and subsystem error conversion.

use core::fmt::Display;

use pyo3::{
    PyErr, PyResult, Python, create_exception, exceptions::PyValueError, types::PyAnyMethods,
};
use symb_anafis::ParseError as CoreParseError;

create_exception!(
    symb_anafis,
    ParseError,
    PyValueError,
    "A formula contains an invalid token, name, or syntax span."
);
create_exception!(
    symb_anafis,
    RewriteError,
    PyValueError,
    "A rule has invalid captures or exceeded its matching bound."
);
create_exception!(
    symb_anafis,
    SimplifyError,
    PyValueError,
    "A simplification exceeded its configured work or growth bounds."
);
create_exception!(
    symb_anafis,
    EvaluationError,
    PyValueError,
    "Numerical inputs, function bodies, or program shapes are invalid."
);
create_exception!(
    symb_anafis,
    UncertaintyError,
    PyValueError,
    "Covariance data or real first-order propagation conditions are invalid."
);

pub struct Diagnostics;

impl Diagnostics {
    pub fn value(error: impl Display) -> PyErr {
        PyValueError::new_err(error.to_string())
    }

    pub fn parse(error: &CoreParseError, source: &str, py: Python<'_>) -> PyResult<PyErr> {
        let span = match error {
            CoreParseError::InvalidCharacter { span, .. }
            | CoreParseError::InvalidNumber { span }
            | CoreParseError::ExpectedExpression { span }
            | CoreParseError::UnexpectedToken { span }
            | CoreParseError::UnclosedParenthesis { span }
            | CoreParseError::Function { span, .. }
            | CoreParseError::Symbol { span, .. } => Some((span.start(), span.end())),
            CoreParseError::EmptyInput | _ => None,
        };
        let exception = ParseError::new_err(error.to_string());
        exception.value(py).setattr("span", span)?;
        exception.value(py).setattr("source", source)?;
        Ok(exception)
    }
}
