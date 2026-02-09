//! Mathematical function evaluations
//!
//! This module centralizes all mathematical function implementations,
//! organized by category for maintainability.
//!
//! # Academic References
//!
//! Implementations follow standard numerical methods from:
//!
//! - **DLMF**: NIST Digital Library of Mathematical Functions <https://dlmf.nist.gov>
//! - **A&S**: Abramowitz & Stegun, "Handbook of Mathematical Functions" (1964)
//! - **NR**: Press et al., "Numerical Recipes" (3rd ed., 2007)
//! - Lanczos, C. "A Precision Approximation of the Gamma Function" (1964)
//! - Corless et al. "On the Lambert W Function" (1996)
//!
//! # Domain Validation
//!
//! Functions that can produce undefined results (poles, branch cuts, domain errors)
//! return `Option<T>` and check their inputs. Key validations include:
//!
//! - **Gamma functions**: Non-positive integers are poles
//! - **Zeta function**: s=1 is a pole
//! - **Logarithms**: Non-positive inputs are domain errors
//! - **Inverse trig**: |x| > 1 is a domain error for asin/acos
//! - **Square root**: Negative inputs return NaN or None depending on context

use crate::core::traits::MathScalar;

// Submodules for different function categories
/// Bessel functions
pub mod bessel;
pub mod dual;
/// Elliptic integrals
pub mod elliptic;
/// Polynomial functions
pub mod polynomials;
/// Special mathematical functions
pub mod special;

// Re-exports for backward compatibility
pub use bessel::*;
pub use elliptic::*;
pub use polynomials::*;
pub use special::*;

/// Here to remenber to implement properly when adding proper imaginary support
/// Exponential function for polar representation
///
/// Currently just wraps `exp()`. This function exists as a placeholder
/// for potential future polar-form exponential implementations.
pub fn eval_exp_polar<T: MathScalar>(x: T) -> T {
    x.exp()
}
