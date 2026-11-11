//! `riscv64` LP64D machine-code encoder with standard `F`/`D` extensions and hardware FMA.

use alloc::vec::Vec;

use crate::errors::EvalError;

use super::{ArchBackend, Combine, FloatPrecision, Fused, HwReg, InlineUnary};

const CALLEE_FPREGS: [u8; 12] = [8, 9, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27];
const LEAF_BASE_GPR: u8 = 10; // a0 (x10)
const CALL_BASE_GPR: u8 = 8; // s0 (x8)
const LARGE_BASE_BIAS: i32 = 2040;

/// `riscv64` LP64D encoder state.
#[derive(Clone, Copy, Debug)]
pub struct Riscv64Backend {
    base_gpr: u8,
    base_bias: i32,
    cached_hi20: Option<i32>,
}

impl Riscv64Backend {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            base_gpr: CALL_BASE_GPR,
            base_bias: 0,
            cached_hi20: None,
        }
    }

    fn resolve_base_offset(
        &mut self,
        buf: &mut Vec<u8>,
        byte_offset: u32,
    ) -> Result<(u8, i32), EvalError> {
        let rel = i64::from(byte_offset).wrapping_sub(i64::from(self.base_bias));
        let off = i32::try_from(rel)
            .ok()
            .filter(|&val| val <= i32::MAX - 0x800)
            .ok_or(EvalError::ProgramTooLarge)?;
        if (-2048..2048).contains(&off) {
            Ok((self.base_gpr, off))
        } else {
            let hi20 = off.wrapping_add(0x800) >> 12;
            let lo12 = off.wrapping_sub(hi20 << 12);
            if self.cached_hi20 != Some(hi20) {
                let hi_u = u32::from_le_bytes(hi20.to_le_bytes()) & 0x000F_FFFF;
                let lui = (hi_u << 12) | (5_u32 << 7) | 0x37;
                let add = (5_u32 << 20) | (u32::from(self.base_gpr) << 15) | (5_u32 << 7) | 0x33;
                buf.extend_from_slice(&lui.to_le_bytes());
                buf.extend_from_slice(&add.to_le_bytes());
                self.cached_hi20 = Some(hi20);
            }
            Ok((5, lo12))
        }
    }

    fn emit_callee_fpregs(
        buf: &mut Vec<u8>,
        used_callee_mask: u32,
        is_load: bool,
        base_offset: usize,
    ) {
        for (i, &freg) in CALLEE_FPREGS.iter().enumerate() {
            if (used_callee_mask & (1_u32 << u32::from(freg))) != 0 {
                let off =
                    i32::try_from(i.saturating_mul(8).saturating_add(base_offset)).unwrap_or(0);
                if is_load {
                    Self::emit_i(buf, 0x07, freg, 3, 2, off);
                } else {
                    Self::emit_s(buf, 0x27, 3, 2, freg, off);
                }
            }
        }
    }

    fn emit_r_fp(
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        funct7_base: u8,
        rs2: u8,
        rs1: u8,
        rm: u8,
        rd: u8,
    ) {
        let funct7 = u32::from(funct7_base) | u32::from(matches!(precision, FloatPrecision::F64));
        let inst = (funct7 << 25)
            | (u32::from(rs2) << 20)
            | (u32::from(rs1) << 15)
            | (u32::from(rm) << 12)
            | (u32::from(rd) << 7)
            | 0x53;
        buf.extend_from_slice(&inst.to_le_bytes());
    }

    fn emit_i(buf: &mut Vec<u8>, opcode: u8, rd: u8, funct3: u8, rs1: u8, imm12: i32) {
        let imm_u = u32::from_le_bytes(imm12.to_le_bytes()) & 0x0FFF;
        let inst = (imm_u << 20)
            | (u32::from(rs1) << 15)
            | (u32::from(funct3) << 12)
            | (u32::from(rd) << 7)
            | u32::from(opcode);
        buf.extend_from_slice(&inst.to_le_bytes());
    }

    fn emit_s(buf: &mut Vec<u8>, opcode: u8, funct3: u8, rs1: u8, rs2: u8, imm12: i32) {
        let imm_u = u32::from_le_bytes(imm12.to_le_bytes()) & 0x0FFF;
        let hi = (imm_u >> 5) & 0x7F;
        let lo = imm_u & 0x1F;
        let inst = (hi << 25)
            | (u32::from(rs2) << 20)
            | (u32::from(rs1) << 15)
            | (u32::from(funct3) << 12)
            | (lo << 7)
            | u32::from(opcode);
        buf.extend_from_slice(&inst.to_le_bytes());
    }
}

