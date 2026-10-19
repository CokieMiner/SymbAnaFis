//! Compiled scalar numerical evaluation over canonical expression DAGs.

mod direct;
mod program;
mod traversal;

pub use crate::core::EvaluationValue;
pub use program::Evaluator;
pub use traversal::Traversal;

#[cfg(test)]
mod tests;
