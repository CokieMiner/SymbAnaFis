//! Crate-internal API for the `core` module.
//!
//! Exposes foundational logic modules as submodules to preserve
//! internal module paths (`crate::core::poly`, `crate::core::error`, etc).

pub mod error {
    pub use super::super::logic::error::*;
}
pub mod known_symbols {
    pub use super::super::logic::known_symbols::*;
}
pub mod poly {
    pub use super::super::logic::poly::*;
}
pub mod traits {
    pub use super::super::logic::traits::*;
}
