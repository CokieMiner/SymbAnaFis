//! Operator overloading for Symbol, Expr, and numeric types.
//!
//! Contains all the `Add`, `Sub`, `Mul`, `Div`, `Neg` implementations
//! for various type combinations.

use std::ops::{Add, Div, Mul, Neg, Sub};
use std::sync::Arc;

use super::Symbol;
use crate::Expr;
use crate::core::known_symbols as ks;

// ============================================================================
// Operator Overloading Macro
// ============================================================================

macro_rules! impl_binary_ops {
    (symbol_ops $lhs:ty, $rhs:ty, $to_lhs:expr, $to_rhs:expr) => {
        impl Add<$rhs> for $lhs {
            type Output = Expr;
            fn add(self, rhs: $rhs) -> Expr {
                Expr::add_expr($to_lhs(self), $to_rhs(rhs))
            }
        }
        impl Sub<$rhs> for $lhs {
            type Output = Expr;
            fn sub(self, rhs: $rhs) -> Expr {
                Expr::sub_expr($to_lhs(self), $to_rhs(rhs))
            }
        }
        impl Mul<$rhs> for $lhs {
            type Output = Expr;
            fn mul(self, rhs: $rhs) -> Expr {
                Expr::mul_expr($to_lhs(self), $to_rhs(rhs))
            }
        }
        impl Div<$rhs> for $lhs {
            type Output = Expr;
            fn div(self, rhs: $rhs) -> Expr {
                Expr::div_expr($to_lhs(self), $to_rhs(rhs))
            }
        }
    };
}

// Symbol operations (value)
impl_binary_ops!(symbol_ops Symbol, Symbol, |s: Symbol| s.to_expr(), |r: Symbol| r.to_expr());
impl_binary_ops!(symbol_ops Symbol, Expr, |s: Symbol| s.to_expr(), |r: Expr| r);
impl_binary_ops!(symbol_ops Symbol, f64, |s: Symbol| s.to_expr(), |r: f64| Expr::number(r));

// &Symbol operations (reference)
impl_binary_ops!(
    symbol_ops & Symbol,
    &Symbol,
    |s: &Symbol| s.to_expr(),
    |r: &Symbol| r.to_expr()
);
impl_binary_ops!(
    symbol_ops & Symbol,
    Symbol,
    |s: &Symbol| s.to_expr(),
    |r: Symbol| r.to_expr()
);
impl_binary_ops!(
    symbol_ops & Symbol,
    Expr,
    |s: &Symbol| s.to_expr(),
    |r: Expr| r
);
impl_binary_ops!(
    symbol_ops & Symbol,
    f64,
    |s: &Symbol| s.to_expr(),
    |r: f64| Expr::number(r)
);

// Expr operations
impl_binary_ops!(symbol_ops Expr, Expr, |s: Expr| s, |r: Expr| r);
impl_binary_ops!(symbol_ops Expr, Symbol, |s: Expr| s, |r: Symbol| r.to_expr());
impl_binary_ops!(symbol_ops Expr, &Symbol, |s: Expr| s, |r: &Symbol| r.to_expr());
impl_binary_ops!(symbol_ops Expr, f64, |s: Expr| s, |r: f64| Expr::number(r));
impl_binary_ops!(symbol_ops Expr, &Expr, |s: Expr| s, |r: &Expr| r.clone());

// Symbol + &Symbol and Symbol + &Expr
impl_binary_ops!(symbol_ops Symbol, &Symbol, |s: Symbol| s.to_expr(), |r: &Symbol| r.to_expr());
impl_binary_ops!(symbol_ops Symbol, &Expr, |s: Symbol| s.to_expr(), |r: &Expr| r.clone());

// &Expr operations (reference - allows &expr + &expr without explicit .clone())
impl_binary_ops!(
    symbol_ops & Expr,
    &Expr,
    |e: &Expr| e.clone(),
    |r: &Expr| r.clone()
);
impl_binary_ops!(symbol_ops & Expr, Expr, |e: &Expr| e.clone(), |r: Expr| r);
impl_binary_ops!(
    symbol_ops & Expr,
    Symbol,
    |e: &Expr| e.clone(),
    |r: Symbol| r.to_expr()
);
impl_binary_ops!(
    symbol_ops & Expr,
    &Symbol,
    |e: &Expr| e.clone(),
    |r: &Symbol| r.to_expr()
);
impl_binary_ops!(symbol_ops & Expr, f64, |e: &Expr| e.clone(), |r: f64| {
    Expr::number(r)
});