impl ArchBackend for Riscv64Backend {
    const ARG0: HwReg = HwReg(10);
    const ARG1: HwReg = HwReg(11);
    const ARG2: HwReg = HwReg(12);
    const CALL_SCRATCH: HwReg = HwReg(13); // fa3 (f13), volatile in LP64D

    fn allocatable_mask(&self, _has_calls: bool, _scratch_regs: u8, _max_live: u32) -> u32 {
        0xFFFF_FFFF
    }

    fn callee_saved_mask(&self) -> u32 {
        (1_u32 << 8) | (1_u32 << 9) | (0x03FF_u32 << 18)
    }

    fn configure_frame(&mut self, has_calls: bool, max_byte_offset: u32) {
        self.base_gpr = if has_calls {
            CALL_BASE_GPR
        } else {
            LEAF_BASE_GPR
        };
        self.base_bias = if max_byte_offset >= 2048 {
            LARGE_BASE_BIAS
        } else {
            0
        };
        self.cached_hi20 = None;
    }

    fn non_destructive_binary(&self) -> bool {
        true
    }

    fn non_destructive_fused(&self) -> bool {
        true
    }

    fn emit_prologue(
        &mut self,
        buf: &mut Vec<u8>,
        used_hw_mask: u32,
        has_calls: bool,
    ) -> Result<(), EvalError> {
        let used_callee_mask = used_hw_mask & self.callee_saved_mask();
        if !has_calls {
            if used_callee_mask != 0 {
                Self::emit_i(buf, 0x13, 2, 0, 2, -96);
                Self::emit_callee_fpregs(buf, used_callee_mask, false, 0);
            }
            if self.base_bias != 0 {
                Self::emit_i(buf, 0x13, LEAF_BASE_GPR, 0, LEAF_BASE_GPR, self.base_bias);
            }
            return Ok(());
        }
        Self::emit_i(buf, 0x13, 2, 0, 2, -128);
        Self::emit_s(buf, 0x23, 3, 2, 1, 0);
        Self::emit_s(buf, 0x23, 3, 2, 9, 16);
        Self::emit_s(buf, 0x23, 3, 2, 8, 8);
        Self::emit_callee_fpregs(buf, used_callee_mask, false, 24);
        Self::emit_i(buf, 0x13, CALL_BASE_GPR, 0, 10, self.base_bias);
        Self::emit_i(buf, 0x13, 9, 0, 11, 0);
        Ok(())
    }

    fn emit_epilogue(
        &mut self,
        buf: &mut Vec<u8>,
        used_hw_mask: u32,
        has_calls: bool,
    ) -> Result<(), EvalError> {
        let used_callee_mask = used_hw_mask & self.callee_saved_mask();
        if !has_calls {
            if used_callee_mask != 0 {
                Self::emit_callee_fpregs(buf, used_callee_mask, true, 0);
                Self::emit_i(buf, 0x13, 2, 0, 2, 96);
            }
            buf.extend_from_slice(&0x0000_8067_u32.to_le_bytes());
            return Ok(());
        }
        Self::emit_callee_fpregs(buf, used_callee_mask, true, 24);
        Self::emit_i(buf, 0x03, 1, 3, 2, 0);
        Self::emit_i(buf, 0x03, 9, 3, 2, 16);
        Self::emit_i(buf, 0x03, 8, 3, 2, 8);
        Self::emit_i(buf, 0x13, 2, 0, 2, 128);
        buf.extend_from_slice(&0x0000_8067_u32.to_le_bytes());
        Ok(())
    }

    fn emit_load(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        dst: HwReg,
        byte_offset: u32,
    ) -> Result<(), EvalError> {
        let funct3 = if matches!(precision, FloatPrecision::F64) {
            3
        } else {
            2
        };
        let (base_gpr, lo12) = self.resolve_base_offset(buf, byte_offset)?;
        Self::emit_i(buf, 0x07, dst.0, funct3, base_gpr, lo12);
        Ok(())
    }

    fn emit_store(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        src: HwReg,
        byte_offset: u32,
    ) -> Result<(), EvalError> {
        let funct3 = if matches!(precision, FloatPrecision::F64) {
            3
        } else {
            2
        };
        let (base_gpr, lo12) = self.resolve_base_offset(buf, byte_offset)?;
        Self::emit_s(buf, 0x27, funct3, base_gpr, src.0, lo12);
        Ok(())
    }

    fn emit_mov(&mut self, buf: &mut Vec<u8>, precision: FloatPrecision, dst: HwReg, src: HwReg) {
        if dst != src {
            Self::emit_r_fp(buf, precision, 0x10, src.0, src.0, 0, dst.0);
        }
    }

    fn emit_zero(&mut self, buf: &mut Vec<u8>, precision: FloatPrecision, dst: HwReg) {
        // `fmv.d.x fd, x0` / `fmv.w.x fd, x0` materializes `+0.0`.
        Self::emit_r_fp(buf, precision, 0x78, 0, 0, 0, dst.0);
    }

