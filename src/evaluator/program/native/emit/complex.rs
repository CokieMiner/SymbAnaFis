//! Complex (`Complex<f64>`, `Complex<f32>`) native lowering over `#[repr(C)]` slots
//! with caller-saved `(ARG0, ARG1)` register forwarding across linear chains.

#![expect(
    unsafe_code,
    reason = "validated ProgramData guarantees all pooled operand spans are in bounds"
)]

use alloc::{vec, vec::Vec};

use crate::{core::BuiltinFunction, errors::EvalError};

use super::{
    ArchBackend, Combine, FloatPrecision, Fold, Instruction, Operation, ProgramData, Trampolines,
};

/// Native codegen emitter for complex scalars stored in `#[repr(C)]` `(re, im)` slots.
pub struct ComplexEmitter<'program, B> {
    backend: &'program mut B,
    buf: &'program mut Vec<u8>,
    operands: &'program [u32],
    precision: FloatPrecision,
    scratch: [u32; 2],
    cached_pair: Option<u32>,
}

impl<'program, B: ArchBackend> ComplexEmitter<'program, B> {
    /// Lower a complex program into `buf` and report whether any trampoline calls were emitted.
    pub fn compile<Value>(
        program: &'program ProgramData<Value>,
        precision: FloatPrecision,
        backend: &'program mut B,
        buf: &'program mut Vec<u8>,
    ) -> Result<bool, EvalError> {
        let has_calls = program.instructions.iter().any(|inst| {
            !matches!(
                inst.operation,
                Operation::Combine(Combine::Add | Combine::Subtract, _)
                    | Operation::Negate(_)
                    | Operation::Fold(Fold::Sum, _)
                    | Operation::Unary(
                        BuiltinFunction::Re | BuiltinFunction::Im | BuiltinFunction::Conj,
                        _,
                    )
            )
        });
        let reg_count_u32 = u32::try_from(program.register_count)
            .map_err(|_overflow| EvalError::ProgramTooLarge)?;
        let scratch = [
            reg_count_u32,
            reg_count_u32
                .checked_add(1)
                .ok_or(EvalError::ProgramTooLarge)?,
        ];
        let max_offset = Self::slot_offset(scratch[1], precision)?;
        backend.configure_frame(has_calls, max_offset);

        let def_uses = Self::compute_def_uses(program);
        let mut emitter = Self {
            backend,
            buf,
            operands: &program.operands,
            precision,
            scratch,
            cached_pair: None,
        };
        for (idx, instruction) in program.instructions.iter().enumerate() {
            let uses = def_uses.get(idx).copied().unwrap_or(1);
            let next_linear =
                program
                    .instructions
                    .get(idx.saturating_add(1))
                    .and_then(|next_inst| {
                        Self::linear_first_input(&next_inst.operation, &program.operands)
                    });
            let keep_in_regs = uses == 1 && Some(instruction.output) == next_linear;
            emitter.emit_instruction(instruction, keep_in_regs)?;
        }
        Ok(has_calls)
    }

    fn emit_instruction(
        &mut self,
        instruction: &Instruction,
        keep_in_regs: bool,
    ) -> Result<(), EvalError> {
        let out = instruction.output;
        match &instruction.operation {
            Operation::Combine(kind @ (Combine::Add | Combine::Subtract), [a, b]) => {
                self.load_pair(*a)?;
                self.combine_slot(*kind, *b)?;
                return self.finish_pair(out, keep_in_regs);
            }
            Operation::Negate([a]) => {
                self.load_pair(*a)?;
                self.backend
                    .emit_neg(self.buf, self.precision, B::ARG0, B::ARG0);
                self.backend
                    .emit_neg(self.buf, self.precision, B::ARG1, B::ARG1);
                return self.finish_pair(out, keep_in_regs);
            }
            Operation::Fold(Fold::Sum, span) => {
                let ops = span
                    .range()
                    .and_then(|range| self.operands.get(range))
                    .ok_or(EvalError::InvalidProgram)?;
                let &[first, ref rest @ ..] = ops else {
                    return Err(EvalError::InvalidProgram);
                };
                self.load_pair(first)?;
                for &op in rest {
                    self.combine_slot(Combine::Add, op)?;
                }
                return self.finish_pair(out, keep_in_regs);
            }
            Operation::Combine(Combine::Multiply, [a, b]) => {
                self.call_slots(Trampolines::COMPLEX_MUL, [out, *a, *b])?;
            }
            Operation::Divide([a, b]) => {
                self.call_slots(Trampolines::COMPLEX_DIV, [out, *a, *b])?;
            }
            Operation::NegatedMultiply([a, b]) => {
                self.call_slots(Trampolines::COMPLEX_NEG_MUL, [out, *a, *b])?;
            }
            Operation::Power([a, b]) => {
                self.call_slots(Trampolines::POWER, [out, *a, *b])?;
            }
            Operation::Fold(fold @ (Fold::Product | Fold::Dot), span) => {
                let ops = span
                    .range()
                    .and_then(|range| self.operands.get(range))
                    .ok_or(EvalError::InvalidProgram)?;
                let out_off = Self::slot_offset(out, self.precision)?;
                self.emit_nonlinear_fold(*fold, ops, out_off)?;
            }
            Operation::Fused(fused, [a, b, c]) => {
                self.call_slots(Trampolines::complex_fused(*fused), [out, *a, *b, *c])?;
            }
            Operation::Unary(BuiltinFunction::Re, [a]) => {
                self.load_pair(*a)?;
                self.backend.emit_zero(self.buf, self.precision, B::ARG1);
                return self.finish_pair(out, keep_in_regs);
            }
            Operation::Unary(BuiltinFunction::Im, [a]) => {
                self.load_pair(*a)?;
                self.backend
                    .emit_mov(self.buf, self.precision, B::ARG0, B::ARG1);
                self.backend.emit_zero(self.buf, self.precision, B::ARG1);
                return self.finish_pair(out, keep_in_regs);
            }
            Operation::Unary(BuiltinFunction::Conj, [a]) => {
                self.load_pair(*a)?;
                self.backend
                    .emit_neg(self.buf, self.precision, B::ARG1, B::ARG1);
                return self.finish_pair(out, keep_in_regs);
            }
            Operation::Unary(f, [a]) => {
                self.call_slots(Trampolines::builtin(*f), [out, *a])?;
            }
            Operation::Binary(f, [a, b]) => {
                self.call_slots(Trampolines::builtin(*f), [out, *a, *b])?;
            }
            Operation::Ternary(f, [a, b, c]) => {
                self.call_slots(Trampolines::builtin(*f), [out, *a, *b, *c])?;
            }
            Operation::Pair(p, [a], second) => {
                self.call_slots(Trampolines::paired(*p), [out, *second, *a])?;
            }
        }
        self.cached_pair = None;
        Ok(())
    }

