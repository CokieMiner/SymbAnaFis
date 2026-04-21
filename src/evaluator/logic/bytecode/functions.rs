use std::fmt::{Display, Formatter, Result as FmtResult};

macro_rules! define_functions {
    (
        $(
            $( #[doc = $doc:expr] )?
            $name:ident => ($arity:expr, $str:expr)
        ),* $(,)?
    ) => {
        /// Mathematical operations supported by the `BuiltinFun` instruction.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum FnOp {
            $(
                $( #[doc = $doc] )?
                $name,
            )*
        }

        impl FnOp {
            /// Lowercase function name used for display/debug output.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $( Self::$name => $str, )*
                }
            }

            /// Number of scalar arguments this builtin expects.
            pub const fn arity(self) -> usize {
                match self {
                    $( Self::$name => $arity, )*
                }
            }
        }
    };
}

define_functions! {
    // --- Basic Trigonometric ---
    Sin => (1, "sin"),
    Cos => (1, "cos"),
    Tan => (1, "tan"),
    Cot => (1, "cot"),
    Sec => (1, "sec"),
    Csc => (1, "csc"),

    // --- Inverse Trigonometric ---
    Asin => (1, "asin"),
    Acos => (1, "acos"),
    Atan => (1, "atan"),
    Acot => (1, "acot"),
    Asec => (1, "asec"),
    Acsc => (1, "acsc"),

    // --- Hyperbolic ---
    Sinh => (1, "sinh"),
    Cosh => (1, "cosh"),
    Tanh => (1, "tanh"),
    Coth => (1, "coth"),
    Sech => (1, "sech"),
    Csch => (1, "csch"),

    // --- Inverse Hyperbolic ---
    Asinh => (1, "asinh"),
    Acosh => (1, "acosh"),
    Atanh => (1, "atanh"),
    Acoth => (1, "acoth"),
    Acsch => (1, "acsch"),
    Asech => (1, "asech"),

    // --- Exponential & Logarithmic ---
    Exp => (1, "exp"),
    Expm1 => (1, "expm1"),
    ExpNeg => (1, "exp_neg"),
    Ln => (1, "ln"),
    Log1p => (1, "log1p"),

    // --- Powers & Roots ---
    Sqrt => (1, "sqrt"),
    Cbrt => (1, "cbrt"),

    // --- Basic Math ---
    Abs => (1, "abs"),
    Signum => (1, "signum"),
    Floor => (1, "floor"),
    Ceil => (1, "ceil"),
    Round => (1, "round"),

    // --- Special Functions (Unary) ---
    Erf => (1, "erf"),
    Erfc => (1, "erfc"),
    Gamma => (1, "gamma"),
    Lgamma => (1, "lgamma"),
    Digamma => (1, "digamma"),
    Trigamma => (1, "trigamma"),
    Tetragamma => (1, "tetragamma"),
    Sinc => (1, "sinc"),
    LambertW => (1, "lambert_w"),
    EllipticK => (1, "elliptic_k"),
    EllipticE => (1, "elliptic_e"),
    Zeta => (1, "zeta"),
    ExpPolar => (1, "exp_polar"),

    // --- Multi-Argument Functions ---
    Atan2 => (2, "atan2"),
    Log => (2, "log"),
    BesselJ => (2, "bessel_j"),
    BesselY => (2, "bessel_y"),
    BesselI => (2, "bessel_i"),
    BesselK => (2, "bessel_k"),
    Polygamma => (2, "polygamma"),
    Beta => (2, "beta"),
    ZetaDeriv => (2, "zeta_deriv"),
    Hermite => (2, "hermite"),
    AssocLegendre => (3, "assoc_legendre"),
    SphericalHarmonic => (4, "spherical_harmonic"),
}

impl Display for FnOp {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter.write_str(self.as_str())
    }
}
