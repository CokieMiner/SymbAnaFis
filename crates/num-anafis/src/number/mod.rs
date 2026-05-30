mod api;
mod constructors;
mod logic;

#[cfg(feature = "serde")]
mod serde_impl;

pub use api::*;
pub use constructors::{IntoScalar, r, s};
#[cfg(feature = "clifford")]
pub use logic::clifford::CliffordNumber;
#[cfg(feature = "clifford")]
pub use logic::clifford::GeneratorSet;
#[cfg(feature = "clifford")]
pub use logic::clifford::fast::FastClifford;
#[cfg(feature = "clifford")]
pub use logic::clifford::{
    cga_gens, ci, e_minus, e_plus, e1, e2, e3, eps, inf, orig, pseudo3d, pseudo5d, qi, qj, qk, sj,
};
pub use logic::float_ops::FloatType;
pub use logic::int_math::IntType;
pub use logic::math_ext::AnafisMathExt;
pub use logic::rational_math::RationalType;
