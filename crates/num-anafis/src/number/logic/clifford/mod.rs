//! Dense multivectors for Clifford (geometric) algebras with labeled generators.
//!
//! This module implements **Conformal Geometric Algebra** CGA Cl(4,1) with 5
//! base generators: `e1, e2, e3` (Euclidean 3-space), `e+` (extra positive),
//! and `e−` (extra negative).  All derived objects (quaternions, complex unit,
//! conformal origin/infinity) are constructed as products of these generators.

#![allow(
    clippy::indexing_slicing,
    clippy::module_name_repetitions,
    reason = "Indexing is bounds-checked by construction; type name repeats module name for discoverability."
)]

/// Stable IDs for the CGA Cl(4,1) base generators and user-defined extensions.
///
/// The five fundamental generators are ordered by ID so that a
/// [`GeneratorSet`] built from all five is already sorted.
///
/// IDs 0–255 are reserved by the library.
/// User-defined generators should start at 256.
pub mod gen_id {
    /// Euclidean basis vector `e1` — `e1² = +1`.
    pub const E1: u32 = 0;
    /// Euclidean basis vector `e2` — `e2² = +1`.
    pub const E2: u32 = 1;
    /// Euclidean basis vector `e3` — `e3² = +1`.
    pub const E3: u32 = 2;
    /// Extra positive dimension `e+` — `e+² = +1`.
    pub const E_PLUS: u32 = 3;
    /// Extra negative dimension `e−` — `e−² = −1`.
    pub const E_MINUS: u32 = 4;
}

mod arithmetic;
mod constructors;
mod core;
mod display;
pub mod fast;
mod large_mat;
mod matrix;
mod spectral;
mod types;

pub use constructors::{
    cga_gens, ci, e_minus, e_plus, e1, e2, e3, eps, inf, orig, pseudo3d, pseudo5d, qi, qj, qk, sj,
};
pub use types::CliffordNumber;
pub use types::GeneratorSet;
