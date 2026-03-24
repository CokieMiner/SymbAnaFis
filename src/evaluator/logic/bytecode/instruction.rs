//! Bytecode instruction set for the register-based expression evaluator.
use std::fmt;

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

impl fmt::Display for FnOp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Bytecode instruction for the register-based expression evaluator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Instruction {
    /// Load constant from constant pool into dest
    LoadConst {
        dest: u32,
        const_idx: u32,
    },
    /// Copy register value
    Copy {
        dest: u32,
        src: u32,
    },

    Add {
        dest: u32,
        a: u32,
        b: u32,
    },
    Mul {
        dest: u32,
        a: u32,
        b: u32,
    },
    /// dest = sum(registers[`start_idx..start_idx + count`])
    AddN {
        dest: u32,
        start_idx: u32,
        count: u32,
    },
    /// dest = product(registers[`start_idx..start_idx + count`])
    MulN {
        dest: u32,
        start_idx: u32,
        count: u32,
    },
    Sub {
        dest: u32,
        a: u32,
        b: u32,
    },
    AddConst {
        dest: u32,
        src: u32,
        const_idx: u32,
    },
    MulConst {
        dest: u32,
        src: u32,
        const_idx: u32,
    },
    SubConst {
        dest: u32,
        src: u32,
        const_idx: u32,
    },
    ConstSub {
        dest: u32,
        src: u32,
        const_idx: u32,
    },
    DivConst {
        dest: u32,
        src: u32,
        const_idx: u32,
    },
    ConstDiv {
        dest: u32,
        src: u32,
        const_idx: u32,
    },
    Div {
        dest: u32,
        num: u32,
        den: u32,
    },
    Pow {
        dest: u32,
        base: u32,
        exp: u32,
    },

    Neg {
        dest: u32,
        src: u32,
    },

    Builtin1 {
        dest: u32,
        op: FnOp,
        arg: u32,
    },
    Builtin2 {
        dest: u32,
        op: FnOp,
        arg1: u32,
        arg2: u32,
    },
    Builtin3 {
        dest: u32,
        op: FnOp,
        start_idx: u32,
    },
    Builtin4 {
        dest: u32,
        op: FnOp,
        start_idx: u32,
    },

    Square {
        dest: u32,
        src: u32,
    },
    Cube {
        dest: u32,
        src: u32,
    },
    Pow4 {
        dest: u32,
        src: u32,
    },
    Pow3_2 {
        dest: u32,
        src: u32,
    },
    InvPow3_2 {
        dest: u32,
        src: u32,
    },
    InvSqrt {
        dest: u32,
        src: u32,
    },
    InvSquare {
        dest: u32,
        src: u32,
    },
    InvCube {
        dest: u32,
        src: u32,
    },
    Recip {
        dest: u32,
        src: u32,
    },
    Powi {
        dest: u32,
        src: u32,
        n: i32,
    },

    MulAdd {
        dest: u32,
        a: u32,
        b: u32,
        c: u32,
    },
    MulAddConst {
        dest: u32,
        a: u32,
        b: u32,
        const_idx: u32,
    },
    MulSub {
        dest: u32,
        a: u32,
        b: u32,
        c: u32,
    },
    NegMul {
        dest: u32,
        a: u32,
        b: u32,
    },
    NegMulConst {
        dest: u32,
        src: u32,
        const_idx: u32,
    },
    MulSubConst {
        dest: u32,
        a: u32,
        b: u32,
        const_idx: u32,
    },
    NegMulAdd {
        dest: u32,
        a: u32,
        b: u32,
        c: u32,
    },
    NegMulAddConst {
        dest: u32,
        a: u32,
        b: u32,
        const_idx: u32,
    },

    RecipExpm1 {
        dest: u32,
        src: u32,
    },
    ExpSqr {
        dest: u32,
        src: u32,
    },
    ExpSqrNeg {
        dest: u32,
        src: u32,
    },
}

