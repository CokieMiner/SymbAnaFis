//! Internal simplification implementation details.

pub(super) mod engine;
pub(super) mod helpers;
pub(super) mod rules;

pub(super) use engine::Simplifier;
pub(super) use helpers::prettify_roots;

#[cfg(test)]
mod tests;
