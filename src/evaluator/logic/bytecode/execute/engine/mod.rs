//! Sub-module for instruction-level evaluation engines.

#[macro_use]
pub mod macros;
pub mod builtins;
pub mod helpers;
pub mod scalar;

#[cfg(feature = "parallel")]
pub mod simd;

pub use super::CompiledEvaluator;
