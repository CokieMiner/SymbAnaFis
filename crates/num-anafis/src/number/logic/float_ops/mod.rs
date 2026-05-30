// Default: f64
#[cfg(all(
    any(not(feature = "backend32"), feature = "backend64"),
    not(feature = "backendrug")
))]
mod f64_ops;

// Override: rug (MPFR-based arbitrary precision)
#[cfg(feature = "backendrug")]
mod rug_ops;

// Override: f32 (memory-optimized)
#[cfg(all(
    feature = "backend32",
    not(feature = "backend64"),
    not(feature = "backendrug")
))]
mod f32_ops;

// Special function algorithms shared by f32/f64 via SpecFloat trait.
// Rug uses native MPFR implementations instead.
#[cfg(not(feature = "backendrug"))]
pub(super) mod special;

mod api;

pub use api::*;
