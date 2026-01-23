//! Math function methods generated via macros.
//!
//! Contains macro definitions and implementations for math functions on Symbol and Expr.
//! Uses pre-interned symbol IDs from `known_symbols` for O(1) function construction.

use super::Symbol;
use crate::Expr;
use crate::core::known_symbols as ks;

// ============================================================================
// Macro for generating math function methods using pre-interned IDs
// ============================================================================

/// Generate math function methods for Symbol (taking &self)
/// Uses pre-interned symbol IDs to avoid `HashMap` lookup at construction time
macro_rules! impl_math_functions_symbol {
    ($($fn_name:ident => $symbol_id:expr),* $(,)?) => {
        impl Symbol {
            $(
                #[doc = concat!("Apply the `", stringify!($fn_name), "` function to this expression.")]
                pub fn $fn_name(&self) -> Expr {
                    Expr::func_symbol(ks::get_interned($symbol_id), self.to_expr())
                }
            )*
        }
    };
}

/// Generate math function methods for Expr (consumes self)
/// Uses pre-interned symbol IDs to avoid `HashMap` lookup at construction time
macro_rules! impl_math_functions_expr {
    ($($fn_name:ident => $symbol_id:expr),* $(,)?) => {
        impl Expr {
            $(
                #[doc = concat!("Apply the `", stringify!($fn_name), "` function to this expression.")]
                #[must_use]
                pub fn $fn_name(self) -> Expr {
                    Expr::func_symbol(ks::get_interned($symbol_id), self)
                }
            )*
        }
    };
}

// Apply to Symbol (takes &self, uses pre-interned IDs)
impl_math_functions_symbol! {
    // Trigonometric functions
    sin => ks::KS.sin, cos => ks::KS.cos, tan => ks::KS.tan,
    cot => ks::KS.cot, sec => ks::KS.sec, csc => ks::KS.csc,
    // Inverse trigonometric functions
    asin => ks::KS.asin, acos => ks::KS.acos, atan => ks::KS.atan,
    acot => ks::KS.acot, asec => ks::KS.asec, acsc => ks::KS.acsc,
    // Hyperbolic functions
    sinh => ks::KS.sinh, cosh => ks::KS.cosh, tanh => ks::KS.tanh,
    coth => ks::KS.coth, sech => ks::KS.sech, csch => ks::KS.csch,
    // Inverse hyperbolic functions
    asinh => ks::KS.asinh, acosh => ks::KS.acosh, atanh => ks::KS.atanh,
    acoth => ks::KS.acoth, asech => ks::KS.asech, acsch => ks::KS.acsch,
    // Exponential and logarithmic functions
    exp => ks::KS.exp, ln => ks::KS.ln,
    log10 => ks::KS.log10, log2 => ks::KS.log2,
    // Root functions
    sqrt => ks::KS.sqrt, cbrt => ks::KS.cbrt,
    // Rounding functions
    floor => ks::KS.floor, ceil => ks::KS.ceil, round => ks::KS.round,
    // Special functions (single-argument only)
    abs => ks::KS.abs, signum => ks::KS.signum, sinc => ks::KS.sinc,
    erf => ks::KS.erf, erfc => ks::KS.erfc, gamma => ks::KS.gamma,
    digamma => ks::KS.digamma, trigamma => ks::KS.trigamma, tetragamma => ks::KS.tetragamma,
    zeta => ks::KS.zeta, lambertw => ks::KS.lambertw,
    elliptic_k => ks::KS.elliptic_k, elliptic_e => ks::KS.elliptic_e,
    exp_polar => ks::KS.exp_polar,
}

// Apply to Expr (consumes self, uses pre-interned IDs)
impl_math_functions_expr! {
    // Trigonometric functions
    sin => ks::KS.sin, cos => ks::KS.cos, tan => ks::KS.tan,
    cot => ks::KS.cot, sec => ks::KS.sec, csc => ks::KS.csc,
    // Inverse trigonometric functions
    asin => ks::KS.asin, acos => ks::KS.acos, atan => ks::KS.atan,
    acot => ks::KS.acot, asec => ks::KS.asec, acsc => ks::KS.acsc,
    // Hyperbolic functions
    sinh => ks::KS.sinh, cosh => ks::KS.cosh, tanh => ks::KS.tanh,
    coth => ks::KS.coth, sech => ks::KS.sech, csch => ks::KS.csch,
    // Inverse hyperbolic functions
    asinh => ks::KS.asinh, acosh => ks::KS.acosh, atanh => ks::KS.atanh,
    acoth => ks::KS.acoth, asech => ks::KS.asech, acsch => ks::KS.acsch,
    // Exponential and logarithmic functions
    exp => ks::KS.exp, ln => ks::KS.ln,
    log10 => ks::KS.log10, log2 => ks::KS.log2,
    // Root functions
    sqrt => ks::KS.sqrt, cbrt => ks::KS.cbrt,
    // Rounding functions
    floor => ks::KS.floor, ceil => ks::KS.ceil, round => ks::KS.round,
    // Special functions (single-argument only)
    abs => ks::KS.abs, signum => ks::KS.signum, sinc => ks::KS.sinc,
    erf => ks::KS.erf, erfc => ks::KS.erfc, gamma => ks::KS.gamma,
    digamma => ks::KS.digamma, trigamma => ks::KS.trigamma, tetragamma => ks::KS.tetragamma,
    zeta => ks::KS.zeta, lambertw => ks::KS.lambertw,
    elliptic_k => ks::KS.elliptic_k, elliptic_e => ks::KS.elliptic_e,
    exp_polar => ks::KS.exp_polar,
}
