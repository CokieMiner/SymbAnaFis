//! Virtual instruction set and register definitions for the compiler's intermediate representation.
//!
//! [`VReg`] (virtual registers) and [`VInstruction`] are used by the [`VirGenerator`] before
//! final physical register allocation via [`RegAllocator`].

use super::instruction::FnOp;
use std::mem::swap;

/// Virtual register used during expression compilation.
///
/// These registers are later mapped to physical registers in the final bytecode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VReg {
    /// A parameter register (e.g. x, y)
    Param(u32),
    /// A constant register (e.g. 1.0, 2.0)
    Const(u32),
    /// A temporary intermediate value register
    Temp(u32),
}

/// Intermediate instruction format using virtual registers.
///
/// This set is more expressive than the final [`Instruction`] set,
/// often including N-ary operations that are later decomposed or optimized.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum VInstruction {
    Add {
        dest: VReg,
        srcs: Vec<VReg>,
    },
    Add2 {
        dest: VReg,
        a: VReg,
        b: VReg,
    },
    Mul {
        dest: VReg,
        srcs: Vec<VReg>,
    },
    Mul2 {
        dest: VReg,
        a: VReg,
        b: VReg,
    },
    Sub {
        dest: VReg,
        a: VReg,
        b: VReg,
    },
    Div {
        dest: VReg,
        num: VReg,
        den: VReg,
    },
    Pow {
        dest: VReg,
        base: VReg,
        exp: VReg,
    },
    Neg {
        dest: VReg,
        src: VReg,
    },
    BuiltinFun {
        dest: VReg,
        op: FnOp,
        args: Vec<VReg>,
    },
    Builtin1 {
        dest: VReg,
        op: FnOp,
        arg: VReg,
    },
    Builtin2 {
        dest: VReg,
        op: FnOp,
        arg1: VReg,
        arg2: VReg,
    },
    Square {
        dest: VReg,
        src: VReg,
    },
    Cube {
        dest: VReg,
        src: VReg,
    },
    Pow4 {
        dest: VReg,
        src: VReg,
    },
    Pow3_2 {
        dest: VReg,
        src: VReg,
    },
    InvPow3_2 {
        dest: VReg,
        src: VReg,
    },
    InvSqrt {
        dest: VReg,
        src: VReg,
    },
    InvSquare {
        dest: VReg,
        src: VReg,
    },
    InvCube {
        dest: VReg,
        src: VReg,
    },
    Recip {
        dest: VReg,
        src: VReg,
    },
    Powi {
        dest: VReg,
        src: VReg,
        n: i32,
    },
    MulAdd {
        dest: VReg,
        a: VReg,
        b: VReg,
        c: VReg,
    },
    MulSub {
        dest: VReg,
        a: VReg,
        b: VReg,
        c: VReg,
    },
    NegMulAdd {
        dest: VReg,
        a: VReg,
        b: VReg,
        c: VReg,
    },
    RecipExpm1 {
        dest: VReg,
        src: VReg,
    },
    ExpSqr {
        dest: VReg,
        src: VReg,
    },
    ExpSqrNeg {
        dest: VReg,
        src: VReg,
    },
}

impl VInstruction {
    /// Gets the destination register of this instruction.
    pub const fn dest(&self) -> VReg {
        match self {
            Self::Add { dest, .. }
            | Self::Add2 { dest, .. }
            | Self::Mul { dest, .. }
            | Self::Mul2 { dest, .. }
            | Self::Sub { dest, .. }
            | Self::Div { dest, .. }
            | Self::Pow { dest, .. }
            | Self::Neg { dest, .. }
            | Self::BuiltinFun { dest, .. }
            | Self::Builtin1 { dest, .. }
            | Self::Builtin2 { dest, .. }
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
            | Self::MulSub { dest, .. }
            | Self::NegMulAdd { dest, .. }
            | Self::RecipExpm1 { dest, .. }
            | Self::ExpSqr { dest, .. }
            | Self::ExpSqrNeg { dest, .. } => *dest,
        }
    }

