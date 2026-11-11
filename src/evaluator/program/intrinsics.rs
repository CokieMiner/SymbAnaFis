//! Hardware-inlineable builtin classification shared by the VM, WASM, and native backends.

use crate::core::BuiltinFunction;

/// Unary builtins with direct hardware or WebAssembly instructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InlineUnary {
    Sqrt,
    Abs,
    Floor,
    Ceil,
    Trunc,
    Round,
}

/// Binary builtins with direct hardware or WebAssembly IEEE 754-2019 instructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InlineBinary {
    Max,
    Min,
}

impl InlineBinary {
    /// Classify a [`BuiltinFunction`] if it admits an inline hardware/WASM binary instruction.
    #[must_use]
    pub const fn of(builtin: BuiltinFunction) -> Option<Self> {
        if matches!(builtin, BuiltinFunction::Max) {
            Some(Self::Max)
        } else if matches!(builtin, BuiltinFunction::Min) {
            Some(Self::Min)
        } else {
            None
        }
    }
}

impl InlineUnary {
    /// Classify a [`BuiltinFunction`] if it admits an inline hardware/WASM instruction.
    #[must_use]
    pub const fn of(builtin: BuiltinFunction) -> Option<Self> {
        match builtin {
            BuiltinFunction::Sqrt => Some(Self::Sqrt),
            BuiltinFunction::Abs => Some(Self::Abs),
            BuiltinFunction::Floor => Some(Self::Floor),
            BuiltinFunction::Ceil => Some(Self::Ceil),
            BuiltinFunction::Trunc => Some(Self::Trunc),
            BuiltinFunction::Round => Some(Self::Round),
            BuiltinFunction::Sin
            | BuiltinFunction::Cos
            | BuiltinFunction::Tan
            | BuiltinFunction::Cot
            | BuiltinFunction::Sec
            | BuiltinFunction::Csc
            | BuiltinFunction::Asin
            | BuiltinFunction::Acos
            | BuiltinFunction::Atan
            | BuiltinFunction::Acot
            | BuiltinFunction::Asec
            | BuiltinFunction::Acsc
            | BuiltinFunction::Sinh
            | BuiltinFunction::Cosh
            | BuiltinFunction::Tanh
            | BuiltinFunction::Coth
            | BuiltinFunction::Sech
            | BuiltinFunction::Csch
            | BuiltinFunction::Asinh
            | BuiltinFunction::Acosh
            | BuiltinFunction::Atanh
            | BuiltinFunction::Acoth
            | BuiltinFunction::Asech
            | BuiltinFunction::Acsch
            | BuiltinFunction::Exp
            | BuiltinFunction::ExpM1
            | BuiltinFunction::Ln
            | BuiltinFunction::Ln1p
            | BuiltinFunction::Log10
            | BuiltinFunction::Log2
            | BuiltinFunction::Cbrt
            | BuiltinFunction::Fract
            | BuiltinFunction::Signum
            | BuiltinFunction::Re
            | BuiltinFunction::Im
            | BuiltinFunction::Conj
            | BuiltinFunction::Heaviside
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
            | BuiltinFunction::Hypot
            | BuiltinFunction::Max
            | BuiltinFunction::Min
            | BuiltinFunction::Modulo
            | BuiltinFunction::Rem
            | BuiltinFunction::AssocLegendre
            | BuiltinFunction::Clamp => None,
        }
    }
}
