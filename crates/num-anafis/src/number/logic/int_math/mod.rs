// Default: i64
#[cfg(all(not(feature = "backend32"), not(feature = "backendrug")))]
mod i64_math;

// Override: rug (GMP-based arbitrary precision)
#[cfg(feature = "backendrug")]
mod rug_int;

// Override: i32 (memory-optimized)
#[cfg(all(feature = "backend32", not(feature = "backendrug")))]
mod i32_math;

mod api;

pub use api::*;
