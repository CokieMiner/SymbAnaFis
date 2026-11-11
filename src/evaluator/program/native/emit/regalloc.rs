//! Belady hardware floating-point register allocator for real scalar and SIMD kernels.

use alloc::{vec, vec::Vec};

use crate::errors::EvalError;

use super::{ArchBackend, FloatPrecision, HwReg, LivenessPlan, ProgramData};

/// Maximum instruction distance over which loading a spilled/constant memory operand
/// into a free register is preferred over folding it directly from memory.
const FOLD_REUSE_WINDOW: u32 = 24;

/// Register-file tracker for linear Belady allocation of real scalar and SIMD programs.
pub struct RealEmitter<'prog, B: ArchBackend> {
    pub backend: &'prog mut B,
    pub buf: &'prog mut Vec<u8>,
    pub precision: FloatPrecision,
    pub stride: u32,
    pub vector_lanes: usize,
    pub has_calls: bool,
    pub allocatable_mask: u32,
    pub current_ip: u32,
    pub result_hint: u32,
    pub remaining_uses: Vec<u32>,
    pub next_use: Vec<u32>,
    pub use_next: Vec<u32>,
    pub use_cursor: usize,
    pub in_memory: Vec<bool>,
    pub vm_to_hw: Vec<Option<HwReg>>,
    pub hw_to_vm: [Option<usize>; 32],
    pub hw_next: [u32; 32],
    pub hw_clean: u32,
    pub free_mask: u32,
    pub used_hw_mask: u32,
}

impl<'prog, B: ArchBackend> RealEmitter<'prog, B> {
    pub fn compile<Value>(
        program: &ProgramData<Value>,
        precision: FloatPrecision,
        stride: u32,
        vector_lanes: usize,
        store_to_slots: bool,
        backend: &'prog mut B,
        buf: &'prog mut Vec<u8>,
    ) -> Result<(u32, bool), EvalError> {
        let liveness = LivenessPlan::analyze(program, backend);
        let has_calls = liveness.has_calls;
        let allocatable_mask =
            backend.allocatable_mask(has_calls, liveness.scratch_regs, liveness.max_live);
        let max_offset = u32::try_from(program.register_count.saturating_sub(1))
            .ok()
            .and_then(|idx| idx.checked_mul(stride))
            .filter(|&off| off <= i32::MAX.unsigned_abs())
            .ok_or(EvalError::ProgramTooLarge)?;
        backend.configure_frame(has_calls, max_offset);

        let prefix = program
            .parameters
            .len()
            .saturating_add(program.constants.len());
        let mut in_memory = vec![false; program.register_count];
        if let Some(fixed) = in_memory.get_mut(..prefix) {
            fixed.fill(true);
        }
        let mut emitter = Self {
            backend,
            buf,
            precision,
            stride,
            vector_lanes,
            has_calls,
            allocatable_mask,
            current_ip: 0,
            result_hint: 0,
            remaining_uses: liveness.initial_uses,
            next_use: liveness.initial_next,
            use_next: liveness.use_next,
            use_cursor: 0,
            in_memory,
            vm_to_hw: vec![None; program.register_count],
            hw_to_vm: [None; 32],
            hw_next: [u32::MAX; 32],
            hw_clean: 0,
            free_mask: allocatable_mask,
            used_hw_mask: 0,
        };
        let flush_slots = store_to_slots || vector_lanes > 1;
        let single_return_slot = if flush_slots {
            None
        } else {
            program.results.first().copied()
        };
        emitter.emit_program(
            program,
            single_return_slot,
            &liveness.primary_uses,
            &liveness.primary_next,
            &liveness.secondary_uses,
            &liveness.secondary_next,
        )?;
        if flush_slots {
            for &result_slot in &program.results {
                emitter.ensure_in_memory(result_slot)?;
            }
        } else {
            let result_slot = single_return_slot.ok_or(EvalError::InvalidProgram)?;
            let res_idx = usize::try_from(result_slot).unwrap_or(usize::MAX);
            if let Some(Some(hw)) = emitter.vm_to_hw.get(res_idx).copied() {
                emitter
                    .backend
                    .emit_mov(emitter.buf, precision, B::ARG0, hw);
            } else {
                let offset = emitter.slot_offset(result_slot)?;
                emitter
                    .backend
                    .emit_load(emitter.buf, precision, B::ARG0, offset)?;
            }
            emitter.used_hw_mask |= B::ARG0.bit();
        }
        Ok((emitter.used_hw_mask, has_calls))
    }

