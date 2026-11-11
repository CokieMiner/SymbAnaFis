//! `aarch64` AAPCS64 machine-code encoder supporting 64-bit/32-bit scalar FP
//! and 128-bit NEON SIMD batch (`v.2d` `f64x2` / `v.4s` `f32x4`) with hardware FMA.

use alloc::vec::Vec;

use crate::errors::EvalError;

use super::{ArchBackend, Combine, FloatPrecision, Fused, HwReg, InlineBinary, InlineUnary};

const LEAF_BASE_GPR: u8 = 0; // x0
const CALL_BASE_GPR: u8 = 19; // x19
const NEON_FMA_SCRATCH: HwReg = HwReg(31);

/// `aarch64` AAPCS64 scalar and 128-bit NEON batch encoder state.
#[derive(Clone, Copy, Debug)]
pub struct AArch64Backend {
    base_gpr: u8,
    vector: bool,
}

impl AArch64Backend {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            base_gpr: CALL_BASE_GPR,
            vector: false,
        }
    }

    /// Upgrade this encoder to 128-bit NEON batch mode (`2` `f64` lanes or `4` `f32` lanes).
    #[cfg(any(target_arch = "aarch64", test))]
    #[must_use]
    pub const fn into_batch(mut self, precision: FloatPrecision) -> (Self, usize) {
        self.vector = true;
        let lanes = match precision {
            FloatPrecision::F64 => 2,
            FloatPrecision::F32 => 4,
        };
        (self, lanes)
    }

    fn emit_fp_pairs(buf: &mut Vec<u8>, used_callee_mask: u32, is_load: bool, base_slot: u32) {
        let op = if is_load {
            0x6D40_03E0_u32
        } else {
            0x6D00_03E0_u32
        };
        let pairs: [(u32, u32, u32); 4] = [
            (0x0300, 8, 0),
            (0x0C00, 10, 1),
            (0x3000, 12, 2),
            (0xC000, 14, 3),
        ];
        for i in 0..4 {
            let idx = if is_load {
                3_usize.saturating_sub(i)
            } else {
                i
            };
            let Some(&(mask, rt, offset_slot)) = pairs.get(idx) else {
                continue;
            };
            if (used_callee_mask & mask) != 0 {
                let slot = base_slot.saturating_add(offset_slot);
                let inst = op | (slot << 16) | ((rt.saturating_add(1)) << 10) | rt;
                buf.extend_from_slice(&inst.to_le_bytes());
            }
        }
    }

    fn emit_mov_x9(buf: &mut Vec<u8>, byte_offset: u32) {
        let lo = byte_offset & 0xFFFF;
        let hi = (byte_offset >> 16) & 0xFFFF;
        buf.extend_from_slice(&(0xD280_0009_u32 | (lo << 5)).to_le_bytes());
        if hi != 0 {
            buf.extend_from_slice(&(0xF2A0_0009_u32 | (hi << 5)).to_le_bytes());
        }
    }

    fn emit_mem_base(
        self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        is_load: bool,
        reg: u8,
        byte_offset: u32,
    ) {
        let rn = u32::from(self.base_gpr) << 5;
        let (shift, imm_base, reg_base) = match (self.vector, precision, is_load) {
            (true, _, true) => (4_u32, 0x3DC0_0000_u32, 0x3CE9_6800_u32),
            (true, _, false) => (4, 0x3D80_0000, 0x3CA9_6800),
            (false, FloatPrecision::F64, true) => (3, 0xFD40_0000, 0xFC69_6800),
            (false, FloatPrecision::F64, false) => (3, 0xFD00_0000, 0xFC29_6800),
            (false, FloatPrecision::F32, true) => (2, 0xBD40_0000, 0xBC69_6800),
            (false, FloatPrecision::F32, false) => (2, 0xBD00_0000, 0xBC29_6800),
        };
        let scaled = byte_offset >> shift;
        let inst = if (byte_offset & (1_u32 << shift).saturating_sub(1)) == 0 && scaled <= 0x0FFF {
            imm_base | (scaled << 10) | rn | u32::from(reg)
        } else {
            Self::emit_mov_x9(buf, byte_offset);
            reg_base | rn | u32::from(reg)
        };
        buf.extend_from_slice(&inst.to_le_bytes());
    }

    fn emit_add_base(self, buf: &mut Vec<u8>, dst_gpr: u8, byte_offset: u32) {
        let rn = u32::from(self.base_gpr) << 5;
        let inst = if byte_offset <= 0x0FFF {
            0x9100_0000_u32 | (byte_offset << 10) | rn | u32::from(dst_gpr)
        } else {
            Self::emit_mov_x9(buf, byte_offset);
            0x8B09_0000_u32 | rn | u32::from(dst_gpr)
        };
        buf.extend_from_slice(&inst.to_le_bytes());
    }
}

