//! Bytecode instruction set for the register-based expression evaluator.

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
    MulSubRev {
        dest: u32,
        a: u32,
        b: u32,
        c: u32,
    },
    MulSubRevConst {
        dest: u32,
        a: u32,
        b: u32,
        const_idx: u32,
    },

    PolyEval {
        dest: u32,
        x: u32,
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
    /// Execute a closure for every register index (dest or src) in this instruction.
    ///
    /// NOTE: For N-ary instructions (`AddN`, `MulN`, `Builtin3`, `Builtin4`), only `dest` is
    /// visited here. Register indices in the `arg_pool` must be handled separately by the caller.
    #[allow(
        clippy::too_many_lines,
        reason = "Exhaustive match on all instruction variants"
    )]
    pub(crate) fn for_each_reg(&self, mut f: impl FnMut(u32)) {
        match *self {
            Self::LoadConst { dest, .. }
            | Self::AddN { dest, .. }
            | Self::MulN { dest, .. }
            | Self::Builtin3 { dest, .. }
            | Self::Builtin4 { dest, .. } => f(dest),
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
            | Self::ConstDiv { dest, src, .. } => {
                f(dest);
                f(src);
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
            | Self::MulAddConst { dest, a, b, .. }
            | Self::MulSubConst { dest, a, b, .. }
            | Self::NegMulAddConst { dest, a, b, .. }
            | Self::MulSubRevConst { dest, a, b, .. } => {
                f(dest);
                f(a);
                f(b);
            }
            Self::Builtin1 { dest, arg, .. } => {
                f(dest);
                f(arg);
            }
            Self::Builtin2 {
                dest, arg1, arg2, ..
            } => {
                f(dest);
                f(arg1);
                f(arg2);
            }
            Self::MulAdd { dest, a, b, c }
            | Self::MulSub { dest, a, b, c }
            | Self::NegMulAdd { dest, a, b, c }
            | Self::MulSubRev { dest, a, b, c } => {
                f(dest);
                f(a);
                f(b);
                f(c);
            }
            Self::PolyEval { dest, x, .. } => {
                f(dest);
                f(x);
            }
        }
    }

    /// Execute a closure for every register index being READ in this instruction.
    ///
    /// NOTE: For N-ary instructions (`AddN`, `MulN`, `Builtin3`, `Builtin4`), reads from
    /// the `arg_pool` must be handled separately by the caller.
    pub(crate) fn for_each_read(&self, mut f: impl FnMut(u32)) {
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
            | Self::ConstDiv { src, .. } => {
                f(src);
            }
            Self::Add { a, b, .. }
            | Self::Mul { a, b, .. }
            | Self::Sub { a, b, .. }
            | Self::Div { num: a, den: b, .. }
            | Self::Pow {
                base: a, exp: b, ..
            }
            | Self::MulAddConst { a, b, .. }
            | Self::MulSubConst { a, b, .. }
            | Self::NegMulAddConst { a, b, .. }
            | Self::MulSubRevConst { a, b, .. } => {
                f(a);
                f(b);
            }
            Self::Builtin1 { arg, .. } => {
                f(arg);
            }
            Self::Builtin2 { arg1, arg2, .. } => {
                f(arg1);
                f(arg2);
            }
            Self::MulAdd { a, b, c, .. }
            | Self::MulSub { a, b, c, .. }
            | Self::NegMulAdd { a, b, c, .. }
            | Self::MulSubRev { a, b, c, .. } => {
                f(a);
                f(b);
                f(c);
            }
            Self::PolyEval { x, .. } => {
                f(x);
            }
        }
    }

    /// Map all source register indices (reads) using the provided function.
    ///
    /// NOTE: For N-ary instructions (`AddN`, `MulN`, `Builtin3`, `Builtin4`), reads from
    /// the `arg_pool` must be handled separately by the caller.
    pub(crate) fn map_reads(&mut self, mut f: impl FnMut(u32) -> u32) {
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
            | Self::ConstDiv { src, .. } => {
                *src = f(*src);
            }
            Self::Add { a, b, .. }
            | Self::Mul { a, b, .. }
            | Self::Sub { a, b, .. }
            | Self::Div { num: a, den: b, .. }
            | Self::Pow {
                base: a, exp: b, ..
            }
            | Self::MulAddConst { a, b, .. }
            | Self::MulSubConst { a, b, .. }
            | Self::NegMulAddConst { a, b, .. }
            | Self::MulSubRevConst { a, b, .. } => {
                *a = f(*a);
                *b = f(*b);
            }
            Self::Builtin1 { arg, .. } => {
                *arg = f(*arg);
            }
            Self::Builtin2 { arg1, arg2, .. } => {
                *arg1 = f(*arg1);
                *arg2 = f(*arg2);
            }
            Self::MulAdd { a, b, c, .. }
            | Self::MulSub { a, b, c, .. }
            | Self::NegMulAdd { a, b, c, .. }
            | Self::MulSubRev { a, b, c, .. } => {
                *a = f(*a);
                *b = f(*b);
                *c = f(*c);
            }
            Self::PolyEval { x, .. } => {
                *x = f(*x);
            }
        }
    }

    /// Map all register indices in this instruction using the provided function.
    ///
    /// NOTE: For N-ary instructions (`AddN`, `MulN`, `Builtin3`, `Builtin4`), registers in
    /// the `arg_pool` must be handled separately by the caller.
    pub(crate) fn map_regs(&mut self, mut f: impl FnMut(u32) -> u32) {
        match self {
            Self::LoadConst { dest, .. }
            | Self::AddN { dest, .. }
            | Self::MulN { dest, .. }
            | Self::Builtin3 { dest, .. }
            | Self::Builtin4 { dest, .. } => {
                *dest = f(*dest);
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
            | Self::ConstDiv { dest, src, .. } => {
                *dest = f(*dest);
                *src = f(*src);
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
            | Self::MulAddConst { dest, a, b, .. }
            | Self::MulSubConst { dest, a, b, .. }
            | Self::NegMulAddConst { dest, a, b, .. }
            | Self::MulSubRevConst { dest, a, b, .. } => {
                *dest = f(*dest);
                *a = f(*a);
                *b = f(*b);
            }
            Self::Builtin1 { dest, arg, .. } => {
                *dest = f(*dest);
                *arg = f(*arg);
            }
            Self::Builtin2 {
                dest, arg1, arg2, ..
            } => {
                *dest = f(*dest);
                *arg1 = f(*arg1);
                *arg2 = f(*arg2);
            }
            Self::MulAdd { dest, a, b, c }
            | Self::MulSub { dest, a, b, c }
            | Self::NegMulAdd { dest, a, b, c }
            | Self::MulSubRev { dest, a, b, c } => {
                *dest = f(*dest);
                *a = f(*a);
                *b = f(*b);
                *c = f(*c);
            }
            Self::PolyEval { dest, x, .. } => {
                *dest = f(*dest);
                *x = f(*x);
            }
        }
    }
}