    pub fn release_hw(&mut self, hw: HwReg) {
        let bit = hw.bit();
        self.free_mask |= bit;
        self.hw_clean &= !bit;
        if let Some(slot) = self.hw_next.get_mut(hw.index()) {
            *slot = u32::MAX;
        }
    }

    pub fn record_memory_def(&mut self, vm_reg: u32, (uses, next): (u32, u32)) {
        self.detach_vm(vm_reg);
        let idx = usize::try_from(vm_reg).unwrap_or(usize::MAX);
        if let Some(rem) = self.remaining_uses.get_mut(idx) {
            *rem = uses;
        }
        if let Some(next_slot) = self.next_use.get_mut(idx) {
            *next_slot = next;
        }
        if let Some(mem) = self.in_memory.get_mut(idx) {
            *mem = true;
        }
    }

    pub fn ensure_in_memory(&mut self, vm_reg: u32) -> Result<(), EvalError> {
        let idx = usize::try_from(vm_reg).unwrap_or(usize::MAX);
        if !self.in_memory.get(idx).copied().unwrap_or(false)
            && let Some(Some(hw)) = self.vm_to_hw.get(idx).copied()
        {
            let offset = self.slot_offset(vm_reg)?;
            self.backend
                .emit_store(self.buf, self.precision, hw, offset)?;
            if let Some(flag) = self.in_memory.get_mut(idx) {
                *flag = true;
            }
            self.hw_clean |= hw.bit();
        }
        Ok(())
    }

    pub fn load_operand(&mut self, vm_reg: u32, exclude_mask: u32) -> Result<HwReg, EvalError> {
        let idx = usize::try_from(vm_reg).unwrap_or(usize::MAX);
        if let Some(Some(hw)) = self.vm_to_hw.get(idx).copied() {
            return Ok(hw);
        }
        let hw = self.alloc_hw(exclude_mask)?;
        let offset = self.slot_offset(vm_reg)?;
        self.backend
            .emit_load(self.buf, self.precision, hw, offset)?;
        let hw_idx = hw.index();
        if let Some(slot) = self.vm_to_hw.get_mut(idx) {
            *slot = Some(hw);
        }
        if let Some(slot) = self.hw_to_vm.get_mut(hw_idx) {
            *slot = Some(idx);
        }
        if let Some(slot) = self.hw_next.get_mut(hw_idx) {
            *slot = self.next_use.get(idx).copied().unwrap_or(u32::MAX);
        }
        self.hw_clean |= hw.bit();
        Ok(hw)
    }

    pub fn load_binary_inputs(
        &mut self,
        [a, b]: [u32; 2],
        [next_a, next_b]: [u32; 2],
    ) -> Result<(HwReg, HwReg, u32), EvalError> {
        let hw_a = self.load_operand(a, 0)?;
        let mask_a = hw_a.bit();
        let hw_b = self.load_operand(b, mask_a)?;
        let in_mask = mask_a | hw_b.bit();
        self.consume_operand(a, next_a);
        self.consume_operand(b, next_b);
        Ok((hw_a, hw_b, in_mask))
    }

    pub fn load_ternary_inputs(
        &mut self,
        [a, b, c]: [u32; 3],
        [next_a, next_b, next_c]: [u32; 3],
    ) -> Result<(HwReg, HwReg, HwReg, u32), EvalError> {
        let hw_a = self.load_operand(a, 0)?;
        let mask_a = hw_a.bit();
        let hw_b = self.load_operand(b, mask_a)?;
        let mask_ab = mask_a | hw_b.bit();
        let hw_c = self.load_operand(c, mask_ab)?;
        self.consume_operand(a, next_a);
        self.consume_operand(b, next_b);
        self.consume_operand(c, next_c);
        Ok((hw_a, hw_b, hw_c, mask_ab | hw_c.bit()))
    }

