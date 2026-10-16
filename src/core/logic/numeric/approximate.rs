//! Builtin numerical evaluation for inputs with stored floating components.
//!
//! Exact inputs never enter this path. A finite approximate call uses `NumAnaFis`'s
//! representation, precision, and complex-domain policy; nonfinite inputs or
//! results retain the original function node.

use num_anafis::{Number, Numeric};

use super::{BuiltinFunction, NumericValue};

impl NumericValue {
    /// Evaluate a builtin when all arguments are finite literals and some are approximate.
    ///
    /// # Panics
    /// Numeric evaluation can propagate the backend's rational capacity panics.
    pub fn evaluate_approximate(builtin: BuiltinFunction, arguments: &[&Self]) -> Option<Number> {
        if !arguments.iter().any(|value| value.has_float())
            || !arguments.iter().all(|value| value.0.is_finite())
        {
            return None;
        }
        let value = match builtin {
            BuiltinFunction::Sin => Self::unary(arguments, Numeric::sin),
            BuiltinFunction::Cos => Self::unary(arguments, Numeric::cos),
            BuiltinFunction::Tan => Self::unary(arguments, Numeric::tan),
            BuiltinFunction::Cot => Self::unary(arguments, Numeric::cot),
            BuiltinFunction::Sec => Self::unary(arguments, Numeric::sec),
            BuiltinFunction::Csc => Self::unary(arguments, Numeric::csc),
            BuiltinFunction::Asin => Self::unary(arguments, Numeric::asin),
            BuiltinFunction::Acos => Self::unary(arguments, Numeric::acos),
            BuiltinFunction::Atan => Self::unary(arguments, Numeric::atan),
            BuiltinFunction::Acot => Self::unary(arguments, Numeric::acot),
            BuiltinFunction::Asec => Self::unary(arguments, Numeric::asec),
            BuiltinFunction::Acsc => Self::unary(arguments, Numeric::acsc),
            BuiltinFunction::Sinh => Self::unary(arguments, Numeric::sinh),
            BuiltinFunction::Cosh => Self::unary(arguments, Numeric::cosh),
            BuiltinFunction::Tanh => Self::unary(arguments, Numeric::tanh),
            BuiltinFunction::Coth => Self::unary(arguments, Numeric::coth),
            BuiltinFunction::Sech => Self::unary(arguments, Numeric::sech),
            BuiltinFunction::Csch => Self::unary(arguments, Numeric::csch),
            BuiltinFunction::Asinh => Self::unary(arguments, Numeric::asinh),
            BuiltinFunction::Acosh => Self::unary(arguments, Numeric::acosh),
            BuiltinFunction::Atanh => Self::unary(arguments, Numeric::atanh),
            BuiltinFunction::Acoth => Self::unary(arguments, Numeric::acoth),
            BuiltinFunction::Asech => Self::unary(arguments, Numeric::asech),
            BuiltinFunction::Acsch => Self::unary(arguments, Numeric::acsch),
            BuiltinFunction::Exp => Self::unary(arguments, Numeric::exp),
            BuiltinFunction::ExpM1 => Self::unary(arguments, Numeric::exp_m1),
            BuiltinFunction::ExpNeg => Self::unary(arguments, Numeric::exp_neg),
            BuiltinFunction::Ln => Self::unary(arguments, Numeric::ln),
            BuiltinFunction::Ln1p => Self::unary(arguments, Numeric::ln_1p),
            BuiltinFunction::Log10 => Self::unary(arguments, Numeric::log10),
            BuiltinFunction::Log2 => Self::unary(arguments, Numeric::log2),
            BuiltinFunction::Sqrt => Self::unary(arguments, Numeric::sqrt),
            BuiltinFunction::Cbrt => Self::unary(arguments, Numeric::cbrt),
            BuiltinFunction::Floor => Self::unary(arguments, Numeric::floor),
            BuiltinFunction::Ceil => Self::unary(arguments, Numeric::ceil),
            BuiltinFunction::Round => Self::unary(arguments, Numeric::round),
            BuiltinFunction::Trunc => Self::unary(arguments, Numeric::trunc),
            BuiltinFunction::Fract => Self::unary(arguments, Numeric::fract),
            BuiltinFunction::Negate => Self::unary(arguments, Numeric::negate),
            BuiltinFunction::Abs => Self::unary(arguments, Numeric::abs),
            BuiltinFunction::Signum => Self::unary(arguments, Numeric::signum),
            BuiltinFunction::Sinc => Self::unary(arguments, Numeric::sinc),
            BuiltinFunction::Erf => Self::unary(arguments, Numeric::erf),
            BuiltinFunction::Erfc => Self::unary(arguments, Numeric::erfc),
            BuiltinFunction::Gamma => Self::unary(arguments, Numeric::gamma),
            BuiltinFunction::Lgamma => Self::unary(arguments, Numeric::lgamma),
            BuiltinFunction::Digamma => Self::unary(arguments, Numeric::digamma),
            BuiltinFunction::Trigamma => Self::unary(arguments, Numeric::trigamma),
            BuiltinFunction::Tetragamma => Self::unary(arguments, Numeric::tetragamma),
            BuiltinFunction::Zeta => Self::unary(arguments, Numeric::zeta),
            BuiltinFunction::EllipticK => Self::unary(arguments, Numeric::elliptic_k),
            BuiltinFunction::EllipticE => Self::unary(arguments, Numeric::elliptic_e),
            BuiltinFunction::Polygamma => {
                Self::binary(arguments, |order, value| value.polygamma(order))
            }
            BuiltinFunction::BesselJ => {
                Self::binary(arguments, |order, value| value.bessel_j(order))
            }
            BuiltinFunction::BesselY => {
                Self::binary(arguments, |order, value| value.bessel_y(order))
            }
            BuiltinFunction::BesselI => {
                Self::binary(arguments, |order, value| value.bessel_i(order))
            }
            BuiltinFunction::BesselK => {
                Self::binary(arguments, |order, value| value.bessel_k(order))
            }
            BuiltinFunction::ZetaDeriv => {
                Self::binary(arguments, |order, value| value.zeta_deriv(order))
            }
            BuiltinFunction::Log => Self::binary(arguments, |base, value| value.log(base)),
            BuiltinFunction::LogBase => Self::binary(arguments, Numeric::log),
            BuiltinFunction::Hermite => Self::discrete_hermite(arguments),
            BuiltinFunction::LambertW => {
                Self::binary(arguments, |branch, value| value.lambert_w(branch))
            }
            BuiltinFunction::Beta => Self::binary(arguments, Numeric::beta),
            BuiltinFunction::Atan2 => Self::binary(arguments, Numeric::atan2),
            BuiltinFunction::AssocLegendre => {
                let [degree, order, value] = arguments else {
                    return None;
                };
                Some(value.0.assoc_legendre(&degree.0, &order.0))
            }
            BuiltinFunction::SphericalHarmonic => {
                let [degree, order, theta, phi] = arguments else {
                    return None;
                };
                Some(theta.0.spherical_harmonic(&degree.0, &order.0, &phi.0))
            }
        }?;
        value.is_finite().then_some(value)
    }

    fn unary(arguments: &[&Self], evaluate: fn(&Number) -> Number) -> Option<Number> {
        let [argument] = arguments else {
            return None;
        };
        Some(evaluate(&argument.0))
    }

    fn binary(arguments: &[&Self], evaluate: fn(&Number, &Number) -> Number) -> Option<Number> {
        let [left, right] = arguments else {
            return None;
        };
        Some(evaluate(&left.0, &right.0))
    }

    fn discrete_hermite(arguments: &[&Self]) -> Option<Number> {
        let [degree, argument] = arguments else {
            return None;
        };
        // Hermite's symbolic degree is discrete. Project its rounded real
        // component before calling the evaluator's continuous-degree API.
        let rounded = Number::from(degree.parts()?.re.round());
        if rounded < Number::from_int(i64::MIN) || rounded > Number::from_int(i64::MAX) {
            return None;
        }
        Some(argument.0.hermite(&Number::from_int(rounded.to_int())))
    }
}
