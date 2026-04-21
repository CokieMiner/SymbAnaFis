//! Bytecode instruction set for the register-based expression evaluator.
use std::fmt::{Display, Formatter, Result as FmtResult};

use super::functions::FnOp;

/// Bytecode instruction for the register-based expression evaluator.
///
/// This ISA follows a Unified Memory Layout:
/// - Registers `0..param_count` are input parameters.
/// - Registers `param_count..param_count + const_count` are hardwired to the constant pool.
/// - Registers `param_count + const_count..` are temporaries.
///
/// Because constants are mapped directly into the register file, there are no
/// specialized `LoadConst` or `AddConst` instructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Instruction {
    /// Copy register value: `dest = src`
    Copy { dest: u32, src: u32 },

    /// Addition: `dest = a + b`
    Add { dest: u32, a: u32, b: u32 },
    /// Ternary Addition: `dest = a + b + c`
    Add3 { dest: u32, a: u32, b: u32, c: u32 },
    /// Quaternary Addition: `dest = a + b + c + d`
    Add4 {
        dest: u32,
        a: u32,
        b: u32,
        c: u32,
        d: u32,
    },
    /// Multiplication: `dest = a * b`
    Mul { dest: u32, a: u32, b: u32 },
    /// Ternary Multiplication: `dest = a * b * c`
    Mul3 { dest: u32, a: u32, b: u32, c: u32 },
    /// Quaternary Multiplication: `dest = a * b * c * d`
    Mul4 {
        dest: u32,
        a: u32,
        b: u32,
        c: u32,
        d: u32,
    },
    /// Subtraction: `dest = a - b`
    Sub { dest: u32, a: u32, b: u32 },
    /// Division: `dest = num / den`
    Div { dest: u32, num: u32, den: u32 },
    /// Power: `dest = base ^ exp`
    Pow { dest: u32, base: u32, exp: u32 },
    /// Negation: `dest = -src`
    Neg { dest: u32, src: u32 },

    /// N-ary Addition: `dest = sum(registers[start_idx..start_idx + count])`
    AddN {
        dest: u32,
        start_idx: u32,
        count: u32,
    },
    /// N-ary Multiplication: `dest = product(registers[start_idx..start_idx + count])`
    MulN {
        dest: u32,
        start_idx: u32,
        count: u32,
    },

    /// Unary Builtin: `dest = op(arg)`
    Builtin1 { dest: u32, op: FnOp, arg: u32 },
    /// Binary Builtin: `dest = op(arg1, arg2)`
    Builtin2 {
        dest: u32,
        op: FnOp,
        arg1: u32,
        arg2: u32,
    },
    /// Ternary Builtin: `dest = op(registers[start_idx..start_idx + 3])`
    Builtin3 { dest: u32, op: FnOp, start_idx: u32 },
    /// Quaternary Builtin: `dest = op(registers[start_idx..start_idx + 4])`
    Builtin4 { dest: u32, op: FnOp, start_idx: u32 },

    /// Square: `dest = src^2`
    Square { dest: u32, src: u32 },
    /// Cube: `dest = src^3`
    Cube { dest: u32, src: u32 },
    /// Fourth Power: `dest = src^4`
    Pow4 { dest: u32, src: u32 },
    /// Power 3/2: `dest = src^(3/2)`
    Pow3_2 { dest: u32, src: u32 },
    /// Inverse Power 3/2: `dest = src^(-3/2)`
    InvPow3_2 { dest: u32, src: u32 },
    /// Inverse Square Root: `dest = 1/sqrt(src)`
    InvSqrt { dest: u32, src: u32 },
    /// Inverse Square: `dest = 1/src^2`
    InvSquare { dest: u32, src: u32 },
    /// Inverse Cube: `dest = 1/src^3`
    InvCube { dest: u32, src: u32 },
    /// Reciprocal: `dest = 1/src`
    Recip { dest: u32, src: u32 },
    /// Integer Power: `dest = src^n`
    Powi { dest: u32, src: u32, n: i32 },

    /// Fused Multiply-Add: `dest = a * b + c`
    MulAdd { dest: u32, a: u32, b: u32, c: u32 },
    /// Fused Multiply-Subtract: `dest = a * b - c`
    MulSub { dest: u32, a: u32, b: u32, c: u32 },
    /// Negated Multiplication: `dest = -(a * b)`
    NegMul { dest: u32, a: u32, b: u32 },
    /// Negated Fused Multiply-Add: `dest = -(a * b) + c`
    NegMulAdd { dest: u32, a: u32, b: u32, c: u32 },
    /// Negated Fused Multiply-Subtract: `dest = -(a * b) - c`
    NegMulSub { dest: u32, a: u32, b: u32, c: u32 },

    /// Reciprocal of exp(x)-1: `dest = 1/(exp(src)-1)`
    RecipExpm1 { dest: u32, src: u32 },
    /// Exponential of Square: `dest = exp(src^2)`
    ExpSqr { dest: u32, src: u32 },
    /// Exponential of Negative Square: `dest = exp(-src^2)`
    ExpSqrNeg { dest: u32, src: u32 },

    /// Fused Sine and Cosine: `(sin_dest, cos_dest) = sincos(arg)`
    SinCos {
        sin_dest: u32,
        cos_dest: u32,
        arg: u32,
    },
}