    pub fn can_fold_mem(&self, vm_reg: u32, next_use_after: u32) -> bool {
        let idx = usize::try_from(vm_reg).unwrap_or(usize::MAX);
        self.backend.supports_mem_operand()
            && self.vm_to_hw.get(idx).copied().flatten().is_none()
            && self.in_memory.get(idx).copied().unwrap_or(false)
            && (self.remaining_uses.get(idx).copied().unwrap_or(0) <= 1
                || self.free_mask == 0
                || next_use_after > self.current_ip.saturating_add(FOLD_REUSE_WINDOW))
    }

    pub fn pop_use_next(&mut self) -> u32 {
        let next = self
            .use_next
            .get(self.use_cursor)
            .copied()
            .unwrap_or(u32::MAX);
        self.use_cursor = self.use_cursor.saturating_add(1);
        next
    }

    pub fn consume_operand(&mut self, vm_reg: u32, next_use_after: u32) {
        let idx = usize::try_from(vm_reg).unwrap_or(usize::MAX);
        if let Some(next_slot) = self.next_use.get_mut(idx) {
            *next_slot = next_use_after;
        }
        if let Some(Some(hw)) = self.vm_to_hw.get(idx).copied()
            && let Some(slot) = self.hw_next.get_mut(hw.index())
        {
            *slot = next_use_after;
        }
        let Some(rem) = self.remaining_uses.get_mut(idx) else {
            return;
        };
        *rem = rem.saturating_sub(1);
        if *rem == 0 {
            self.detach_vm(vm_reg);
        }
    }

    pub fn bind_output(
        &mut self,
        vm_reg: u32,
        preferred_mask: u32,
        exclude_mask: u32,
        out_info: (u32, u32),
    ) -> Result<HwReg, EvalError> {
        self.detach_vm(vm_reg);
        let hw = self.alloc_reusable(preferred_mask, exclude_mask)?;
        self.attach_acc(vm_reg, hw, out_info);
        Ok(hw)
    }

    pub fn attach_acc(&mut self, vm_reg: u32, hw: HwReg, (new_uses, new_next): (u32, u32)) {
        self.detach_vm(vm_reg);
        let idx = usize::try_from(vm_reg).unwrap_or(usize::MAX);
        let hw_idx = hw.index();
        if let Some(slot) = self.vm_to_hw.get_mut(idx) {
            *slot = Some(hw);
        }
        if let Some(slot) = self.hw_to_vm.get_mut(hw_idx) {
            *slot = Some(idx);
        }
        if let Some(slot) = self.hw_next.get_mut(hw_idx) {
            *slot = new_next;
        }
        self.hw_clean &= !hw.bit();
        if let Some(rem) = self.remaining_uses.get_mut(idx) {
            *rem = new_uses;
        }
        if let Some(next_slot) = self.next_use.get_mut(idx) {
            *next_slot = new_next;
        }
        if let Some(mem) = self.in_memory.get_mut(idx) {
            *mem = false;
        }
        if new_uses == 0 {
            self.detach_vm(vm_reg);
        }
    }

    pub fn alloc_reusable(
        &mut self,
        preferred_mask: u32,
        exclude_mask: u32,
    ) -> Result<HwReg, EvalError> {
        let reusable = self.free_mask & preferred_mask;
        if reusable != 0 {
            self.take_hw_bit(reusable.trailing_zeros())
        } else {
            self.alloc_hw(exclude_mask)
        }
    }

    pub fn spill_caller_saved(&mut self) -> Result<(), EvalError> {
        let callee_saved = self.backend.callee_saved_mask();
        let mut active = (self.allocatable_mask & !callee_saved) & !self.free_mask;
        while active != 0 {
            let bit = active.trailing_zeros();
            active &= active.wrapping_sub(1);
            if let Ok(reg_id) = u8::try_from(bit) {
                self.evict_hw(HwReg(reg_id))?;
            }
        }
        Ok(())
    }

