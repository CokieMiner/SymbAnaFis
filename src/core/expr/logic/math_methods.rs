//! Math function methods for `Expr`.
//!
//! Contains macro definitions and implementations for math functions on `Expr` and `Arc<Expr>`.
//! Uses pre-interned symbol IDs from `known_symbols` for O(1) function construction.

use std::sync::Arc;

use crate::Expr;
use crate::core::known_symbols as ks;

// ============================================================================
// Macro for generating math function methods using pre-interned IDs
// ============================================================================

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
    erf => ks::KS.erf, erfc => ks::KS.erfc, gamma => ks::KS.gamma, lgamma => ks::KS.lgamma,
    digamma => ks::KS.digamma, trigamma => ks::KS.trigamma, tetragamma => ks::KS.tetragamma,
    zeta => ks::KS.zeta, lambertw => ks::KS.lambertw,
    elliptic_k => ks::KS.elliptic_k, elliptic_e => ks::KS.elliptic_e,
    exp_polar => ks::KS.exp_polar,
}

// =============================================================================
// ArcExprExt trait for ergonomic Arc<Expr> operations
// =============================================================================

/// Extension trait for `Arc<Expr>` providing ergonomic math operations.
pub trait ArcExprExt {
    /// Raise to a power
    fn pow(&self, exp: impl Into<Expr>) -> Expr;
    /// Sine
    fn sin(&self) -> Expr;
    /// Cosine
    fn cos(&self) -> Expr;
    /// Tangent
    fn tan(&self) -> Expr;
    /// Cotangent
    fn cot(&self) -> Expr;
    /// Secant
    fn sec(&self) -> Expr;
    /// Cosecant
    fn csc(&self) -> Expr;
    /// Arcsine
    fn asin(&self) -> Expr;
    /// Arccosine
    fn acos(&self) -> Expr;
    /// Arctangent
    fn atan(&self) -> Expr;
    /// Arccotangent
    fn acot(&self) -> Expr;
    /// Arcsecant
    fn asec(&self) -> Expr;
    /// Arccosecant
    fn acsc(&self) -> Expr;
    /// Hyperbolic sine
    fn sinh(&self) -> Expr;
    /// Hyperbolic cosine
    fn cosh(&self) -> Expr;
    /// Hyperbolic tangent
    fn tanh(&self) -> Expr;
    /// Hyperbolic cotangent
    fn coth(&self) -> Expr;
    /// Hyperbolic secant
    fn sech(&self) -> Expr;
    /// Hyperbolic cosecant
    fn csch(&self) -> Expr;
    /// Inverse hyperbolic sine
    fn asinh(&self) -> Expr;
    /// Inverse hyperbolic cosine
    fn acosh(&self) -> Expr;
    /// Inverse hyperbolic tangent
    fn atanh(&self) -> Expr;
    /// Inverse hyperbolic cotangent
    fn acoth(&self) -> Expr;
    /// Inverse hyperbolic secant
    fn asech(&self) -> Expr;
    /// Inverse hyperbolic cosecant
    fn acsch(&self) -> Expr;
    /// Exponential
    fn exp(&self) -> Expr;
    /// Natural logarithm
    fn ln(&self) -> Expr;
    /// Logarithm with arbitrary base
    fn log(&self, base: impl Into<Expr>) -> Expr;
    /// Logarithm base 10
    fn log10(&self) -> Expr;
    /// Logarithm base 2
    fn log2(&self) -> Expr;
    /// Square root
    fn sqrt(&self) -> Expr;
    /// Cube root
    fn cbrt(&self) -> Expr;
    /// Floor rounding
    fn floor(&self) -> Expr;
    /// Ceil rounding
    fn ceil(&self) -> Expr;
    /// Nearest rounding
    fn round(&self) -> Expr;
    /// Absolute value
    fn abs(&self) -> Expr;
    /// Signum function
    fn signum(&self) -> Expr;
    /// Sinc function
    fn sinc(&self) -> Expr;
    /// Error function
    fn erf(&self) -> Expr;
    /// Complementary error function
    fn erfc(&self) -> Expr;
    /// Gamma function
    fn gamma(&self) -> Expr;
    /// Log-gamma function
    fn lgamma(&self) -> Expr;
    /// Digamma function
    fn digamma(&self) -> Expr;
    /// Trigamma function
    fn trigamma(&self) -> Expr;
    /// Tetragamma function
    fn tetragamma(&self) -> Expr;
    /// Riemann zeta function
    fn zeta(&self) -> Expr;
    /// Lambert W function
    fn lambertw(&self) -> Expr;
    /// Complete elliptic integral of first kind
    fn elliptic_k(&self) -> Expr;
    /// Complete elliptic integral of second kind
    fn elliptic_e(&self) -> Expr;
    /// Polar exponential
    fn exp_polar(&self) -> Expr;
}

