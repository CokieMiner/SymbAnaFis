//! Sub-module for bulk execution drivers (batch processing, multi-threading).

#[cfg(feature = "parallel")]
pub mod batch;

#[cfg(feature = "parallel")]
pub mod parallel;
