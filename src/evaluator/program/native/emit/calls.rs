//! Inline unary, scalar C-ABI register call, and SIMD slot-trampoline call lowering
//! for [`RealEmitter`], including cycle-free parallel argument marshalling.

use crate::errors::EvalError;

use super::{
    ArchBackend, HwReg, InlineBinary, InlineUnary, Instruction, Operation, RealEmitter, Trampolines,
};

impl<B: ArchBackend> RealEmitter<'_, B> {
    pub fn emit_call_instruction(
        &mut self,
        instruction: &Instruction,
        out_info: (u32, u32),
        pair_info: (u32, u32),
    ) -> Result<(), EvalError> {
        let precision = self.precision;
        let out = instruction.output;
        if let Operation::Unary(builtin, [a]) = &instruction.operation
            && let Some(inline_op) = InlineUnary::of(*builtin)
            && self.backend.supports_inline_unary(inline_op)
        {
            let next_a = self.pop_use_next();
            if self.backend.supports_inline_unary_mem(inline_op) && self.can_fold_mem(*a, next_a) {
                self.consume_operand(*a, next_a);
                let dst = self.bind_output(out, self.result_hint, 0, out_info)?;
                let offset = self.slot_offset(*a)?;
                return self
                    .backend
                    .emit_inline_unary_mem(self.buf, precision, inline_op, dst, offset);
            }
            let hw_a = self.load_operand(*a, 0)?;
            let mask_a = hw_a.bit();
            self.consume_operand(*a, next_a);
            let dst = self.bind_output(out, self.result_hint | mask_a, mask_a, out_info)?;
            return self
                .backend
                .emit_inline_unary(self.buf, precision, inline_op, dst, hw_a);
        }
        if let Operation::Binary(builtin, [a, b]) = &instruction.operation
            && let Some(inline_op) = InlineBinary::of(*builtin)
            && self.backend.supports_inline_binary(inline_op)
        {
            let next = [self.pop_use_next(), self.pop_use_next()];
            let (hw_a, hw_b, in_mask) = self.load_binary_inputs([*a, *b], next)?;
            let dst = self.bind_output(out, self.result_hint | in_mask, in_mask, out_info)?;
            return self
                .backend
                .emit_inline_binary(self.buf, precision, inline_op, dst, hw_a, hw_b);
        }
        if self.vector_lanes > 1 {
            return self.emit_vector_call(instruction, out_info, pair_info);
        }
        match &instruction.operation {
            Operation::Unary(builtin, [a]) => {
                let next_a = self.pop_use_next();
                let hw_a = self.load_operand(*a, 0)?;
                self.consume_operand(*a, next_a);
                self.spill_caller_saved()?;
                self.emit_parallel_arg_moves(&[hw_a]);
                self.finish_scalar_call(Trampolines::builtin(*builtin), out, out_info)?;
            }
            Operation::Binary(builtin, [a, b]) => {
                self.emit_scalar_binary_call(
                    Trampolines::builtin(*builtin),
                    [*a, *b],
                    out,
                    out_info,
                )?;
            }
            Operation::Power([a, b]) => {
                self.emit_scalar_binary_call(Trampolines::POWER, [*a, *b], out, out_info)?;
            }
            Operation::Ternary(builtin, [a, b, c]) => {
                let next = [
                    self.pop_use_next(),
                    self.pop_use_next(),
                    self.pop_use_next(),
                ];
                let (hw_a, hw_b, hw_c, _) = self.load_ternary_inputs([*a, *b, *c], next)?;
                self.spill_caller_saved()?;
                self.emit_parallel_arg_moves(&[hw_a, hw_b, hw_c]);
                self.finish_scalar_call(Trampolines::builtin(*builtin), out, out_info)?;
            }
            Operation::Pair(paired, [a], second) => {
                let next_a = self.pop_use_next();
                let second_offset = self.slot_offset(*second)?;
                let hw_a = self.load_operand(*a, 0)?;
                self.consume_operand(*a, next_a);
                self.detach_vm(*second);
                self.spill_caller_saved()?;
                self.emit_parallel_arg_moves(&[hw_a]);
                self.backend
                    .emit_store(self.buf, precision, B::ARG0, second_offset)?;
                self.backend.emit_call_pair(
                    self.buf,
                    Trampolines::paired(*paired),
                    second_offset,
                )?;
                self.record_memory_def(*second, pair_info);
                if out_info.0 == 0 {
                    self.detach_vm(out);
                } else {
                    let dst = self.bind_output(out, B::ARG0.bit(), 0, out_info)?;
                    self.backend.emit_mov(self.buf, precision, dst, B::ARG0);
                }
            }
            Operation::Combine(_, _)
            | Operation::Divide(_)
            | Operation::Negate(_)
            | Operation::NegatedMultiply(_)
            | Operation::Fused(_, _)
            | Operation::Fold(_, _) => {}
        }
        Ok(())
    }

    fn emit_vector_call(
        &mut self,
        instruction: &Instruction,
        out_info: (u32, u32),
        pair_info: (u32, u32),
    ) -> Result<(), EvalError> {
        let out = instruction.output;
        match &instruction.operation {
            Operation::Unary(builtin, [a]) => {
                self.emit_vector_slot_call(Trampolines::builtin(*builtin), [*a], out, out_info)?;
            }
            Operation::Binary(builtin, [a, b]) => {
                self.emit_vector_slot_call(
                    Trampolines::builtin(*builtin),
                    [*a, *b],
                    out,
                    out_info,
                )?;
            }
            Operation::Power([a, b]) => {
                self.emit_vector_slot_call(Trampolines::POWER, [*a, *b], out, out_info)?;
            }
            Operation::Ternary(builtin, [a, b, c]) => {
                self.emit_vector_slot_call(
                    Trampolines::builtin(*builtin),
                    [*a, *b, *c],
                    out,
                    out_info,
                )?;
            }
            Operation::Pair(paired, [a], second) => {
                let next_a = self.pop_use_next();
                self.ensure_in_memory(*a)?;
                self.consume_operand(*a, next_a);
                self.detach_vm(out);
                self.detach_vm(*second);
                self.spill_caller_saved()?;
                let offsets = [
                    self.slot_offset(out)?,
                    self.slot_offset(*second)?,
                    self.slot_offset(*a)?,
                ];
                self.backend
                    .emit_call_slots(self.buf, Trampolines::paired(*paired), &offsets)?;
                self.record_memory_def(out, out_info);
                self.record_memory_def(*second, pair_info);
            }
            Operation::Combine(_, _)
            | Operation::Divide(_)
            | Operation::Negate(_)
            | Operation::NegatedMultiply(_)
            | Operation::Fused(_, _)
            | Operation::Fold(_, _) => {}
        }
        Ok(())
    }

    fn emit_scalar_binary_call(
        &mut self,
        trampoline_idx: usize,
        operands: [u32; 2],
        out: u32,
        out_info: (u32, u32),
    ) -> Result<(), EvalError> {
        let next = [self.pop_use_next(), self.pop_use_next()];
        let (hw_a, hw_b, _) = self.load_binary_inputs(operands, next)?;
        self.spill_caller_saved()?;
        self.emit_parallel_arg_moves(&[hw_a, hw_b]);
        self.finish_scalar_call(trampoline_idx, out, out_info)
    }

    /// Cycle-free parallel move of up to 3 loaded operands into `[ARG0, ARG1, ARG2]`
    /// after all live caller-saved registers have been spilled.
    fn emit_parallel_arg_moves(&mut self, sources: &[HwReg]) {
        let precision = self.precision;
        let targets = [B::ARG0, B::ARG1, B::ARG2];
        let scratch_pool = [B::CALL_SCRATCH, B::ARG0, B::ARG1, B::ARG2];
        let mut cur = [B::ARG0; 3];
        let count = sources.len().min(3);
        for (slot, &src) in cur.iter_mut().zip(sources.iter().take(count)) {
            *slot = src;
        }
        for i in 0..count {
            let Some(&dst) = targets.get(i) else {
                continue;
            };
            self.used_hw_mask |= dst.bit();
            let Some(&src) = cur.get(i) else {
                continue;
            };
            if src == dst {
                continue;
            }
            let clobbers_later = cur
                .get(i.saturating_add(1)..count)
                .is_some_and(|rest| rest.contains(&dst));
            if clobbers_later {
                let occupied = targets
                    .get(..i)
                    .unwrap_or(&[])
                    .iter()
                    .chain(cur.get(i..count).unwrap_or(&[]))
                    .fold(0_u32, |mask, reg| mask | reg.bit());
                if let Some(&temp) = scratch_pool.iter().find(|reg| (occupied & reg.bit()) == 0) {
                    self.used_hw_mask |= temp.bit();
                    self.backend.emit_mov(self.buf, precision, temp, dst);
                    if let Some(rest) = cur.get_mut(i.saturating_add(1)..count) {
                        for item in rest {
                            if *item == dst {
                                *item = temp;
                            }
                        }
                    }
                }
            }
            let current_src = cur.get(i).copied().unwrap_or(src);
            self.backend.emit_mov(self.buf, precision, dst, current_src);
            if let Some(slot) = cur.get_mut(i) {
                *slot = dst;
            }
        }
    }

    fn finish_scalar_call(
        &mut self,
        trampoline_idx: usize,
        out: u32,
        out_info: (u32, u32),
    ) -> Result<(), EvalError> {
        let precision = self.precision;
        self.backend.emit_call_reg(self.buf, trampoline_idx)?;
        if out_info.0 == 0 {
            self.detach_vm(out);
        } else {
            let dst = self.bind_output(out, B::ARG0.bit(), 0, out_info)?;
            self.backend.emit_mov(self.buf, precision, dst, B::ARG0);
        }
        Ok(())
    }

    fn emit_vector_slot_call<const N: usize>(
        &mut self,
        trampoline_idx: usize,
        inputs: [u32; N],
        out: u32,
        out_info: (u32, u32),
    ) -> Result<(), EvalError> {
        let next = inputs.map(|_| self.pop_use_next());
        for reg in inputs {
            self.ensure_in_memory(reg)?;
        }
        for (reg, next_use) in inputs.into_iter().zip(next) {
            self.consume_operand(reg, next_use);
        }
        self.detach_vm(out);
        self.spill_caller_saved()?;
        let mut offsets = [0_u32; 4];
        if let Some(slot) = offsets.first_mut() {
            *slot = self.slot_offset(out)?;
        }
        for (dst, reg) in offsets.iter_mut().skip(1).zip(inputs) {
            *dst = self.slot_offset(reg)?;
        }
        let call_offsets = offsets
            .get(..N.saturating_add(1))
            .ok_or(EvalError::InvalidProgram)?;
        self.backend
            .emit_call_slots(self.buf, trampoline_idx, call_offsets)?;
        self.record_memory_def(out, out_info);
        Ok(())
    }
}
