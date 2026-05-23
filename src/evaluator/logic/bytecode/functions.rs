use crate::EPSILON;
use crate::math::{
    bessel_i, bessel_j, bessel_k, bessel_y, eval_assoc_legendre, eval_beta, eval_digamma,
    eval_elliptic_e, eval_elliptic_k, eval_erf, eval_erfc, eval_exp_polar, eval_gamma,
    eval_hermite, eval_lambert_w, eval_lgamma, eval_polygamma, eval_spherical_harmonic,
    eval_tetragamma, eval_trigamma, eval_zeta, eval_zeta_deriv,
};
use std::f64::consts::FRAC_PI_2;
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

            /// Folds a unary operation. Returns `None` if the operation is not unary
            /// or if folding is not supported for this variant.
            pub fn fold1(self, arg: f64) -> Option<f64> {
                if self.arity() != 1 { return None; }
                match self {
                    Self::Sin => Some(arg.sin()),
                    Self::Cos => Some(arg.cos()),
                    Self::Tan => Some(arg.tan()),
                    Self::Asin => Some(arg.asin()),
                    Self::Acos => Some(arg.acos()),
                    Self::Atan => Some(arg.atan()),
                    Self::Sinh => Some(arg.sinh()),
                    Self::Cosh => Some(arg.cosh()),
                    Self::Tanh => Some(arg.tanh()),
                    Self::Asinh => Some(arg.asinh()),
                    Self::Acosh => Some(arg.acosh()),
                    Self::Atanh => Some(arg.atanh()),
                    Self::Exp => Some(arg.exp()),
                    Self::Expm1 => Some(arg.exp_m1()),
                    Self::ExpNeg => Some((-arg).exp()),
                    Self::Ln => Some(arg.ln()),
                    Self::Log1p => Some(arg.ln_1p()),
                    Self::Sqrt => Some(arg.sqrt()),
                    Self::Cbrt => Some(arg.cbrt()),
                    Self::Abs => Some(arg.abs()),
                    Self::Floor => Some(arg.floor()),
                    Self::Ceil => Some(arg.ceil()),
                    Self::Round => Some(arg.round()),
                    Self::Signum => Some(arg.signum()),
                    Self::Cot => Some(1.0 / arg.tan()),
                    Self::Sec => Some(1.0 / arg.cos()),
                    Self::Csc => Some(1.0 / arg.sin()),
                    Self::Acot => Some(FRAC_PI_2 - arg.atan()),
                    Self::Asec => Some((1.0 / arg).acos()),
                    Self::Acsc => Some((1.0 / arg).asin()),
                    Self::Coth => Some(1.0 / arg.tanh()),
                    Self::Sech => Some(1.0 / arg.cosh()),
                    Self::Csch => Some(1.0 / arg.sinh()),
                    Self::Acoth => Some((1.0 / arg).atanh()),
                    Self::Acsch => Some((1.0 / arg).asinh()),
                    Self::Asech => Some((1.0 / arg).acosh()),
                    Self::Sinc => Some(if arg.abs() < EPSILON { 1.0 } else { arg.sin() / arg }),
                    Self::Erf => Some(eval_erf(arg)),
                    Self::Erfc => Some(eval_erfc(arg)),
                    Self::Gamma => Some(eval_gamma(arg)),
                    Self::Lgamma => Some(eval_lgamma(arg)),
                    Self::Digamma => Some(eval_digamma(arg)),
                    Self::Trigamma => Some(eval_trigamma(arg)),
                    Self::Tetragamma => Some(eval_tetragamma(arg)),
                    Self::LambertW => Some(eval_lambert_w(arg)),
                    Self::EllipticK => Some(eval_elliptic_k(arg)),
                    Self::EllipticE => Some(eval_elliptic_e(arg)),
                    Self::Zeta => Some(eval_zeta(arg)),
                    Self::ExpPolar => Some(eval_exp_polar(arg)),
                    _ => self.fold1_fallback(),
                }
            }

            #[inline]
            const fn fold1_fallback(self) -> Option<f64> {
                match self {
                    $( Self::$name => None, )*
                }
            }

            /// Folds a binary operation. Returns `None` if the operation is not binary
            /// or if folding is not supported for this variant.
            pub fn fold2(self, arg1: f64, arg2: f64) -> Option<f64> {
                if self.arity() != 2 { return None; }
                match self {
                    Self::Atan2 => Some(arg1.atan2(arg2)),
                    Self::Log => Some(arg2.log(arg1)),
                    Self::Beta => Some(eval_beta(arg1, arg2)),
                    op @ (Self::BesselJ
                    | Self::BesselY
                    | Self::BesselI
                    | Self::BesselK
                    | Self::Polygamma
                    | Self::ZetaDeriv
                    | Self::Hermite) => {
                        let v1_r = arg1.round();
                        (v1_r >= f64::from(i32::MIN) && v1_r <= f64::from(i32::MAX)).then(|| {
                            #[allow(
                                clippy::cast_possible_truncation,
                                reason = "Safe after manual range check"
                            )]
                            let n = v1_r as i32;
                            match op {
                                Self::BesselJ => bessel_j(n, arg2),
                                Self::BesselY => bessel_y(n, arg2),
                                Self::BesselI => bessel_i(n, arg2),
                                Self::BesselK => bessel_k(n, arg2),
                                Self::Polygamma => eval_polygamma(n, arg2),
                                Self::ZetaDeriv => eval_zeta_deriv(n, arg2),
                                Self::Hermite => eval_hermite(n, arg2),
                                _ => f64::NAN,
                            }
                        })
                    }
                    _ => self.fold2_fallback(),
                }
            }

            #[inline]
            const fn fold2_fallback(self) -> Option<f64> {
                match self {
                    $( Self::$name => None, )*
                }
            }

            /// Folds an N-ary operation.
            pub fn fold_n(self, args: &[f64]) -> Option<f64> {
                if args.len() != self.arity() { return None; }
                match args.len() {
                    1 => self.fold1(args[0]),
                    2 => self.fold2(args[0], args[1]),
                    3 => {
                        if let Self::AssocLegendre = self {
                            let lr = args[0].round();
                            let mr = args[1].round();
                            if lr >= f64::from(i32::MIN)
                                && lr <= f64::from(i32::MAX)
                                && mr >= f64::from(i32::MIN)
                                && mr <= f64::from(i32::MAX)
                            {
                                #[allow(
                                    clippy::cast_possible_truncation,
                                    reason = "Safe after manual range check"
                                )]
                                return Some(eval_assoc_legendre(lr as i32, mr as i32, args[2]));
                            }
                        }
                        None
                    }
                    4 => {
                        if let Self::SphericalHarmonic = self {
                            let lr = args[0].round();
                            let mr = args[1].round();
                            if lr >= f64::from(i32::MIN)
                                && lr <= f64::from(i32::MAX)
                                && mr >= f64::from(i32::MIN)
                                && mr <= f64::from(i32::MAX)
                            {
                                #[allow(
                                    clippy::cast_possible_truncation,
                                    reason = "Safe after manual range check"
                                )]
                                return Some(eval_spherical_harmonic(lr as i32, mr as i32, args[2], args[3]));
                            }
                        }
                        None
                    }
                    _ => None,
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
