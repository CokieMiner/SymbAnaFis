//! Logic layer for the `core` root module.

pub(super) mod error;
pub(super) mod known_symbols;
pub(super) mod poly;
pub(super) mod traits;
pub(super) mod visitor;

#[cfg(test)]
mod tests;