impl Instruction {
    /// Range in the arg-pool used by this instruction, if any.
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

    /// Get the primary destination register for this instruction.
    pub(crate) const fn dest_reg(&self) -> u32 {
        match self {
            Self::Copy { dest, .. }
            | Self::Add { dest, .. }
            | Self::Add3 { dest, .. }
            | Self::Add4 { dest, .. }
            | Self::Mul { dest, .. }
            | Self::Mul3 { dest, .. }
            | Self::Mul4 { dest, .. }
            | Self::AddN { dest, .. }
            | Self::MulN { dest, .. }
            | Self::Sub { dest, .. }
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
            | Self::MulSub { dest, .. }
            | Self::NegMul { dest, .. }
            | Self::NegMulAdd { dest, .. }
            | Self::NegMulSub { dest, .. }
            | Self::RecipExpm1 { dest, .. }
            | Self::ExpSqr { dest, .. }
            | Self::ExpSqrNeg { dest, .. } => *dest,
            Self::SinCos { sin_dest, .. } => *sin_dest,
        }
    }

    /// Execute a closure for every register index being WRITTEN in this instruction.
    pub(crate) fn for_each_write(&self, mut callback: impl FnMut(u32)) {
        match *self {
            Self::SinCos {
                sin_dest, cos_dest, ..
            } => {
                callback(sin_dest);
                callback(cos_dest);
            }
            _ => callback(self.dest_reg()),
        }
    }

    /// Execute a closure for every register index being READ in this instruction.
    pub(crate) fn for_each_read(&self, mut callback: impl FnMut(u32)) {
        match self {
            Self::Copy { src: a, .. }
            | Self::Neg { src: a, .. }
            | Self::Builtin1 { arg: a, .. }
            | Self::Square { src: a, .. }
            | Self::Cube { src: a, .. }
            | Self::Pow4 { src: a, .. }
            | Self::Pow3_2 { src: a, .. }
            | Self::InvPow3_2 { src: a, .. }
            | Self::InvSqrt { src: a, .. }
            | Self::InvSquare { src: a, .. }
            | Self::InvCube { src: a, .. }
            | Self::Recip { src: a, .. }
            | Self::Powi { src: a, .. }
            | Self::RecipExpm1 { src: a, .. }
            | Self::ExpSqr { src: a, .. }
            | Self::ExpSqrNeg { src: a, .. }
            | Self::SinCos { arg: a, .. } => callback(*a),
            Self::Add { a, b, .. }
            | Self::Mul { a, b, .. }
            | Self::Sub { a, b, .. }
            | Self::Div { num: a, den: b, .. }
            | Self::Pow {
                base: a, exp: b, ..
            }
            | Self::Builtin2 {
                arg1: a, arg2: b, ..
            }
            | Self::NegMul { a, b, .. } => {
                callback(*a);
                callback(*b);
            }
            Self::Add3 { a, b, c, .. }
            | Self::Mul3 { a, b, c, .. }
            | Self::MulAdd { a, b, c, .. }
            | Self::MulSub { a, b, c, .. }
            | Self::NegMulAdd { a, b, c, .. }
            | Self::NegMulSub { a, b, c, .. } => {
                callback(*a);
                callback(*b);
                callback(*c);
            }
            Self::Add4 { a, b, c, d, .. } | Self::Mul4 { a, b, c, d, .. } => {
                callback(*a);
                callback(*b);
                callback(*c);
                callback(*d);
            }
            Self::AddN { .. }
            | Self::MulN { .. }
            | Self::Builtin3 { .. }
            | Self::Builtin4 { .. } => {}
        }
    }

