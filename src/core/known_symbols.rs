//! Pre-interned symbol IDs for O(1) comparison
//!
//! This module provides lazily-initialized symbol IDs for common function names.
//! Comparison is O(1) - just a u64 integer comparison.

use crate::core::symbol::{InternedSymbol, lookup_by_id, symb_interned};
use std::sync::LazyLock;

/// Get the ID for an interned symbol (helper for the macro)
fn intern_id(name: &str) -> u64 {
    symb_interned(name).id()
}

/// Collection of pre-interned symbol IDs for all built-in mathematical functions.
pub struct KnownSymbols {
    // Roots
    /// Square root function
    pub sqrt: u64,
    /// Cube root function
    pub cbrt: u64,

    // Exponential / Log
    /// Exponential function
    pub exp: u64,
    /// Natural logarithm
    pub ln: u64,
    /// General logarithm
    pub log: u64,
    /// Base-10 logarithm
    pub log10: u64,
    /// Base-2 logarithm
    pub log2: u64,

    // Trigonometric
    /// Sine function
    pub sin: u64,
    /// Cosine function
    pub cos: u64,
    /// Tangent function
    pub tan: u64,
    /// Cotangent function
    pub cot: u64,
    /// Secant function
    pub sec: u64,
    /// Cosecant function
    pub csc: u64,

    // Inverse Trigonometric
    /// Inverse sine
    pub asin: u64,
    /// Inverse cosine
    pub acos: u64,
    /// Inverse tangent
    pub atan: u64,
    /// Two-argument inverse tangent
    pub atan2: u64,
    /// Inverse cotangent
    pub acot: u64,
    /// Inverse secant
    pub asec: u64,
    /// Inverse cosecant
    pub acsc: u64,

    // Hyperbolic
    /// Hyperbolic sine
    pub sinh: u64,
    /// Hyperbolic cosine
    pub cosh: u64,
    /// Hyperbolic tangent
    pub tanh: u64,
    /// Hyperbolic cotangent
    pub coth: u64,
    /// Hyperbolic secant
    pub sech: u64,
    /// Hyperbolic cosecant
    pub csch: u64,

    // Inverse Hyperbolic
    /// Inverse hyperbolic sine
    pub asinh: u64,
    /// Inverse hyperbolic cosine
    pub acosh: u64,
    /// Inverse hyperbolic tangent
    pub atanh: u64,
    /// Inverse hyperbolic cotangent
    pub acoth: u64,
    /// Inverse hyperbolic secant
    pub asech: u64,
    /// Inverse hyperbolic cosecant
    pub acsch: u64,

    // Special
    /// Absolute value
    pub abs: u64,
    /// Sign function
    pub signum: u64,

    // Rounding functions
    /// Floor function
    pub floor: u64,
    /// Ceiling function
    pub ceil: u64,
    /// Round function
    pub round: u64,

    // Aliases found in codebase (for compatibility)
    /// Sign function alias
    pub sign: u64,
    /// Sign function alias
    pub sgn: u64,

    // Other special functions
    /// Error function
    pub erf: u64,
    /// Complementary error function
    pub erfc: u64,
    /// Gamma function
    pub gamma: u64,
    /// Digamma function
    pub digamma: u64,
    /// Trigamma function
    pub trigamma: u64,
    /// Beta function
    pub beta: u64,
    /// Bessel function of the first kind
    pub besselj: u64,
    /// Bessel function of the second kind
    pub bessely: u64,
    /// Modified Bessel function of the first kind
    pub besseli: u64,
    /// Modified Bessel function of the second kind
    pub besselk: u64,
    /// Polygamma function
    pub polygamma: u64,
    /// Tetragamma function
    pub tetragamma: u64,
    /// Sinc function
    pub sinc: u64,
    /// Lambert W function
    pub lambertw: u64,
    /// Complete elliptic integral of the first kind
    pub elliptic_k: u64,
    /// Complete elliptic integral of the second kind
    pub elliptic_e: u64,
    /// Riemann zeta function
    pub zeta: u64,
    /// Derivative of Riemann zeta function
    pub zeta_deriv: u64,
    /// Hermite polynomial
    pub hermite: u64,
    /// Associated Legendre polynomial
    pub assoc_legendre: u64,
    /// Spherical harmonic
    pub spherical_harmonic: u64,
    /// Spherical harmonic Ynm
    pub ynm: u64,
    /// Exponential in polar form
    pub exp_polar: u64,

