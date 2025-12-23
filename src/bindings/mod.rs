//! External bindings (Python, parallel)

#[cfg(feature = "python")]
pub mod python;

#[cfg(feature = "parallel")]
pub mod parallel;

#[cfg(feature = "parallel")]
pub mod eval_f64;