    /// Map all destination, read, and pooled register indices in this instruction using the provided function.
    #[inline]
    pub fn map_all_regs(&mut self, arg_pool: &mut [u32], mapper: &impl Fn(u32) -> u32) {
        self.map_dests(mapper);
        self.map_reads(mapper);
        self.map_pooled_regs(arg_pool, mapper);
    }

    /// Map all destination register indices in this instruction using the provided function.
    pub fn map_dests(&mut self, mapper: &impl Fn(u32) -> u32) {
        match self {
            Self::SinCos {
                sin_dest, cos_dest, ..
            } => {
                *sin_dest = mapper(*sin_dest);
                *cos_dest = mapper(*cos_dest);
            }
            Self::Copy { dest, .. }
            | Self::Add { dest, .. }
            | Self::Add3 { dest, .. }
            | Self::Add4 { dest, .. }
            | Self::Mul { dest, .. }
            | Self::Mul3 { dest, .. }
            | Self::Mul4 { dest, .. }
            | Self::AddN { dest, .. }
            | Self::MulN { dest, .. }
            | Self::Sub { dest, .. }
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
            | Self::MulSub { dest, .. }
            | Self::NegMul { dest, .. }
            | Self::NegMulAdd { dest, .. }
            | Self::NegMulSub { dest, .. }
            | Self::RecipExpm1 { dest, .. }
            | Self::ExpSqr { dest, .. }
            | Self::ExpSqrNeg { dest, .. } => {
                *dest = mapper(*dest);
            }
        }
    }