impl Instruction {
    /// Arg-pool slice used by this instruction, if any.
    pub(crate) const fn arg_pool_range(&self) -> Option<(u32, u32)> {
        match *self {
            Self::AddN {
                start_idx, count, ..
            }
            | Self::MulN {
                start_idx, count, ..
            } => Some((start_idx, count)),
            Self::Builtin3 { start_idx, .. } => Some((start_idx, 3)),
            Self::Builtin4 { start_idx, .. } => Some((start_idx, 4)),
            _ => None,
        }
    }

    /// Execute a closure for every register index read from the arg-pool.
    pub(crate) fn for_each_pooled_reg(&self, arg_pool: &[u32], mut callback: impl FnMut(u32)) {
        if let Some((start_idx, count)) = self.arg_pool_range() {
            for offset in 0..count {
                callback(arg_pool[(start_idx + offset) as usize]);
            }
        }
    }

    /// Map every register index read from the arg-pool.
    pub(crate) fn map_pooled_regs(&self, arg_pool: &mut [u32], mut mapper: impl FnMut(u32) -> u32) {
        if let Some((start_idx, count)) = self.arg_pool_range() {
            for offset in 0..count {
                let reg = &mut arg_pool[(start_idx + offset) as usize];
                *reg = mapper(*reg);
            }
        }
    }

