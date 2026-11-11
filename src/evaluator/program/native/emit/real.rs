//! Real scalar and SIMD batch arithmetic, FMA, fold, and dot-product lowering.

use alloc::vec::Vec;

use crate::errors::EvalError;

use super::{
    ArchBackend, Combine, Fold, Fused, FusedMemOperand, HwReg, Instruction, Operation, ProgramData,
    RealEmitter,
};

impl<B: ArchBackend> RealEmitter<'_, B> {
    /// Emit native machine code for every instruction in `program`.
    pub fn emit_program<Value>(
        &mut self,
        program: &ProgramData<Value>,
        single_return_slot: Option<u32>,
        primary_uses: &[u32],
        primary_next: &[u32],
        secondary_uses: &[u32],
        secondary_next: &[u32],
    ) -> Result<(), EvalError> {
        let last_idx = program.instructions.len().saturating_sub(1);
        for (idx, instruction) in program.instructions.iter().enumerate() {
            self.current_ip = u32::try_from(idx).unwrap_or(u32::MAX);
            self.result_hint = if self.vector_lanes == 1
                && idx == last_idx
                && single_return_slot == Some(instruction.output)
            {
                B::ARG0.bit()
            } else {
                0
            };
            let out_info = (
                primary_uses.get(idx).copied().unwrap_or(0),
                primary_next.get(idx).copied().unwrap_or(u32::MAX),
            );
            let pair_info = (
                secondary_uses.get(idx).copied().unwrap_or(0),
                secondary_next.get(idx).copied().unwrap_or(u32::MAX),
            );
            self.emit_instruction(&program.operands, instruction, out_info, pair_info)?;
        }
        Ok(())
    }

    fn emit_instruction(
        &mut self,
        operands: &[u32],
        instruction: &Instruction,
        out_info: (u32, u32),
        pair_info: (u32, u32),
    ) -> Result<(), EvalError> {
        let precision = self.precision;
        let out = instruction.output;
        match &instruction.operation {
            Operation::Combine(kind, [a, b]) => {
                let kind = *kind;
                self.emit_binary_op(
                    [*a, *b],
                    out,
                    out_info,
                    matches!(kind, Combine::Add | Combine::Multiply),
                    |be, buf, dst, lhs, off| {
                        be.emit_combine_mem(buf, precision, kind, dst, lhs, off)
                    },
                    |be, buf, dst, lhs, rhs| be.emit_combine(buf, precision, kind, dst, lhs, rhs),
                )?;
            }
            Operation::Divide([a, b]) => {
                self.emit_binary_op(
                    [*a, *b],
                    out,
                    out_info,
                    false,
                    |be, buf, dst, lhs, off| be.emit_div_mem(buf, precision, dst, lhs, off),
                    |be, buf, dst, lhs, rhs| be.emit_div(buf, precision, dst, lhs, rhs),
                )?;
            }
            Operation::NegatedMultiply([a, b]) => {
                self.emit_binary_op(
                    [*a, *b],
                    out,
                    out_info,
                    true,
                    |be, buf, dst, lhs, off| {
                        be.emit_combine_mem(buf, precision, Combine::Multiply, dst, lhs, off)?;
                        be.emit_neg(buf, precision, dst, dst);
                        Ok(())
                    },
                    |be, buf, dst, lhs, rhs| be.emit_neg_mul(buf, precision, dst, lhs, rhs),
                )?;
            }
            Operation::Negate([a]) => {
                let next_a = self.pop_use_next();
                let hw_a = self.load_operand(*a, 0)?;
                let mask_a = hw_a.bit();
                self.consume_operand(*a, next_a);
                let dst = self.bind_output(out, self.result_hint | mask_a, mask_a, out_info)?;
                self.backend.emit_neg(self.buf, precision, dst, hw_a);
            }
            Operation::Fused(fused, [a, b, c]) => {
                self.emit_fused(*fused, [*a, *b, *c], out, out_info)?;
            }
            Operation::Fold(fold, span) => {
                let range = span.range().ok_or(EvalError::InvalidProgram)?;
                let ops = operands.get(range).ok_or(EvalError::InvalidProgram)?;
                match fold {
                    Fold::Sum => self.emit_fold(Combine::Add, ops, out, out_info)?,
                    Fold::Product => self.emit_fold(Combine::Multiply, ops, out, out_info)?,
                    Fold::Dot => self.emit_dot(ops, out, out_info)?,
                }
            }
            Operation::Unary(_, _)
            | Operation::Binary(_, _)
            | Operation::Power(_)
            | Operation::Ternary(_, _)
            | Operation::Pair(_, _, _) => {
                self.emit_call_instruction(instruction, out_info, pair_info)?;
            }
        }
        Ok(())
    }

    /// Pick a memory-foldable operand from a binary pair, preferring `b` (or `a` if `commutative`).
    ///
    /// Returns `((reg_op, reg_next), (mem_op, mem_next))` when `a != b` and one operand can fold.
    fn pick_mem_operand(
        &self,
        [a, b]: [u32; 2],
        [next_a, next_b]: [u32; 2],
        commutative: bool,
    ) -> Option<((u32, u32), (u32, u32))> {
        if a == b {
            return None;
        }
        if self.can_fold_mem(b, next_b) {
            Some(((a, next_a), (b, next_b)))
        } else if commutative && self.can_fold_mem(a, next_a) {
            Some(((b, next_b), (a, next_a)))
        } else {
            None
        }
    }

    fn emit_binary_op(
        &mut self,
        [a, b]: [u32; 2],
        out: u32,
        out_info: (u32, u32),
        commutative: bool,
        emit_mem: impl FnOnce(&mut B, &mut Vec<u8>, HwReg, HwReg, u32) -> Result<(), EvalError>,
        emit_reg: impl FnOnce(&mut B, &mut Vec<u8>, HwReg, HwReg, HwReg),
    ) -> Result<(), EvalError> {
        let next_a = self.pop_use_next();
        let next_b = self.pop_use_next();
        if let Some(((reg_op, reg_next), (mem_op, mem_next))) =
            self.pick_mem_operand([a, b], [next_a, next_b], commutative)
        {
            let hw_lhs = self.load_operand(reg_op, 0)?;
            let mask_lhs = hw_lhs.bit();
            self.consume_operand(reg_op, reg_next);
            self.consume_operand(mem_op, mem_next);
            let pref = if self.backend.non_destructive_binary() {
                self.result_hint | mask_lhs
            } else {
                mask_lhs
            };
            let dst = self.bind_output(out, pref, mask_lhs, out_info)?;
            let offset = self.slot_offset(mem_op)?;
            return emit_mem(self.backend, self.buf, dst, hw_lhs, offset);
        }

        let (hw_a, hw_b, in_mask) = self.load_binary_inputs([a, b], [next_a, next_b])?;
        let pref = if self.backend.non_destructive_binary() {
            self.result_hint | in_mask
        } else {
            hw_a.bit()
        };
        let dst = self.bind_output(out, pref, in_mask, out_info)?;
        emit_reg(self.backend, self.buf, dst, hw_a, hw_b);
        Ok(())
    }

    fn emit_fused(
        &mut self,
        fused: Fused,
        [a, b, c]: [u32; 3],
        out: u32,
        out_info: (u32, u32),
    ) -> Result<(), EvalError> {
        let precision = self.precision;
        let next_a = self.pop_use_next();
        let next_b = self.pop_use_next();
        let next_c = self.pop_use_next();

        if self.backend.supports_fused_mem() {
            let fold_c = c != a && c != b && self.can_fold_mem(c, next_c);
            let fold_b = !fold_c && b != a && b != c && self.can_fold_mem(b, next_b);
            let fold_a = !fold_c && !fold_b && a != b && a != c && self.can_fold_mem(a, next_a);
            if fold_c || fold_b || fold_a {
                let (r0, n0, r1, n1, mem_op, mem_next, is_addend) = if fold_c {
                    (a, next_a, b, next_b, c, next_c, true)
                } else if fold_b {
                    (a, next_a, c, next_c, b, next_b, false)
                } else {
                    (b, next_b, c, next_c, a, next_a, false)
                };
                let hw0 = self.load_operand(r0, 0)?;
                let mask0 = hw0.bit();
                let hw1 = self.load_operand(r1, mask0)?;
                let in_mask = mask0 | hw1.bit();
                self.consume_operand(r0, n0);
                self.consume_operand(r1, n1);
                self.consume_operand(mem_op, mem_next);
                let dst = self.bind_output(out, self.result_hint | in_mask, in_mask, out_info)?;
                let offset = self.slot_offset(mem_op)?;
                let operand = if is_addend {
                    FusedMemOperand::Addend {
                        lhs: hw0,
                        rhs: hw1,
                        addend_offset: offset,
                    }
                } else {
                    FusedMemOperand::Factor {
                        factor: hw0,
                        addend: hw1,
                        mem_offset: offset,
                    }
                };
                return self
                    .backend
                    .emit_fused_mem(self.buf, precision, fused, dst, operand);
            }
        }

        let (hw_a, hw_b, hw_c, in_mask) =
            self.load_ternary_inputs([a, b, c], [next_a, next_b, next_c])?;
        let pref = if self.backend.non_destructive_fused() {
            self.result_hint | in_mask
        } else if a != c {
            hw_a.bit()
        } else if b != c && self.backend.non_destructive_binary() {
            hw_b.bit()
        } else {
            0
        };
        let dst = self.bind_output(out, pref, in_mask, out_info)?;
        self.backend
            .emit_fused(self.buf, precision, fused, dst, [hw_a, hw_b, hw_c]);
        Ok(())
    }

    fn emit_fold(
        &mut self,
        kind: Combine,
        ops: &[u32],
        out: u32,
        out_info: (u32, u32),
    ) -> Result<(), EvalError> {
        let precision = self.precision;
        let &[first, second, ref rest @ ..] = ops else {
            return Err(EvalError::InvalidProgram);
        };
        let (acc, acc_mask) = self.emit_fold_head(kind, first, second)?;
        for &op in rest {
            let next_op = self.pop_use_next();
            if self.can_fold_mem(op, next_op) {
                self.consume_operand(op, next_op);
                let offset = self.slot_offset(op)?;
                self.backend
                    .emit_combine_mem(self.buf, precision, kind, acc, acc, offset)?;
            } else {
                let hw_op = self.load_operand(op, acc_mask)?;
                self.consume_operand(op, next_op);
                self.backend
                    .emit_combine(self.buf, precision, kind, acc, acc, hw_op);
            }
        }
        self.attach_acc(out, acc, out_info);
        Ok(())
    }

    fn emit_dot(&mut self, ops: &[u32], out: u32, out_info: (u32, u32)) -> Result<(), EvalError> {
        let precision = self.precision;
        let &[first_lhs, first_rhs, ref rest @ ..] = ops else {
            return Err(EvalError::InvalidProgram);
        };
        let (acc, acc_mask) = self.emit_fold_head(Combine::Multiply, first_lhs, first_rhs)?;
        let fuse_dot = self.backend.has_hardware_fma();
        let pos_fused = Fused {
            negate_product: false,
            negate_addend: false,
        };
        for &[lhs, rhs] in rest.as_chunks::<2>().0 {
            let next_l = self.pop_use_next();
            let next_r = self.pop_use_next();
            if fuse_dot {
                if self.backend.supports_fused_mem()
                    && let Some(((reg_op, reg_next), (mem_op, mem_next))) =
                        self.pick_mem_operand([lhs, rhs], [next_l, next_r], true)
                {
                    let hw_reg = self.load_operand(reg_op, acc_mask)?;
                    self.consume_operand(reg_op, reg_next);
                    self.consume_operand(mem_op, mem_next);
                    let offset = self.slot_offset(mem_op)?;
                    let operand = FusedMemOperand::Factor {
                        factor: hw_reg,
                        addend: acc,
                        mem_offset: offset,
                    };
                    self.backend
                        .emit_fused_mem(self.buf, precision, pos_fused, acc, operand)?;
                    continue;
                }
                let hw_l = self.load_operand(lhs, acc_mask)?;
                let mask_l = hw_l.bit();
                let hw_r = self.load_operand(rhs, acc_mask | mask_l)?;
                self.consume_operand(lhs, next_l);
                self.consume_operand(rhs, next_r);
                self.backend
                    .emit_fused(self.buf, precision, pos_fused, acc, [hw_l, hw_r, acc]);
                continue;
            }

            let prod = if let Some(((reg_op, reg_next), (mem_op, mem_next))) =
                self.pick_mem_operand([lhs, rhs], [next_l, next_r], true)
            {
                let hw_reg = self.load_operand(reg_op, acc_mask)?;
                let mask_reg = hw_reg.bit();
                self.consume_operand(reg_op, reg_next);
                self.consume_operand(mem_op, mem_next);
                let dst = self.alloc_reusable(mask_reg, acc_mask | mask_reg)?;
                let offset = self.slot_offset(mem_op)?;
                self.backend.emit_combine_mem(
                    self.buf,
                    precision,
                    Combine::Multiply,
                    dst,
                    hw_reg,
                    offset,
                )?;
                dst
            } else {
                let hw_l = self.load_operand(lhs, acc_mask)?;
                let mask_l = hw_l.bit();
                let hw_r = self.load_operand(rhs, acc_mask | mask_l)?;
                let in_mask = mask_l | hw_r.bit();
                self.consume_operand(lhs, next_l);
                self.consume_operand(rhs, next_r);
                let pref = if self.backend.non_destructive_binary() {
                    in_mask
                } else {
                    mask_l
                };
                let dst = self.alloc_reusable(pref, acc_mask | in_mask)?;
                self.backend
                    .emit_combine(self.buf, precision, Combine::Multiply, dst, hw_l, hw_r);
                dst
            };
            self.backend
                .emit_combine(self.buf, precision, Combine::Add, acc, acc, prod);
            self.release_hw(prod);
        }
        self.attach_acc(out, acc, out_info);
        Ok(())
    }

    fn emit_fold_head(
        &mut self,
        kind: Combine,
        first: u32,
        second: u32,
    ) -> Result<(HwReg, u32), EvalError> {
        let precision = self.precision;
        let next_first = self.pop_use_next();
        let next_second = self.pop_use_next();
        if let Some(((reg_op, reg_next), (mem_op, mem_next))) =
            self.pick_mem_operand([first, second], [next_first, next_second], true)
        {
            let hw0 = self.load_operand(reg_op, 0)?;
            let mask0 = hw0.bit();
            self.consume_operand(reg_op, reg_next);
            self.consume_operand(mem_op, mem_next);
            let acc = self.alloc_reusable(mask0, mask0)?;
            let offset = self.slot_offset(mem_op)?;
            self.backend
                .emit_combine_mem(self.buf, precision, kind, acc, hw0, offset)?;
            return Ok((acc, acc.bit()));
        }

        let (hw0, hw1, in_mask) =
            self.load_binary_inputs([first, second], [next_first, next_second])?;
        let pref = if self.backend.non_destructive_binary() {
            in_mask
        } else {
            hw0.bit()
        };
        let acc = self.alloc_reusable(pref, in_mask)?;
        self.backend
            .emit_combine(self.buf, precision, kind, acc, hw0, hw1);
        Ok((acc, acc.bit()))
    }
}
