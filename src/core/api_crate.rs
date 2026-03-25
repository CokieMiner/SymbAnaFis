//! Crate-internal API for the `core` module.
//!
//! Exposes foundational logic modules as submodules to preserve
//! internal module paths (`crate::core::poly`, `crate::core::error`, etc).

pub use super::helpers::known_symbols;
pub use super::helpers::poly;
pub use super::helpers::traits;

pub mod error {
    pub use super::super::helpers::DiffError;
}