    /// Get the destination register for this instruction.
    /// Every instruction in this architecture produces a single result.
    #[allow(
        clippy::too_many_lines,
        reason = "Exhaustive match on all instruction variants"
    )]
    pub(crate) const fn dest_reg(&self) -> u32 {
        match *self {
            Self::LoadConst { dest, .. }
            | Self::Copy { dest, .. }
            | Self::Add { dest, .. }
            | Self::Mul { dest, .. }
            | Self::AddN { dest, .. }
            | Self::MulN { dest, .. }
            | Self::Sub { dest, .. }
            | Self::AddConst { dest, .. }
            | Self::MulConst { dest, .. }
            | Self::SubConst { dest, .. }
            | Self::ConstSub { dest, .. }
            | Self::DivConst { dest, .. }
            | Self::ConstDiv { dest, .. }
            | Self::Div { dest, .. }
            | Self::Pow { dest, .. }
            | Self::Neg { dest, .. }
            | Self::Builtin1 { dest, .. }
            | Self::Builtin2 { dest, .. }
            | Self::Builtin3 { dest, .. }
            | Self::Builtin4 { dest, .. }
            | Self::Square { dest, .. }
            | Self::Cube { dest, .. }
            | Self::Pow4 { dest, .. }
            | Self::Pow3_2 { dest, .. }
            | Self::InvPow3_2 { dest, .. }
            | Self::InvSqrt { dest, .. }
            | Self::InvSquare { dest, .. }
            | Self::InvCube { dest, .. }
            | Self::Recip { dest, .. }
            | Self::Powi { dest, .. }
            | Self::MulAdd { dest, .. }
            | Self::MulAddConst { dest, .. }
            | Self::MulSub { dest, .. }
            | Self::MulSubConst { dest, .. }
            | Self::NegMulAdd { dest, .. }
            | Self::NegMulAddConst { dest, .. }
            | Self::NegMul { dest, .. }
            | Self::NegMulConst { dest, .. }
            | Self::RecipExpm1 { dest, .. }
            | Self::ExpSqr { dest, .. }
            | Self::ExpSqrNeg { dest, .. } => dest,
        }
    }

    /// Execute a closure for every register index (dest or src) in this instruction.
    ///
    /// NOTE: For N-ary instructions (`AddN`, `MulN`, `Builtin3`, `Builtin4`), only `dest` is
    /// visited here. Register indices in the `arg_pool` must be handled separately by the caller.
    #[allow(
        clippy::too_many_lines,
        reason = "Exhaustive match on all instruction variants"
    )]
    pub(crate) fn for_each_reg(&self, mut callback: impl FnMut(u32)) {
        match *self {
            Self::LoadConst { dest, .. }
            | Self::AddN { dest, .. }
            | Self::MulN { dest, .. }
            | Self::Builtin3 { dest, .. }
            | Self::Builtin4 { dest, .. } => callback(dest),
            Self::Copy { dest, src }
            | Self::Neg { dest, src }
            | Self::Square { dest, src }
            | Self::Cube { dest, src }
            | Self::Pow4 { dest, src }
            | Self::Pow3_2 { dest, src }
            | Self::InvPow3_2 { dest, src }
            | Self::InvSqrt { dest, src }
            | Self::InvSquare { dest, src }
            | Self::InvCube { dest, src }
            | Self::Recip { dest, src }
            | Self::Powi { dest, src, .. }
            | Self::RecipExpm1 { dest, src }
            | Self::ExpSqr { dest, src }
            | Self::ExpSqrNeg { dest, src }
            | Self::AddConst { dest, src, .. }
            | Self::MulConst { dest, src, .. }
            | Self::SubConst { dest, src, .. }
            | Self::ConstSub { dest, src, .. }
            | Self::DivConst { dest, src, .. }
            | Self::NegMulConst { dest, src, .. }
            | Self::ConstDiv { dest, src, .. } => {
                callback(dest);
                callback(src);
            }
            Self::Add { dest, a, b }
            | Self::Mul { dest, a, b }
            | Self::Sub { dest, a, b }
            | Self::Div {
                dest,
                num: a,
                den: b,
            }
            | Self::Pow {
                dest,
                base: a,
                exp: b,
            }
            | Self::NegMul { dest, a, b }
            | Self::MulAddConst { dest, a, b, .. }
            | Self::MulSubConst { dest, a, b, .. }
            | Self::NegMulAddConst { dest, a, b, .. } => {
                callback(dest);
                callback(a);
                callback(b);
            }
            Self::Builtin1 { dest, arg, .. } => {
                callback(dest);
                callback(arg);
            }
            Self::Builtin2 {
                dest, arg1, arg2, ..
            } => {
                callback(dest);
                callback(arg1);
                callback(arg2);
            }
            Self::MulAdd { dest, a, b, c }
            | Self::MulSub { dest, a, b, c }
            | Self::NegMulAdd { dest, a, b, c } => {
                callback(dest);
                callback(a);
                callback(b);
                callback(c);
            }
        }
    }

    /// Execute a closure for every register index being READ in this instruction.
    ///
    /// NOTE: For N-ary instructions (`AddN`, `MulN`, `Builtin3`, `Builtin4`), reads from
    /// the `arg_pool` must be handled separately by the caller.
    pub(crate) fn for_each_read(&self, mut callback: impl FnMut(u32)) {
        match *self {
            Self::LoadConst { .. }
            | Self::AddN { .. }
            | Self::MulN { .. }
            | Self::Builtin3 { .. }
            | Self::Builtin4 { .. } => {}
            Self::Copy { src, .. }
            | Self::Neg { src, .. }
            | Self::Square { src, .. }
            | Self::Cube { src, .. }
            | Self::Pow4 { src, .. }
            | Self::Pow3_2 { src, .. }
            | Self::InvPow3_2 { src, .. }
            | Self::InvSqrt { src, .. }
            | Self::InvSquare { src, .. }
            | Self::InvCube { src, .. }
            | Self::Recip { src, .. }
            | Self::Powi { src, .. }
            | Self::RecipExpm1 { src, .. }
            | Self::ExpSqr { src, .. }
            | Self::ExpSqrNeg { src, .. }
            | Self::AddConst { src, .. }
            | Self::MulConst { src, .. }
            | Self::SubConst { src, .. }
            | Self::ConstSub { src, .. }
            | Self::DivConst { src, .. }
            | Self::NegMulConst { src, .. }
            | Self::ConstDiv { src, .. } => {
                callback(src);
            }
            Self::Add { a, b, .. }
            | Self::Mul { a, b, .. }
            | Self::Sub { a, b, .. }
            | Self::Div { num: a, den: b, .. }
            | Self::Pow {
                base: a, exp: b, ..
            }
            | Self::NegMul { a, b, .. }
            | Self::MulAddConst { a, b, .. }
            | Self::MulSubConst { a, b, .. }
            | Self::NegMulAddConst { a, b, .. } => {
                callback(a);
                callback(b);
            }
            Self::Builtin1 { arg, .. } => {
                callback(arg);
            }
            Self::Builtin2 { arg1, arg2, .. } => {
                callback(arg1);
                callback(arg2);
            }
            Self::MulAdd { a, b, c, .. }
            | Self::MulSub { a, b, c, .. }
            | Self::NegMulAdd { a, b, c, .. } => {
                callback(a);
                callback(b);
                callback(c);
            }
        }
    }

    /// Map all source register indices (reads) using the provided function.
    ///
    /// NOTE: For N-ary instructions (`AddN`, `MulN`, `Builtin3`, `Builtin4`), reads from
    /// the `arg_pool` must be handled separately by the caller.
    pub(crate) fn map_reads(&mut self, mut mapper: impl FnMut(u32) -> u32) {
        match self {
            Self::LoadConst { .. }
            | Self::AddN { .. }
            | Self::MulN { .. }
            | Self::Builtin3 { .. }
            | Self::Builtin4 { .. } => {}
            Self::Copy { src, .. }
            | Self::Neg { src, .. }
            | Self::Square { src, .. }
            | Self::Cube { src, .. }
            | Self::Pow4 { src, .. }
            | Self::Pow3_2 { src, .. }
            | Self::InvPow3_2 { src, .. }
            | Self::InvSqrt { src, .. }
            | Self::InvSquare { src, .. }
            | Self::InvCube { src, .. }
            | Self::Recip { src, .. }
            | Self::Powi { src, .. }
            | Self::RecipExpm1 { src, .. }
            | Self::ExpSqr { src, .. }
            | Self::ExpSqrNeg { src, .. }
            | Self::AddConst { src, .. }
            | Self::MulConst { src, .. }
            | Self::SubConst { src, .. }
            | Self::ConstSub { src, .. }
            | Self::DivConst { src, .. }
            | Self::NegMulConst { src, .. }
            | Self::ConstDiv { src, .. } => {
                *src = mapper(*src);
            }
            Self::Add { a, b, .. }
            | Self::Mul { a, b, .. }
            | Self::Sub { a, b, .. }
            | Self::Div { num: a, den: b, .. }
            | Self::Pow {
                base: a, exp: b, ..
            }
            | Self::NegMul { a, b, .. }
            | Self::MulAddConst { a, b, .. }
            | Self::MulSubConst { a, b, .. }
            | Self::NegMulAddConst { a, b, .. } => {
                *a = mapper(*a);
                *b = mapper(*b);
            }
            Self::Builtin1 { arg, .. } => {
                *arg = mapper(*arg);
            }
            Self::Builtin2 { arg1, arg2, .. } => {
                *arg1 = mapper(*arg1);
                *arg2 = mapper(*arg2);
            }
            Self::MulAdd { a, b, c, .. }
            | Self::MulSub { a, b, c, .. }
            | Self::NegMulAdd { a, b, c, .. } => {
                *a = mapper(*a);
                *b = mapper(*b);
                *c = mapper(*c);
            }
        }
    }

    /// Map all register indices in this instruction using the provided function.
    ///
    /// NOTE: For N-ary instructions (`AddN`, `MulN`, `Builtin3`, `Builtin4`), registers in
    /// the `arg_pool` must be handled separately by the caller.
    pub(crate) fn map_regs(&mut self, mut mapper: impl FnMut(u32) -> u32) {
        match self {
            Self::LoadConst { dest, .. }
            | Self::AddN { dest, .. }
            | Self::MulN { dest, .. }
            | Self::Builtin3 { dest, .. }
            | Self::Builtin4 { dest, .. } => {
                *dest = mapper(*dest);
            }
            Self::Copy { dest, src }
            | Self::Neg { dest, src }
            | Self::Square { dest, src }
            | Self::Cube { dest, src }
            | Self::Pow4 { dest, src }
            | Self::Pow3_2 { dest, src }
            | Self::InvPow3_2 { dest, src }
            | Self::InvSqrt { dest, src }
            | Self::InvSquare { dest, src }
            | Self::InvCube { dest, src }
            | Self::Recip { dest, src }
            | Self::Powi { dest, src, .. }
            | Self::RecipExpm1 { dest, src }
            | Self::ExpSqr { dest, src }
            | Self::ExpSqrNeg { dest, src }
            | Self::AddConst { dest, src, .. }
            | Self::MulConst { dest, src, .. }
            | Self::SubConst { dest, src, .. }
            | Self::ConstSub { dest, src, .. }
            | Self::DivConst { dest, src, .. }
            | Self::NegMulConst { dest, src, .. }
            | Self::ConstDiv { dest, src, .. } => {
                *dest = mapper(*dest);
                *src = mapper(*src);
            }
            Self::Add { dest, a, b }
            | Self::Mul { dest, a, b }
            | Self::Sub { dest, a, b }
            | Self::Div {
                dest,
                num: a,
                den: b,
            }
            | Self::Pow {
                dest,
                base: a,
                exp: b,
            }
            | Self::NegMul { dest, a, b }
            | Self::MulAddConst { dest, a, b, .. }
            | Self::MulSubConst { dest, a, b, .. }
            | Self::NegMulAddConst { dest, a, b, .. } => {
                *dest = mapper(*dest);
                *a = mapper(*a);
                *b = mapper(*b);
            }
            Self::Builtin1 { dest, arg, .. } => {
                *dest = mapper(*dest);
                *arg = mapper(*arg);
            }
            Self::Builtin2 {
                dest, arg1, arg2, ..
            } => {
                *dest = mapper(*dest);
                *arg1 = mapper(*arg1);
                *arg2 = mapper(*arg2);
            }
            Self::MulAdd { dest, a, b, c }
            | Self::MulSub { dest, a, b, c }
            | Self::NegMulAdd { dest, a, b, c } => {
                *dest = mapper(*dest);
                *a = mapper(*a);
                *b = mapper(*b);
                *c = mapper(*c);
            }
        }
    }
}

