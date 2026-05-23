#[cfg(not(feature = "backendrug"))]
mod primitive_math;

// Override: rug (GMP-based arbitrary precision)
#[cfg(feature = "backendrug")]
mod rug_ops;

mod api;

pub use api::*;
