//! Scalar conversion and arity-typed numerical function kernels.

#![expect(
    unsafe_code,
    reason = "a range check proves the float-to-integer conversion of discrete arguments"
)]

use core::{
    fmt::Debug,
    ops::{Add, Div, Mul, Neg, Sub},
};

use num_anafis::{Complex, F64Ext, Number, Numeric, Real};

use super::BuiltinFunction;

/// Literal conversion for a commutative scalar type implementing [`Numeric`].
///
/// Arithmetic and functions come from `Numeric`. Implementations keep exact
/// components when representable and reject unsupported domains.
/// Noncommutative algebras do not fit, since products are commutative.
pub trait EvaluationValue: Numeric {
    /// Convert a literal, keeping its imaginary component; `None` when this
    /// type cannot represent it.
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
}

impl EvaluationValue for Number {
    fn from_number(value: &Self) -> Option<Self> {
        Some(value.clone())
    }
}

impl EvaluationValue for Real {
    fn from_number(value: &Number) -> Option<Self> {
        value.is_real().then(|| value.components().0.clone())
    }
}

impl EvaluationValue for Complex {
    fn from_number(value: &Number) -> Option<Self> {
        Some(value.to_complex())
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
        }
    }
}

impl Scalar for f64 {
    const OPTIMIZE_FLOATS: bool = true;
    /// Fused operations are emitted only where the target compiles the FMA
    /// instruction inline; a runtime-selected provider is an indirect call per
    /// operation, slower than a separate product and sum and opaque to
    /// vectorization.
    const FUSE: bool = cfg!(any(target_feature = "fma", target_arch = "aarch64"));
    fn literal(value: &Number) -> Option<Self> {
        value.is_real().then(|| value.to_f64())
    }

    fn is_negative_one(value: &Self) -> bool {
        value.to_bits() == (-1.0_f64).to_bits()
    }

    fn constant_reciprocal(value: &Self) -> Option<Self> {
        if !value.is_finite() || *value == 0.0 {
            return None;
        }
        let reciprocal = 1.0 / value;
        (reciprocal.is_finite() && reciprocal != 0.0).then_some(reciprocal)
    }

    fn pi() -> Option<Self> {
        Some(Number::pi().to_f64())
    }

    fn euler() -> Option<Self> {
        Some(Number::e().to_f64())
    }

    fn power(base: &Self, exponent: &Self) -> Self {
        F64Ext::powf(*base, *exponent)
    }

    #[inline]
    fn fused(left: &Self, right: &Self, addend: &Self) -> Self {
        F64Ext::mul_add(*left, *right, *addend)
    }

    fn sin_cos(value: &Self) -> (Self, Self) {
        F64Ext::sin_cos(*value)
    }

    fn sinh_cosh(value: &Self) -> (Self, Self) {
        F64Ext::sinh_cosh(*value)
    }

