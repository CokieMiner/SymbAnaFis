//! API surface for the `math` module.

// Public to external library users
pub use super::logic::number_types::dual::Dual;

// Crate-internal functions
pub use super::logic::functions::bessel::*;
pub use super::logic::functions::beta::*;
pub use super::logic::functions::elliptic::*;
pub use super::logic::functions::erf::*;
pub use super::logic::functions::gamma::*;
pub use super::logic::functions::lambert_w::*;
pub use super::logic::functions::polar::*;
pub use super::logic::functions::polygamma::*;
pub use super::logic::functions::polynomials::*;
pub use super::logic::functions::zeta::*;
