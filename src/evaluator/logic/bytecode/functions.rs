use std::fmt::{Display, Formatter, Result as FmtResult};

/// Mathematical operations supported by the `BuiltinFun` instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FnOp {
    Sin,
    Cos,
    Tan,
    Cot,
    Sec,
    Csc,
    Asin,
    Acos,
    Atan,
    Acot,
    Asec,
    Acsc,
    Sinh,
    Cosh,
    Tanh,
    Coth,
    Sech,
    Csch,
    Asinh,
    Acosh,
    Atanh,
    Acoth,
    Acsch,
    Asech,
    Exp,
    Expm1,
    ExpNeg,
    Ln,
    Log1p,
    Sqrt,
    Cbrt,
    Abs,
    Signum,
    Floor,
    Ceil,
    Round,
    Erf,
    Erfc,
    Gamma,
    Lgamma,
    Digamma,
    Trigamma,
    Tetragamma,
    Sinc,
    LambertW,
    EllipticK,
    EllipticE,
    Zeta,
    ExpPolar,
    Atan2,
    Log,
    BesselJ,
    BesselY,
    BesselI,
    BesselK,
    Polygamma,
    Beta,
    ZetaDeriv,
    Hermite,
    AssocLegendre,
    SphericalHarmonic,
}

impl FnOp {
    /// Lowercase function name used for display/debug output.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sin => "sin",
            Self::Cos => "cos",
            Self::Tan => "tan",
            Self::Cot => "cot",
            Self::Sec => "sec",
            Self::Csc => "csc",
            Self::Asin => "asin",
            Self::Acos => "acos",
            Self::Atan => "atan",
            Self::Acot => "acot",
            Self::Asec => "asec",
            Self::Acsc => "acsc",
            Self::Sinh => "sinh",
            Self::Cosh => "cosh",
            Self::Tanh => "tanh",
            Self::Coth => "coth",
            Self::Sech => "sech",
            Self::Csch => "csch",
            Self::Asinh => "asinh",
            Self::Acosh => "acosh",
            Self::Atanh => "atanh",
            Self::Acoth => "acoth",
            Self::Acsch => "acsch",
            Self::Asech => "asech",
            Self::Exp => "exp",
            Self::Expm1 => "expm1",
            Self::ExpNeg => "expneg",
            Self::Ln => "ln",
            Self::Log1p => "log1p",
            Self::Sqrt => "sqrt",
            Self::Cbrt => "cbrt",
            Self::Abs => "abs",
            Self::Signum => "signum",
            Self::Floor => "floor",
            Self::Ceil => "ceil",
            Self::Round => "round",
            Self::Erf => "erf",
            Self::Erfc => "erfc",
            Self::Gamma => "gamma",
            Self::Lgamma => "lgamma",
            Self::Digamma => "digamma",
            Self::Trigamma => "trigamma",
            Self::Tetragamma => "tetragamma",
            Self::Sinc => "sinc",
            Self::LambertW => "lambertw",
            Self::EllipticK => "elliptick",
            Self::EllipticE => "elliptice",
            Self::Zeta => "zeta",
            Self::ExpPolar => "exp_polar",
            Self::Atan2 => "atan2",
            Self::Log => "log",
            Self::BesselJ => "besselj",
            Self::BesselY => "bessely",
            Self::BesselI => "besseli",
            Self::BesselK => "besselk",
            Self::Polygamma => "polygamma",
            Self::Beta => "beta",
            Self::ZetaDeriv => "zeta_deriv",
            Self::Hermite => "hermite",
            Self::AssocLegendre => "assoc_legendre",
            Self::SphericalHarmonic => "spherical_harmonic",
        }
    }

    /// Number of scalar arguments this builtin expects.
    pub const fn arity(self) -> usize {
        match self {
            Self::Atan2
            | Self::Log
            | Self::BesselJ
            | Self::BesselY
            | Self::BesselI
            | Self::BesselK
            | Self::Polygamma
            | Self::Beta
            | Self::ZetaDeriv
            | Self::Hermite => 2,
            Self::AssocLegendre => 3,
            Self::SphericalHarmonic => 4,
            _ => 1,
        }
    }
}

impl Display for FnOp {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter.write_str(self.as_str())
    }
}
