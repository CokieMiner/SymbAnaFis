#![allow(
    clippy::doc_markdown,
    reason = "LaTeX math notation in $...$ / $$...$$ is not recognized by clippy"
)]
//! Special mathematical functions implemented for the `rug` backend.

mod bessel;
mod elliptic;
mod gamma;
mod lambertw;
mod orthogonal;
mod zeta;

pub use bessel::*;
pub use elliptic::*;
pub use gamma::*;
pub use lambertw::*;
pub use orthogonal::*;
pub use zeta::*;
