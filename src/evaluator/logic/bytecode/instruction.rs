//! Bytecode instruction set for the register-based expression evaluator.
use std::fmt::{Display, Formatter, Result as FmtResult};

use super::functions::FnOp;

macro_rules! define_isa {
    (
        $(
            $( #[$attr:meta] )*
            $name:ident {
                $( $field:ident : $type:ty $(, @$tag:ident)? ),* $(,)?
            }
            $( => ( $fmt:expr $(, $fmt_arg:expr)* ) )?
        ),* $(,)?
    ) => {
        /// Bytecode instruction for the register-based expression evaluator.
        ///
        /// This ISA follows a Unified Memory Layout:
        /// - Registers `0..param_count` are input parameters.
        /// - Registers `param_count..param_count + const_count` are hardwired to the constant pool.
        /// - Registers `param_count + const_count..` are temporaries.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum Instruction {
            $(
                $( #[$attr] )*
                $name {
                    $( $field : $type, )*
                },
            )*
        }

        impl Instruction {
            /// Returns the u32 opcode for this instruction.
            #[inline]
            pub const fn opcode(&self) -> u32 {
                #[allow(
                    unsafe_code,
                    reason = "High-performance opcode extraction via pointer cast is safe for repr(u8) enums"
                )]
                // SAFETY: Instruction is repr(u8), ensuring the discriminant is the first byte.
                unsafe { *std::ptr::from_ref::<Self>(self).cast::<u8>() as u32 }
            }

            /// Range in the arg-pool used by this instruction, if any.
            #[inline]
            pub(crate) const fn arg_pool_range(&self) -> Option<(u32, u32)> {
                match self {
                    $(
                        Self::$name { $( $field, )* } => {
                            let _ = ( $( &$field, )* );
                            let mut start = None;
                            let mut count = None;
                            let _ = (&mut start, &mut count);
                            $(
                                define_isa!(@pool_fields $field, start, count $(, @$tag)?);
                            )*
                            if let (Some(s), Some(c)) = (start, count) {
                                Some((s, c))
                            } else {
                                None
                            }
                        }
                    )*
                }
            }

            /// Executes the provided callback for each register read by this instruction.
            #[inline]
            pub fn for_each_read<F>(&self, mut callback: F)
            where
                F: FnMut(u32),
            {
                match self {
                    $(
                        Self::$name { $( $field, )* } => {
                            let _ = ( $( &$field, )* );
                            $(
                                define_isa!(@read_field $field, callback $(, @$tag)?);
                            )*
                        }
                    )*
                }
            }

            /// Executes the provided callback for each register written by this instruction.
            #[inline]
            pub fn for_each_write<F>(&self, mut callback: F)
            where
                F: FnMut(u32),
            {
                match self {
                    $(
                        Self::$name { $( $field, )* } => {
                            let _ = ( $( &$field, )* );
                            $(
                                define_isa!(@write_field $field, callback $(, @$tag)?);
                            )*
                        }
                    )*
                }
            }

            /// Executes the provided callback for each register in this instruction (read or write).
            #[inline]
            pub fn for_each_reg<F>(&self, mut callback: F)
            where
                F: FnMut(u32),
            {
                match self {
                    $(
                        Self::$name { $( $field, )* } => {
                            let _ = ( $( &$field, )* );
                            $(
                                define_isa!(@any_reg_field $field, callback $(, @$tag)?);
                            )*
                        }
                    )*
                }
            }

            /// Executes the provided callback for each register in a pool used by this instruction.
            #[inline]
            pub fn for_each_pooled_reg<F>(&self, arg_pool: &[u32], mut callback: F)
            where
                F: FnMut(u32),
            {
                if let Some((start, count)) = self.arg_pool_range() {
                    for i in 0..count {
                        callback(arg_pool[(start + i) as usize]);
                    }
                }
            }

            /// Maps all destination registers using the provided mapper function.
            #[inline]
            pub fn map_dests<F>(&mut self, mut mapper: F)
            where
                F: FnMut(u32) -> u32,
            {
                match self {
                    $(
                        Self::$name { $( $field, )* } => {
                            let _ = ( $( &$field, )* );
                            $(
                                define_isa!(@map_dest_field $field, mapper $(, @$tag)?);
                            )*
                        }
                    )*
                }
            }

            /// Maps all read registers using the provided mapper function.
            #[inline]
            pub fn map_reads<F>(&mut self, mut mapper: F)
            where
                F: FnMut(u32) -> u32,
            {
                match self {
                    $(
                        Self::$name { $( $field, )* } => {
                            let _ = ( $( &$field, )* );
                            $(
                                define_isa!(@map_read_field $field, mapper $(, @$tag)?);
                            )*
                        }
                    )*
                }
            }

            /// Maps all registers in a pool using the provided mapper function.
            #[inline]
            pub fn map_pooled_regs<F>(&mut self, arg_pool: &mut [u32], mut mapper: F)
            where
                F: FnMut(u32) -> u32,
            {
                if let Some((start, count)) = self.arg_pool_range() {
                    for i in 0..count {
                        let r = &mut arg_pool[(start + i) as usize];
                        *r = mapper(*r);
                    }
                }
            }

            /// Returns the primary destination register of this instruction, if any.
            #[inline]
            pub const fn primary_dest(&self) -> Option<u32> {
                let mut res = None;
                match self {
                    $(
                        Self::$name { $( $field, )* } => {
                            let _ = ( $( &$field, )* );
                            $(
                                define_isa!(@primary_dest_helper $field, res $(, @$tag)?);
                            )*
                        }
                    )*
                }
                res
            }

            /// Utility to map EVERY register in the instruction (including pool) via a single mapper.
            #[inline]
            pub fn map_all_regs<F>(&mut self, arg_pool: &mut [u32], mut mapper: F)
            where
                F: FnMut(u32) -> u32,
            {
                self.map_pooled_regs(arg_pool, &mut mapper);
                match self {
                    $(
                        Self::$name { $( $field, )* } => {
                            let _ = ( $( &$field, )* );
                            $(
                                define_isa!(@map_any_field $field, mapper $(, @$tag)?);
                            )*
                        }
                    )*
                }
            }
        }

        impl Display for Instruction {
            fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
                match self {
                    $(
                        Self::$name { $( $field, )* } => {
                            let _ = ( $( &$field, )* );
                            define_isa!(@fmt_dispatch f, $name { $( $field ),* } $(, display = ( $fmt $(, $fmt_arg)* ) )? )
                        }
                    )*
                }
            }
        }
    };

    // --- Helpers for for_each_write ---
    (@write_field $field:ident, $callback:ident, @dest) => { $callback(*$field); };
    (@write_field $field:ident, $callback:ident $(, @$tag:ident)?) => { };

    // --- Helpers for for_each_read ---
    (@read_field $field:ident, $callback:ident, @read) => { $callback(*$field); };
    (@read_field $field:ident, $callback:ident $(, @$tag:ident)?) => { };

    // --- Helpers for for_each_reg ---
    (@any_reg_field $field:ident, $callback:ident, @read) => { $callback(*$field); };
    (@any_reg_field $field:ident, $callback:ident, @dest) => { $callback(*$field); };
    (@any_reg_field $field:ident, $callback:ident $(, @$tag:ident)?) => { };

    // --- Helpers for map_dests ---
    (@map_dest_field $field:ident, $mapper:ident, @dest) => { *$field = $mapper(*$field); };
    (@map_dest_field $field:ident, $mapper:ident $(, @$tag:ident)?) => { };

    // --- Helpers for map_reads ---
    (@map_read_field $field:ident, $mapper:ident, @read) => { *$field = $mapper(*$field); };
    (@map_read_field $field:ident, $mapper:ident $(, @$tag:ident)?) => { };

    // --- Internal Dispatchers ---
    (@fmt_dispatch $f:ident, $name:ident { $( $field:ident ),* } , display = ( $fmt:expr $(, $fmt_arg:expr)* ) ) => {
        write!($f, $fmt, $( $fmt_arg ),*)
    };
    (@fmt_dispatch $f:ident, $name:ident { $( $field:ident ),* } ) => {{
        write!($f, stringify!($name))?;
        let mut first = true;
        let _ = &mut first;
        $(
            if first { write!($f, "(")?; first = false; } else { write!($f, ", ")?; }
            write!($f, "{}: {}", stringify!($field), $field)?;
        )*
        if !first { write!($f, ")")?; }
        Ok(())
    }};

    (@is_dest @dest) => { true };
    (@is_dest $(@$tag:ident)?) => { false };

    (@primary_dest_helper $field:ident, $res:ident, @dest) => { if $res.is_none() { $res = Some(*$field); } };
    (@primary_dest_helper $field:ident, $res:ident $(, @$tag:ident)?) => { };

    (@is_pool_start @pool_start) => { true };
    (@is_pool_start $(@$tag:ident)?) => { false };

    (@is_pool_start @pool_count) => { true };
    (@is_pool_start $(@$tag:ident)?) => { false };

    (@map_any_field $field:ident, $mapper:ident, @dest) => { *$field = $mapper(*$field); };
    (@map_any_field $field:ident, $mapper:ident, @read) => { *$field = $mapper(*$field); };
    (@map_any_field $field:ident, $mapper:ident $(, @$tag:ident)?) => { };

    // --- Helpers for arg_pool_range ---
    (@pool_fields $field:ident, $start:ident, $count:ident, @pool_start) => { $start = Some(*$field); };
    (@pool_fields $field:ident, $start:ident, $count:ident, @pool_count) => { $count = Some(*$field); };
    (@pool_fields $field:ident, $start:ident, $count:ident $(, @$tag:ident)?) => { };
}

