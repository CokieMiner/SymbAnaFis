//! Exact sparse multivariate polynomials over shared expression indeterminates.
//!
//! [`PolynomialInput`] lowers expressions to rational [`SparsePolynomial`]s,
//! each a rational content times a primitive integer polynomial, and
//! [`SparsePolynomial::to_expr`] returns one as a canonical sum. Products use
//! dense accumulation, a convolution of the Kronecker line, or a heap merge,
//! whichever the shape makes cheapest; division is exact. [`PolynomialGcd`]
//! removes the GCD of a pair over `Z` by modular images in the 50-bit prime
//! fields of `ntt_anafis`: Brown's dense interpolation of the outer variables
//! with Zippel's sparse interpolation of the inner ones, Chinese remaindering
//! over at most two primes, and exact division as the certificate.
//! [`PolynomialWork`] bounds the work of conversion and arithmetic.
//!
//! When and under which domain contract these operations apply is decided by
//! their callers.

mod gcd;
mod input;
mod monomial;
mod sparse;

pub use gcd::{ModularTerm, PolynomialGcd};
pub use input::PolynomialInput;
pub use monomial::{Monomial, PolynomialWork};
pub use sparse::SparsePolynomial;

#[cfg(test)]
mod tests;