    /// Map all source register indices (reads) using the provided function.
    pub(crate) fn map_reads(&mut self, mut mapper: impl FnMut(u32) -> u32) {
        match self {
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
            | Self::ExpSqrNeg { src, .. } => {
                *src = mapper(*src);
            }
            Self::Add { a, b, .. }
            | Self::Mul { a, b, .. }
            | Self::Sub { a, b, .. }
            | Self::Div { num: a, den: b, .. }
            | Self::Pow {
                base: a, exp: b, ..
            }
            | Self::NegMul { a, b, .. } => {
                *a = mapper(*a);
                *b = mapper(*b);
            }
            Self::Builtin1 { arg, .. } | Self::SinCos { arg, .. } => {
                *arg = mapper(*arg);
            }
            Self::Builtin2 { arg1, arg2, .. } => {
                *arg1 = mapper(*arg1);
                *arg2 = mapper(*arg2);
            }
            Self::Add3 { a, b, c, .. }
            | Self::Mul3 { a, b, c, .. }
            | Self::MulAdd { a, b, c, .. }
            | Self::MulSub { a, b, c, .. }
            | Self::NegMulAdd { a, b, c, .. }
            | Self::NegMulSub { a, b, c, .. } => {
                *a = mapper(*a);
                *b = mapper(*b);
                *c = mapper(*c);
            }
            Self::Add4 { a, b, c, d, .. } | Self::Mul4 { a, b, c, d, .. } => {
                *a = mapper(*a);
                *b = mapper(*b);
                *c = mapper(*c);
                *d = mapper(*d);
            }
            Self::AddN { .. }
            | Self::MulN { .. }
            | Self::Builtin3 { .. }
            | Self::Builtin4 { .. } => {}
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

    /// Execute a closure for every register index (dest or src) in this instruction.
    pub(crate) fn for_each_reg(&self, mut callback: impl FnMut(u32)) {
        self.for_each_write(&mut callback);
        self.for_each_read(callback);
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match *self {
            Self::Copy { dest, src } => write!(f, "R{dest} = R{src}"),
            Self::Add { dest, a, b } => write!(f, "R{dest} = R{a} + R{b}"),
            Self::Add3 { dest, a, b, c } => write!(f, "R{dest} = R{a} + R{b} + R{c}"),
            Self::Add4 { dest, a, b, c, d } => write!(f, "R{dest} = R{a} + R{b} + R{c} + R{d}"),
            Self::Mul { dest, a, b } => write!(f, "R{dest} = R{a} * R{b}"),
            Self::Mul3 { dest, a, b, c } => write!(f, "R{dest} = R{a} * R{b} * R{c}"),
            Self::Mul4 { dest, a, b, c, d } => write!(f, "R{dest} = R{a} * R{b} * R{c} * R{d}"),
            Self::Sub { dest, a, b } => write!(f, "R{dest} = R{a} - R{b}"),
            Self::Div { dest, num, den } => write!(f, "R{dest} = R{num} / R{den}"),
            Self::Pow { dest, base, exp } => write!(f, "R{dest} = R{base} ^ R{exp}"),
            Self::Neg { dest, src } => write!(f, "R{dest} = -R{src}"),
            Self::AddN {
                dest,
                start_idx,
                count,
            } => write!(f, "R{dest} = sum(pool[{start_idx}..{}])", start_idx + count),
            Self::MulN {
                dest,
                start_idx,
                count,
            } => write!(
                f,
                "R{dest} = prod(pool[{start_idx}..{}])",
                start_idx + count
            ),
            Self::Builtin1 { dest, op, arg } => write!(f, "R{dest} = {op}(R{arg})"),
            Self::Builtin2 {
                dest,
                op,
                arg1,
                arg2,
            } => write!(f, "R{dest} = {op}(R{arg1}, R{arg2})"),
            Self::Builtin3 {
                dest,
                op,
                start_idx,
            } => write!(f, "R{dest} = {op}(pool[{start_idx}..3])"),
            Self::Builtin4 {
                dest,
                op,
                start_idx,
            } => write!(f, "R{dest} = {op}(pool[{start_idx}..4])"),
            Self::Square { dest, src } => write!(f, "R{dest} = R{src}^2"),
            Self::Cube { dest, src } => write!(f, "R{dest} = R{src}^3"),
            Self::Pow4 { dest, src } => write!(f, "R{dest} = R{src}^4"),
            Self::Pow3_2 { dest, src } => write!(f, "R{dest} = R{src}^(3/2)"),
            Self::InvPow3_2 { dest, src } => write!(f, "R{dest} = R{src}^(-3/2)"),
            Self::InvSqrt { dest, src } => write!(f, "R{dest} = 1/sqrt(R{src})"),
            Self::InvSquare { dest, src } => write!(f, "R{dest} = 1/R{src}^2"),
            Self::InvCube { dest, src } => write!(f, "R{dest} = 1/R{src}^3"),
            Self::Recip { dest, src } => write!(f, "R{dest} = 1/R{src}"),
            Self::Powi { dest, src, n } => write!(f, "R{dest} = R{src}^{n}"),
            Self::MulAdd { dest, a, b, c } => write!(f, "R{dest} = R{a} * R{b} + R{c}"),
            Self::MulSub { dest, a, b, c } => write!(f, "R{dest} = R{a} * R{b} - R{c}"),
            Self::NegMul { dest, a, b } => write!(f, "R{dest} = -(R{a} * R{b})"),
            Self::NegMulAdd { dest, a, b, c } => write!(f, "R{dest} = -(R{a} * R{b}) + R{c}"),
            Self::NegMulSub { dest, a, b, c } => write!(f, "R{dest} = -(R{a} * R{b}) - R{c}"),
            Self::RecipExpm1 { dest, src } => write!(f, "R{dest} = 1/(exp(R{src})-1)"),
            Self::ExpSqr { dest, src } => write!(f, "R{dest} = exp(R{src}^2)"),
            Self::ExpSqrNeg { dest, src } => write!(f, "R{dest} = exp(-R{src}^2)"),
            Self::SinCos {
                sin_dest,
                cos_dest,
                arg,
            } => write!(f, "(R{sin_dest}, R{cos_dest}) = sincos(R{arg})"),
        }
    }
}
