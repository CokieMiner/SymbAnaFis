//! Crate-internal API for the `core` module.
//!
//! Exposes foundational logic modules as submodules to preserve
//! internal module paths (`crate::core::expr`, `crate::core::error`, etc).

use super::helpers;

pub use super::helpers::known_symbols;
pub use super::helpers::traits;

// Re-export shared internal symbol types at the core level
pub use super::symbol::{InternedSymbol, lookup_by_id, symb_interned, symb_new_isolated};

pub use super::expr::{CustomEvalMap, arc_number};

pub mod error {
    pub use super::helpers::DiffError;
}
