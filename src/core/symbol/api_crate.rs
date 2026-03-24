//! Crate-internal API for the `symbol` module.
//!
//! These items are used by other modules within this crate (e.g., `expr`,
//! `context`, `diff`) but are **not** part of the public library surface.

pub use super::logic::interned::InternedSymbol;
pub use super::logic::registry::{lookup_by_id, symb_interned, symb_new_isolated};
// key_from_id is only used inside api_user.rs (Symbol::from_id), not re-exported