    fn kernel(function: BuiltinFunction) -> Kernel<Self> {
        match function {
            BuiltinFunction::Sin => Kernel::Unary(|value| F64Ext::sin(*value)),
            BuiltinFunction::Cos => Kernel::Unary(|value| F64Ext::cos(*value)),
            BuiltinFunction::Tan => Kernel::Unary(|value| F64Ext::tan(*value)),
            BuiltinFunction::Cot => Kernel::Unary(|value| F64Ext::cot(*value)),
            BuiltinFunction::Sec => Kernel::Unary(|value| F64Ext::sec(*value)),
            BuiltinFunction::Csc => Kernel::Unary(|value| F64Ext::csc(*value)),
            BuiltinFunction::Asin => Kernel::Unary(|value| F64Ext::asin(*value)),
            BuiltinFunction::Acos => Kernel::Unary(|value| F64Ext::acos(*value)),
            BuiltinFunction::Atan => Kernel::Unary(|value| F64Ext::atan(*value)),
            BuiltinFunction::Acot => Kernel::Unary(|value| F64Ext::acot(*value)),
            BuiltinFunction::Asec => Kernel::Unary(|value| F64Ext::asec(*value)),
            BuiltinFunction::Acsc => Kernel::Unary(|value| F64Ext::acsc(*value)),
            BuiltinFunction::Sinh => Kernel::Unary(|value| F64Ext::sinh(*value)),
            BuiltinFunction::Cosh => Kernel::Unary(|value| F64Ext::cosh(*value)),
            BuiltinFunction::Tanh => Kernel::Unary(|value| F64Ext::tanh(*value)),
            BuiltinFunction::Coth => Kernel::Unary(|value| F64Ext::coth(*value)),
            BuiltinFunction::Sech => Kernel::Unary(|value| F64Ext::sech(*value)),
            BuiltinFunction::Csch => Kernel::Unary(|value| F64Ext::csch(*value)),
            BuiltinFunction::Asinh => Kernel::Unary(|value| F64Ext::asinh(*value)),
            BuiltinFunction::Acosh => Kernel::Unary(|value| F64Ext::acosh(*value)),
            BuiltinFunction::Atanh => Kernel::Unary(|value| F64Ext::atanh(*value)),
            BuiltinFunction::Acoth => Kernel::Unary(|value| F64Ext::acoth(*value)),
            BuiltinFunction::Asech => Kernel::Unary(|value| F64Ext::asech(*value)),
            BuiltinFunction::Acsch => Kernel::Unary(|value| F64Ext::acsch(*value)),
            BuiltinFunction::Exp => Kernel::Unary(|value| F64Ext::exp(*value)),
            BuiltinFunction::ExpM1 => Kernel::Unary(|value| F64Ext::exp_m1(*value)),
            BuiltinFunction::ExpNeg => Kernel::Unary(|value| F64Ext::exp(-value)),
            BuiltinFunction::Ln => Kernel::Unary(|value| F64Ext::ln(*value)),
            BuiltinFunction::Ln1p => Kernel::Unary(|value| F64Ext::ln_1p(*value)),
            BuiltinFunction::Log10 => Kernel::Unary(|value| F64Ext::log10(*value)),
            BuiltinFunction::Log2 => Kernel::Unary(|value| F64Ext::log2(*value)),
            BuiltinFunction::Sqrt => Kernel::Unary(|value| F64Ext::sqrt(*value)),
            BuiltinFunction::Cbrt => Kernel::Unary(|value| F64Ext::cbrt(*value)),
            BuiltinFunction::Floor => Kernel::Unary(|value| F64Ext::floor(*value)),
            BuiltinFunction::Ceil => Kernel::Unary(|value| F64Ext::ceil(*value)),
            BuiltinFunction::Round => Kernel::Unary(|value| F64Ext::round(*value)),
            BuiltinFunction::Trunc => Kernel::Unary(|value| F64Ext::trunc(*value)),
            BuiltinFunction::Fract => Kernel::Unary(|value| F64Ext::fract(*value)),
            BuiltinFunction::Abs => Kernel::Unary(|value| F64Ext::abs(*value)),
            BuiltinFunction::Signum => Kernel::Unary(|value| F64Ext::signum(*value)),
            BuiltinFunction::Sinc => Kernel::Unary(|value| F64Ext::sinc(*value)),
            BuiltinFunction::Erf => Kernel::Unary(|value| F64Ext::erf(*value)),
            BuiltinFunction::Erfc => Kernel::Unary(|value| F64Ext::erfc(*value)),
            BuiltinFunction::Gamma => Kernel::Unary(|value| F64Ext::gamma(*value)),
            BuiltinFunction::Lgamma => Kernel::Unary(|value| F64Ext::lgamma(*value)),
            BuiltinFunction::Digamma => Kernel::Unary(|value| F64Ext::digamma(*value)),
            BuiltinFunction::Trigamma => Kernel::Unary(|value| F64Ext::trigamma(*value)),
            BuiltinFunction::Tetragamma => Kernel::Unary(|value| F64Ext::tetragamma(*value)),
            BuiltinFunction::Zeta => Kernel::Unary(|value| F64Ext::zeta(*value)),
            BuiltinFunction::EllipticK => Kernel::Unary(|value| F64Ext::elliptic_k(*value)),
            BuiltinFunction::EllipticE => Kernel::Unary(|value| F64Ext::elliptic_e(*value)),
            BuiltinFunction::Polygamma => Kernel::Binary(|order, value| {
                discrete(*order).map_or(Self::NAN, |index| F64Ext::polygamma(*value, index))
            }),
            BuiltinFunction::ZetaDeriv => Kernel::Binary(|order, value| {
                discrete(*order).map_or(Self::NAN, |index| F64Ext::zeta_deriv(*value, index))
            }),
            BuiltinFunction::LambertW => Kernel::Binary(|branch, value| match discrete(*branch) {
                Some(0) => F64Ext::lambert_w0(*value),
                Some(-1) => F64Ext::lambert_wm1(*value),
                Some(_) | None => Self::NAN,
            }),
            BuiltinFunction::BesselJ => {
                Kernel::Binary(|order, value| F64Ext::bessel_j(*value, *order))
            }
            BuiltinFunction::BesselY => {
                Kernel::Binary(|order, value| F64Ext::bessel_y(*value, *order))
            }
            BuiltinFunction::BesselI => {
                Kernel::Binary(|order, value| F64Ext::bessel_i(*value, *order))
            }
            BuiltinFunction::BesselK => {
                Kernel::Binary(|order, value| F64Ext::bessel_k(*value, *order))
            }
            BuiltinFunction::Log => Kernel::Binary(|base, value| F64Ext::log(*value, *base)),
            BuiltinFunction::Hermite => {
                Kernel::Binary(|degree, value| F64Ext::hermite(*value, *degree))
            }
            BuiltinFunction::Beta => Kernel::Binary(|left, right| F64Ext::beta(*left, *right)),
            BuiltinFunction::Atan2 => Kernel::Binary(|left, right| F64Ext::atan2(*left, *right)),
            BuiltinFunction::AssocLegendre => Kernel::Ternary(|degree, order, value| {
                F64Ext::assoc_legendre(*value, *degree, *order)
            }),
        }
    }
}

fn discrete(value: f64) -> Option<i64> {
    let rounded = F64Ext::round(value);
    // The upper bound is exclusive: binary64 rounds i64::MAX to 2^63.
    let bound = f64::from_bits(0x43e0_0000_0000_0000);
    (-bound..bound).contains(&rounded).then(|| {
        // SAFETY: a value in the half-open range \([-2^{63}, 2^{63})\) is
        // finite, and as an integer it is its own truncation, which i64
        // represents.
        unsafe { rounded.to_int_unchecked::<i64>() }
    })
}