    /// Executes the provided function for each register that this instruction reads from.
    pub fn for_each_read(&self, mut f: impl FnMut(VReg)) {
        match self {
            Self::Add { srcs, .. } | Self::Mul { srcs, .. } => {
                for &s in srcs {
                    f(s);
                }
            }
            Self::Add2 { a, b, .. }
            | Self::Mul2 { a, b, .. }
            | Self::Sub { a, b, .. }
            | Self::Div { num: a, den: b, .. }
            | Self::Pow {
                base: a, exp: b, ..
            } => {
                f(*a);
                f(*b);
            }
            Self::BuiltinFun { args, .. } => {
                for &a in args {
                    f(a);
                }
            }
            Self::Builtin1 { arg, .. } => f(*arg),
            Self::Builtin2 { arg1, arg2, .. } => {
                f(*arg1);
                f(*arg2);
            }
            Self::MulAdd { a, b, c, .. }
            | Self::MulSub { a, b, c, .. }
            | Self::NegMulAdd { a, b, c, .. } => {
                f(*a);
                f(*b);
                f(*c);
            }
            Self::Neg { src, .. }
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
            | Self::ExpSqrNeg { src, .. } => f(*src),
        }
    }

    /// Sets the destination register of this instruction.
    pub const fn set_dest(&mut self, new_dest: VReg) {
        match self {
            Self::Add { dest, .. }
            | Self::Add2 { dest, .. }
            | Self::Mul { dest, .. }
            | Self::Mul2 { dest, .. }
            | Self::Sub { dest, .. }
            | Self::Div { dest, .. }
            | Self::Pow { dest, .. }
            | Self::Neg { dest, .. }
            | Self::BuiltinFun { dest, .. }
            | Self::Builtin1 { dest, .. }
            | Self::Builtin2 { dest, .. }
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
            | Self::MulSub { dest, .. }
            | Self::NegMulAdd { dest, .. }
            | Self::RecipExpm1 { dest, .. }
            | Self::ExpSqr { dest, .. }
            | Self::ExpSqrNeg { dest, .. } => *dest = new_dest,
        }
    }

    /// Executes the provided function for each register that this instruction reads from, allowing mutation.
    pub fn for_each_read_mut(&mut self, mut f: impl FnMut(&mut VReg)) {
        match self {
            Self::Add { srcs, .. } | Self::Mul { srcs, .. } => {
                for s in srcs {
                    f(s);
                }
            }
            Self::Add2 { a, b, .. }
            | Self::Mul2 { a, b, .. }
            | Self::Sub { a, b, .. }
            | Self::Div { num: a, den: b, .. }
            | Self::Pow {
                base: a, exp: b, ..
            } => {
                f(a);
                f(b);
            }
            Self::BuiltinFun { args, .. } => {
                for a in args {
                    f(a);
                }
            }
            Self::Builtin1 { arg, .. } => f(arg),
            Self::Builtin2 { arg1, arg2, .. } => {
                f(arg1);
                f(arg2);
            }
            Self::MulAdd { a, b, c, .. }
            | Self::MulSub { a, b, c, .. }
            | Self::NegMulAdd { a, b, c, .. } => {
                f(a);
                f(b);
                f(c);
            }
            Self::Neg { src, .. }
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
            | Self::ExpSqrNeg { src, .. } => f(src),
        }
    }

    /// Sorts operands for commutative operations to canonicalize layout for GVN hashing.
    pub fn sort_operands(&mut self) {
        match self {
            Self::Add2 { a, b, .. } | Self::Mul2 { a, b, .. } => {
                if a > b {
                    swap(a, b);
                }
            }
            Self::Add { srcs, .. } | Self::Mul { srcs, .. } => {
                srcs.sort_unstable();
            }
            _ => {}
        }
    }
}
