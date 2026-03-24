//! Rule infrastructure for the simplification engine

#[macro_use]
pub mod core;
pub mod registry;

// Re-exports
pub use core::*;
pub use registry::*;

/// Numeric simplification rules
pub mod numeric;

/// Algebraic simplification rules
pub mod algebraic;

/// Trigonometric simplification rules
pub mod trigonometric;

/// Exponential and logarithmic simplification rules
pub mod exponential;

/// Root simplification rules
pub mod root;

/// Hyperbolic function simplification rules
pub mod hyperbolic;