impl ArcExprExt for Arc<Expr> {
    fn pow(&self, exp: impl Into<Expr>) -> Expr {
        Expr::pow_static(Expr::from(self), exp.into())
    }
    fn sin(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.sin), Expr::from(self))
    }
    fn cos(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.cos), Expr::from(self))
    }
    fn tan(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.tan), Expr::from(self))
    }
    fn cot(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.cot), Expr::from(self))
    }
    fn sec(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.sec), Expr::from(self))
    }
    fn csc(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.csc), Expr::from(self))
    }
    fn asin(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.asin), Expr::from(self))
    }
    fn acos(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.acos), Expr::from(self))
    }
    fn atan(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.atan), Expr::from(self))
    }
    fn acot(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.acot), Expr::from(self))
    }
    fn asec(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.asec), Expr::from(self))
    }
    fn acsc(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.acsc), Expr::from(self))
    }
    fn sinh(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.sinh), Expr::from(self))
    }
    fn cosh(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.cosh), Expr::from(self))
    }
    fn tanh(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.tanh), Expr::from(self))
    }
    fn coth(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.coth), Expr::from(self))
    }
    fn sech(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.sech), Expr::from(self))
    }
    fn csch(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.csch), Expr::from(self))
    }
    fn asinh(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.asinh), Expr::from(self))
    }
    fn acosh(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.acosh), Expr::from(self))
    }
    fn atanh(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.atanh), Expr::from(self))
    }
    fn acoth(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.acoth), Expr::from(self))
    }
    fn asech(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.asech), Expr::from(self))
    }
    fn acsch(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.acsch), Expr::from(self))
    }
    fn exp(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.exp), Expr::from(self))
    }
    fn ln(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.ln), Expr::from(self))
    }
    fn log(&self, base: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_interned(ks::KS.log),
            vec![base.into(), Expr::from(self)],
        )
    }
    fn log10(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.log10), Expr::from(self))
    }
    fn log2(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.log2), Expr::from(self))
    }
    fn sqrt(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.sqrt), Expr::from(self))
    }
    fn cbrt(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.cbrt), Expr::from(self))
    }
    fn floor(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.floor), Expr::from(self))
    }
    fn ceil(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.ceil), Expr::from(self))
    }
    fn round(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.round), Expr::from(self))
    }
    fn abs(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.abs), Expr::from(self))
    }
    fn signum(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.signum), Expr::from(self))
    }
    fn sinc(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.sinc), Expr::from(self))
    }
    fn erf(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.erf), Expr::from(self))
    }
    fn erfc(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.erfc), Expr::from(self))
    }
    fn gamma(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.gamma), Expr::from(self))
    }
    fn lgamma(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.lgamma), Expr::from(self))
    }
    fn digamma(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.digamma), Expr::from(self))
    }
    fn trigamma(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.trigamma), Expr::from(self))
    }
    fn tetragamma(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.tetragamma), Expr::from(self))
    }
    fn zeta(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.zeta), Expr::from(self))
    }
    fn lambertw(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.lambertw), Expr::from(self))
    }
    fn elliptic_k(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.elliptic_k), Expr::from(self))
    }
    fn elliptic_e(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.elliptic_e), Expr::from(self))
    }
    fn exp_polar(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.exp_polar), Expr::from(self))
    }
}

// =============================================================================
// Expr Methods (parametric)
// =============================================================================