define_isa! {
    /// Sentinel to mark end of bytecode stream.
    End { },

    /// Copy register value: `dest = src`
    Copy { dest: u32, @dest, src: u32, @read } => ("R{} = R{}", dest, src),

    /// Negation: `dest = -src`
    Neg { dest: u32, @dest, src: u32, @read } => ("R{} = -R{}", dest, src),

    /// Fused Sine and Cosine: `(sin_dest, cos_dest) = sincos(arg)`
    SinCos { sin_dest: u32, @dest, cos_dest: u32, @dest, arg: u32, @read } => ("(R{}, R{}) = sincos(R{})", sin_dest, cos_dest, arg),

    /// Addition: `dest = a + b`
    Add { dest: u32, @dest, a: u32, @read, b: u32, @read } => ("R{} = R{} + R{}", dest, a, b),
    /// Ternary Addition: `dest = a + b + c`
    Add3 { dest: u32, @dest, a: u32, @read, b: u32, @read, c: u32, @read } => ("R{} = R{} + R{} + R{}", dest, a, b, c),
    /// Quaternary Addition: `dest = a + b + c + d`
    Add4 { dest: u32, @dest, a: u32, @read, b: u32, @read, c: u32, @read, d: u32, @read } => ("R{} = R{} + R{} + R{} + R{}", dest, a, b, c, d),

    /// N-ary Addition: `dest = sum(registers[start_idx..start_idx + count])`
    AddN { dest: u32, @dest, start_idx: u32, @pool_start, count: u32, @pool_count } => ("R{} = sum(pool[{}..{}])", dest, start_idx, start_idx + count),

    /// Multiplication: `dest = a * b`
    Mul { dest: u32, @dest, a: u32, @read, b: u32, @read } => ("R{} = R{} * R{}", dest, a, b),
    /// Ternary Multiplication: `dest = a * b * c`
    Mul3 { dest: u32, @dest, a: u32, @read, b: u32, @read, c: u32, @read } => ("R{} = R{} * R{} * R{}", dest, a, b, c),
    /// Quaternary Multiplication: `dest = a * b * c * d`
    Mul4 { dest: u32, @dest, a: u32, @read, b: u32, @read, c: u32, @read, d: u32, @read } => ("R{} = R{} * R{} * R{} * R{}", dest, a, b, c, d),

    /// N-ary Multiplication: `dest = product(registers[start_idx..start_idx + count])`
    MulN { dest: u32, @dest, start_idx: u32, @pool_start, count: u32, @pool_count } => ("R{} = prod(pool[{}..{}])", dest, start_idx, start_idx + count),

    /// Subtraction: `dest = a - b`
    Sub { dest: u32, @dest, a: u32, @read, b: u32, @read } => ("R{} = R{} - R{}", dest, a, b),

    /// Division: `dest = num / den`
    Div { dest: u32, @dest, num: u32, @read, den: u32, @read } => ("R{} = R{} / R{}", dest, num, den),

    /// Power: `dest = base ^ exp`
    Pow { dest: u32, @dest, base: u32, @read, exp: u32, @read } => ("R{} = R{} ^ R{}", dest, base, exp),

    /// Fused Multiply-Add: `dest = a * b + c`
    MulAdd { dest: u32, @dest, a: u32, @read, b: u32, @read, c: u32, @read } => ("R{} = R{} * R{} + R{}", dest, a, b, c),
    /// Fused Multiply-Subtract: `dest = a * b - c`
    MulSub { dest: u32, @dest, a: u32, @read, b: u32, @read, c: u32, @read } => ("R{} = R{} * R{} - R{}", dest, a, b, c),
    /// Negated Multiplication: `dest = -(a * b)`
    NegMul { dest: u32, @dest, a: u32, @read, b: u32, @read } => ("R{} = -(R{} * R{})", dest, a, b),
    /// Negated Fused Multiply-Add: `dest = -(a * b) + c`
    NegMulAdd { dest: u32, @dest, a: u32, @read, b: u32, @read, c: u32, @read } => ("R{} = -(R{} * R{}) + R{}", dest, a, b, c),
    /// Negated Fused Multiply-Subtract: `dest = -(a * b) - c`
    NegMulSub { dest: u32, @dest, a: u32, @read, b: u32, @read, c: u32, @read } => ("R{} = -(R{} * R{}) - R{}", dest, a, b, c),

    /// Square: `dest = src^2`
    Square { dest: u32, @dest, src: u32, @read } => ("R{}^2", dest),
    /// Cube: `dest = src^3`
    Cube { dest: u32, @dest, src: u32, @read } => ("R{}^3", dest),
    /// Fourth Power: `dest = src^4`
    Pow4 { dest: u32, @dest, src: u32, @read } => ("R{}^4", dest),
    /// Power 3/2: `dest = src^(3/2)`
    Pow3_2 { dest: u32, @dest, src: u32, @read } => ("R{}^(3/2)", dest),
    /// Inverse Power 3/2: `dest = src^(-3/2)`
    InvPow3_2 { dest: u32, @dest, src: u32, @read } => ("R{}^(-3/2)", dest),
    /// Inverse Square Root: `dest = 1/sqrt(src)`
    InvSqrt { dest: u32, @dest, src: u32, @read } => ("1/sqrt(R{})", src),
    /// Inverse Square: `dest = 1/src^2`
    InvSquare { dest: u32, @dest, src: u32, @read } => ("1/R{}^2", src),
    /// Inverse Cube: `dest = 1/src^3`
    InvCube { dest: u32, @dest, src: u32, @read } => ("1/R{}^3", src),
    /// Reciprocal: `dest = 1/src`
    Recip { dest: u32, @dest, src: u32, @read } => ("1/R{}", src),
    /// Integer Power: `dest = src^n`
    Powi { dest: u32, @dest, src: u32, @read, n: i32 } => ("R{}^{}", dest, n),

    /// Sine: `dest = sin(arg)`
    Sin { dest: u32, @dest, arg: u32, @read } => ("R{} = sin(R{})", dest, arg),
    /// Cosine: `dest = cos(arg)`
    Cos { dest: u32, @dest, arg: u32, @read } => ("R{} = cos(R{})", dest, arg),
    /// Exponential: `dest = exp(arg)`
    Exp { dest: u32, @dest, arg: u32, @read } => ("R{} = exp(R{})", dest, arg),
    /// Natural Logarithm: `dest = ln(arg)`
    Ln { dest: u32, @dest, arg: u32, @read } => ("R{} = ln(R{})", dest, arg),
    /// Square Root: `dest = sqrt(arg)`
    Sqrt { dest: u32, @dest, arg: u32, @read } => ("R{} = sqrt(R{})", dest, arg),

    /// Reciprocal of exp(x)-1: `dest = 1/(exp(src)-1)`
    RecipExpm1 { dest: u32, @dest, src: u32, @read } => ("R{} = 1/(exp(R{})-1)", dest, src),
    /// Exponential of Square: `dest = exp(src^2)`
    ExpSqr { dest: u32, @dest, src: u32, @read } => ("R{} = exp(R{}^2)", dest, src),
    /// Exponential of Negative Square: `dest = exp(-src^2)`
    ExpSqrNeg { dest: u32, @dest, src: u32, @read } => ("R{} = exp(-R{}^2)", dest, src),

    /// Unary Builtin: `dest = u32, op: FnOp, arg: u32`
    Builtin1 { dest: u32, @dest, op: FnOp, arg: u32, @read } => ("R{} = {}(R{})", dest, op, arg),
    /// Binary Builtin: `dest = u32, op: FnOp, arg1: u32, arg2: u32`
    Builtin2 { dest: u32, @dest, op: FnOp, arg1: u32, @read, arg2: u32, @read } => ("R{} = {}(R{}, R{})", dest, op, arg1, arg2),
    /// Ternary Builtin: `dest = u32, op: FnOp, arg1: u32, arg2: u32, arg3: u32`
    Builtin3 { dest: u32, @dest, op: FnOp, arg1: u32, @read, arg2: u32, @read, arg3: u32, @read } => ("R{} = {}(R{}, R{}, R{})", dest, op, arg1, arg2, arg3),
    /// Quaternary Builtin: `dest = u32, op: FnOp, arg1: u32, arg2: u32, arg3: u32, arg4: u32`
    Builtin4 { dest: u32, @dest, op: FnOp, arg1: u32, @read, arg2: u32, @read, arg3: u32, @read, arg4: u32, @read } => ("R{} = {}(R{}, R{}, R{}, R{})", dest, op, arg1, arg2, arg3, arg4),
}
