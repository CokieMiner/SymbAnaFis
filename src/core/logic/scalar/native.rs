//! Native binary64 numerical kernels using `NumAnaFis` functions.

use num_anafis::{F64Ext, Number};

use super::{BuiltinFunction, Kernel, Scalar};

impl Scalar for f64 {
    const OPTIMIZE_FLOATS: bool = true;
    const FUSE: bool = cfg!(any(
        feature = "std",
        target_feature = "fma",
        target_arch = "aarch64"
    ));
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
            BuiltinFunction::SphericalHarmonic => {
                Kernel::Quaternary(|degree, order, theta, phi| {
                    F64Ext::spherical_harmonic(*theta, *degree, *order, *phi)
                })
            }
        }
    }
}

fn discrete(value: f64) -> Option<i64> {
    let rounded = F64Ext::round(value);
    // The upper bound is exclusive: binary64 rounds i64::MAX to 2^63.
    let bound = f64::from_bits(0x43e0_0000_0000_0000);
    if (-bound..bound).contains(&rounded) {
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "the finite rounded value lies in the half-open signed 64-bit range"
        )]
        return Some(rounded as i64);
    }
    None
}
