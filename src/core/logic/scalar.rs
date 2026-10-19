//! Scalar conversion and arity-typed numerical function kernels.

#![expect(
    clippy::self_named_module_files,
    reason = "scalar implementations are descendants of their sealed kernel owner"
)]

use core::{
    fmt::Debug,
    ops::{Add, Div, Mul, Neg, Sub},
};

use num_anafis::{Complex, Number, Numeric, Real};

use super::BuiltinFunction;

/// Literal conversion for a commutative scalar implementing [`Numeric`].
///
/// Arithmetic and functions come directly from `Numeric`. Implementations must
/// preserve exact components when representable and reject unsupported domains.
/// Noncommutative algebras cannot interpret the core's commutative products.
pub trait EvaluationValue: Numeric {
    /// Convert a stored literal without discarding its imaginary component.
    /// Return `None` when the scalar cannot represent its domain or capacity.
    #[must_use]
    fn from_number(value: &Number) -> Option<Self>;
}

/// Sealed execution requirements shared by native and generic scalar values.
pub trait Scalar:
    Clone
    + Debug
    + Add<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Mul<Self, Output = Self>
    + Div<Self, Output = Self>
    + Neg<Output = Self>
    + for<'value> Add<&'value Self, Output = Self>
    + for<'value> Sub<&'value Self, Output = Self>
    + for<'value> Mul<&'value Self, Output = Self>
    + for<'value> Div<&'value Self, Output = Self>
{
    /// Whether algebraic reassociation is permitted for native floating execution.
    const OPTIMIZE_FLOATS: bool = false;
    /// Whether compilation may emit fused operations for this build.
    const FUSE: bool = false;
    /// Convert a stored scalar literal.
    fn literal(value: &Number) -> Option<Self>;
    /// Recognize the native negative unit for sign lowering.
    fn is_negative_one(value: &Self) -> bool;
    /// Admit a finite nonzero constant reciprocal for native strength reduction.
    /// Other backends retain division and return `None`.
    fn constant_reciprocal(value: &Self) -> Option<Self>;
    /// Convert pi supplied by the numerical backend.
    fn pi() -> Option<Self>;
    /// Convert Euler's number supplied by the numerical backend.
    fn euler() -> Option<Self>;
    /// Evaluate a power using the backend's domain policy.
    fn power(base: &Self, exponent: &Self) -> Self;
    /// Compute a single-rounding multiply-add.
    fn fused(left: &Self, right: &Self, addend: &Self) -> Self;
    /// Evaluate sine and cosine in that order through the numerical backend.
    fn sin_cos(value: &Self) -> (Self, Self);
    /// Evaluate hyperbolic sine and cosine in that order through the backend.
    fn sinh_cosh(value: &Self) -> (Self, Self);
    /// Select the signature fixed by a builtin identity.
    fn kernel(function: BuiltinFunction) -> Kernel<Self>;
}

/// A validated function signature in expression argument order.
#[derive(Clone, Copy, Debug)]
pub enum Kernel<Value> {
    /// A single scalar argument.
    Unary(fn(&Value) -> Value),
    /// Two ordered scalar arguments.
    Binary(fn(&Value, &Value) -> Value),
    /// Three ordered scalar arguments.
    Ternary(fn(&Value, &Value, &Value) -> Value),
    /// Four ordered scalar arguments.
    Quaternary(fn(&Value, &Value, &Value, &Value) -> Value),
}

impl EvaluationValue for Number {
    fn from_number(value: &Self) -> Option<Self> {
        Some(value.clone())
    }
}

impl EvaluationValue for Real {
    fn from_number(value: &Number) -> Option<Self> {
        value
            .is_real()
            .then(|| value.extract_complex().ok().map(|parts| parts.re))
            .flatten()
    }
}

impl EvaluationValue for Complex {
    fn from_number(value: &Number) -> Option<Self> {
        value.extract_complex().ok()
    }
}

impl<Value: EvaluationValue> Scalar for Value {
    fn literal(value: &Number) -> Option<Self> {
        Self::from_number(value)
    }

    fn is_negative_one(_value: &Self) -> bool {
        false
    }

    fn constant_reciprocal(_value: &Self) -> Option<Self> {
        None
    }

    fn pi() -> Option<Self> {
        Self::from_number(&Number::pi())
    }

    fn euler() -> Option<Self> {
        Self::from_number(&Number::e())
    }

    fn power(base: &Self, exponent: &Self) -> Self {
        base.pow(exponent)
    }

    fn fused(left: &Self, right: &Self, addend: &Self) -> Self {
        left.mul_add(right, addend)
    }

    fn sin_cos(value: &Self) -> (Self, Self) {
        Numeric::sin_cos(value)
    }

    fn sinh_cosh(value: &Self) -> (Self, Self) {
        Numeric::sinh_cosh(value)
    }