// f64 on left side
impl Add<Expr> for f64 {
    type Output = Expr;
    fn add(self, rhs: Expr) -> Expr {
        Expr::add_expr(Expr::number(self), rhs)
    }
}

impl Mul<Expr> for f64 {
    type Output = Expr;
    fn mul(self, rhs: Expr) -> Expr {
        Expr::mul_expr(Expr::number(self), rhs)
    }
}

impl Sub<Expr> for f64 {
    type Output = Expr;
    fn sub(self, rhs: Expr) -> Expr {
        Expr::sub_expr(Expr::number(self), rhs)
    }
}

impl Div<Expr> for f64 {
    type Output = Expr;
    fn div(self, rhs: Expr) -> Expr {
        Expr::div_expr(Expr::number(self), rhs)
    }
}

impl Sub<Symbol> for f64 {
    type Output = Expr;
    fn sub(self, rhs: Symbol) -> Expr {
        Expr::sub_expr(Expr::number(self), rhs.to_expr())
    }
}

impl Mul<Symbol> for f64 {
    type Output = Expr;
    fn mul(self, rhs: Symbol) -> Expr {
        Expr::mul_expr(Expr::number(self), rhs.to_expr())
    }
}

impl Add<Symbol> for f64 {
    type Output = Expr;
    fn add(self, rhs: Symbol) -> Expr {
        Expr::add_expr(Expr::number(self), rhs.to_expr())
    }
}

impl Div<Symbol> for f64 {
    type Output = Expr;
    fn div(self, rhs: Symbol) -> Expr {
        Expr::div_expr(Expr::number(self), rhs.to_expr())
    }
}

// f64 on left side with &Expr
impl Add<&Expr> for f64 {
    type Output = Expr;
    fn add(self, rhs: &Expr) -> Expr {
        Expr::add_expr(Expr::number(self), rhs.clone())
    }
}

impl Mul<&Expr> for f64 {
    type Output = Expr;
    fn mul(self, rhs: &Expr) -> Expr {
        Expr::mul_expr(Expr::number(self), rhs.clone())
    }
}

impl Sub<&Expr> for f64 {
    type Output = Expr;
    fn sub(self, rhs: &Expr) -> Expr {
        Expr::sub_expr(Expr::number(self), rhs.clone())
    }
}

impl Div<&Expr> for f64 {
    type Output = Expr;
    fn div(self, rhs: &Expr) -> Expr {
        Expr::div_expr(Expr::number(self), rhs.clone())
    }
}

// f64 on left side with &Symbol
impl Add<&Symbol> for f64 {
    type Output = Expr;
    fn add(self, rhs: &Symbol) -> Expr {
        Expr::add_expr(Expr::number(self), rhs.to_expr())
    }
}

impl Sub<&Symbol> for f64 {
    type Output = Expr;
    fn sub(self, rhs: &Symbol) -> Expr {
        Expr::sub_expr(Expr::number(self), rhs.to_expr())
    }
}

impl Mul<&Symbol> for f64 {
    type Output = Expr;
    fn mul(self, rhs: &Symbol) -> Expr {
        Expr::mul_expr(Expr::number(self), rhs.to_expr())
    }
}

impl Div<&Symbol> for f64 {
    type Output = Expr;
    fn div(self, rhs: &Symbol) -> Expr {
        Expr::div_expr(Expr::number(self), rhs.to_expr())
    }
}

// =============================================================================
// i32 operators (Symbol/Expr on left side)
// =============================================================================

// Symbol + i32
impl_binary_ops!(symbol_ops Symbol, i32, |s: Symbol| s.to_expr(), |r: i32| Expr::number(f64::from(r)));
impl_binary_ops!(
    symbol_ops & Symbol,
    i32,
    |s: &Symbol| s.to_expr(),
    |r: i32| Expr::number(f64::from(r))
);

