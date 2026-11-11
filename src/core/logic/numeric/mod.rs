//! Numeric literals with representation identity and scalar evaluation.

use super::{BinaryRule, BuiltinFunction, UnaryRule};

mod scalar;
mod value;

pub use scalar::Kernel;
pub use value::NumericValue;
