//! Numerical evaluation backends for symbolic expressions.
//!
//! The public evaluator API is exposed through [`api`], while the concrete
//! compilation and execution backends stay internal to this module.
//!
//! Today the primary fast path is a compiled bytecode evaluator with scalar,
//! SIMD, and parallel execution.

mod api;
pub mod logic;

pub use api::*;
