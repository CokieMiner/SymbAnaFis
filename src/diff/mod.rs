//! Memoized symbolic differentiation on regular principal-branch domains.

mod engine;

pub use engine::Differentiation;

#[cfg(test)]
mod tests;
