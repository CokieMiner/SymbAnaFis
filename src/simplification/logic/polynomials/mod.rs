//! Structural polynomial transformations: bounded expansion and complete factoring.

mod grouping;
mod transforms;

pub use grouping::{Factoring, Level, Start, Term};
pub use transforms::Polynomials;
