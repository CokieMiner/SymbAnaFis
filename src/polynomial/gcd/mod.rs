//! Certified GCDs of sparse rational polynomials over $\mathbb{Z}$ via
//! modular images.
//!
//! A pair is reduced over $\mathbb{Z}$ before any modular work: monomial
//! contents, rational contents (Gauss's lemma), equal and constant inputs, and
//! exponent strides (deflation) are removed. One random image per variable
//! modulo a prime bounds the degree of the GCD in that variable; all-zero
//! bounds prove a unit GCD, and bounds equal to the degrees of one input reduce
//! the GCD to one trial division. Otherwise the variables with positive bounds
//! are ordered with the first one minimizing its bound plus the term count of
//! its leading coefficient, and the rest by descending bound. Variables of
//! bound zero are specialized at random points, and [`Zippel`] computes the
//! GCD modulo catalog primes. With $\gamma = \gcd(\operatorname{lc} A,
//! \operatorname{lc} B)$ over $\mathbb{Z}$, each image is scaled to leading
//! coefficient $\gamma$, an image of $(\gamma / \operatorname{lc} G)\,G$ for
//! every prime. One image lifts by symmetric residues, two by Chinese
//! remaindering. A lifted candidate is accepted only when it divides both
//! inputs exactly, so every returned factor is a proved common divisor.

use super::{Monomial, PolynomialWork, SparsePolynomial};

mod cancel;
mod modular;
mod shape;
mod zippel;

pub use cancel::{Divisor, PolynomialGcd};
pub use modular::{Dense, Sampler};
pub use shape::Shape;
pub use zippel::{ModularTerm, Zippel};
