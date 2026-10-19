//! Function-domain proofs dispatched by static builtin identities.

use super::{BuiltinFunction, Expr, Facts, Property, Truth};

#[derive(Clone, Copy, PartialEq, Eq)]
enum FunctionDomain {
    Absolute,
    Exponential,
    SquareRoot,
    CubeRoot,
    Logarithm,
    FiniteImage(Property),
    RealImage(Property),
    Unknown,
}

impl Facts {
    pub fn function(
        identity: Option<BuiltinFunction>,
        arguments: &[Expr],
        get: impl Fn(&Expr) -> Self,
    ) -> Self {
        let mut facts = Self::default();
        let Some(builtin) = identity else {
            return facts;
        };
        let [argument] = arguments else { return facts };
        let input = get(argument);
        if input.get(Property::Finite) != Truth::True {
            return facts;
        }
        let domain = FunctionDomain::of(builtin);
        match domain {
            FunctionDomain::Absolute => {
                facts.prove(Property::Nonnegative);
                for property in [Property::Zero, Property::Nonzero] {
                    if input.get(property) == Truth::True {
                        facts.prove(property);
                    }
                }
            }
            FunctionDomain::Exponential => {
                facts.prove(Property::Nonzero);
                if input.get(Property::Real) == Truth::True {
                    facts.prove(Property::Positive);
                }
            }
            FunctionDomain::SquareRoot | FunctionDomain::CubeRoot => {
                facts.prove(Property::Finite);
                for property in [
                    Property::Zero,
                    Property::Nonzero,
                    Property::Positive,
                    Property::Nonnegative,
                ] {
                    if input.get(property) == Truth::True {
                        facts.prove(property);
                    }
                }
                if domain == FunctionDomain::SquareRoot
                    && input.get(Property::Negative) == Truth::True
                {
                    facts.refute(Property::Real);
                }
                if domain == FunctionDomain::CubeRoot && input.get(Property::Real) == Truth::True {
                    facts.prove(Property::Real);
                    if input.get(Property::Negative) == Truth::True {
                        facts.prove(Property::Negative);
                    }
                }
            }
            FunctionDomain::Logarithm => {
                if input.get(Property::Zero) == Truth::True {
                    facts.refute(Property::Finite);
                } else if input.get(Property::Nonzero) == Truth::True {
                    facts.prove(Property::Finite);
                    if input.get(Property::Positive) == Truth::True {
                        facts.prove(Property::Real);
                    } else if input.get(Property::Negative) == Truth::True {
                        facts.refute(Property::Real);
                    }
                }
            }
            FunctionDomain::FiniteImage(image) => {
                facts.prove(Property::Finite);
                if input.get(Property::Real) == Truth::True {
                    facts.prove(image);
                }
            }
            FunctionDomain::RealImage(image) => {
                if input.get(Property::Real) == Truth::True {
                    facts.prove(image);
                }
            }
            FunctionDomain::Unknown => {}
        }
        facts
    }
}

impl FunctionDomain {
    const fn of(builtin: BuiltinFunction) -> Self {
        match builtin {
            BuiltinFunction::Abs => Self::Absolute,
            BuiltinFunction::Exp | BuiltinFunction::ExpNeg => Self::Exponential,
            BuiltinFunction::Sqrt => Self::SquareRoot,
            BuiltinFunction::Cbrt => Self::CubeRoot,
            BuiltinFunction::Ln | BuiltinFunction::Log2 | BuiltinFunction::Log10 => Self::Logarithm,
            BuiltinFunction::Cosh => Self::FiniteImage(Property::Positive),
            BuiltinFunction::Sin
            | BuiltinFunction::Cos
            | BuiltinFunction::Sinh
            | BuiltinFunction::ExpM1
            | BuiltinFunction::Erf
            | BuiltinFunction::Erfc
            | BuiltinFunction::Sinc
            | BuiltinFunction::Asinh => Self::FiniteImage(Property::Real),
            BuiltinFunction::Floor
            | BuiltinFunction::Ceil
            | BuiltinFunction::Round
            | BuiltinFunction::Trunc
            | BuiltinFunction::Signum => Self::RealImage(Property::Integer),
            BuiltinFunction::Tanh | BuiltinFunction::Atan | BuiltinFunction::Fract => {
                Self::RealImage(Property::Real)
            }
            BuiltinFunction::Sech => Self::RealImage(Property::Positive),
            BuiltinFunction::Tan
            | BuiltinFunction::Cot
            | BuiltinFunction::Sec
            | BuiltinFunction::Csc
            | BuiltinFunction::Asin
            | BuiltinFunction::Acos
            | BuiltinFunction::Acot
            | BuiltinFunction::Asec
            | BuiltinFunction::Acsc
            | BuiltinFunction::Coth
            | BuiltinFunction::Csch
            | BuiltinFunction::Acosh
            | BuiltinFunction::Atanh
            | BuiltinFunction::Acoth
            | BuiltinFunction::Asech
            | BuiltinFunction::Acsch
            | BuiltinFunction::Ln1p
            | BuiltinFunction::Gamma
            | BuiltinFunction::Lgamma
            | BuiltinFunction::Digamma
            | BuiltinFunction::Trigamma
            | BuiltinFunction::Tetragamma
            | BuiltinFunction::Zeta
            | BuiltinFunction::EllipticK
            | BuiltinFunction::EllipticE
            | BuiltinFunction::Polygamma
            | BuiltinFunction::BesselJ
            | BuiltinFunction::BesselY
            | BuiltinFunction::BesselI
            | BuiltinFunction::BesselK
            | BuiltinFunction::ZetaDeriv
            | BuiltinFunction::Log
            | BuiltinFunction::Hermite
            | BuiltinFunction::LambertW
            | BuiltinFunction::Beta
            | BuiltinFunction::Atan2
            | BuiltinFunction::AssocLegendre
            | BuiltinFunction::SphericalHarmonic => Self::Unknown,
        }
    }
}
