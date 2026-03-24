//! Sub-module for instruction-level evaluation engines.

pub(super) mod math;
pub mod scalar;

#[cfg(feature = "parallel")]
pub mod simd;
