//! Implementation details for the `symbol` module.
//!
//! Contains symbol interning, registry, operator overloads, and math methods.
//! All items here are internal to the symbol subsystem.

pub(super) mod conversions;
pub(super) mod interned;
pub(super) mod math_methods;
pub(super) mod operators;
pub(super) mod registry;

// Staircase re-exports — one hop up to api.rs
pub use registry::{
    clear_symbols, remove_symbol, symb, symb_anon, symb_get, symb_new, symbol_count, symbol_exists,
    symbol_names,
};

pub use interned::InternedSymbol;
pub use registry::{key_from_id, lookup_by_id, symb_interned, symb_new_isolated};