    pub fn detach_vm(&mut self, vm_reg: u32) {
        let idx = usize::try_from(vm_reg).unwrap_or(usize::MAX);
        if let Some(Some(hw)) = self.vm_to_hw.get(idx).copied() {
            let hw_idx = hw.index();
            if let Some(slot) = self.vm_to_hw.get_mut(idx) {
                *slot = None;
            }
            if let Some(slot) = self.hw_to_vm.get_mut(hw_idx) {
                *slot = None;
            }
            if let Some(slot) = self.hw_next.get_mut(hw_idx) {
                *slot = u32::MAX;
            }
            let bit = hw.bit();
            self.hw_clean &= !bit;
            self.free_mask |= bit;
        }
    }

    pub fn slot_offset(&self, reg: u32) -> Result<u32, EvalError> {
        reg.checked_mul(self.stride)
            .filter(|&off| off <= i32::MAX.unsigned_abs())
            .ok_or(EvalError::ProgramTooLarge)
    }

    fn alloc_hw(&mut self, exclude_mask: u32) -> Result<HwReg, EvalError> {
        let available = self.free_mask & !exclude_mask;
        if available != 0 {
            let callee_saved = self.backend.callee_saved_mask();
            let preferred_pool = if self.has_calls {
                available & callee_saved
            } else {
                available & !callee_saved
            };
            let pool = if preferred_pool != 0 {
                preferred_pool
            } else {
                available
            };
            return self.take_hw_bit(pool.trailing_zeros());
        }

        let candidates = self.allocatable_mask & !exclude_mask;
        if candidates == 0 {
            return Err(EvalError::InvalidProgram);
        }
        let mut best_bit = candidates.trailing_zeros();
        let mut best_score = 0_u32;
        let mut scan = candidates;
        while scan != 0 {
            let bit = scan.trailing_zeros();
            scan &= scan.wrapping_sub(1);
            let next = self
                .hw_next
                .get(usize::try_from(bit).unwrap_or(0) & 31)
                .copied()
                .unwrap_or(u32::MAX);
            let clean_bonus = if (self.hw_clean & (1_u32 << bit)) != 0 {
                16_u32
            } else {
                0_u32
            };
            let score = next.saturating_add(clean_bonus);
            if score >= best_score {
                best_score = score;
                best_bit = bit;
                if best_score == u32::MAX {
                    break;
                }
            }
        }
        let hw = u8::try_from(best_bit)
            .ok()
            .map(HwReg)
            .ok_or(EvalError::InvalidProgram)?;
        self.evict_hw(hw)?;
        self.take_hw_bit(best_bit)
    }

    fn evict_hw(&mut self, hw: HwReg) -> Result<(), EvalError> {
        let hw_idx = hw.index();
        if let Some(Some(vm_idx)) = self.hw_to_vm.get(hw_idx).copied() {
            let needed = self.remaining_uses.get(vm_idx).copied().unwrap_or(0) > 0;
            if needed && !self.in_memory.get(vm_idx).copied().unwrap_or(false) {
                let vm_u32 =
                    u32::try_from(vm_idx).map_err(|_overflow| EvalError::ProgramTooLarge)?;
                let offset = self.slot_offset(vm_u32)?;
                self.backend
                    .emit_store(self.buf, self.precision, hw, offset)?;
                if let Some(flag) = self.in_memory.get_mut(vm_idx) {
                    *flag = true;
                }
            }
            if let Some(slot) = self.vm_to_hw.get_mut(vm_idx) {
                *slot = None;
            }
            if let Some(slot) = self.hw_to_vm.get_mut(hw_idx) {
                *slot = None;
            }
        }
        if let Some(slot) = self.hw_next.get_mut(hw_idx) {
            *slot = u32::MAX;
        }
        let bit = hw.bit();
        self.hw_clean &= !bit;
        self.free_mask |= bit;
        Ok(())
    }

    fn take_hw_bit(&mut self, bit: u32) -> Result<HwReg, EvalError> {
        let mask = 1_u32 << bit;
        self.free_mask &= !mask;
        self.used_hw_mask |= mask;
        u8::try_from(bit)
            .ok()
            .map(HwReg)
            .ok_or(EvalError::InvalidProgram)
    }
}
