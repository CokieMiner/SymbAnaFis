//! Numeric core for `SymbAnaFis`.

#![cfg_attr(not(feature = "python"), no_std)]
extern crate alloc;

// Backend features are mutually exclusive — enforce at compile time.
#[cfg(all(feature = "backend32", feature = "backendrug"))]
compile_error!("backend32 and backendrug are mutually exclusive");
#[cfg(all(feature = "backend64", feature = "backendrug"))]
compile_error!("backend64 and backendrug are mutually exclusive");

mod error;
mod number;

pub use error::NumAnafisError;
pub use number::FloatRepr;
pub use number::IntRepr;
pub use number::IntoScalar;
pub use number::Number;
pub use number::RationalRepr;
pub use number::Scalar;
pub use number::r;
pub use number::s;

#[cfg(feature = "python")]
#[allow(
    missing_docs,
    reason = "Python bindings are not the main focus of this crate, so we can afford to be a bit less strict here."
)]
pub mod python_binding;
