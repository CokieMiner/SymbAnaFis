//! Compiled scalar numerical evaluation over canonical expression DAGs.

mod direct;
mod program;
mod traversal;

pub use crate::core::EvaluationValue;
pub use program::{Program, Workspace};
pub use traversal::Traversal;

#[cfg(test)]
mod tests;
