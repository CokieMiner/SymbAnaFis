//! Sub-module for instruction-level evaluation engines.

#[cfg(feature = "parallel")]
const MAX_INLINE_PARAMS: usize = 16;

#[macro_use]
pub mod macros;
pub mod builtins;
pub mod scalar;

#[cfg(feature = "parallel")]
pub mod simd;

pub use super::VmEvaluator;