// Expr + i32
impl_binary_ops!(symbol_ops Expr, i32, |s: Expr| s, |r: i32| Expr::number(f64::from(r)));
impl_binary_ops!(symbol_ops & Expr, i32, |e: &Expr| e.clone(), |r: i32| {
    Expr::number(f64::from(r))
});

// i32 on left side with Symbol
impl Add<Symbol> for i32 {
    type Output = Expr;
    fn add(self, rhs: Symbol) -> Expr {
        Expr::add_expr(Expr::number(f64::from(self)), rhs.to_expr())
    }
}

impl Sub<Symbol> for i32 {
    type Output = Expr;
    fn sub(self, rhs: Symbol) -> Expr {
        Expr::sub_expr(Expr::number(f64::from(self)), rhs.to_expr())
    }
}

impl Mul<Symbol> for i32 {
    type Output = Expr;
    fn mul(self, rhs: Symbol) -> Expr {
        Expr::mul_expr(Expr::number(f64::from(self)), rhs.to_expr())
    }
}

impl Div<Symbol> for i32 {
    type Output = Expr;
    fn div(self, rhs: Symbol) -> Expr {
        Expr::div_expr(Expr::number(f64::from(self)), rhs.to_expr())
    }
}

// i32 on left side with Expr
impl Add<Expr> for i32 {
    type Output = Expr;
    fn add(self, rhs: Expr) -> Expr {
        Expr::add_expr(Expr::number(f64::from(self)), rhs)
    }
}

impl Sub<Expr> for i32 {
    type Output = Expr;
    fn sub(self, rhs: Expr) -> Expr {
        Expr::sub_expr(Expr::number(f64::from(self)), rhs)
    }
}

impl Mul<Expr> for i32 {
    type Output = Expr;
    fn mul(self, rhs: Expr) -> Expr {
        Expr::mul_expr(Expr::number(f64::from(self)), rhs)
    }
}

impl Div<Expr> for i32 {
    type Output = Expr;
    fn div(self, rhs: Expr) -> Expr {
        Expr::div_expr(Expr::number(f64::from(self)), rhs)
    }
}

// i32 on left side with &Expr
impl Add<&Expr> for i32 {
    type Output = Expr;
    fn add(self, rhs: &Expr) -> Expr {
        Expr::add_expr(Expr::number(f64::from(self)), rhs.clone())
    }
}

impl Sub<&Expr> for i32 {
    type Output = Expr;
    fn sub(self, rhs: &Expr) -> Expr {
        Expr::sub_expr(Expr::number(f64::from(self)), rhs.clone())
    }
}

impl Mul<&Expr> for i32 {
    type Output = Expr;
    fn mul(self, rhs: &Expr) -> Expr {
        Expr::mul_expr(Expr::number(f64::from(self)), rhs.clone())
    }
}

impl Div<&Expr> for i32 {
    type Output = Expr;
    fn div(self, rhs: &Expr) -> Expr {
        Expr::div_expr(Expr::number(f64::from(self)), rhs.clone())
    }
}

// =============================================================================
// Arc<Expr> operations
// =============================================================================

// --- Expr with Arc<Expr> (Expr on left side works due to local type) ---
impl Add<Arc<Self>> for Expr {
    type Output = Self;
    fn add(self, rhs: Arc<Self>) -> Self {
        Self::add_expr(self, Self::from(rhs))
    }
}

impl Add<&Arc<Self>> for Expr {
    type Output = Self;
    fn add(self, rhs: &Arc<Self>) -> Self {
        Self::add_expr(self, Self::from(rhs))
    }
}

impl Sub<Arc<Self>> for Expr {
    type Output = Self;
    fn sub(self, rhs: Arc<Self>) -> Self {
        Self::sub_expr(self, Self::from(rhs))
    }
}

impl Sub<&Arc<Self>> for Expr {
    type Output = Self;
    fn sub(self, rhs: &Arc<Self>) -> Self {
        Self::sub_expr(self, Self::from(rhs))
    }
}

impl Mul<Arc<Self>> for Expr {
    type Output = Self;
    fn mul(self, rhs: Arc<Self>) -> Self {
        Self::mul_expr(self, Self::from(rhs))
    }
}