    fn emit_combine(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        kind: Combine,
        dst: HwReg,
        lhs: HwReg,
        rhs: HwReg,
    ) {
        let funct7 = match kind {
            Combine::Add => 0x00,
            Combine::Subtract => 0x04,
            Combine::Multiply => 0x08,
        };
        Self::emit_r_fp(buf, precision, funct7, rhs.0, lhs.0, 7, dst.0);
    }

    fn emit_div(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        dst: HwReg,
        lhs: HwReg,
        rhs: HwReg,
    ) {
        Self::emit_r_fp(buf, precision, 0x0C, rhs.0, lhs.0, 7, dst.0);
    }

    fn emit_neg(&mut self, buf: &mut Vec<u8>, precision: FloatPrecision, dst: HwReg, src: HwReg) {
        Self::emit_r_fp(buf, precision, 0x10, src.0, src.0, 1, dst.0);
    }

    fn emit_neg_mul(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        dst: HwReg,
        lhs: HwReg,
        rhs: HwReg,
    ) {
        Self::emit_r_fp(buf, precision, 0x08, rhs.0, lhs.0, 7, dst.0);
        Self::emit_r_fp(buf, precision, 0x10, dst.0, dst.0, 1, dst.0);
    }

    fn emit_fused(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        fused: Fused,
        dst: HwReg,
        [a, b, c]: [HwReg; 3],
    ) {
        let opcode = match (fused.negate_product, fused.negate_addend) {
            (false, false) => 0x43_u32,
            (false, true) => 0x47,
            (true, false) => 0x4B,
            (true, true) => 0x4F,
        };
        let fmt = u32::from(matches!(precision, FloatPrecision::F64));
        let inst = (u32::from(c.0) << 27)
            | (fmt << 25)
            | (u32::from(b.0) << 20)
            | (u32::from(a.0) << 15)
            | (7_u32 << 12)
            | (u32::from(dst.0) << 7)
            | opcode;
        buf.extend_from_slice(&inst.to_le_bytes());
    }

    fn supports_inline_unary(&self, op: InlineUnary) -> bool {
        matches!(op, InlineUnary::Sqrt | InlineUnary::Abs)
    }

    fn emit_inline_unary(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        op: InlineUnary,
        dst: HwReg,
        src: HwReg,
    ) -> Result<(), EvalError> {
        match op {
            InlineUnary::Sqrt => Self::emit_r_fp(buf, precision, 0x2C, 0, src.0, 7, dst.0),
            InlineUnary::Abs => Self::emit_r_fp(buf, precision, 0x10, src.0, src.0, 2, dst.0),
            InlineUnary::Floor | InlineUnary::Ceil | InlineUnary::Trunc | InlineUnary::Round => {
                return Err(EvalError::InvalidProgram);
            }
        }
        Ok(())
    }

    fn emit_call_reg(
        &mut self,
        buf: &mut Vec<u8>,
        trampoline_index: usize,
    ) -> Result<(), EvalError> {
        self.cached_hi20 = None;
        let byte_off = i32::try_from(trampoline_index)
            .ok()
            .and_then(|idx| idx.checked_mul(8))
            .filter(|&off| off < 2048)
            .ok_or(EvalError::ProgramTooLarge)?;
        Self::emit_i(buf, 0x03, 5, 3, 9, byte_off);
        Self::emit_i(buf, 0x67, 1, 0, 5, 0);
        Ok(())
    }

    fn emit_call_pair(
        &mut self,
        buf: &mut Vec<u8>,
        trampoline_index: usize,
        second_byte_offset: u32,
    ) -> Result<(), EvalError> {
        let (base, lo12) = self.resolve_base_offset(buf, second_byte_offset)?;
        Self::emit_i(buf, 0x13, 10, 0, base, lo12);
        self.emit_call_reg(buf, trampoline_index)
    }

    fn emit_call_slots(
        &mut self,
        buf: &mut Vec<u8>,
        trampoline_index: usize,
        byte_offsets: &[u32],
    ) -> Result<(), EvalError> {
        if byte_offsets.len() > 8 {
            return Err(EvalError::InvalidProgram);
        }
        for (idx, &offset) in byte_offsets.iter().enumerate() {
            let gpr = u8::try_from(idx.saturating_add(10))
                .ok()
                .ok_or(EvalError::InvalidProgram)?;
            let (base, lo12) = self.resolve_base_offset(buf, offset)?;
            Self::emit_i(buf, 0x13, gpr, 0, base, lo12);
        }
        self.emit_call_reg(buf, trampoline_index)
    }
}
