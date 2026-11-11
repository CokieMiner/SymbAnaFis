//! Function identities, static builtins, and user-defined functions.

use super::{Expr, ExprView, FunctionError, NumericValue};

mod builtin;
mod names;
mod user_function;

pub use builtin::{BinaryRule, BuiltinFunction, BuiltinSet, FunctionDefinition, UnaryRule};
pub use names::Function;
pub use user_function::UserFunction;
