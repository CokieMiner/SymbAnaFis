//! Math function methods generated via macros.
//!
//! Contains macro definitions and implementations for math functions on Symbol and Expr.
//! Uses pre-interned symbol IDs from `known_symbols` for O(1) function construction.

use super::super::Symbol;
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
    erf => ks::KS.erf, erfc => ks::KS.erfc, gamma => ks::KS.gamma, lgamma => ks::KS.lgamma,
    digamma => ks::KS.digamma, trigamma => ks::KS.trigamma, tetragamma => ks::KS.tetragamma,
    zeta => ks::KS.zeta, lambertw => ks::KS.lambertw,
    elliptic_k => ks::KS.elliptic_k, elliptic_e => ks::KS.elliptic_e,
    exp_polar => ks::KS.exp_polar,
}

// =============================================================================
// Symbol Methods (parametric)
// =============================================================================

impl Symbol {
    /// Raise this symbol to a power
    pub fn pow(&self, exp: impl Into<Expr>) -> Expr {
        Expr::pow_static(self.to_expr(), exp.into())
    }

    /// Polygamma function on this symbol: ψ^(n)(x)
    pub fn polygamma(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.polygamma),
            vec![n.into(), self.to_expr()],
        )
    }

    /// Beta function with this symbol: B(this, other)
    pub fn beta(&self, other: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.beta),
            vec![self.to_expr(), other.into()],
        )
    }

    /// Bessel function of the first kind on this symbol: `J_n(x)`
    pub fn besselj(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.besselj),
            vec![n.into(), self.to_expr()],
        )
    }

    /// Bessel function of the second kind on this symbol: `Y_n(x)`
    pub fn bessely(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.bessely),
            vec![n.into(), self.to_expr()],
        )
    }

    /// Modified Bessel function of the first kind on this symbol: `I_n(x)`
    pub fn besseli(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.besseli),
            vec![n.into(), self.to_expr()],
        )
    }

    /// Modified Bessel function of the second kind on this symbol: `K_n(x)`
    pub fn besselk(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.besselk),
            vec![n.into(), self.to_expr()],
        )
    }

    /// Logarithm with arbitrary base on this symbol
    pub fn log(&self, base: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.log),
            vec![base.into(), self.to_expr()],
        )
    }

    /// Two-argument arctangent on this symbol: atan2(self, x)
    pub fn atan2(&self, x: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(ks::get_symbol(ks::KS.atan2), vec![self.to_expr(), x.into()])
    }

    /// Hermite polynomial on this symbol: `H_n(x)`
    pub fn hermite(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.hermite),
            vec![n.into(), self.to_expr()],
        )
    }

    /// Associated Legendre polynomial on this symbol
    pub fn assoc_legendre(&self, l: impl Into<Expr>, m: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.assoc_legendre),
            vec![l.into(), m.into(), self.to_expr()],
        )
    }

    /// Spherical harmonic on this symbol
    pub fn spherical_harmonic(
        &self,
        l: impl Into<Expr>,
        m: impl Into<Expr>,
        phi: impl Into<Expr>,
    ) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.spherical_harmonic),
            vec![l.into(), m.into(), self.to_expr(), phi.into()],
        )
    }

    /// Alternative spherical harmonic notation on this symbol
    pub fn ynm(&self, l: impl Into<Expr>, m: impl Into<Expr>, phi: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.ynm),
            vec![l.into(), m.into(), self.to_expr(), phi.into()],
        )
    }

    /// Derivative of Riemann zeta function on this symbol
    pub fn zeta_deriv(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.zeta_deriv),
            vec![n.into(), self.to_expr()],
        )
    }
}
