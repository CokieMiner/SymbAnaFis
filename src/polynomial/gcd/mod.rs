//! Certified GCDs of sparse rational polynomials over $\mathbb{Z}$ via
//! modular images.
//!
//! A pair is reduced over $\mathbb{Z}$ before any modular work: monomial
//! contents, rational contents (Gauss's lemma), equal and constant inputs, and
//! exponent strides (deflation) are removed. Pure univariate pairs lift
//! directly from dense modular GCDs; multivariate pairs bound each variable's
//! GCD degree from one random image per variable modulo a prime. All-zero
//! bounds prove a unit GCD, and bounds equal to the degrees of one input reduce
//! the GCD to one trial division. Otherwise the variables with positive bounds
//! are ordered with the first one preferring a monomial leading coefficient and
//! minimizing its bound plus leading-coefficient term count, and the rest by
//! descending bound. Variables of bound zero are specialized at random points,
//! and [`Zippel`] computes the GCD modulo catalog primes. With
//! $\gamma = \gcd(\operatorname{lc} A, \operatorname{lc} B)$ over $\mathbb{Z}$,
//! each image is scaled to leading coefficient $\gamma$, an image of
//! $(\gamma / \operatorname{lc} G)\,G$ for every prime. One image lifts by
//! symmetric residues, two by Chinese remaindering. A lifted candidate is
//! accepted only when it divides both inputs exactly, so every returned factor
//! is a proved common divisor.

use super::{Monomial, PolynomialLimitExceeded, PolynomialWork, SparsePolynomial};

mod cancel;
mod modular;
mod shape;
mod univariate;
mod zippel;

pub use cancel::PolynomialGcd;
pub use modular::{Divisor, Modular};
pub use shape::Shape;
pub use univariate::{Dense, ModularTerm, Sampler};
pub use zippel::Zippel;

#[cfg(test)]
mod tests;
