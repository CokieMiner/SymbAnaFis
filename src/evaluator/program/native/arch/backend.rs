//! Target architecture backend contract for native machine-code encoders.

use alloc::vec::Vec;

use crate::errors::EvalError;

use super::{Combine, Fused, InlineBinary, InlineUnary};

/// Physical floating-point register identifier on the target architecture (`0..32`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HwReg(pub u8);

impl HwReg {
    /// Bounded index in `0..32` for direct register-table access.
    #[inline]
    #[must_use]
    pub fn index(self) -> usize {
        usize::from(self.0 & 31)
    }

    /// Single-bit mask `1 << (self.0 & 31)`.
    #[inline]
    #[must_use]
    pub const fn bit(self) -> u32 {
        1_u32 << (self.0 & 31)
    }
}

/// Scalar floating-point component precision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatPrecision {
    F64,
    F32,
}

/// Fused multiply-add operand layout when one operand is folded directly from memory.
#[derive(Clone, Copy, Debug)]
pub enum FusedMemOperand {
    /// `dst = ±(lhs * rhs) ± [regs + addend_offset]`
    Addend {
        lhs: HwReg,
        rhs: HwReg,
        addend_offset: u32,
    },
    /// `dst = ±(factor * [regs + mem_offset]) ± addend`
    Factor {
        factor: HwReg,
        addend: HwReg,
        mem_offset: u32,
    },
}

/// Target architecture machine-code encoder contract driven by the shared emitter.
pub trait ArchBackend {
    const ARG0: HwReg;
    const ARG1: HwReg;
    const ARG2: HwReg;
    /// Fourth volatile/caller-saved FP register used to break 3-register cycles
    /// when marshalling scalar call arguments into `[ARG0, ARG1, ARG2]`.
    const CALL_SCRATCH: HwReg;

    /// Bitmask of allocatable FP registers for a kernel with the given call,
    /// reserved scratch register count, and peak live-register pressure (`max_live`).
    fn allocatable_mask(&self, has_calls: bool, scratch_regs: u8, max_live: u32) -> u32;

    /// Bitmask of FP registers preserved across C-ABI calls made by this kernel.
    fn callee_saved_mask(&self) -> u32;

    /// Configure frame addressing before emitting the instruction body.
    fn configure_frame(&mut self, _has_calls: bool, _max_byte_offset: u32) {}

    /// Whether binary operations support non-destructive 3-operand encoding (`dst` may alias `rhs`).
    fn non_destructive_binary(&self) -> bool;

    /// Whether fused multiply-add instructions support non-destructive 3-input encoding
    /// where `dst` may alias any of `a`, `b`, or `c`.
    fn non_destructive_fused(&self) -> bool;

    /// Whether single-rounding hardware FMA instructions are available on this backend.
    fn has_hardware_fma(&self) -> bool {
        self.non_destructive_fused()
    }

    /// Whether binary operations (`Combine`, `Divide`) can read the second operand directly from memory.
    fn supports_mem_operand(&self) -> bool {
        false
    }

    /// Whether fused operations can read one operand directly from memory.
    fn supports_fused_mem(&self) -> bool {
        false
    }

    fn emit_prologue(
        &mut self,
        buf: &mut Vec<u8>,
        used_hw_mask: u32,
        has_calls: bool,
    ) -> Result<(), EvalError>;

    fn emit_epilogue(
        &mut self,
        buf: &mut Vec<u8>,
        used_hw_mask: u32,
        has_calls: bool,
    ) -> Result<(), EvalError>;

    fn emit_load(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        dst: HwReg,
        byte_offset: u32,
    ) -> Result<(), EvalError>;

    fn emit_store(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        src: HwReg,
        byte_offset: u32,
    ) -> Result<(), EvalError>;

    fn emit_mov(&mut self, buf: &mut Vec<u8>, precision: FloatPrecision, dst: HwReg, src: HwReg);

    /// Materialize `+0.0` in `dst` without touching memory.
    fn emit_zero(&mut self, buf: &mut Vec<u8>, precision: FloatPrecision, dst: HwReg);

    fn emit_combine(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        kind: Combine,
        dst: HwReg,
        lhs: HwReg,
        rhs: HwReg,
    );

    fn emit_combine_mem(
        &mut self,
        _buf: &mut Vec<u8>,
        _precision: FloatPrecision,
        _kind: Combine,
        _dst: HwReg,
        _lhs: HwReg,
        _rhs_offset: u32,
    ) -> Result<(), EvalError> {
        Err(EvalError::InvalidProgram)
    }

    fn emit_div(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        dst: HwReg,
        lhs: HwReg,
        rhs: HwReg,
    );

    fn emit_div_mem(
        &mut self,
        _buf: &mut Vec<u8>,
        _precision: FloatPrecision,
        _dst: HwReg,
        _lhs: HwReg,
        _rhs_offset: u32,
    ) -> Result<(), EvalError> {
        Err(EvalError::InvalidProgram)
    }

    fn emit_neg(&mut self, buf: &mut Vec<u8>, precision: FloatPrecision, dst: HwReg, src: HwReg);

    fn emit_neg_mul(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        dst: HwReg,
        lhs: HwReg,
        rhs: HwReg,
    );

    fn emit_fused(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        fused: Fused,
        dst: HwReg,
        operands: [HwReg; 3],
    );

    fn emit_fused_mem(
        &mut self,
        _buf: &mut Vec<u8>,
        _precision: FloatPrecision,
        _fused: Fused,
        _dst: HwReg,
        _operand: FusedMemOperand,
    ) -> Result<(), EvalError> {
        Err(EvalError::InvalidProgram)
    }

    fn supports_inline_unary(&self, op: InlineUnary) -> bool;

    /// Whether an inline unary operation can read its operand directly from memory.
    fn supports_inline_unary_mem(&self, _op: InlineUnary) -> bool {
        false
    }

    fn emit_inline_unary(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        op: InlineUnary,
        dst: HwReg,
        src: HwReg,
    ) -> Result<(), EvalError>;

    fn emit_inline_unary_mem(
        &mut self,
        _buf: &mut Vec<u8>,
        _precision: FloatPrecision,
        _op: InlineUnary,
        _dst: HwReg,
        _src_offset: u32,
    ) -> Result<(), EvalError> {
        Err(EvalError::InvalidProgram)
    }

    fn supports_inline_binary(&self, _op: InlineBinary) -> bool {
        false
    }

    /// Number of architecture-reserved scratch FP registers required by `op`.
    fn inline_binary_scratch(&self, _op: InlineBinary) -> u8 {
        0
    }

    fn emit_inline_binary(
        &mut self,
        _buf: &mut Vec<u8>,
        _precision: FloatPrecision,
        _op: InlineBinary,
        _dst: HwReg,
        _lhs: HwReg,
        _rhs: HwReg,
    ) -> Result<(), EvalError> {
        Err(EvalError::InvalidProgram)
    }

    fn emit_call_reg(
        &mut self,
        buf: &mut Vec<u8>,
        trampoline_index: usize,
    ) -> Result<(), EvalError>;

    fn emit_call_pair(
        &mut self,
        buf: &mut Vec<u8>,
        trampoline_index: usize,
        second_byte_offset: u32,
    ) -> Result<(), EvalError>;

    fn emit_call_slots(
        &mut self,
        buf: &mut Vec<u8>,
        trampoline_index: usize,
        byte_offsets: &[u32],
    ) -> Result<(), EvalError>;
}
