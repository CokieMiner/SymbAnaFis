//! Bytecode execution (Interpreters)

#[cfg(feature = "parallel")]
pub mod batch;
pub mod math;
#[cfg(feature = "parallel")]
pub mod parallel;
pub mod scalar;
#[cfg(feature = "parallel")]
pub mod simd;
pub mod tree;

#[cfg(test)]
mod tests;
