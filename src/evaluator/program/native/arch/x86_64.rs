//! `x86_64` machine-code encoder supporting System V AMD64 (Linux/macOS) and Win64 (Windows)
//! across SSE2, AVX+FMA (128-bit scalar and 256-bit `ymm` batch), and AVX-512 EVEX
//! (compressed `disp8*N` scalar, adaptive `xmm16..xmm31` expansion under high pressure,
//! and 32-register 512-bit `zmm` batch) with memory-operand folding, zero-stack leaf
//! frames, and lazy sign-mask caching.

use std::arch::is_x86_feature_detected;

use alloc::vec::Vec;

use crate::errors::EvalError;

use super::{
    ArchBackend, Combine, FloatPrecision, Fused, FusedMemOperand, HwReg, InlineBinary, InlineUnary,
};

const OP_MOV_LOAD: u8 = 0x10;
const OP_MOV_STORE: u8 = 0x11;
const OP_MOVAPD: u8 = 0x28;
const OP_SQRT: u8 = 0x51;
const OP_ANDPD: u8 = 0x54;
const OP_ORPD: u8 = 0x56;
const OP_XORPD: u8 = 0x57;
const OP_ADD: u8 = 0x58;
const OP_MUL: u8 = 0x59;
const OP_SUB: u8 = 0x5C;
const OP_MIN: u8 = 0x5D;
const OP_DIV: u8 = 0x5E;
const OP_MAX: u8 = 0x5F;
const OP_CMP: u8 = 0xC2;

const VEX_PP_NP: u8 = 0x00;
const VEX_PP_66: u8 = 0x01;
const VEX_MAP_0F: u8 = 0x01;
const VEX_MAP_0F38: u8 = 0x02;
const VEX_MAP_0F3A: u8 = 0x03;

const SCRATCH: HwReg = HwReg(15);
const SCRATCH2: HwReg = HwReg(14);

#[cfg(target_os = "windows")]
const LEAF_BASE_GPR: u8 = 1; // rcx
#[cfg(not(target_os = "windows"))]
const LEAF_BASE_GPR: u8 = 7; // rdi
const CALL_BASE_GPR: u8 = 3; // rbx

pub const FEAT_SSE41: u8 = 1 << 0;
pub const FEAT_AVX: u8 = 1 << 1;
pub const FEAT_AVX2: u8 = 1 << 2;
pub const FEAT_FMA: u8 = 1 << 3;
pub const FEAT_AVX512: u8 = 1 << 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VectorWidth {
    Scalar,
    Vec256,
    Vec512,
}

/// `x86_64` SSE2 / AVX+FMA / AVX-512 encoder state for scalar and SIMD batch kernels.
#[derive(Clone, Copy, Debug)]
pub struct X86_64Backend {
    features: u8,
    width: VectorWidth,
    base_gpr: u8,
    base_bias: i32,
    scratch_mask: Option<(bool, FloatPrecision)>,
    used_scratch_mask: u32,
}

