//! Top-level native machine-code compilation entry points and shrink-wrapped buffer assembly.

use alloc::vec::Vec;

use crate::errors::EvalError;

use super::{ArchBackend, ComplexEmitter, FloatPrecision, NativeLayout, ProgramData, RealEmitter};

/// Space reserved at the head of the code buffer so shrink-wrapped prologues
/// are written in place without reallocating or shifting machine-code bytes.
const PROLOGUE_PAD: usize = 128;

/// Top-level native machine-code compiler over a validated [`ProgramData`].
pub struct NativeEmitter;

impl NativeEmitter {
    /// Compile a scalar or complex `program` into position-independent machine code,
    /// returning `(buffer, entry_offset)`.
    pub fn compile<Value, B: ArchBackend>(
        program: &ProgramData<Value>,
        layout: NativeLayout,
        store_to_slots: bool,
        backend: &mut B,
    ) -> Result<(Vec<u8>, usize), EvalError> {
        let mut buf = Self::alloc_code_buffer(program.instructions.len());
        let (used_hw_mask, has_calls) = match layout {
            NativeLayout::F64 => RealEmitter::compile(
                program,
                FloatPrecision::F64,
                8,
                1,
                store_to_slots,
                backend,
                &mut buf,
            )?,
            NativeLayout::F32 => RealEmitter::compile(
                program,
                FloatPrecision::F32,
                4,
                1,
                store_to_slots,
                backend,
                &mut buf,
            )?,
            NativeLayout::ComplexF64 => (
                0,
                ComplexEmitter::compile(program, FloatPrecision::F64, backend, &mut buf)?,
            ),
            NativeLayout::ComplexF32 => (
                0,
                ComplexEmitter::compile(program, FloatPrecision::F32, backend, &mut buf)?,
            ),
        };
        Self::finish_buffer(buf, backend, used_hw_mask, has_calls)
    }

    /// Compile a real floating-point `program` into a multi-lane SIMD batch kernel,
    /// returning `(buffer, entry_offset)`.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    pub fn compile_vector<Value, B: ArchBackend>(
        program: &ProgramData<Value>,
        precision: FloatPrecision,
        lanes: usize,
        backend: &mut B,
    ) -> Result<(Vec<u8>, usize), EvalError> {
        let elem_bytes: u32 = match precision {
            FloatPrecision::F64 => 8,
            FloatPrecision::F32 => 4,
        };
        let stride = u32::try_from(lanes)
            .ok()
            .and_then(|count| count.checked_mul(elem_bytes))
            .ok_or(EvalError::ProgramTooLarge)?;
        let mut buf = Self::alloc_code_buffer(program.instructions.len());
        let (used_hw_mask, has_calls) =
            RealEmitter::compile(program, precision, stride, lanes, true, backend, &mut buf)?;
        Self::finish_buffer(buf, backend, used_hw_mask, has_calls)
    }

    fn alloc_code_buffer(instruction_count: usize) -> Vec<u8> {
        let capacity = instruction_count
            .saturating_mul(20)
            .saturating_add(PROLOGUE_PAD.saturating_add(64));
        let mut buf = Vec::with_capacity(capacity);
        buf.resize(PROLOGUE_PAD, 0);
        buf
    }

    fn finish_buffer<B: ArchBackend>(
        mut buf: Vec<u8>,
        backend: &mut B,
        used_hw_mask: u32,
        has_calls: bool,
    ) -> Result<(Vec<u8>, usize), EvalError> {
        backend.emit_epilogue(&mut buf, used_hw_mask, has_calls)?;
        // Temporarily append the prologue at the tail of `buf` to avoid a second
        // heap allocation, then copy it into the reserved head pad and truncate.
        let tail_start = buf.len();
        backend.emit_prologue(&mut buf, used_hw_mask, has_calls)?;
        let prologue_len = buf.len().saturating_sub(tail_start);
        if let Some(start) = PROLOGUE_PAD.checked_sub(prologue_len)
            && let Some((head, tail)) = buf.split_at_mut_checked(tail_start)
            && let Some(dst) = head.get_mut(start..PROLOGUE_PAD)
        {
            dst.copy_from_slice(tail);
            buf.truncate(tail_start);
            return Ok((buf, start));
        }
        let mut fallback = Vec::with_capacity(prologue_len.saturating_add(tail_start));
        if let Some(tail) = buf.get(tail_start..) {
            fallback.extend_from_slice(tail);
        }
        if let Some(body) = buf.get(PROLOGUE_PAD..tail_start) {
            fallback.extend_from_slice(body);
        }
        Ok((fallback, 0))
    }
}