    // Constants sometimes used as symbols
    /// Pi constant
    pub pi: u64,
    /// Euler's number
    pub e: u64,
}

impl KnownSymbols {
    /// Create a new instance of `KnownSymbols`
    fn new() -> Self {
        Self {
            sqrt: intern_id("sqrt"),
            cbrt: intern_id("cbrt"),
            exp: intern_id("exp"),
            ln: intern_id("ln"),
            log: intern_id("log"),
            log10: intern_id("log10"),
            log2: intern_id("log2"),
            sin: intern_id("sin"),
            cos: intern_id("cos"),
            tan: intern_id("tan"),
            cot: intern_id("cot"),
            sec: intern_id("sec"),
            csc: intern_id("csc"),
            asin: intern_id("asin"),
            acos: intern_id("acos"),
            atan: intern_id("atan"),
            atan2: intern_id("atan2"),
            acot: intern_id("acot"),
            asec: intern_id("asec"),
            acsc: intern_id("acsc"),
            sinh: intern_id("sinh"),
            cosh: intern_id("cosh"),
            tanh: intern_id("tanh"),
            coth: intern_id("coth"),
            sech: intern_id("sech"),
            csch: intern_id("csch"),
            asinh: intern_id("asinh"),
            acosh: intern_id("acosh"),
            atanh: intern_id("atanh"),
            acoth: intern_id("acoth"),
            asech: intern_id("asech"),
            acsch: intern_id("acsch"),
            abs: intern_id("abs"),
            signum: intern_id("signum"),
            floor: intern_id("floor"),
            ceil: intern_id("ceil"),
            round: intern_id("round"),
            sign: intern_id("sign"),
            sgn: intern_id("sgn"),
            erf: intern_id("erf"),
            erfc: intern_id("erfc"),
            gamma: intern_id("gamma"),
            digamma: intern_id("digamma"),
            trigamma: intern_id("trigamma"),
            beta: intern_id("beta"),
            besselj: intern_id("besselj"),
            bessely: intern_id("bessely"),
            besseli: intern_id("besseli"),
            besselk: intern_id("besselk"),
            polygamma: intern_id("polygamma"),
            tetragamma: intern_id("tetragamma"),
            sinc: intern_id("sinc"),
            lambertw: intern_id("lambertw"),
            elliptic_k: intern_id("elliptic_k"),
            elliptic_e: intern_id("elliptic_e"),
            zeta: intern_id("zeta"),
            zeta_deriv: intern_id("zeta_deriv"),
            hermite: intern_id("hermite"),
            assoc_legendre: intern_id("assoc_legendre"),
            spherical_harmonic: intern_id("spherical_harmonic"),
            ynm: intern_id("ynm"),
            exp_polar: intern_id("exp_polar"),
            pi: intern_id("pi"),
            e: intern_id("e"),
        }
    }
}

/// Global instance of pre-interned symbol IDs.
pub static KS: LazyLock<KnownSymbols> = LazyLock::new(KnownSymbols::new);

/// Get the `InternedSymbol` for a known function by its ID.
///
/// This is an internal function for the function registry system.
/// External users should use the symbol management functions in the main API.
#[inline]
pub fn get_interned(id: u64) -> InternedSymbol {
    lookup_by_id(id).expect("Known symbol ID not found in registry")
}

/// Compatibility wrapper for `get_interned`.
///
/// Internal function for backward compatibility.
#[inline]
pub fn get_symbol(id: u64) -> InternedSymbol {
    get_interned(id)
}

/// Check if a name is a known mathematical constant (pi, e, etc.)
/// Returns true for any case variation: "pi", "PI", "Pi", "e", "E"
#[inline]
pub fn is_known_constant(name: &str) -> bool {
    matches!(name, "pi" | "PI" | "Pi" | "e" | "E")
}

/// Get the numeric value of a known constant, if it matches.
/// Returns `Some(value)` for known constants, `None` otherwise.
#[inline]
pub fn get_constant_value(name: &str) -> Option<f64> {
    match name {
        "pi" | "PI" | "Pi" => Some(std::f64::consts::PI),
        "e" | "E" => Some(std::f64::consts::E),
        _ => None,
    }
}
