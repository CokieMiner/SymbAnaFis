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

/// Get the `InternedSymbol` for a known ID.
/// Panics if the ID is not found (should never happen for known symbols).
pub fn get_symbol(id: &LazyLock<u64>) -> InternedSymbol {
    lookup_by_id(**id).expect("Known symbol ID not found in registry")
}

macro_rules! define_symbol_ids {
    ($($name:ident => $str:expr),* $(,)?) => {
        $(
            #[allow(dead_code)]
            pub static $name: LazyLock<u64> = LazyLock::new(|| intern_id($str));
        )*
    };
}

define_symbol_ids! {
    // Roots
    SQRT => "sqrt",
    CBRT => "cbrt",

    // Exponential / Log
    EXP => "exp",
    LN => "ln",
    LOG => "log",
    LOG10 => "log10",
    LOG2 => "log2",

    // Trigonometric
    SIN => "sin",
    COS => "cos",
    TAN => "tan",
    COT => "cot",
    SEC => "sec",
    CSC => "csc",

    // Inverse Trigonometric
    ASIN => "asin",
    ACOS => "acos",
    ATAN => "atan",
    ATAN2 => "atan2",
    ACOT => "acot",
    ASEC => "asec",
    ACSC => "acsc",

    // Hyperbolic
    SINH => "sinh",
    COSH => "cosh",
    TANH => "tanh",
    COTH => "coth",
    SECH => "sech",
    CSCH => "csch",

    // Inverse Hyperbolic
    ASINH => "asinh",
    ACOSH => "acosh",
    ATANH => "atanh",
    ACOTH => "acoth",
    ASECH => "asech",
    ACSCH => "acsch",

    // Special
    ABS => "abs",
    SIGNUM => "signum",

    // Aliases found in codebase (for compatibility)
    SIGN => "sign",
    SGN => "sgn",

    // Other special functions
    ERF => "erf",
    ERFC => "erfc",
    GAMMA => "gamma",
    DIGAMMA => "digamma",
    TRIGAMMA => "trigamma",
    BETA => "beta",
    BESSELJ => "besselj",
    BESSELY => "bessely",
    BESSELI => "besseli",
    BESSELK => "besselk",
    POLYGAMMA => "polygamma",
    TETRAGAMMA => "tetragamma",
    SINC => "sinc",
    LAMBERTW => "lambertw",
    ELLIPTIC_K => "elliptic_k",
    ELLIPTIC_E => "elliptic_e",
    ZETA => "zeta",
    ZETA_DERIV => "zeta_deriv",
    HERMITE => "hermite",
    ASSOC_LEGENDRE => "assoc_legendre",
    SPHERICAL_HARMONIC => "spherical_harmonic",
    YNM => "ynm",
    EXP_POLAR => "exp_polar",

    // Constants sometimes used as symbols
    PI => "pi",
    E => "e",
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