impl ArchBackend for AArch64Backend {
    const ARG0: HwReg = HwReg(0);
    const ARG1: HwReg = HwReg(1);
    const ARG2: HwReg = HwReg(2);
    const CALL_SCRATCH: HwReg = HwReg(3);

    fn allocatable_mask(&self, _has_calls: bool, _scratch_regs: u8, _max_live: u32) -> u32 {
        if self.vector {
            // Reserve `v31` to resolve destructive NEON `fmla`/`fmls` operand aliasing.
            !NEON_FMA_SCRATCH.bit()
        } else {
            0xFFFF_FFFF
        }
    }

    fn callee_saved_mask(&self) -> u32 {
        // AAPCS64 preserves only the lower 64 bits (`d8..d15`) of `v8..v15`.
        if self.vector { 0 } else { 0x0000_FF00 }
    }

    fn configure_frame(&mut self, has_calls: bool, _max_byte_offset: u32) {
        self.base_gpr = if has_calls {
            CALL_BASE_GPR
        } else {
            LEAF_BASE_GPR
        };
    }

    fn non_destructive_binary(&self) -> bool {
        true
    }

    fn non_destructive_fused(&self) -> bool {
        !self.vector
    }

    fn has_hardware_fma(&self) -> bool {
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
                buf.extend_from_slice(&0xD101_03FF_u32.to_le_bytes()); // sub sp, sp, #64
                Self::emit_fp_pairs(buf, used_callee_mask, false, 0);
            }
            return Ok(());
        }
        buf.extend_from_slice(&0xA9BA_7BFD_u32.to_le_bytes()); // stp x29, x30, [sp, #-96]!
        buf.extend_from_slice(&0x9100_03FD_u32.to_le_bytes()); // mov x29, sp
        buf.extend_from_slice(&0xA901_53F3_u32.to_le_bytes()); // stp x19, x20, [sp, #16]
        Self::emit_fp_pairs(buf, used_callee_mask, false, 2);
        buf.extend_from_slice(&0xAA00_03F3_u32.to_le_bytes()); // mov x19, x0
        buf.extend_from_slice(&0xAA01_03F4_u32.to_le_bytes()); // mov x20, x1
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
                Self::emit_fp_pairs(buf, used_callee_mask, true, 0);
                buf.extend_from_slice(&0x9101_03FF_u32.to_le_bytes()); // add sp, sp, #64
            }
            buf.extend_from_slice(&0xD65F_03C0_u32.to_le_bytes()); // ret
            return Ok(());
        }
        Self::emit_fp_pairs(buf, used_callee_mask, true, 2);
        buf.extend_from_slice(&0xA941_53F3_u32.to_le_bytes()); // ldp x19, x20, [sp, #16]
        buf.extend_from_slice(&0xA8C6_7BFD_u32.to_le_bytes()); // ldp x29, x30, [sp], #96
        buf.extend_from_slice(&0xD65F_03C0_u32.to_le_bytes()); // ret
        Ok(())
    }

    fn emit_load(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        dst: HwReg,
        byte_offset: u32,
    ) -> Result<(), EvalError> {
        self.emit_mem_base(buf, precision, true, dst.0, byte_offset);
        Ok(())
    }

    fn emit_store(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        src: HwReg,
        byte_offset: u32,
    ) -> Result<(), EvalError> {
        self.emit_mem_base(buf, precision, false, src.0, byte_offset);
        Ok(())
    }

    fn emit_mov(&mut self, buf: &mut Vec<u8>, precision: FloatPrecision, dst: HwReg, src: HwReg) {
        if dst != src {
            let inst = if self.vector {
                0x4EA0_1C00_u32
                    | (u32::from(src.0) << 16)
                    | (u32::from(src.0) << 5)
                    | u32::from(dst.0)
            } else {
                let ftype = u32::from(matches!(precision, FloatPrecision::F64)) << 22;
                0x1E20_4000_u32 | ftype | (u32::from(src.0) << 5) | u32::from(dst.0)
            };
            buf.extend_from_slice(&inst.to_le_bytes());
        }
    }

    fn emit_zero(&mut self, buf: &mut Vec<u8>, _precision: FloatPrecision, dst: HwReg) {
        // `movi v<dst>.2d, #0` clears all 128 bits to `+0.0` in both scalar and NEON modes.
        let inst = 0x6F00_E400_u32 | u32::from(dst.0);
        buf.extend_from_slice(&inst.to_le_bytes());
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
        let base = if self.vector {
            match kind {
                Combine::Add => 0x4E20_D400_u32,
                Combine::Subtract => 0x4EA0_D400,
                Combine::Multiply => 0x6E20_DC00,
            }
        } else {
            match kind {
                Combine::Multiply => 0x1E20_0800_u32,
                Combine::Add => 0x1E20_2800,
                Combine::Subtract => 0x1E20_3800,
            }
        };
        let ftype = u32::from(matches!(precision, FloatPrecision::F64)) << 22;
        let inst =
            base | ftype | (u32::from(rhs.0) << 16) | (u32::from(lhs.0) << 5) | u32::from(dst.0);
        buf.extend_from_slice(&inst.to_le_bytes());
    }

    fn emit_div(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        dst: HwReg,
        lhs: HwReg,
        rhs: HwReg,
    ) {
        let base = if self.vector {
            0x6E20_FC00_u32
        } else {
            0x1E20_1800
        };
        let ftype = u32::from(matches!(precision, FloatPrecision::F64)) << 22;
        let inst =
            base | ftype | (u32::from(rhs.0) << 16) | (u32::from(lhs.0) << 5) | u32::from(dst.0);
        buf.extend_from_slice(&inst.to_le_bytes());
    }

    fn emit_neg(&mut self, buf: &mut Vec<u8>, precision: FloatPrecision, dst: HwReg, src: HwReg) {
        let base = if self.vector {
            0x6EA0_F800_u32
        } else {
            0x1E21_4000
        };
        let ftype = u32::from(matches!(precision, FloatPrecision::F64)) << 22;
        let inst = base | ftype | (u32::from(src.0) << 5) | u32::from(dst.0);
        buf.extend_from_slice(&inst.to_le_bytes());
    }

    fn emit_neg_mul(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        dst: HwReg,
        lhs: HwReg,
        rhs: HwReg,
    ) {
        if self.vector {
            self.emit_combine(buf, precision, Combine::Multiply, dst, lhs, rhs);
            self.emit_neg(buf, precision, dst, dst);
        } else {
            let ftype = u32::from(matches!(precision, FloatPrecision::F64)) << 22;
            let inst = 0x1E20_8800_u32
                | ftype
                | (u32::from(rhs.0) << 16)
                | (u32::from(lhs.0) << 5)
                | u32::from(dst.0);
            buf.extend_from_slice(&inst.to_le_bytes());
        }
    }

    fn emit_fused(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        fused: Fused,
        dst: HwReg,
        [a, b, c]: [HwReg; 3],
    ) {
        let ftype = u32::from(matches!(precision, FloatPrecision::F64)) << 22;
        if self.vector {
            // NEON `fmla`/`fmls` accumulate into `acc`: `(±c) ± (a * b)`.
            // Use `NEON_FMA_SCRATCH` (`v31`) whenever writing `c` into `dst` would clobber `a` or `b`.
            let acc = if dst != c && (dst == a || dst == b) {
                NEON_FMA_SCRATCH
            } else {
                dst
            };
            if fused.negate_addend {
                self.emit_neg(buf, precision, acc, c);
            } else {
                self.emit_mov(buf, precision, acc, c);
            }
            let fma_base = if fused.negate_product {
                0x4EA0_CC00_u32 // fmls acc, a, b
            } else {
                0x4E20_CC00_u32 // fmla acc, a, b
            };
            let inst = fma_base
                | ftype
                | (u32::from(b.0) << 16)
                | (u32::from(a.0) << 5)
                | u32::from(acc.0);
            buf.extend_from_slice(&inst.to_le_bytes());
            self.emit_mov(buf, precision, dst, acc);
            return;
        }
        let base = match (fused.negate_product, fused.negate_addend) {
            (false, false) => 0x1F00_0000_u32, // fmadd
            (false, true) => 0x1F20_8000,      // fnmsub
            (true, false) => 0x1F00_8000,      // fmsub
            (true, true) => 0x1F20_0000,       // fnmadd
        };
        let inst = base
            | ftype
            | (u32::from(b.0) << 16)
            | (u32::from(c.0) << 10)
            | (u32::from(a.0) << 5)
            | u32::from(dst.0);
        buf.extend_from_slice(&inst.to_le_bytes());
    }

    fn supports_inline_unary(&self, _op: InlineUnary) -> bool {
        true
    }

    fn emit_inline_unary(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        op: InlineUnary,
        dst: HwReg,
        src: HwReg,
    ) -> Result<(), EvalError> {
        let base = if self.vector {
            match op {
                InlineUnary::Abs => 0x4EA0_F800_u32,
                InlineUnary::Sqrt => 0x6EA1_F800,
                InlineUnary::Ceil => 0x4EA1_8800,  // frintp
                InlineUnary::Floor => 0x4E21_9800, // frintm
                InlineUnary::Trunc => 0x4EA1_9800, // frintz
                InlineUnary::Round => 0x6E21_8800, // frinta
            }
        } else {
            match op {
                InlineUnary::Abs => 0x1E20_C000_u32,
                InlineUnary::Sqrt => 0x1E21_C000,
                InlineUnary::Ceil => 0x1E24_C000,  // frintp
                InlineUnary::Floor => 0x1E25_4000, // frintm
                InlineUnary::Trunc => 0x1E25_C000, // frintz
                InlineUnary::Round => 0x1E26_4000, // frinta
            }
        };
        let ftype = u32::from(matches!(precision, FloatPrecision::F64)) << 22;
        let inst = base | ftype | (u32::from(src.0) << 5) | u32::from(dst.0);
        buf.extend_from_slice(&inst.to_le_bytes());
        Ok(())
    }

    fn supports_inline_binary(&self, _op: InlineBinary) -> bool {
        true
    }

    fn emit_inline_binary(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        op: InlineBinary,
        dst: HwReg,
        lhs: HwReg,
        rhs: HwReg,
    ) -> Result<(), EvalError> {
        // ARMv8 `fmax`/`fmin` match IEEE 754-2019 (propagate NaN and order `-0.0 < +0.0`).
        let base = if self.vector {
            match op {
                InlineBinary::Max => 0x4E20_F400_u32,
                InlineBinary::Min => 0x4EA0_F400,
            }
        } else {
            match op {
                InlineBinary::Max => 0x1E20_4800_u32,
                InlineBinary::Min => 0x1E20_5800,
            }
        };
        let ftype = u32::from(matches!(precision, FloatPrecision::F64)) << 22;
        let inst =
            base | ftype | (u32::from(rhs.0) << 16) | (u32::from(lhs.0) << 5) | u32::from(dst.0);
        buf.extend_from_slice(&inst.to_le_bytes());
        Ok(())
    }

    fn emit_call_reg(
        &mut self,
        buf: &mut Vec<u8>,
        trampoline_index: usize,
    ) -> Result<(), EvalError> {
        let idx = u32::try_from(trampoline_index)
            .ok()
            .filter(|&val| val <= 0x0FFF)
            .ok_or(EvalError::ProgramTooLarge)?;
        buf.extend_from_slice(&(0xF940_0289_u32 | (idx << 10)).to_le_bytes());
        buf.extend_from_slice(&0xD63F_0120_u32.to_le_bytes());
        Ok(())
    }

    fn emit_call_pair(
        &mut self,
        buf: &mut Vec<u8>,
        trampoline_index: usize,
        second_byte_offset: u32,
    ) -> Result<(), EvalError> {
        self.emit_add_base(buf, 0, second_byte_offset);
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
        for (gpr, &offset) in byte_offsets.iter().enumerate() {
            let reg = u8::try_from(gpr).ok().ok_or(EvalError::InvalidProgram)?;
            self.emit_add_base(buf, reg, offset);
        }
        self.emit_call_reg(buf, trampoline_index)
    }
}