impl Mul<&Arc<Self>> for Expr {
    type Output = Self;
    fn mul(self, rhs: &Arc<Self>) -> Self {
        Self::mul_expr(self, Self::from(rhs))
    }
}

impl Div<Arc<Self>> for Expr {
    type Output = Self;
    fn div(self, rhs: Arc<Self>) -> Self {
        Self::div_expr(self, Self::from(rhs))
    }
}

impl Div<&Arc<Self>> for Expr {
    type Output = Self;
    fn div(self, rhs: &Arc<Self>) -> Self {
        Self::div_expr(self, Self::from(rhs))
    }
}

// =============================================================================
// Negation operators
// =============================================================================

impl Neg for Expr {
    type Output = Self;
    fn neg(self) -> Self {
        Self::mul_expr(Self::number(-1.0), self)
    }
}

impl Neg for &Expr {
    type Output = Expr;
    fn neg(self) -> Expr {
        Expr::mul_expr(Expr::number(-1.0), self.clone())
    }
}

impl Neg for &Symbol {
    type Output = Expr;
    fn neg(self) -> Expr {
        Expr::mul_expr(Expr::number(-1.0), self.to_expr())
    }
}

// =============================================================================
// ArcExprExt trait for ergonomic Arc<Expr> operations
// =============================================================================

/// Extension trait for `Arc<Expr>` providing ergonomic math operations.
///
/// This trait is automatically in scope when using the library, allowing
/// you to call methods like `.pow()`, `.sin()`, `.cos()` directly on `Arc<Expr>`.
///
/// # Example
/// ```
/// use symb_anafis::{UserFunction, Expr, ArcExprExt};
/// use std::sync::Arc;
///
/// // f(x) = x^2 + sin(x)
/// let f = UserFunction::new(1..=1)
///     .body(|args| args[0].pow(2.0) + args[0].sin());
/// ```
pub trait ArcExprExt {
    /// Raise to a power: `args[0].pow(2.0)` → `args[0]^2`
    fn pow(&self, exp: impl Into<Expr>) -> Expr;

    // Trigonometric
    /// Sine function: sin(x)
    fn sin(&self) -> Expr;
    /// Cosine function: cos(x)
    fn cos(&self) -> Expr;
    /// Tangent function: tan(x)
    fn tan(&self) -> Expr;
    /// Cotangent function: cot(x)
    fn cot(&self) -> Expr;
    /// Secant function: sec(x)
    fn sec(&self) -> Expr;
    /// Cosecant function: csc(x)
    fn csc(&self) -> Expr;

    // Inverse trigonometric
    /// Arcsine function: asin(x)
    fn asin(&self) -> Expr;
    /// Arccosine function: acos(x)
    fn acos(&self) -> Expr;
    /// Arctangent function: atan(x)
    fn atan(&self) -> Expr;
    /// Arccotangent function: acot(x)
    fn acot(&self) -> Expr;
    /// Arcsecant function: asec(x)
    fn asec(&self) -> Expr;
    /// Arccosecant function: acsc(x)
    fn acsc(&self) -> Expr;

    // Hyperbolic
    /// Hyperbolic sine: sinh(x)
    fn sinh(&self) -> Expr;
    /// Hyperbolic cosine: cosh(x)
    fn cosh(&self) -> Expr;
    /// Hyperbolic tangent: tanh(x)
    fn tanh(&self) -> Expr;
    /// Hyperbolic cotangent: coth(x)
    fn coth(&self) -> Expr;
    /// Hyperbolic secant: sech(x)
    fn sech(&self) -> Expr;
    /// Hyperbolic cosecant: csch(x)
    fn csch(&self) -> Expr;

    // Inverse hyperbolic
    /// Inverse hyperbolic sine: asinh(x)
    fn asinh(&self) -> Expr;
    /// Inverse hyperbolic cosine: acosh(x)
    fn acosh(&self) -> Expr;
    /// Inverse hyperbolic tangent: atanh(x)
    fn atanh(&self) -> Expr;
    /// Inverse hyperbolic cotangent: acoth(x)
    fn acoth(&self) -> Expr;
    /// Inverse hyperbolic secant: asech(x)
    fn asech(&self) -> Expr;
    /// Inverse hyperbolic cosecant: acsch(x)
    fn acsch(&self) -> Expr;

