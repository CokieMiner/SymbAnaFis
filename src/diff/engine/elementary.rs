//! Elementary slopes on regular principal branches.

use core::ops::{Add, Div, Mul, Neg, Sub};

use alloc::vec;

use crate::core::{BuiltinFunction, Expr};

use super::Differentiation;

impl Differentiation<'_> {
    /// Construct the local slope \(f'(u)\) without differentiating `u` again.
    /// Reciprocal inverse functions retain their composed square-root branches.
    pub fn elementary(function: BuiltinFunction, argument: &Expr) -> Option<Expr> {
        let slope = match function {
            BuiltinFunction::Sin => argument.cos(),
            BuiltinFunction::Cos => Neg::neg(argument.sin()),
            BuiltinFunction::Tan => argument.sec().pow(2),
            BuiltinFunction::Cot => Neg::neg(argument.csc().pow(2)),
            BuiltinFunction::Sec => Mul::mul(argument.sec(), argument.tan()),
            BuiltinFunction::Csc => Neg::neg(Mul::mul(argument.csc(), argument.cot())),
            BuiltinFunction::Asin => Div::div(1, Sub::sub(1, argument.pow(2)).sqrt()),
            BuiltinFunction::Acos => Div::div(-1, Sub::sub(1, argument.pow(2)).sqrt()),
            BuiltinFunction::Atan => Div::div(1, Add::add(1, argument.pow(2))),
            BuiltinFunction::Acot => Div::div(-1, Add::add(1, argument.pow(2))),
            BuiltinFunction::Asec | BuiltinFunction::Acsc => {
                let inverse_square = Div::div(1, argument.pow(2));
                let denominator = Mul::mul(argument.pow(2), Sub::sub(1, inverse_square).sqrt());
                Div::div(
                    if function == BuiltinFunction::Asec {
                        1
                    } else {
                        -1
                    },
                    denominator,
                )
            }
            BuiltinFunction::Sinh => argument.cosh(),
            BuiltinFunction::Cosh => argument.sinh(),
            BuiltinFunction::Tanh => argument.sech().pow(2),
            BuiltinFunction::Coth => Neg::neg(argument.csch().pow(2)),
            BuiltinFunction::Sech => Neg::neg(Mul::mul(argument.sech(), argument.tanh())),
            BuiltinFunction::Csch => Neg::neg(Mul::mul(argument.csch(), argument.coth())),
            BuiltinFunction::Asinh => Div::div(1, Add::add(1, argument.pow(2)).sqrt()),
            BuiltinFunction::Acosh => Div::div(
                1,
                Mul::mul(Sub::sub(argument, 1).sqrt(), Add::add(argument, 1).sqrt()),
            ),
            BuiltinFunction::Atanh | BuiltinFunction::Acoth => {
                Div::div(1, Sub::sub(1, argument.pow(2)))
            }
            BuiltinFunction::Asech => {
                let reciprocal = Div::div(1, argument);
                Div::div(
                    -1,
                    Expr::product(vec![
                        argument.pow(2),
                        Sub::sub(&reciprocal, 1).sqrt(),
                        Add::add(reciprocal, 1).sqrt(),
                    ]),
                )
            }
            BuiltinFunction::Acsch => Div::div(
                -1,
                Mul::mul(
                    argument.pow(2),
                    Add::add(1, Div::div(1, argument.pow(2))).sqrt(),
                ),
            ),
            BuiltinFunction::Exp | BuiltinFunction::ExpM1 => argument.exp(),
            BuiltinFunction::ExpNeg => Neg::neg(argument.exp_neg()),
            BuiltinFunction::Ln => Div::div(1, argument),
            BuiltinFunction::Ln1p => Div::div(1, Add::add(argument, 1)),
            BuiltinFunction::Sqrt => Div::div(1, Mul::mul(2, argument.sqrt())),
            BuiltinFunction::Cbrt => Div::div(1, Mul::mul(3, argument.cbrt().pow(2))),
            BuiltinFunction::Floor
            | BuiltinFunction::Log2
            | BuiltinFunction::Log10
            | BuiltinFunction::Ceil
            | BuiltinFunction::Round
            | BuiltinFunction::Trunc
            | BuiltinFunction::Fract
            | BuiltinFunction::Abs
            | BuiltinFunction::Signum
            | BuiltinFunction::Sinc
            | BuiltinFunction::Erf
            | BuiltinFunction::Erfc
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
            | BuiltinFunction::SphericalHarmonic => return None,
        };
        Some(slope)
    }
}
