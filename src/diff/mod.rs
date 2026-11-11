//! Memoized symbolic differentiation on regular principal-branch domains.

mod calculus;
mod differentiation;
mod functions;

pub use calculus::Jacobian;
pub use differentiation::Differentiation;
pub use functions::Functions;

#[cfg(test)]
mod tests;
