//! Conversions between Python property names and conservative Rust domain facts.

use pyo3::{PyResult, exceptions::PyValueError};
use symb_anafis::{Properties, Property, Truth};

/// Parse Python property names into [`Property`] values.
pub fn parse(names: impl IntoIterator<Item = String>) -> PyResult<Vec<Property>> {
    names
        .into_iter()
        .map(|name| match name.as_str() {
            "finite" => Ok(Property::Finite),
            "real" => Ok(Property::Real),
            "integer" => Ok(Property::Integer),
            "rational" => Ok(Property::Rational),
            "zero" => Ok(Property::Zero),
            "nonzero" => Ok(Property::Nonzero),
            "positive" => Ok(Property::Positive),
            "negative" => Ok(Property::Negative),
            "nonnegative" => Ok(Property::Nonnegative),
            "nonpositive" => Ok(Property::Nonpositive),
            _ => Err(PyValueError::new_err("unknown scalar property")),
        })
        .collect()
}

/// Export a [`Properties`] snapshot as `(name, Option<bool>)` pairs for Python.
pub fn facts(properties: Properties) -> Vec<(&'static str, Option<bool>)> {
    [
        ("finite", Property::Finite),
        ("real", Property::Real),
        ("integer", Property::Integer),
        ("rational", Property::Rational),
        ("zero", Property::Zero),
        ("nonzero", Property::Nonzero),
        ("positive", Property::Positive),
        ("negative", Property::Negative),
        ("nonnegative", Property::Nonnegative),
        ("nonpositive", Property::Nonpositive),
    ]
    .into_iter()
    .map(|(name, property)| {
        (
            name,
            match properties.get(property) {
                Truth::True => Some(true),
                Truth::False => Some(false),
                Truth::Unknown | _ => None,
            },
        )
    })
    .collect()
}
