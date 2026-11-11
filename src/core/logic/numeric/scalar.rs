//! Arity-typed numerical function kernels.

use num_anafis::Numeric;

use super::BuiltinFunction;

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

impl<Value: Numeric> Kernel<Value> {
    /// The signature and `Numeric` function of a builtin identity, in
    /// expression argument order.
    #[must_use]
    pub fn of(function: BuiltinFunction) -> Self {
        match function {
            BuiltinFunction::Sin => Self::Unary(Value::sin),
            BuiltinFunction::Cos => Self::Unary(Value::cos),
            BuiltinFunction::Tan => Self::Unary(Value::tan),
            BuiltinFunction::Cot => Self::Unary(Value::cot),
            BuiltinFunction::Sec => Self::Unary(Value::sec),
            BuiltinFunction::Csc => Self::Unary(Value::csc),
            BuiltinFunction::Asin => Self::Unary(Value::asin),
            BuiltinFunction::Acos => Self::Unary(Value::acos),
            BuiltinFunction::Atan => Self::Unary(Value::atan),
            BuiltinFunction::Acot => Self::Unary(Value::acot),
            BuiltinFunction::Asec => Self::Unary(Value::asec),
            BuiltinFunction::Acsc => Self::Unary(Value::acsc),
            BuiltinFunction::Sinh => Self::Unary(Value::sinh),
            BuiltinFunction::Cosh => Self::Unary(Value::cosh),
            BuiltinFunction::Tanh => Self::Unary(Value::tanh),
            BuiltinFunction::Coth => Self::Unary(Value::coth),
            BuiltinFunction::Sech => Self::Unary(Value::sech),
            BuiltinFunction::Csch => Self::Unary(Value::csch),
            BuiltinFunction::Asinh => Self::Unary(Value::asinh),
            BuiltinFunction::Acosh => Self::Unary(Value::acosh),
            BuiltinFunction::Atanh => Self::Unary(Value::atanh),
            BuiltinFunction::Acoth => Self::Unary(Value::acoth),
            BuiltinFunction::Asech => Self::Unary(Value::asech),
            BuiltinFunction::Acsch => Self::Unary(Value::acsch),
            BuiltinFunction::Exp => Self::Unary(Value::exp),
            BuiltinFunction::ExpM1 => Self::Unary(Value::exp_m1),
            BuiltinFunction::Ln => Self::Unary(Value::ln),
            BuiltinFunction::Ln1p => Self::Unary(Value::ln_1p),
            BuiltinFunction::Log10 => Self::Unary(Value::log10),
            BuiltinFunction::Log2 => Self::Unary(Value::log2),
            BuiltinFunction::Sqrt => Self::Unary(Value::sqrt),
            BuiltinFunction::Cbrt => Self::Unary(Value::cbrt),
            BuiltinFunction::Floor => Self::Unary(Value::floor),
            BuiltinFunction::Ceil => Self::Unary(Value::ceil),
            BuiltinFunction::Round => Self::Unary(Value::round),
            BuiltinFunction::Trunc => Self::Unary(Value::trunc),
            BuiltinFunction::Fract => Self::Unary(Value::fract),
            BuiltinFunction::Abs => Self::Unary(Value::abs),
            BuiltinFunction::Signum => Self::Unary(Value::signum),
            BuiltinFunction::Re => Self::Unary(Value::re),
            BuiltinFunction::Im => Self::Unary(Value::im),
            BuiltinFunction::Conj => Self::Unary(Value::conj),
            BuiltinFunction::Heaviside => Self::Unary(Value::heaviside),
            BuiltinFunction::Sinc => Self::Unary(Value::sinc),
            BuiltinFunction::Erf => Self::Unary(Value::erf),
            BuiltinFunction::Erfc => Self::Unary(Value::erfc),
            BuiltinFunction::Gamma => Self::Unary(Value::gamma),
            BuiltinFunction::Lgamma => Self::Unary(Value::lgamma),
            BuiltinFunction::Digamma => Self::Unary(Value::digamma),
            BuiltinFunction::Trigamma => Self::Unary(Value::trigamma),
            BuiltinFunction::Tetragamma => Self::Unary(Value::tetragamma),
            BuiltinFunction::Zeta => Self::Unary(Value::zeta),
            BuiltinFunction::EllipticK => Self::Unary(Value::elliptic_k),
            BuiltinFunction::EllipticE => Self::Unary(Value::elliptic_e),
            BuiltinFunction::Polygamma => Self::Binary(|order, value| value.polygamma(order)),
            BuiltinFunction::BesselJ => Self::Binary(|order, value| value.bessel_j(order)),
            BuiltinFunction::BesselY => Self::Binary(|order, value| value.bessel_y(order)),
            BuiltinFunction::BesselI => Self::Binary(|order, value| value.bessel_i(order)),
            BuiltinFunction::BesselK => Self::Binary(|order, value| value.bessel_k(order)),
            BuiltinFunction::ZetaDeriv => Self::Binary(|order, value| value.zeta_deriv(order)),
            BuiltinFunction::Log => Self::Binary(|base, value| value.log(base)),
            BuiltinFunction::Hermite => Self::Binary(|degree, value| value.hermite(degree)),
            BuiltinFunction::LambertW => Self::Binary(|branch, value| value.lambert_w(branch)),
            BuiltinFunction::Beta => Self::Binary(Value::beta),
            BuiltinFunction::Atan2 => Self::Binary(Value::atan2),
            BuiltinFunction::Hypot => Self::Binary(Value::hypot),
            BuiltinFunction::Max => Self::Binary(Value::max),
            BuiltinFunction::Min => Self::Binary(Value::min),
            BuiltinFunction::Modulo => Self::Binary(Value::modulo),
            BuiltinFunction::Rem => Self::Binary(Value::rem),
            BuiltinFunction::AssocLegendre => {
                Self::Ternary(|degree, order, value| value.assoc_legendre(degree, order))
            }
            BuiltinFunction::Clamp => Self::Ternary(Value::clamp),
        }
    }
}