    fn kernel(function: BuiltinFunction) -> Kernel<Self> {
        match function {
            BuiltinFunction::Sin => Kernel::Unary(Self::sin),
            BuiltinFunction::Cos => Kernel::Unary(Self::cos),
            BuiltinFunction::Tan => Kernel::Unary(Self::tan),
            BuiltinFunction::Cot => Kernel::Unary(Self::cot),
            BuiltinFunction::Sec => Kernel::Unary(Self::sec),
            BuiltinFunction::Csc => Kernel::Unary(Self::csc),
            BuiltinFunction::Asin => Kernel::Unary(Self::asin),
            BuiltinFunction::Acos => Kernel::Unary(Self::acos),
            BuiltinFunction::Atan => Kernel::Unary(Self::atan),
            BuiltinFunction::Acot => Kernel::Unary(Self::acot),
            BuiltinFunction::Asec => Kernel::Unary(Self::asec),
            BuiltinFunction::Acsc => Kernel::Unary(Self::acsc),
            BuiltinFunction::Sinh => Kernel::Unary(Self::sinh),
            BuiltinFunction::Cosh => Kernel::Unary(Self::cosh),
            BuiltinFunction::Tanh => Kernel::Unary(Self::tanh),
            BuiltinFunction::Coth => Kernel::Unary(Self::coth),
            BuiltinFunction::Sech => Kernel::Unary(Self::sech),
            BuiltinFunction::Csch => Kernel::Unary(Self::csch),
            BuiltinFunction::Asinh => Kernel::Unary(Self::asinh),
            BuiltinFunction::Acosh => Kernel::Unary(Self::acosh),
            BuiltinFunction::Atanh => Kernel::Unary(Self::atanh),
            BuiltinFunction::Acoth => Kernel::Unary(Self::acoth),
            BuiltinFunction::Asech => Kernel::Unary(Self::asech),
            BuiltinFunction::Acsch => Kernel::Unary(Self::acsch),
            BuiltinFunction::Exp => Kernel::Unary(Self::exp),
            BuiltinFunction::ExpM1 => Kernel::Unary(Self::exp_m1),
            BuiltinFunction::ExpNeg => Kernel::Unary(Self::exp_neg),
            BuiltinFunction::Ln => Kernel::Unary(Self::ln),
            BuiltinFunction::Ln1p => Kernel::Unary(Self::ln_1p),
            BuiltinFunction::Log10 => Kernel::Unary(Self::log10),
            BuiltinFunction::Log2 => Kernel::Unary(Self::log2),
            BuiltinFunction::Sqrt => Kernel::Unary(Self::sqrt),
            BuiltinFunction::Cbrt => Kernel::Unary(Self::cbrt),
            BuiltinFunction::Floor => Kernel::Unary(Self::floor),
            BuiltinFunction::Ceil => Kernel::Unary(Self::ceil),
            BuiltinFunction::Round => Kernel::Unary(Self::round),
            BuiltinFunction::Trunc => Kernel::Unary(Self::trunc),
            BuiltinFunction::Fract => Kernel::Unary(Self::fract),
            BuiltinFunction::Abs => Kernel::Unary(Self::abs),
            BuiltinFunction::Signum => Kernel::Unary(Self::signum),
            BuiltinFunction::Sinc => Kernel::Unary(Self::sinc),
            BuiltinFunction::Erf => Kernel::Unary(Self::erf),
            BuiltinFunction::Erfc => Kernel::Unary(Self::erfc),
            BuiltinFunction::Gamma => Kernel::Unary(Self::gamma),
            BuiltinFunction::Lgamma => Kernel::Unary(Self::lgamma),
            BuiltinFunction::Digamma => Kernel::Unary(Self::digamma),
            BuiltinFunction::Trigamma => Kernel::Unary(Self::trigamma),
            BuiltinFunction::Tetragamma => Kernel::Unary(Self::tetragamma),
            BuiltinFunction::Zeta => Kernel::Unary(Self::zeta),
            BuiltinFunction::EllipticK => Kernel::Unary(Self::elliptic_k),
            BuiltinFunction::EllipticE => Kernel::Unary(Self::elliptic_e),
            BuiltinFunction::Polygamma => Kernel::Binary(|order, value| value.polygamma(order)),
            BuiltinFunction::BesselJ => Kernel::Binary(|order, value| value.bessel_j(order)),
            BuiltinFunction::BesselY => Kernel::Binary(|order, value| value.bessel_y(order)),
            BuiltinFunction::BesselI => Kernel::Binary(|order, value| value.bessel_i(order)),
            BuiltinFunction::BesselK => Kernel::Binary(|order, value| value.bessel_k(order)),
            BuiltinFunction::ZetaDeriv => Kernel::Binary(|order, value| value.zeta_deriv(order)),
            BuiltinFunction::Log => Kernel::Binary(|base, value| value.log(base)),
            BuiltinFunction::Hermite => Kernel::Binary(|degree, value| value.hermite(degree)),
            BuiltinFunction::LambertW => Kernel::Binary(|branch, value| value.lambert_w(branch)),
            BuiltinFunction::Beta => Kernel::Binary(Self::beta),
            BuiltinFunction::Atan2 => Kernel::Binary(Self::atan2),
            BuiltinFunction::AssocLegendre => {
                Kernel::Ternary(|degree, order, value| value.assoc_legendre(degree, order))
            }
            BuiltinFunction::SphericalHarmonic => {
                Kernel::Quaternary(|degree, order, theta, phi| {
                    theta.spherical_harmonic(degree, order, phi)
                })
            }
        }
    }
}

mod native;
