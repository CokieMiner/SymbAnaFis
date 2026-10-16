//! Static builtin identities, spellings, arities, and construction rules.

use super::{BinaryRule, FunctionDefinition, UnaryRule};

macro_rules! builtin_functions {
    ($($variant:ident => ($name:literal $(| $alias:literal)*, $arity:literal, $unary:expr, $binary:expr)),* $(,)?) => {
        /// A builtin identity independent of text interning and custom namespaces.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(u8)]
        pub enum BuiltinFunction {
            $(#[doc = concat!("The builtin `", $name, "` function.")] $variant,)*
        }

        impl BuiltinFunction {
            /// Resolve a builtin spelling without allocating or taking a lock.
            #[must_use]
            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $($name => Some(Self::$variant), $($alias => Some(Self::$variant),)*)*
                    _ => None,
                }
            }

            /// Borrow the builtin's static display name.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)*
                }
            }

            /// Dispatch directly to an immutable constructor definition.
            #[must_use]
            pub const fn definition(self) -> &'static FunctionDefinition {
                match self {
                    $(Self::$variant => {
                        const DEFINITION: FunctionDefinition =
                            FunctionDefinition::new(BuiltinFunction::$variant, $arity, $unary, $binary);
                        &DEFINITION
                    },)*
                }
            }
        }
    };
}

builtin_functions! {
    Sin => ("sin", 1, Some(UnaryRule::AtZero(0)), None),
    Cos => ("cos", 1, Some(UnaryRule::AtZero(1)), None),
    Tan => ("tan", 1, Some(UnaryRule::AtZero(0)), None),
    Cot => ("cot", 1, None, None),
    Sec => ("sec", 1, Some(UnaryRule::AtZero(1)), None),
    Csc => ("csc", 1, None, None),
    Asin => ("asin", 1, Some(UnaryRule::AtZero(0)), None),
    Acos => ("acos", 1, Some(UnaryRule::AtOneZero), None),
    Atan => ("atan", 1, Some(UnaryRule::AtZero(0)), None),
    Acot => ("acot", 1, None, None),
    Asec => ("asec", 1, Some(UnaryRule::AtOneZero), None),
    Acsc => ("acsc", 1, None, None),
    Sinh => ("sinh", 1, Some(UnaryRule::AtZero(0)), None),
    Cosh => ("cosh", 1, Some(UnaryRule::AtZero(1)), None),
    Tanh => ("tanh", 1, Some(UnaryRule::AtZero(0)), None),
    Coth => ("coth", 1, None, None),
    Sech => ("sech", 1, Some(UnaryRule::AtZero(1)), None),
    Csch => ("csch", 1, None, None),
    Asinh => ("asinh", 1, Some(UnaryRule::AtZero(0)), None),
    Acosh => ("acosh", 1, Some(UnaryRule::AtOneZero), None),
    Atanh => ("atanh", 1, Some(UnaryRule::AtZero(0)), None),
    Acoth => ("acoth", 1, None, None),
    Asech => ("asech", 1, Some(UnaryRule::AtOneZero), None),
    Acsch => ("acsch", 1, None, None),
    Exp => ("exp", 1, Some(UnaryRule::AtZero(1)), None),
    ExpM1 => ("exp_m1", 1, Some(UnaryRule::AtZero(0)), None),
    ExpNeg => ("exp_neg", 1, Some(UnaryRule::AtZero(1)), None),
    Ln => ("ln", 1, Some(UnaryRule::AtOneZero), None),
    Ln1p => ("ln_1p", 1, Some(UnaryRule::AtZero(0)), None),
    Log10 => ("log10", 1, Some(UnaryRule::Log(10)), None),
    Log2 => ("log2", 1, Some(UnaryRule::Log(2)), None),
    Sqrt => ("sqrt", 1, Some(UnaryRule::Sqrt), None),
    Cbrt => ("cbrt", 1, Some(UnaryRule::Cbrt), None),
    Floor => ("floor", 1, Some(UnaryRule::Identity), None),
    Ceil => ("ceil", 1, Some(UnaryRule::Identity), None),
    Round => ("round", 1, Some(UnaryRule::Identity), None),
    Trunc => ("trunc", 1, Some(UnaryRule::Identity), None),
    Fract => ("fract", 1, Some(UnaryRule::Zero), None),
    Negate => ("negate", 1, Some(UnaryRule::Negate), None),
    Abs => ("abs", 1, Some(UnaryRule::Abs), None),
    Signum => ("signum", 1, Some(UnaryRule::Sign), None),
    Sinc => ("sinc", 1, Some(UnaryRule::AtZero(1)), None),
    Erf => ("erf", 1, Some(UnaryRule::AtZero(0)), None),
    Erfc => ("erfc", 1, Some(UnaryRule::AtZero(1)), None),
    Gamma => ("gamma", 1, Some(UnaryRule::Gamma), None),
    Lgamma => ("lgamma", 1, Some(UnaryRule::Lgamma), None),
    Digamma => ("digamma", 1, None, None),
    Trigamma => ("trigamma", 1, None, None),
    Tetragamma => ("tetragamma", 1, None, None),
    Zeta => ("zeta", 1, None, None),
    EllipticK => ("elliptic_k", 1, None, None),
    EllipticE => ("elliptic_e", 1, None, None),
    Polygamma => ("polygamma", 2, None, None),
    BesselJ => ("besselj" | "bessel_j", 2, None, Some(BinaryRule::Bessel)),
    BesselY => ("bessely" | "bessel_y", 2, None, None),
    BesselI => ("besseli" | "bessel_i", 2, None, Some(BinaryRule::Bessel)),
    BesselK => ("besselk" | "bessel_k", 2, None, None),
    ZetaDeriv => ("zeta_deriv", 2, None, None),
    Log => ("log", 2, None, Some(BinaryRule::Log)),
    LogBase => ("log_base", 2, None, Some(BinaryRule::LogBase)),
    Hermite => ("hermite", 2, None, Some(BinaryRule::Hermite)),
    LambertW => ("lambertw" | "lambert_w", 2, None, Some(BinaryRule::LambertW)),
    Beta => ("beta", 2, None, Some(BinaryRule::Beta)),
    Atan2 => ("atan2", 2, None, Some(BinaryRule::Atan2)),
    AssocLegendre => ("assoc_legendre", 3, None, None),
    SphericalHarmonic => ("spherical_harmonic", 4, None, None),
}