impl fmt::Display for Instruction {
    #[allow(
        clippy::too_many_lines,
        reason = "Exhaustive formatting of all instruction variants"
    )]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::LoadConst { dest, const_idx } => write!(formatter, "R{dest} = C{const_idx}"),
            Self::Copy { dest, src } => write!(formatter, "R{dest} = R{src}"),
            Self::Add { dest, a, b } => write!(formatter, "R{dest} = R{a} + R{b}"),
            Self::Mul { dest, a, b } => write!(formatter, "R{dest} = R{a} * R{b}"),
            Self::AddN {
                dest,
                start_idx,
                count,
            } => {
                write!(
                    formatter,
                    "R{dest} = sum(pool[{start_idx}..{}])",
                    start_idx + count
                )
            }
            Self::MulN {
                dest,
                start_idx,
                count,
            } => {
                write!(
                    formatter,
                    "R{dest} = prod(pool[{start_idx}..{}])",
                    start_idx + count
                )
            }
            Self::Sub { dest, a, b } => write!(formatter, "R{dest} = R{a} - R{b}"),
            Self::AddConst {
                dest,
                src,
                const_idx,
            } => write!(formatter, "R{dest} = R{src} + C{const_idx}"),
            Self::MulConst {
                dest,
                src,
                const_idx,
            } => write!(formatter, "R{dest} = R{src} * C{const_idx}"),
            Self::SubConst {
                dest,
                src,
                const_idx,
            } => write!(formatter, "R{dest} = R{src} - C{const_idx}"),
            Self::ConstSub {
                dest,
                src,
                const_idx,
            } => write!(formatter, "R{dest} = C{const_idx} - R{src}"),
            Self::DivConst {
                dest,
                src,
                const_idx,
            } => write!(formatter, "R{dest} = R{src} / C{const_idx}"),
            Self::ConstDiv {
                dest,
                src,
                const_idx,
            } => write!(formatter, "R{dest} = C{const_idx} / R{src}"),
            Self::Div { dest, num, den } => write!(formatter, "R{dest} = R{num} / R{den}"),
            Self::Pow { dest, base, exp } => write!(formatter, "R{dest} = R{base} ^ R{exp}"),
            Self::Neg { dest, src } => write!(formatter, "R{dest} = -R{src}"),
            Self::Builtin1 { dest, op, arg } => {
                write!(formatter, "R{dest} = {op}(R{arg})")
            }
            Self::Builtin2 {
                dest,
                op,
                arg1,
                arg2,
            } => write!(formatter, "R{dest} = {op}(R{arg1}, R{arg2})"),
            Self::Builtin3 {
                dest,
                op,
                start_idx,
            } => write!(
                formatter,
                "R{dest} = {op}(pool[{start_idx}..{}])",
                start_idx + 3
            ),
            Self::Builtin4 {
                dest,
                op,
                start_idx,
            } => write!(
                formatter,
                "R{dest} = {op}(pool[{start_idx}..{}])",
                start_idx + 4
            ),
            Self::Square { dest, src } => write!(formatter, "R{dest} = R{src}^2"),
            Self::Cube { dest, src } => write!(formatter, "R{dest} = R{src}^3"),
            Self::Pow4 { dest, src } => write!(formatter, "R{dest} = R{src}^4"),
            Self::Pow3_2 { dest, src } => write!(formatter, "R{dest} = R{src}^(3/2)"),
            Self::InvPow3_2 { dest, src } => write!(formatter, "R{dest} = R{src}^(-3/2)"),
            Self::InvSqrt { dest, src } => write!(formatter, "R{dest} = 1/sqrt(R{src})"),
            Self::InvSquare { dest, src } => write!(formatter, "R{dest} = 1/R{src}^2"),
            Self::InvCube { dest, src } => write!(formatter, "R{dest} = 1/R{src}^3"),
            Self::Recip { dest, src } => write!(formatter, "R{dest} = 1/R{src}"),
            Self::Powi { dest, src, n } => write!(formatter, "R{dest} = R{src}^{n}"),
            Self::MulAdd { dest, a, b, c } => {
                write!(formatter, "R{dest} = R{a} * R{b} + R{c}")
            }
            Self::MulAddConst {
                dest,
                a,
                b,
                const_idx,
            } => write!(formatter, "R{dest} = R{a} * R{b} + C{const_idx}"),
            Self::MulSub { dest, a, b, c } => {
                write!(formatter, "R{dest} = R{a} * R{b} - R{c}")
            }
            Self::MulSubConst {
                dest,
                a,
                b,
                const_idx,
            } => write!(formatter, "R{dest} = R{a} * R{b} - C{const_idx}"),
            Self::NegMul { dest, a, b } => write!(formatter, "R{dest} = -(R{a} * R{b})"),
            Self::NegMulConst {
                dest,
                src,
                const_idx,
            } => write!(formatter, "R{dest} = -(R{src} * C{const_idx})"),
            Self::NegMulAdd { dest, a, b, c } => {
                write!(formatter, "R{dest} = -(R{a} * R{b}) + R{c}")
            }
            Self::NegMulAddConst {
                dest,
                a,
                b,
                const_idx,
            } => write!(formatter, "R{dest} = -(R{a} * R{b}) + C{const_idx}"),
            Self::RecipExpm1 { dest, src } => {
                write!(formatter, "R{dest} = 1/(exp(R{src})-1)")
            }
            Self::ExpSqr { dest, src } => write!(formatter, "R{dest} = exp(R{src}^2)"),
            Self::ExpSqrNeg { dest, src } => write!(formatter, "R{dest} = exp(-R{src}^2)"),
        }
    }
}