    fn load_pair(&mut self, reg: u32) -> Result<(), EvalError> {
        if self.cached_pair == Some(reg) {
            self.cached_pair = None;
            return Ok(());
        }
        self.cached_pair = None;
        let part_stride: u32 = match self.precision {
            FloatPrecision::F64 => 8,
            FloatPrecision::F32 => 4,
        };
        let off = Self::slot_offset(reg, self.precision)?;
        self.backend
            .emit_load(self.buf, self.precision, B::ARG0, off)?;
        self.backend.emit_load(
            self.buf,
            self.precision,
            B::ARG1,
            off.saturating_add(part_stride),
        )
    }

    fn combine_slot(&mut self, kind: Combine, rhs: u32) -> Result<(), EvalError> {
        let part_stride: u32 = match self.precision {
            FloatPrecision::F64 => 8,
            FloatPrecision::F32 => 4,
        };
        let off = Self::slot_offset(rhs, self.precision)?;
        self.backend
            .emit_load(self.buf, self.precision, B::ARG2, off)?;
        self.backend
            .emit_combine(self.buf, self.precision, kind, B::ARG0, B::ARG0, B::ARG2);
        self.backend.emit_load(
            self.buf,
            self.precision,
            B::ARG2,
            off.saturating_add(part_stride),
        )?;
        self.backend
            .emit_combine(self.buf, self.precision, kind, B::ARG1, B::ARG1, B::ARG2);
        Ok(())
    }

    fn finish_pair(&mut self, out: u32, keep_in_regs: bool) -> Result<(), EvalError> {
        if keep_in_regs {
            self.cached_pair = Some(out);
            return Ok(());
        }
        self.cached_pair = None;
        let part_stride: u32 = match self.precision {
            FloatPrecision::F64 => 8,
            FloatPrecision::F32 => 4,
        };
        let out_off = Self::slot_offset(out, self.precision)?;
        self.backend
            .emit_store(self.buf, self.precision, B::ARG0, out_off)?;
        self.backend.emit_store(
            self.buf,
            self.precision,
            B::ARG1,
            out_off.saturating_add(part_stride),
        )
    }

    fn call_slots<const N: usize>(
        &mut self,
        trampoline_idx: usize,
        regs: [u32; N],
    ) -> Result<(), EvalError> {
        let mut offsets = [0_u32; N];
        for (dst, reg) in offsets.iter_mut().zip(regs) {
            *dst = Self::slot_offset(reg, self.precision)?;
        }
        self.backend
            .emit_call_slots(self.buf, trampoline_idx, &offsets)
    }