impl X86_64Backend {
    #[must_use]
    pub fn new() -> Self {
        let has_avx = cfg!(target_feature = "avx") || is_x86_feature_detected!("avx");
        let has_avx2 =
            has_avx && (cfg!(target_feature = "avx2") || is_x86_feature_detected!("avx2"));
        let has_fma = has_avx && (cfg!(target_feature = "fma") || is_x86_feature_detected!("fma"));
        let has_sse41 =
            has_avx || cfg!(target_feature = "sse4.1") || is_x86_feature_detected!("sse4.1");
        let has_avx512 = has_avx2
            && has_fma
            && (cfg!(target_feature = "avx512f") || is_x86_feature_detected!("avx512f"))
            && (cfg!(target_feature = "avx512vl") || is_x86_feature_detected!("avx512vl"))
            && (cfg!(target_feature = "avx512dq") || is_x86_feature_detected!("avx512dq"));
        let features = if has_sse41 { FEAT_SSE41 } else { 0 }
            | if has_avx { FEAT_AVX } else { 0 }
            | if has_avx2 { FEAT_AVX2 } else { 0 }
            | if has_fma { FEAT_FMA } else { 0 }
            | if has_avx512 { FEAT_AVX512 } else { 0 };
        Self {
            features,
            width: VectorWidth::Scalar,
            base_gpr: CALL_BASE_GPR,
            base_bias: 0,
            scratch_mask: None,
            used_scratch_mask: 0,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub const fn with_features(features: u8) -> Self {
        Self {
            features,
            width: VectorWidth::Scalar,
            base_gpr: CALL_BASE_GPR,
            base_bias: 0,
            scratch_mask: None,
            used_scratch_mask: 0,
        }
    }

    #[must_use]
    pub const fn into_batch(mut self, precision: FloatPrecision) -> Option<(Self, usize)> {
        if (self.features & FEAT_AVX512) != 0 {
            self.width = VectorWidth::Vec512;
            let lanes = match precision {
                FloatPrecision::F64 => 8,
                FloatPrecision::F32 => 16,
            };
            Some((self, lanes))
        } else if (self.features & FEAT_AVX2) != 0 {
            self.width = VectorWidth::Vec256;
            let lanes = match precision {
                FloatPrecision::F64 => 4,
                FloatPrecision::F32 => 8,
            };
            Some((self, lanes))
        } else {
            None
        }
    }

    fn rel_disp(&self, byte_offset: u32) -> Result<i32, EvalError> {
        i32::try_from(i64::from(byte_offset).wrapping_sub(i64::from(self.base_bias)))
            .ok()
            .ok_or(EvalError::ProgramTooLarge)
    }

    const fn evex_disp8_scale(&self, precision: FloatPrecision) -> i32 {
        match (self.width, precision) {
            (VectorWidth::Scalar, FloatPrecision::F64) => 8,
            (VectorWidth::Scalar, FloatPrecision::F32) => 4,
            (VectorWidth::Vec256, _) => 32,
            (VectorWidth::Vec512, _) => 64,
        }
    }

    const fn evex_ll(&self) -> u8 {
        match self.width {
            VectorWidth::Scalar => 0,
            VectorWidth::Vec256 => 1,
            VectorWidth::Vec512 => 2,
        }
    }

    const fn packed_pp(precision: FloatPrecision) -> u8 {
        match precision {
            FloatPrecision::F64 => VEX_PP_66,
            FloatPrecision::F32 => VEX_PP_NP,
        }
    }

    const fn fp_pp(&self, precision: FloatPrecision) -> u8 {
        match (self.width, precision) {
            (VectorWidth::Scalar, FloatPrecision::F64) => 0x03, // F2
            (VectorWidth::Scalar, FloatPrecision::F32) => 0x02, // F3
            (_, _) => Self::packed_pp(precision),
        }
    }

    const fn fma_213_opcode(fused: Fused, width: VectorWidth) -> u8 {
        let scalar = match (fused.negate_product, fused.negate_addend) {
            (false, false) => 0xA9_u8,
            (false, true) => 0xAB,
            (true, false) => 0xAD,
            (true, true) => 0xAF,
        };
        if matches!(width, VectorWidth::Scalar) {
            scalar
        } else {
            scalar.saturating_sub(1)
        }
    }

    fn emit_rr(
        &self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        [pp, map, vex_w, opcode]: [u8; 4],
        [reg, vvvv, rm]: [u8; 3],
    ) {
        let needs_evex =
            matches!(self.width, VectorWidth::Vec512) || reg >= 16 || vvvv >= 16 || rm >= 16;
        if needs_evex {
            self.emit_evex_rr(buf, precision, [pp, map, opcode], [reg, vvvv, rm], 0);
            return;
        }
        let r_inv = ((reg >> 3) ^ 1) & 1;
        let v_inv = (!vvvv) & 0x0F;
        let modrm = 0xC0 | ((reg & 7) << 3) | (rm & 7);
        let l = u8::from(matches!(self.width, VectorWidth::Vec256));
        if map == VEX_MAP_0F && vex_w == 0 && rm < 8 {
            let byte1 = (r_inv << 7) | (v_inv << 3) | (l << 2) | pp;
            buf.extend_from_slice(&[0xC5, byte1, opcode, modrm]);
        } else {
            let b_inv = ((rm >> 3) ^ 1) & 1;
            let byte1 = (r_inv << 7) | (1 << 6) | (b_inv << 5) | map;
            let byte2 = (vex_w << 7) | (v_inv << 3) | (l << 2) | pp;
            buf.extend_from_slice(&[0xC4, byte1, byte2, opcode, modrm]);
        }
    }

    fn emit_evex_rr(
        &self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        [pp, map, opcode]: [u8; 3],
        [reg, vvvv, rm]: [u8; 3],
        mask_k: u8,
    ) {
        let w = u8::from(matches!(precision, FloatPrecision::F64));
        let r_inv = ((reg >> 3) ^ 1) & 1;
        let x_inv = ((rm >> 4) ^ 1) & 1;
        let b_inv = ((rm >> 3) ^ 1) & 1;
        let reg_hi_inv = ((reg >> 4) ^ 1) & 1;
        let v_inv = (!vvvv) & 0x0F;
        let vvvv_hi_inv = ((vvvv >> 4) ^ 1) & 1;
        let p0 = (r_inv << 7) | (x_inv << 6) | (b_inv << 5) | (reg_hi_inv << 4) | map;
        let p1 = (w << 7) | (v_inv << 3) | (1 << 2) | pp;
        let p2 = (self.evex_ll() << 5) | (vvvv_hi_inv << 3) | (mask_k & 7);
        let modrm = 0xC0 | ((reg & 7) << 3) | (rm & 7);
        buf.extend_from_slice(&[0x62, p0, p1, p2, opcode, modrm]);
    }

    /// Compute `{op(lhs, rhs), op(rhs, lhs)}` into `{SCRATCH, dst}` without clobbering aliases.
    fn emit_commuted_extrema(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        opcode: u8,
        dst: HwReg,
        lhs: HwReg,
        rhs: HwReg,
    ) {
        if (self.features & FEAT_AVX) != 0 {
            let fp_pp = self.fp_pp(precision);
            self.emit_rr(
                buf,
                precision,
                [fp_pp, VEX_MAP_0F, 0, opcode],
                [SCRATCH.0, lhs.0, rhs.0],
            );
            self.emit_rr(
                buf,
                precision,
                [fp_pp, VEX_MAP_0F, 0, opcode],
                [dst.0, rhs.0, lhs.0],
            );
        } else if dst == lhs {
            self.emit_mov(buf, precision, SCRATCH, rhs);
            Self::emit_sse_rr(buf, precision, opcode, SCRATCH.0, lhs.0);
            Self::emit_sse_rr(buf, precision, opcode, dst.0, rhs.0);
        } else if dst == rhs {
            self.emit_mov(buf, precision, SCRATCH, lhs);
            Self::emit_sse_rr(buf, precision, opcode, SCRATCH.0, rhs.0);
            Self::emit_sse_rr(buf, precision, opcode, dst.0, lhs.0);
        } else {
            self.emit_mov(buf, precision, SCRATCH, lhs);
            Self::emit_sse_rr(buf, precision, opcode, SCRATCH.0, rhs.0);
            self.emit_mov(buf, precision, dst, rhs);
            Self::emit_sse_rr(buf, precision, opcode, dst.0, lhs.0);
        }
    }

    fn emit_mem(
        &self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        [pp, map, vex_w, opcode]: [u8; 4],
        [reg, vvvv]: [u8; 2],
        rel_disp: i32,
    ) {
        let r_inv = ((reg >> 3) ^ 1) & 1;
        let v_inv = (!vvvv) & 0x0F;
        let scale = self.evex_disp8_scale(precision);
        let fits_evex_disp8 = rel_disp.checked_rem(scale) == Some(0)
            && rel_disp
                .checked_div(scale)
                .is_some_and(|scaled| i8::try_from(scaled).is_ok());
        let use_evex = matches!(self.width, VectorWidth::Vec512)
            || reg >= 16
            || vvvv >= 16
            || ((self.features & FEAT_AVX512) != 0
                && i8::try_from(rel_disp).is_err()
                && fits_evex_disp8);
        if use_evex {
            let w = u8::from(matches!(precision, FloatPrecision::F64));
            let reg_hi_inv = ((reg >> 4) ^ 1) & 1;
            let vvvv_hi_inv = ((vvvv >> 4) ^ 1) & 1;
            let p0 = (r_inv << 7) | (1 << 6) | (1 << 5) | (reg_hi_inv << 4) | map;
            let p1 = (w << 7) | (v_inv << 3) | (1 << 2) | pp;
            let p2 = (self.evex_ll() << 5) | (vvvv_hi_inv << 3);
            buf.extend_from_slice(&[0x62, p0, p1, p2, opcode]);
            Self::emit_evex_base_disp(buf, reg, self.base_gpr, rel_disp, scale);
        } else {
            let l = u8::from(matches!(self.width, VectorWidth::Vec256));
            if map == VEX_MAP_0F && vex_w == 0 {
                let byte1 = (r_inv << 7) | (v_inv << 3) | (l << 2) | pp;
                buf.extend_from_slice(&[0xC5, byte1, opcode]);
            } else {
                let byte1 = (r_inv << 7) | (1 << 6) | (1 << 5) | map;
                let byte2 = (vex_w << 7) | (v_inv << 3) | (l << 2) | pp;
                buf.extend_from_slice(&[0xC4, byte1, byte2, opcode]);
            }
            Self::emit_base_disp(buf, reg, self.base_gpr, rel_disp);
        }
    }

    const fn round_opcode(&self, precision: FloatPrecision, scalar_vvvv: u8) -> (u8, u8) {
        match (self.width, precision) {
            (VectorWidth::Scalar, FloatPrecision::F64) => (0x0B_u8, scalar_vvvv),
            (VectorWidth::Scalar, FloatPrecision::F32) => (0x0A, scalar_vvvv),
            (_, FloatPrecision::F64) => (0x09, 0),
            (_, FloatPrecision::F32) => (0x08, 0),
        }
    }

    fn emit_round(&self, buf: &mut Vec<u8>, precision: FloatPrecision, dst: u8, src: u8, mode: u8) {
        let (opcode, vvvv) = self.round_opcode(precision, src);
        if (self.features & FEAT_AVX) != 0 {
            self.emit_rr(
                buf,
                precision,
                [VEX_PP_66, VEX_MAP_0F3A, 0, opcode],
                [dst, vvvv, src],
            );
            buf.push(mode);
        } else {
            buf.push(0x66);
            let rex = 0x40 | ((dst >> 3) << 2) | (src >> 3);
            if rex != 0x40 {
                buf.push(rex);
            }
            buf.extend_from_slice(&[
                0x0F,
                0x3A,
                opcode,
                0xC0 | ((dst & 7) << 3) | (src & 7),
                mode,
            ]);
        }
    }

    fn emit_round_mem(
        &self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        dst: u8,
        rel_disp: i32,
        mode: u8,
    ) {
        let (opcode, vvvv) = self.round_opcode(precision, dst);
        if (self.features & FEAT_AVX) != 0 {
            self.emit_mem(
                buf,
                precision,
                [VEX_PP_66, VEX_MAP_0F3A, 0, opcode],
                [dst, vvvv],
                rel_disp,
            );
        } else {
            buf.push(0x66);
            if dst >= 8 {
                buf.push(0x44);
            }
            buf.extend_from_slice(&[0x0F, 0x3A, opcode]);
            Self::emit_base_disp(buf, dst, self.base_gpr, rel_disp);
        }
        buf.push(mode);
    }

    fn ensure_scratch_mask(
        &mut self,
        buf: &mut Vec<u8>,
        is_sign_xor: bool,
        precision: FloatPrecision,
    ) {
        self.used_scratch_mask |= SCRATCH.bit();
        if self.scratch_mask == Some((is_sign_xor, precision)) {
            return;
        }
        self.scratch_mask = Some((is_sign_xor, precision));
        if matches!(self.width, VectorWidth::Vec512) {
            match (is_sign_xor, precision) {
                (true, FloatPrecision::F64) => {
                    buf.extend_from_slice(&[0x48, 0xB8]);
                    buf.extend_from_slice(&0x8000_0000_0000_0000_u64.to_le_bytes());
                    buf.extend_from_slice(&[0x62, 0x72, 0xFD, 0x48, 0x7C, 0xF8]);
                }
                (false, FloatPrecision::F64) => {
                    buf.extend_from_slice(&[0x48, 0xB8]);
                    buf.extend_from_slice(&0x7FFF_FFFF_FFFF_FFFF_u64.to_le_bytes());
                    buf.extend_from_slice(&[0x62, 0x72, 0xFD, 0x48, 0x7C, 0xF8]);
                }
                (true, FloatPrecision::F32) => {
                    buf.push(0xB8);
                    buf.extend_from_slice(&0x8000_0000_u32.to_le_bytes());
                    buf.extend_from_slice(&[0x62, 0x72, 0x7D, 0x48, 0x7C, 0xF8]);
                }
                (false, FloatPrecision::F32) => {
                    buf.push(0xB8);
                    buf.extend_from_slice(&0x7FFF_FFFF_u32.to_le_bytes());
                    buf.extend_from_slice(&[0x62, 0x72, 0x7D, 0x48, 0x7C, 0xF8]);
                }
            }
        } else if (self.features & FEAT_AVX) != 0 {
            let v2 = if matches!(self.width, VectorWidth::Vec256) {
                0x05_u8
            } else {
                0x01
            };
            buf.extend_from_slice(&[0xC4, 0x41, v2, 0x76, 0xFF]);
            match (is_sign_xor, precision) {
                (true, FloatPrecision::F64) => {
                    buf.extend_from_slice(&[0xC4, 0xC1, v2, 0x73, 0xF7, 0x3F]);
                }
                (true, FloatPrecision::F32) => {
                    buf.extend_from_slice(&[0xC4, 0xC1, v2, 0x72, 0xF7, 0x1F]);
                }
                (false, FloatPrecision::F64) => {
                    buf.extend_from_slice(&[0xC4, 0xC1, v2, 0x73, 0xD7, 0x01]);
                }
                (false, FloatPrecision::F32) => {
                    buf.extend_from_slice(&[0xC4, 0xC1, v2, 0x72, 0xD7, 0x01]);
                }
            }
        } else {
            buf.extend_from_slice(&[0x66, 0x45, 0x0F, 0x76, 0xFF]);
            match (is_sign_xor, precision) {
                (true, FloatPrecision::F64) => {
                    buf.extend_from_slice(&[0x66, 0x41, 0x0F, 0x73, 0xF7, 0x3F]);
                }
                (true, FloatPrecision::F32) => {
                    buf.extend_from_slice(&[0x66, 0x41, 0x0F, 0x72, 0xF7, 0x1F]);
                }
                (false, FloatPrecision::F64) => {
                    buf.extend_from_slice(&[0x66, 0x41, 0x0F, 0x73, 0xD7, 0x01]);
                }
                (false, FloatPrecision::F32) => {
                    buf.extend_from_slice(&[0x66, 0x41, 0x0F, 0x72, 0xD7, 0x01]);
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn emit_xmm_frame(buf: &mut Vec<u8>, save_mask: u32, base_disp: u32, is_load: bool) {
        let opcode = if is_load { 0x10_u8 } else { 0x11 };
        for xmm in 6_u8..=15 {
            if (save_mask & (1_u32 << u32::from(xmm))) != 0 {
                let disp = u32::from(xmm.saturating_sub(6))
                    .saturating_mul(16)
                    .saturating_add(base_disp);
                if xmm >= 8 {
                    buf.push(0x44);
                }
                buf.extend_from_slice(&[0x0F, opcode, 0x84 | ((xmm & 7) << 3), 0x24]);
                buf.extend_from_slice(&disp.to_le_bytes());
            }
        }
    }

    fn emit_sse_mem(
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        opcode: u8,
        reg: u8,
        base_gpr: u8,
        rel_disp: i32,
    ) {
        buf.push(match precision {
            FloatPrecision::F64 => 0xF2,
            FloatPrecision::F32 => 0xF3,
        });
        if reg >= 8 {
            buf.push(0x44);
        }
        buf.extend_from_slice(&[0x0F, opcode]);
        Self::emit_base_disp(buf, reg, base_gpr, rel_disp);
    }

    fn emit_sse_rr(buf: &mut Vec<u8>, precision: FloatPrecision, opcode: u8, dst: u8, src: u8) {
        buf.push(match precision {
            FloatPrecision::F64 => 0xF2,
            FloatPrecision::F32 => 0xF3,
        });
        let rex = 0x40 | ((dst >> 3) << 2) | (src >> 3);
        if rex != 0x40 {
            buf.push(rex);
        }
        buf.extend_from_slice(&[0x0F, opcode, 0xC0 | ((dst & 7) << 3) | (src & 7)]);
    }

    fn emit_sse_packed_rr(buf: &mut Vec<u8>, opcode: u8, dst: u8, src: u8) {
        buf.push(0x66);
        let rex = 0x40 | ((dst >> 3) << 2) | (src >> 3);
        if rex != 0x40 {
            buf.push(rex);
        }
        buf.extend_from_slice(&[0x0F, opcode, 0xC0 | ((dst & 7) << 3) | (src & 7)]);
    }

    fn emit_evex_base_disp(buf: &mut Vec<u8>, reg: u8, base_gpr: u8, rel_disp: i32, scale: i32) {
        let reg_bits = (reg & 7) << 3;
        if rel_disp == 0 {
            buf.push(reg_bits | base_gpr);
        } else if let Some(scaled) = rel_disp.checked_div(scale)
            && rel_disp.checked_rem(scale) == Some(0)
            && let Ok(disp8) = i8::try_from(scaled)
        {
            buf.extend_from_slice(&[0x40 | reg_bits | base_gpr, disp8.to_le_bytes()[0]]);
        } else {
            buf.push(0x80 | reg_bits | base_gpr);
            buf.extend_from_slice(&rel_disp.to_le_bytes());
        }
    }

    fn emit_base_disp(buf: &mut Vec<u8>, reg: u8, base_gpr: u8, rel_disp: i32) {
        let reg_bits = (reg & 7) << 3;
        if rel_disp == 0 {
            buf.push(reg_bits | base_gpr);
        } else if let Ok(disp8) = i8::try_from(rel_disp) {
            buf.extend_from_slice(&[0x40 | reg_bits | base_gpr, disp8.to_le_bytes()[0]]);
        } else {
            buf.push(0x80 | reg_bits | base_gpr);
            buf.extend_from_slice(&rel_disp.to_le_bytes());
        }
    }
}

impl ArchBackend for X86_64Backend {
    const ARG0: HwReg = HwReg(0);
    const ARG1: HwReg = HwReg(1);
    const ARG2: HwReg = HwReg(2);
    const CALL_SCRATCH: HwReg = HwReg(3);

    fn allocatable_mask(&self, _has_calls: bool, scratch_regs: u8, max_live: u32) -> u32 {
        let vex_cap = 16_u32.saturating_sub(u32::from(scratch_regs));
        // Unlock `xmm16..xmm31` in scalar kernels on AVX-512 hosts when register
        // pressure would otherwise spill to memory; stay in `xmm0..xmm15` for
        // low-pressure kernels to retain compact 2/3-byte VEX encodings.
        let use_evex_regs = matches!(self.width, VectorWidth::Vec512)
            || ((self.features & FEAT_AVX512) != 0 && max_live > vex_cap);
        let mut mask = if use_evex_regs {
            0xFFFF_FFFF_u32
        } else {
            0x0000_FFFF_u32
        };
        if scratch_regs >= 1 {
            mask &= !SCRATCH.bit();
        }
        if scratch_regs >= 2 {
            mask &= !SCRATCH2.bit();
        }
        mask
    }

    fn callee_saved_mask(&self) -> u32 {
        #[cfg(target_os = "windows")]
        {
            if matches!(self.width, VectorWidth::Scalar) {
                0x0000_FFC0
            } else {
                0
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            0
        }
    }

    fn configure_frame(&mut self, has_calls: bool, max_byte_offset: u32) {
        self.base_gpr = if has_calls {
            CALL_BASE_GPR
        } else {
            LEAF_BASE_GPR
        };
        self.base_bias = match self.width {
            VectorWidth::Scalar | VectorWidth::Vec256 => {
                if max_byte_offset <= 127 {
                    0
                } else {
                    128
                }
            }
            VectorWidth::Vec512 => {
                if max_byte_offset <= 8128 {
                    0
                } else {
                    8192
                }
            }
        };
    }

    fn non_destructive_binary(&self) -> bool {
        (self.features & FEAT_AVX) != 0
    }

    fn non_destructive_fused(&self) -> bool {
        (self.features & FEAT_FMA) != 0
    }

    fn supports_mem_operand(&self) -> bool {
        true
    }

    fn supports_fused_mem(&self) -> bool {
        (self.features & FEAT_FMA) != 0
    }

    fn emit_prologue(
        &mut self,
        buf: &mut Vec<u8>,
        used_hw_mask: u32,
        has_calls: bool,
    ) -> Result<(), EvalError> {
        #[cfg(target_os = "windows")]
        {
            let save_xmm_mask = (used_hw_mask | self.used_scratch_mask) & 0xFFC0;
            if has_calls {
                buf.extend_from_slice(&[0x53, 0x41, 0x54]);
                buf.extend_from_slice(&[0x48, 0x81, 0xEC, 200, 0, 0, 0]);
                Self::emit_xmm_frame(buf, save_xmm_mask, 32, false);
                if self.base_bias == 0 {
                    buf.extend_from_slice(&[0x48, 0x89, 0xCB]);
                } else {
                    buf.extend_from_slice(&[0x48, 0x8D, 0x99]);
                    buf.extend_from_slice(&self.base_bias.to_le_bytes());
                }
                buf.extend_from_slice(&[0x49, 0x89, 0xD4]);
            } else {
                if save_xmm_mask != 0 {
                    buf.extend_from_slice(&[0x48, 0x81, 0xEC, 168, 0, 0, 0]);
                    Self::emit_xmm_frame(buf, save_xmm_mask, 0, false);
                }
                if self.base_bias != 0 {
                    buf.extend_from_slice(&[0x48, 0x81, 0xC1]);
                    buf.extend_from_slice(&self.base_bias.to_le_bytes());
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (used_hw_mask, self.used_scratch_mask);
            if has_calls {
                buf.extend_from_slice(&[0x53, 0x41, 0x54, 0x48, 0x83, 0xEC, 0x08]);
                if self.base_bias == 0 {
                    buf.extend_from_slice(&[0x48, 0x89, 0xFB]);
                } else {
                    buf.extend_from_slice(&[0x48, 0x8D, 0x9F]);
                    buf.extend_from_slice(&self.base_bias.to_le_bytes());
                }
                buf.extend_from_slice(&[0x49, 0x89, 0xF4]);
            } else if self.base_bias != 0 {
                buf.extend_from_slice(&[0x48, 0x81, 0xC7]);
                buf.extend_from_slice(&self.base_bias.to_le_bytes());
            }
        }
        Ok(())
    }

    fn emit_epilogue(
        &mut self,
        buf: &mut Vec<u8>,
        used_hw_mask: u32,
        has_calls: bool,
    ) -> Result<(), EvalError> {
        if !matches!(self.width, VectorWidth::Scalar) {
            buf.extend_from_slice(&[0xC5, 0xF8, 0x77]);
        }
        #[cfg(target_os = "windows")]
        {
            let save_xmm_mask = (used_hw_mask | self.used_scratch_mask) & 0xFFC0;
            if has_calls {
                Self::emit_xmm_frame(buf, save_xmm_mask, 32, true);
                buf.extend_from_slice(&[0x48, 0x81, 0xC4, 200, 0, 0, 0]);
                buf.extend_from_slice(&[0x41, 0x5C, 0x5B, 0xC3]);
            } else {
                if save_xmm_mask != 0 {
                    Self::emit_xmm_frame(buf, save_xmm_mask, 0, true);
                    buf.extend_from_slice(&[0x48, 0x81, 0xC4, 168, 0, 0, 0]);
                }
                buf.push(0xC3);
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = used_hw_mask;
            if has_calls {
                buf.extend_from_slice(&[0x48, 0x83, 0xC4, 0x08, 0x41, 0x5C, 0x5B, 0xC3]);
            } else {
                buf.push(0xC3);
            }
        }
        Ok(())
    }

    fn emit_load(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        dst: HwReg,
        byte_offset: u32,
    ) -> Result<(), EvalError> {
        let rel = self.rel_disp(byte_offset)?;
        if (self.features & FEAT_AVX) != 0 {
            let pp = self.fp_pp(precision);
            self.emit_mem(
                buf,
                precision,
                [pp, VEX_MAP_0F, 0, OP_MOV_LOAD],
                [dst.0, 0],
                rel,
            );
        } else {
            Self::emit_sse_mem(buf, precision, OP_MOV_LOAD, dst.0, self.base_gpr, rel);
        }
        Ok(())
    }

    fn emit_store(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        src: HwReg,
        byte_offset: u32,
    ) -> Result<(), EvalError> {
        let rel = self.rel_disp(byte_offset)?;
        if (self.features & FEAT_AVX) != 0 {
            let pp = self.fp_pp(precision);
            self.emit_mem(
                buf,
                precision,
                [pp, VEX_MAP_0F, 0, OP_MOV_STORE],
                [src.0, 0],
                rel,
            );
        } else {
            Self::emit_sse_mem(buf, precision, OP_MOV_STORE, src.0, self.base_gpr, rel);
        }
        Ok(())
    }

    fn emit_mov(&mut self, buf: &mut Vec<u8>, precision: FloatPrecision, dst: HwReg, src: HwReg) {
        if dst != src {
            if (self.features & FEAT_AVX) != 0 {
                self.emit_rr(
                    buf,
                    precision,
                    [Self::packed_pp(precision), VEX_MAP_0F, 0, OP_MOVAPD],
                    [dst.0, 0, src.0],
                );
            } else {
                Self::emit_sse_packed_rr(buf, OP_MOVAPD, dst.0, src.0);
            }
        }
    }

    fn emit_zero(&mut self, buf: &mut Vec<u8>, precision: FloatPrecision, dst: HwReg) {
        if (self.features & FEAT_AVX) != 0 {
            self.emit_rr(
                buf,
                precision,
                [Self::packed_pp(precision), VEX_MAP_0F, 0, OP_XORPD],
                [dst.0, dst.0, dst.0],
            );
        } else {
            Self::emit_sse_packed_rr(buf, OP_XORPD, dst.0, dst.0);
        }
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
        let opcode = match kind {
            Combine::Add => OP_ADD,
            Combine::Multiply => OP_MUL,
            Combine::Subtract => OP_SUB,
        };
        if (self.features & FEAT_AVX) != 0 {
            let pp = self.fp_pp(precision);
            self.emit_rr(
                buf,
                precision,
                [pp, VEX_MAP_0F, 0, opcode],
                [dst.0, lhs.0, rhs.0],
            );
        } else if dst == rhs && dst != lhs && matches!(kind, Combine::Add | Combine::Multiply) {
            Self::emit_sse_rr(buf, precision, opcode, dst.0, lhs.0);
        } else {
            self.emit_mov(buf, precision, dst, lhs);
            Self::emit_sse_rr(buf, precision, opcode, dst.0, rhs.0);
        }
    }

    fn emit_combine_mem(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        kind: Combine,
        dst: HwReg,
        lhs: HwReg,
        rhs_offset: u32,
    ) -> Result<(), EvalError> {
        let rel = self.rel_disp(rhs_offset)?;
        let opcode = match kind {
            Combine::Add => OP_ADD,
            Combine::Multiply => OP_MUL,
            Combine::Subtract => OP_SUB,
        };
        if (self.features & FEAT_AVX) != 0 {
            let pp = self.fp_pp(precision);
            self.emit_mem(
                buf,
                precision,
                [pp, VEX_MAP_0F, 0, opcode],
                [dst.0, lhs.0],
                rel,
            );
        } else {
            self.emit_mov(buf, precision, dst, lhs);
            Self::emit_sse_mem(buf, precision, opcode, dst.0, self.base_gpr, rel);
        }
        Ok(())
    }

    fn emit_div(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        dst: HwReg,
        lhs: HwReg,
        rhs: HwReg,
    ) {
        if (self.features & FEAT_AVX) != 0 {
            let pp = self.fp_pp(precision);
            self.emit_rr(
                buf,
                precision,
                [pp, VEX_MAP_0F, 0, OP_DIV],
                [dst.0, lhs.0, rhs.0],
            );
        } else {
            self.emit_mov(buf, precision, dst, lhs);
            Self::emit_sse_rr(buf, precision, OP_DIV, dst.0, rhs.0);
        }
    }

    fn emit_div_mem(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        dst: HwReg,
        lhs: HwReg,
        rhs_offset: u32,
    ) -> Result<(), EvalError> {
        let rel = self.rel_disp(rhs_offset)?;
        if (self.features & FEAT_AVX) != 0 {
            let pp = self.fp_pp(precision);
            self.emit_mem(
                buf,
                precision,
                [pp, VEX_MAP_0F, 0, OP_DIV],
                [dst.0, lhs.0],
                rel,
            );
        } else {
            self.emit_mov(buf, precision, dst, lhs);
            Self::emit_sse_mem(buf, precision, OP_DIV, dst.0, self.base_gpr, rel);
        }
        Ok(())
    }

    fn emit_neg(&mut self, buf: &mut Vec<u8>, precision: FloatPrecision, dst: HwReg, src: HwReg) {
        self.ensure_scratch_mask(buf, true, precision);
        if (self.features & FEAT_AVX) != 0 {
            self.emit_rr(
                buf,
                precision,
                [Self::packed_pp(precision), VEX_MAP_0F, 0, OP_XORPD],
                [dst.0, src.0, SCRATCH.0],
            );
        } else {
            self.emit_mov(buf, precision, dst, src);
            Self::emit_sse_packed_rr(buf, OP_XORPD, dst.0, SCRATCH.0);
        }
    }

    fn emit_neg_mul(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        dst: HwReg,
        lhs: HwReg,
        rhs: HwReg,
    ) {
        self.emit_combine(buf, precision, Combine::Multiply, dst, lhs, rhs);
        self.emit_neg(buf, precision, dst, dst);
    }

    fn emit_fused(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        fused: Fused,
        dst: HwReg,
        [a, b, c]: [HwReg; 3],
    ) {
        if (self.features & FEAT_FMA) != 0 {
            let w = u8::from(matches!(precision, FloatPrecision::F64));
            let base_213 = Self::fma_213_opcode(fused, self.width);
            if dst == a {
                self.emit_rr(
                    buf,
                    precision,
                    [VEX_PP_66, VEX_MAP_0F38, w, base_213],
                    [dst.0, b.0, c.0],
                );
            } else if dst == b {
                self.emit_rr(
                    buf,
                    precision,
                    [VEX_PP_66, VEX_MAP_0F38, w, base_213],
                    [dst.0, a.0, c.0],
                );
            } else if dst == c {
                let op_231 = base_213.saturating_add(0x10);
                self.emit_rr(
                    buf,
                    precision,
                    [VEX_PP_66, VEX_MAP_0F38, w, op_231],
                    [dst.0, a.0, b.0],
                );
            } else {
                self.emit_mov(buf, precision, dst, a);
                self.emit_rr(
                    buf,
                    precision,
                    [VEX_PP_66, VEX_MAP_0F38, w, base_213],
                    [dst.0, b.0, c.0],
                );
            }
        } else {
            self.emit_combine(buf, precision, Combine::Multiply, dst, a, b);
            if fused.negate_product {
                self.emit_neg(buf, precision, dst, dst);
            }
            let add_kind = if fused.negate_addend {
                Combine::Subtract
            } else {
                Combine::Add
            };
            self.emit_combine(buf, precision, add_kind, dst, dst, c);
        }
    }

    fn emit_fused_mem(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        fused: Fused,
        dst: HwReg,
        operand: FusedMemOperand,
    ) -> Result<(), EvalError> {
        let w = u8::from(matches!(precision, FloatPrecision::F64));
        let base_213 = Self::fma_213_opcode(fused, self.width);
        match operand {
            FusedMemOperand::Addend {
                lhs,
                rhs,
                addend_offset,
            } => {
                let rel = self.rel_disp(addend_offset)?;
                let vvvv = if dst == lhs {
                    rhs
                } else if dst == rhs {
                    lhs
                } else {
                    self.emit_mov(buf, precision, dst, lhs);
                    rhs
                };
                self.emit_mem(
                    buf,
                    precision,
                    [VEX_PP_66, VEX_MAP_0F38, w, base_213],
                    [dst.0, vvvv.0],
                    rel,
                );
            }
            FusedMemOperand::Factor {
                factor,
                addend,
                mem_offset,
            } => {
                let rel = self.rel_disp(mem_offset)?;
                let (opcode, vvvv) = if dst == factor {
                    (base_213.saturating_sub(0x10), addend)
                } else if dst == addend {
                    (base_213.saturating_add(0x10), factor)
                } else {
                    self.emit_mov(buf, precision, dst, factor);
                    (base_213.saturating_sub(0x10), addend)
                };
                self.emit_mem(
                    buf,
                    precision,
                    [VEX_PP_66, VEX_MAP_0F38, w, opcode],
                    [dst.0, vvvv.0],
                    rel,
                );
            }
        }
        Ok(())
    }

    fn supports_inline_unary(&self, op: InlineUnary) -> bool {
        match op {
            InlineUnary::Sqrt | InlineUnary::Abs => true,
            InlineUnary::Floor | InlineUnary::Ceil | InlineUnary::Trunc => {
                (self.features & FEAT_SSE41) != 0
            }
            InlineUnary::Round => false,
        }
    }

    fn supports_inline_unary_mem(&self, op: InlineUnary) -> bool {
        // `Abs` uses packed `andpd`/`vandpd` (16-byte minimum load), so never fold scalar memory.
        match op {
            InlineUnary::Sqrt => true,
            InlineUnary::Floor | InlineUnary::Ceil | InlineUnary::Trunc => {
                (self.features & FEAT_SSE41) != 0
            }
            InlineUnary::Abs | InlineUnary::Round => false,
        }
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
            InlineUnary::Sqrt => {
                if (self.features & FEAT_AVX) != 0 {
                    let pp = self.fp_pp(precision);
                    let vvvv = if matches!(self.width, VectorWidth::Scalar) {
                        src.0
                    } else {
                        0
                    };
                    self.emit_rr(
                        buf,
                        precision,
                        [pp, VEX_MAP_0F, 0, OP_SQRT],
                        [dst.0, vvvv, src.0],
                    );
                } else {
                    Self::emit_sse_rr(buf, precision, OP_SQRT, dst.0, src.0);
                }
            }
            InlineUnary::Abs => {
                self.ensure_scratch_mask(buf, false, precision);
                if (self.features & FEAT_AVX) != 0 {
                    self.emit_rr(
                        buf,
                        precision,
                        [Self::packed_pp(precision), VEX_MAP_0F, 0, OP_ANDPD],
                        [dst.0, src.0, SCRATCH.0],
                    );
                } else {
                    self.emit_mov(buf, precision, dst, src);
                    Self::emit_sse_packed_rr(buf, OP_ANDPD, dst.0, SCRATCH.0);
                }
            }
            InlineUnary::Floor => self.emit_round(buf, precision, dst.0, src.0, 0x09),
            InlineUnary::Ceil => self.emit_round(buf, precision, dst.0, src.0, 0x0A),
            InlineUnary::Trunc => self.emit_round(buf, precision, dst.0, src.0, 0x0B),
            InlineUnary::Round => return Err(EvalError::InvalidProgram),
        }
        Ok(())
    }

    fn emit_inline_unary_mem(
        &mut self,
        buf: &mut Vec<u8>,
        precision: FloatPrecision,
        op: InlineUnary,
        dst: HwReg,
        src_offset: u32,
    ) -> Result<(), EvalError> {
        let rel = self.rel_disp(src_offset)?;
        match op {
            InlineUnary::Sqrt => {
                if (self.features & FEAT_AVX) != 0 {
                    let pp = self.fp_pp(precision);
                    let vvvv = if matches!(self.width, VectorWidth::Scalar) {
                        dst.0
                    } else {
                        0
                    };
                    self.emit_mem(
                        buf,
                        precision,
                        [pp, VEX_MAP_0F, 0, OP_SQRT],
                        [dst.0, vvvv],
                        rel,
                    );
                } else {
                    Self::emit_sse_mem(buf, precision, OP_SQRT, dst.0, self.base_gpr, rel);
                }
            }
            InlineUnary::Floor => self.emit_round_mem(buf, precision, dst.0, rel, 0x09),
            InlineUnary::Ceil => self.emit_round_mem(buf, precision, dst.0, rel, 0x0A),
            InlineUnary::Trunc => self.emit_round_mem(buf, precision, dst.0, rel, 0x0B),
            InlineUnary::Abs | InlineUnary::Round => return Err(EvalError::InvalidProgram),
        }
        Ok(())
    }

    fn supports_inline_binary(&self, _op: InlineBinary) -> bool {
        true
    }

    fn inline_binary_scratch(&self, op: InlineBinary) -> u8 {
        match op {
            InlineBinary::Min => 1,
            InlineBinary::Max => {
                if (self.features & FEAT_AVX512) != 0 {
                    1
                } else {
                    2
                }
            }
        }
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
        // IEEE 754-2019 `min`/`max`: propagate any NaN and order `-0.0 < +0.0`.
        self.scratch_mask = None;
        self.used_scratch_mask |= SCRATCH.bit();
        let fp_pp = self.fp_pp(precision);
        let packed_pp = Self::packed_pp(precision);
        match op {
            InlineBinary::Min => {
                self.emit_commuted_extrema(buf, precision, OP_MIN, dst, lhs, rhs);
                if (self.features & FEAT_AVX) != 0 {
                    self.emit_rr(
                        buf,
                        precision,
                        [packed_pp, VEX_MAP_0F, 0, OP_ORPD],
                        [dst.0, SCRATCH.0, dst.0],
                    );
                } else {
                    Self::emit_sse_packed_rr(buf, OP_ORPD, dst.0, SCRATCH.0);
                }
            }
            InlineBinary::Max => {
                self.emit_commuted_extrema(buf, precision, OP_MAX, dst, lhs, rhs);
                if (self.features & FEAT_AVX512) != 0 {
                    self.emit_evex_rr(
                        buf,
                        precision,
                        [fp_pp, VEX_MAP_0F, OP_CMP],
                        [1, SCRATCH.0, dst.0],
                        0,
                    );
                    buf.push(0x03);
                    self.emit_evex_rr(
                        buf,
                        precision,
                        [fp_pp, VEX_MAP_0F, OP_CMP],
                        [2, SCRATCH.0, dst.0],
                        0,
                    );
                    buf.push(0x07);
                    self.emit_evex_rr(
                        buf,
                        precision,
                        [fp_pp, VEX_MAP_0F, OP_ADD],
                        [dst.0, SCRATCH.0, dst.0],
                        1,
                    );
                    self.emit_evex_rr(
                        buf,
                        precision,
                        [packed_pp, VEX_MAP_0F, OP_ANDPD],
                        [dst.0, SCRATCH.0, dst.0],
                        2,
                    );
                } else if (self.features & FEAT_AVX) != 0 {
                    self.used_scratch_mask |= SCRATCH2.bit();
                    self.emit_rr(
                        buf,
                        precision,
                        [fp_pp, VEX_MAP_0F, 0, OP_CMP],
                        [SCRATCH2.0, SCRATCH.0, dst.0],
                    );
                    buf.push(0x03);
                    self.emit_rr(
                        buf,
                        precision,
                        [packed_pp, VEX_MAP_0F, 0, OP_ANDPD],
                        [dst.0, SCRATCH.0, dst.0],
                    );
                    self.emit_rr(
                        buf,
                        precision,
                        [packed_pp, VEX_MAP_0F, 0, OP_ORPD],
                        [dst.0, dst.0, SCRATCH2.0],
                    );
                } else {
                    self.used_scratch_mask |= SCRATCH2.bit();
                    self.emit_mov(buf, precision, SCRATCH2, SCRATCH);
                    Self::emit_sse_rr(buf, precision, OP_CMP, SCRATCH2.0, dst.0);
                    buf.push(0x03);
                    Self::emit_sse_packed_rr(buf, OP_ANDPD, dst.0, SCRATCH.0);
                    Self::emit_sse_packed_rr(buf, OP_ORPD, dst.0, SCRATCH2.0);
                }
            }
        }
        Ok(())
    }

    fn emit_call_reg(
        &mut self,
        buf: &mut Vec<u8>,
        trampoline_index: usize,
    ) -> Result<(), EvalError> {
        self.scratch_mask = None;
        if !matches!(self.width, VectorWidth::Scalar) {
            buf.extend_from_slice(&[0xC5, 0xF8, 0x77]);
        }
        let byte_off = u32::try_from(trampoline_index)
            .ok()
            .and_then(|idx| idx.checked_mul(8))
            .ok_or(EvalError::ProgramTooLarge)?;
        if byte_off == 0 {
            buf.extend_from_slice(&[0x41, 0xFF, 0x14, 0x24]);
        } else if let Ok(disp8) = u8::try_from(byte_off)
            && disp8 <= 127
        {
            buf.extend_from_slice(&[0x41, 0xFF, 0x54, 0x24, disp8]);
        } else {
            buf.extend_from_slice(&[0x41, 0xFF, 0x94, 0x24]);
            buf.extend_from_slice(&byte_off.to_le_bytes());
        }
        Ok(())
    }

    fn emit_call_pair(
        &mut self,
        buf: &mut Vec<u8>,
        trampoline_index: usize,
        second_byte_offset: u32,
    ) -> Result<(), EvalError> {
        #[cfg(target_os = "windows")]
        let arg0_gpr = 1_u8;
        #[cfg(not(target_os = "windows"))]
        let arg0_gpr = 7_u8;
        let rel = self.rel_disp(second_byte_offset)?;
        buf.extend_from_slice(&[0x48 | ((arg0_gpr >> 3) << 2), 0x8D]);
        Self::emit_base_disp(buf, arg0_gpr, self.base_gpr, rel);
        self.emit_call_reg(buf, trampoline_index)
    }

    fn emit_call_slots(
        &mut self,
        buf: &mut Vec<u8>,
        trampoline_index: usize,
        byte_offsets: &[u32],
    ) -> Result<(), EvalError> {
        #[cfg(target_os = "windows")]
        const GPRS: [u8; 4] = [1, 2, 8, 9];
        #[cfg(not(target_os = "windows"))]
        const GPRS: [u8; 4] = [7, 6, 2, 1];
        if byte_offsets.len() > GPRS.len() {
            return Err(EvalError::InvalidProgram);
        }
        for (&offset, &gpr) in byte_offsets.iter().zip(&GPRS) {
            let rel = self.rel_disp(offset)?;
            buf.extend_from_slice(&[0x48 | ((gpr >> 3) << 2), 0x8D]);
            Self::emit_base_disp(buf, gpr, self.base_gpr, rel);
        }
        self.emit_call_reg(buf, trampoline_index)
    }
}
