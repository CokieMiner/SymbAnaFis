//! Math function methods generated via macros.
//!
//! Contains macro definitions and implementations for math functions on Symbol and Expr.
//! Uses pre-interned symbol IDs from `known_symbols` for O(1) function construction.

use crate::core::Expr;
use crate::core::Symbol;
use crate::core::known_symbols::{KS, get_interned, get_symbol};

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
                    Expr::func_symbol(get_interned($symbol_id), self.to_expr())
                }
            )*
        }
    };
}

// Apply to Symbol (takes &self, uses pre-interned IDs)
impl_math_functions_symbol! {
    // Trigonometric functions
    sin => KS.sin, cos => KS.cos, tan => KS.tan,
    cot => KS.cot, sec => KS.sec, csc => KS.csc,
    // Inverse trigonometric functions
    asin => KS.asin, acos => KS.acos, atan => KS.atan,
    acot => KS.acot, asec => KS.asec, acsc => KS.acsc,
    // Hyperbolic functions
    sinh => KS.sinh, cosh => KS.cosh, tanh => KS.tanh,
    coth => KS.coth, sech => KS.sech, csch => KS.csch,
    // Inverse hyperbolic functions
    asinh => KS.asinh, acosh => KS.acosh, atanh => KS.atanh,
    acoth => KS.acoth, asech => KS.asech, acsch => KS.acsch,
    // Exponential and logarithmic functions
    exp => KS.exp, ln => KS.ln,
    log10 => KS.log10, log2 => KS.log2,
    // Root functions
    sqrt => KS.sqrt, cbrt => KS.cbrt,
    // Rounding functions
    floor => KS.floor, ceil => KS.ceil, round => KS.round,
    // Special functions (single-argument only)
    abs => KS.abs, signum => KS.signum, sinc => KS.sinc,
    erf => KS.erf, erfc => KS.erfc, gamma => KS.gamma, lgamma => KS.lgamma,
    digamma => KS.digamma, trigamma => KS.trigamma, tetragamma => KS.tetragamma,
    zeta => KS.zeta, lambertw => KS.lambertw,
    elliptic_k => KS.elliptic_k, elliptic_e => KS.elliptic_e,
    exp_polar => KS.exp_polar,
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
        Expr::func_multi_symbol(get_symbol(KS.polygamma), vec![n.into(), self.to_expr()])
    }

    /// Beta function with this symbol: B(this, other)
    pub fn beta(&self, other: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(get_symbol(KS.beta), vec![self.to_expr(), other.into()])
    }

    /// Bessel function of the first kind on this symbol: `J_n(x)`
    pub fn besselj(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(get_symbol(KS.besselj), vec![n.into(), self.to_expr()])
    }

    /// Bessel function of the second kind on this symbol: `Y_n(x)`
    pub fn bessely(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(get_symbol(KS.bessely), vec![n.into(), self.to_expr()])
    }

    /// Modified Bessel function of the first kind on this symbol: `I_n(x)`
    pub fn besseli(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(get_symbol(KS.besseli), vec![n.into(), self.to_expr()])
    }

    /// Modified Bessel function of the second kind on this symbol: `K_n(x)`
    pub fn besselk(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(get_symbol(KS.besselk), vec![n.into(), self.to_expr()])
    }

    /// Logarithm with arbitrary base on this symbol
    pub fn log(&self, base: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(get_symbol(KS.log), vec![base.into(), self.to_expr()])
    }

    /// Two-argument arctangent on this symbol: atan2(self, x)
    pub fn atan2(&self, x: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(get_symbol(KS.atan2), vec![self.to_expr(), x.into()])
    }

    /// Hermite polynomial on this symbol: `H_n(x)`
    pub fn hermite(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(get_symbol(KS.hermite), vec![n.into(), self.to_expr()])
    }

    /// Associated Legendre polynomial on this symbol
    pub fn assoc_legendre(&self, l: impl Into<Expr>, m: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            get_symbol(KS.assoc_legendre),
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
            get_symbol(KS.spherical_harmonic),
            vec![l.into(), m.into(), self.to_expr(), phi.into()],
        )
    }

    /// Alternative spherical harmonic notation on this symbol
    pub fn ynm(&self, l: impl Into<Expr>, m: impl Into<Expr>, phi: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            get_symbol(KS.ynm),
            vec![l.into(), m.into(), self.to_expr(), phi.into()],
        )
    }

    /// Derivative of Riemann zeta function on this symbol
    pub fn zeta_deriv(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(get_symbol(KS.zeta_deriv), vec![n.into(), self.to_expr()])
    }
}
