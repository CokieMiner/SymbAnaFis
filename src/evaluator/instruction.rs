//! Bytecode instruction set for the register-based expression evaluator.

/// Mathematical operations supported by the `BuiltinFun` instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
    Sub {
        dest: u32,
        a: u32,
        b: u32,
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
        arg1: u32,
        arg2: u32,
        arg3: u32,
    },
    Builtin4 {
        dest: u32,
        op: FnOp,
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
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
    MulSub {
        dest: u32,
        a: u32,
        b: u32,
        c: u32,
    },
    NegMulAdd {
        dest: u32,
        a: u32,
        b: u32,
        c: u32,
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
