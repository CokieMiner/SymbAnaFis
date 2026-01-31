//! Pre-interned symbol IDs for O(1) comparison
//!
//! This module provides lazily-initialized symbol IDs for common function names.
//! Comparison is O(1) - just a u64 integer comparison.

use crate::core::symbol::{lookup_by_id, symb_interned, InternedSymbol};
use std::sync::LazyLock;

/// Get the ID for an interned symbol (helper for the macro)
fn intern_id(name: &str) -> u64 {
    symb_interned(name).id()
}

/// Collection of pre-interned symbol IDs for all built-in mathematical functions.
pub struct KnownSymbols {
    // Roots
    pub sqrt: u64,
    pub cbrt: u64,

    // Exponential / Log
    pub exp: u64,
    pub ln: u64,
    pub log: u64,
    pub log10: u64,
    pub log2: u64,

    // Trigonometric
    pub sin: u64,
    pub cos: u64,
    pub tan: u64,
    pub cot: u64,
    pub sec: u64,
    pub csc: u64,

    // Inverse Trigonometric
    pub asin: u64,
    pub acos: u64,
    pub atan: u64,
    pub atan2: u64,
    pub acot: u64,
    pub asec: u64,
    pub acsc: u64,

    // Hyperbolic
    pub sinh: u64,
    pub cosh: u64,
    pub tanh: u64,
    pub coth: u64,
    pub sech: u64,
    pub csch: u64,

    // Inverse Hyperbolic
    pub asinh: u64,
    pub acosh: u64,
    pub atanh: u64,
    pub acoth: u64,
    pub asech: u64,
    pub acsch: u64,

    // Special
    pub abs: u64,
    pub signum: u64,

    // Rounding functions
    pub floor: u64,
    pub ceil: u64,
    pub round: u64,

    // Aliases found in codebase (for compatibility)
    pub sign: u64,
    pub sgn: u64,

    // Other special functions
    pub erf: u64,
    pub erfc: u64,
    pub gamma: u64,
    pub digamma: u64,
    pub trigamma: u64,
    pub beta: u64,
    pub besselj: u64,
    pub bessely: u64,
    pub besseli: u64,
    pub besselk: u64,
    pub polygamma: u64,
    pub tetragamma: u64,
    pub sinc: u64,
    pub lambertw: u64,
    pub elliptic_k: u64,
    pub elliptic_e: u64,
    pub zeta: u64,
    pub zeta_deriv: u64,
    pub hermite: u64,
    pub assoc_legendre: u64,
    pub spherical_harmonic: u64,
    pub ynm: u64,
    pub exp_polar: u64,

    // Constants sometimes used as symbols
    pub pi: u64,
    pub e: u64,
}

impl KnownSymbols {
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
#[inline]
pub fn get_interned(id: u64) -> InternedSymbol {
    lookup_by_id(id).expect("Known symbol ID not found in registry")
}

/// Compatibility wrapper for `get_interned`.
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