    fn emit_nonlinear_fold(
        &mut self,
        fold: Fold,
        ops: &[u32],
        out_off: u32,
    ) -> Result<(), EvalError> {
        let part_stride: u32 = match self.precision {
            FloatPrecision::F64 => 8,
            FloatPrecision::F32 => 4,
        };
        let scratch_acc = Self::slot_offset(self.scratch[0], self.precision)?;
        let scratch_prod = Self::slot_offset(self.scratch[1], self.precision)?;
        let &[first, second, ref rest @ ..] = ops else {
            return Err(EvalError::InvalidProgram);
        };
        let first_off = Self::slot_offset(first, self.precision)?;
        let second_off = Self::slot_offset(second, self.precision)?;
        self.backend.emit_call_slots(
            self.buf,
            Trampolines::COMPLEX_MUL,
            &[scratch_acc, first_off, second_off],
        )?;
        match fold {
            Fold::Product => {
                let mut iter = rest.iter();
                while let Some(&op) = iter.next() {
                    let dst = if iter.len() == 0 {
                        out_off
                    } else {
                        scratch_acc
                    };
                    let op_off = Self::slot_offset(op, self.precision)?;
                    self.backend.emit_call_slots(
                        self.buf,
                        Trampolines::COMPLEX_MUL,
                        &[dst, scratch_acc, op_off],
                    )?;
                }
            }
            Fold::Dot => {
                let mut pairs = rest.as_chunks::<2>().0.iter();
                while let Some(&[lhs, rhs]) = pairs.next() {
                    let lhs_off = Self::slot_offset(lhs, self.precision)?;
                    let rhs_off = Self::slot_offset(rhs, self.precision)?;
                    self.backend.emit_call_slots(
                        self.buf,
                        Trampolines::COMPLEX_MUL,
                        &[scratch_prod, lhs_off, rhs_off],
                    )?;
                    let dst = if pairs.len() == 0 {
                        out_off
                    } else {
                        scratch_acc
                    };
                    for part in [0_u32, part_stride] {
                        self.backend.emit_load(
                            self.buf,
                            self.precision,
                            B::ARG0,
                            scratch_acc.saturating_add(part),
                        )?;
                        self.backend.emit_load(
                            self.buf,
                            self.precision,
                            B::ARG1,
                            scratch_prod.saturating_add(part),
                        )?;
                        self.backend.emit_combine(
                            self.buf,
                            self.precision,
                            Combine::Add,
                            B::ARG0,
                            B::ARG0,
                            B::ARG1,
                        );
                        self.backend.emit_store(
                            self.buf,
                            self.precision,
                            B::ARG0,
                            dst.saturating_add(part),
                        )?;
                    }
                }
            }
            Fold::Sum => {}
        }
        Ok(())
    }

    fn slot_offset(reg: u32, precision: FloatPrecision) -> Result<u32, EvalError> {
        let (stride, part_stride): (u32, u32) = match precision {
            FloatPrecision::F64 => (16, 8),
            FloatPrecision::F32 => (8, 4),
        };
        reg.checked_mul(stride)
            .filter(|&off| off.saturating_add(part_stride) <= i32::MAX.unsigned_abs())
            .ok_or(EvalError::ProgramTooLarge)
    }

    fn linear_first_input(operation: &Operation, operands: &[u32]) -> Option<u32> {
        match operation {
            Operation::Combine(Combine::Add | Combine::Subtract, [a, _])
            | Operation::Negate([a]) => Some(*a),
            Operation::Fold(Fold::Sum, span) => {
                let start = usize::try_from(span.start).unwrap_or(0);
                // SAFETY: validated `ProgramData` bounds `span` inside `operands`.
                Some(unsafe { *operands.get_unchecked(start) })
            }
            Operation::Unary(builtin, [a]) => matches!(
                builtin,
                BuiltinFunction::Re | BuiltinFunction::Im | BuiltinFunction::Conj
            )
            .then_some(*a),
            Operation::Combine(Combine::Multiply, _)
            | Operation::Divide(_)
            | Operation::NegatedMultiply(_)
            | Operation::Power(_)
            | Operation::Fold(Fold::Product | Fold::Dot, _)
            | Operation::Fused(..)
            | Operation::Binary(..)
            | Operation::Ternary(..)
            | Operation::Pair(..) => None,
        }
    }

    fn compute_def_uses<Value>(program: &ProgramData<Value>) -> Vec<u32> {
        let mut live_reads = vec![0_u32; program.register_count];
        for &res in &program.results {
            if let Some(slot) = live_reads.get_mut(usize::try_from(res).unwrap_or(usize::MAX)) {
                *slot = slot.saturating_add(1);
            }
        }
        let mut def_uses = vec![0_u32; program.instructions.len()];
        for (idx, inst) in program.instructions.iter().enumerate().rev() {
            let out_idx = usize::try_from(inst.output).unwrap_or(usize::MAX);
            if let Some(slot) = live_reads.get_mut(out_idx) {
                if let Some(du) = def_uses.get_mut(idx) {
                    *du = *slot;
                }
                *slot = 0;
            }
            if let Operation::Pair(_, _, second) = inst.operation
                && let Some(slot) =
                    live_reads.get_mut(usize::try_from(second).unwrap_or(usize::MAX))
            {
                *slot = 0;
            }
            // SAFETY: `program` passed `validate`.
            for &inp in unsafe { inst.operation.inputs(&program.operands) } {
                if let Some(slot) = live_reads.get_mut(usize::try_from(inp).unwrap_or(usize::MAX)) {
                    *slot = slot.saturating_add(1);
                }
            }
        }
        def_uses
    }
}
