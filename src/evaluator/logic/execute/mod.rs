//! Bytecode execution (Interpreters)

pub(super) mod drivers;
pub(super) mod engine;
pub(super) mod tree;

// Re-export modules to preserve flat internal import paths
#[cfg(feature = "parallel")]
pub(super) use drivers::batch;

#[cfg(feature = "parallel")]
pub(super) use drivers::parallel;

#[cfg(test)]
mod tests;
