//! Compiled scalar numerical evaluation over canonical expression DAGs.

use traversal::Traversal;

mod direct;
mod program;
mod traversal;
mod value;

pub use program::{MultiProgram, NativeScalar, Program, WasmScalar, Workspace};
pub use value::EvaluationValue;

#[cfg(test)]
mod tests;