impl Expr {
    /// Raise to a power (method form, consumes self)
    #[inline]
    #[must_use]
    pub fn pow(self, exp: impl Into<Self>) -> Self {
        Self::pow_static(self, exp.into())
    }

    /// Polygamma function: ψ^(n)(x)
    #[must_use]
    pub fn polygamma(self, n: impl Into<Self>) -> Self {
        Self::func_multi_symbol(ks::get_interned(ks::KS.polygamma), vec![n.into(), self])
    }

    /// Beta function: B(a, b)
    #[must_use]
    pub fn beta(self, other: impl Into<Self>) -> Self {
        Self::func_multi_symbol(ks::get_interned(ks::KS.beta), vec![self, other.into()])
    }

    /// Bessel function of the first kind: `J_n(x)`
    #[must_use]
    pub fn besselj(self, n: impl Into<Self>) -> Self {
        Self::func_multi_symbol(ks::get_interned(ks::KS.besselj), vec![n.into(), self])
    }

    /// Bessel function of the second kind: `Y_n(x)`
    #[must_use]
    pub fn bessely(self, n: impl Into<Self>) -> Self {
        Self::func_multi_symbol(ks::get_interned(ks::KS.bessely), vec![n.into(), self])
    }

    /// Modified Bessel function of the first kind: `I_n(x)`
    #[must_use]
    pub fn besseli(self, n: impl Into<Self>) -> Self {
        Self::func_multi_symbol(ks::get_interned(ks::KS.besseli), vec![n.into(), self])
    }

    /// Modified Bessel function of the second kind: `K_n(x)`
    #[must_use]
    pub fn besselk(self, n: impl Into<Self>) -> Self {
        Self::func_multi_symbol(ks::get_interned(ks::KS.besselk), vec![n.into(), self])
    }

    /// Derivative of Riemann zeta function: ζ^(n)(self)
    #[must_use]
    pub fn zeta_deriv(self, n: impl Into<Self>) -> Self {
        Self::func_multi_symbol(ks::get_interned(ks::KS.zeta_deriv), vec![n.into(), self])
    }

    /// Logarithm with arbitrary base: `x.log(base)` → `log(base, x)`
    #[must_use]
    pub fn log(self, base: impl Into<Self>) -> Self {
        Self::func_multi_symbol(ks::get_interned(ks::KS.log), vec![base.into(), self])
    }

    /// Hermite polynomial `H_n(self)`
    #[must_use]
    pub fn hermite(self, n: impl Into<Self>) -> Self {
        Self::func_multi_symbol(ks::get_interned(ks::KS.hermite), vec![n.into(), self])
    }

    /// Associated Legendre polynomial `P_l^m(self)`
    #[must_use]
    pub fn assoc_legendre(self, l: impl Into<Self>, m: impl Into<Self>) -> Self {
        Self::func_multi_symbol(
            ks::get_interned(ks::KS.assoc_legendre),
            vec![l.into(), m.into(), self],
        )
    }

    /// Spherical harmonic `Y_l^m(theta, phi)` where self is theta
    #[must_use]
    pub fn spherical_harmonic(
        self,
        l: impl Into<Self>,
        m: impl Into<Self>,
        phi: impl Into<Self>,
    ) -> Self {
        Self::func_multi_symbol(
            ks::get_interned(ks::KS.spherical_harmonic),
            vec![l.into(), m.into(), self, phi.into()],
        )
    }

    /// Alternative spherical harmonic notation `Y_l^m(theta, phi)`
    #[must_use]
    pub fn ynm(self, l: impl Into<Self>, m: impl Into<Self>, phi: impl Into<Self>) -> Self {
        Self::func_multi_symbol(
            ks::get_interned(ks::KS.ynm),
            vec![l.into(), m.into(), self, phi.into()],
        )
    }

    /// Two-argument arctangent: atan2(self, x) = angle to point (x, self)
    #[must_use]
    pub fn atan2(self, x: impl Into<Self>) -> Self {
        Self::func_multi_symbol(ks::get_interned(ks::KS.atan2), vec![self, x.into()])
    }
}