    // Exponential/logarithmic
    /// Exponential function: exp(x) = e^x
    fn exp(&self) -> Expr;
    /// Natural logarithm: ln(x)
    fn ln(&self) -> Expr;
    /// Logarithm with arbitrary base: `x.log(base)` → `log(base, x)`
    fn log(&self, base: impl Into<Expr>) -> Expr;
    /// Base-10 logarithm: log10(x)
    fn log10(&self) -> Expr;
    /// Base-2 logarithm: log2(x)
    fn log2(&self) -> Expr;
    /// Square root: sqrt(x)
    fn sqrt(&self) -> Expr;
    /// Cube root: cbrt(x)
    fn cbrt(&self) -> Expr;

    // Rounding
    /// Floor function: floor(x)
    fn floor(&self) -> Expr;
    /// Ceiling function: ceil(x)
    fn ceil(&self) -> Expr;
    /// Round to nearest integer: round(x)
    fn round(&self) -> Expr;

    // Special functions
    /// Absolute value: abs(x)
    fn abs(&self) -> Expr;
    /// Sign function: signum(x)
    fn signum(&self) -> Expr;
    /// Sinc function: sin(x)/x
    fn sinc(&self) -> Expr;
    /// Error function: erf(x)
    fn erf(&self) -> Expr;
    /// Complementary error function: erfc(x)
    fn erfc(&self) -> Expr;
    /// Gamma function: Γ(x)
    fn gamma(&self) -> Expr;
    /// Digamma function: ψ(x)
    fn digamma(&self) -> Expr;
    /// Trigamma function: ψ₁(x)
    fn trigamma(&self) -> Expr;
    /// Tetragamma function: ψ₂(x)
    fn tetragamma(&self) -> Expr;
    /// Riemann zeta function: ζ(x)
    fn zeta(&self) -> Expr;
    /// Lambert W function
    fn lambertw(&self) -> Expr;
    /// Complete elliptic integral of the first kind: K(x)
    fn elliptic_k(&self) -> Expr;
    /// Complete elliptic integral of the second kind: E(x)
    fn elliptic_e(&self) -> Expr;
    /// Exponential with polar representation
    fn exp_polar(&self) -> Expr;
}

impl ArcExprExt for Arc<Expr> {
    fn pow(&self, exp: impl Into<Expr>) -> Expr {
        Expr::pow_static(Expr::from(self), exp.into())
    }

    // Trigonometric
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

    // Inverse trigonometric
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

    // Hyperbolic
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

    // Inverse hyperbolic
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

    // Exponential/logarithmic
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

    // Rounding
    fn floor(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.floor), Expr::from(self))
    }
    fn ceil(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.ceil), Expr::from(self))
    }
    fn round(&self) -> Expr {
        Expr::func_symbol(ks::get_interned(ks::KS.round), Expr::from(self))
    }

    // Special functions
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
// Expr Methods (pow, log, special functions that consume self)
// =============================================================================

impl Expr {
    /// Raise to a power (method form, consumes self)
    ///
    /// This is the ergonomic method form for chaining. Accepts any type
    /// that can be converted to Expr (f64, i32, Symbol, Expr).
    ///
    /// # Example
    /// ```
    /// use symb_anafis::symb;
    /// let x = symb("pow_method_x");
    /// let expr = x.sin().pow(2.0);  // sin(x)^2
    /// assert_eq!(format!("{}", expr), "sin(pow_method_x)^2");
    /// ```
    #[inline]
    #[must_use]
    pub fn pow(self, exp: impl Into<Self>) -> Self {
        Self::pow_static(self, exp.into())
    }

    // === Parametric special functions ===

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
    ///
    /// # Example
    /// ```
    /// use symb_anafis::{symb, Expr};
    /// let x = symb("log_expr_example");
    /// let expr = x.sin() + Expr::number(1.0);
    /// let log_base_10 = expr.log(10.0);  // log(10, sin(x) + 1)
    /// assert!(format!("{}", log_base_10).contains("log"));
    /// ```
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

    /// Spherical harmonic `Y_l^m(theta`, phi) where self is theta
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

    /// Alternative spherical harmonic notation `Y_l^m(theta`, phi)
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
