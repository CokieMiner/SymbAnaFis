//! Linear backward liveness, peak live-register pressure, and Belady next-use
//! analysis for native register allocation.

#![expect(
    unsafe_code,
    reason = "validated ProgramData guarantees all pooled operand spans are in bounds"
)]

use alloc::{vec, vec::Vec};

use crate::core::BuiltinFunction;

use super::{ArchBackend, InlineBinary, InlineUnary, Operation, ProgramData};

/// Per-definition remaining-use counts, next-use distances, and peak pressure.
pub struct LivenessPlan {
    pub initial_uses: Vec<u32>,
    pub initial_next: Vec<u32>,
    pub primary_uses: Vec<u32>,
    pub primary_next: Vec<u32>,
    pub secondary_uses: Vec<u32>,
    pub secondary_next: Vec<u32>,
    pub use_next: Vec<u32>,
    pub max_live: u32,
    pub has_calls: bool,
    pub scratch_regs: u8,
}

impl LivenessPlan {
    /// Compute per-definition use counts, Belady next-use distances, and exact
    /// peak live-register pressure (`max_live`) in one backward scan.
    pub fn analyze<Value, B: ArchBackend>(program: &ProgramData<Value>, backend: &B) -> Self {
        let n_inst = program.instructions.len();
        let end_ip = u32::try_from(n_inst).unwrap_or(u32::MAX);
        let mut live = vec![0_u32; program.register_count];
        let mut next_read = vec![u32::MAX; program.register_count];
        let mut live_count = 0_u32;
        for &result in &program.results {
            let res_idx = usize::try_from(result).unwrap_or(usize::MAX);
            if let Some(res) = live.get_mut(res_idx) {
                if *res == 0 {
                    live_count = live_count.saturating_add(1);
                }
                *res = res.saturating_add(1);
            }
            if let Some(res_next) = next_read.get_mut(res_idx) {
                *res_next = end_ip;
            }
        }
        let mut max_live = live_count;

        let mut primary_uses = vec![0_u32; n_inst];
        let mut primary_next = vec![u32::MAX; n_inst];
        let mut secondary_uses = Vec::new();
        let mut secondary_next = Vec::new();
        let mut use_next = Vec::with_capacity(n_inst.saturating_mul(2));
        let mut has_calls = false;
        let mut scratch_regs = 0_u8;

        for (idx, instruction) in program.instructions.iter().enumerate().rev() {
            let ip = u32::try_from(idx).unwrap_or(u32::MAX);
            let out_idx = usize::try_from(instruction.output).unwrap_or(usize::MAX);
            if let Some(slot) = live.get_mut(out_idx) {
                live_count = live_count.saturating_sub(u32::from(*slot > 0));
                if let Some(out) = primary_uses.get_mut(idx) {
                    *out = *slot;
                }
                *slot = 0;
            }
            if let Some(next_slot) = next_read.get_mut(out_idx) {
                if let Some(out_next) = primary_next.get_mut(idx) {
                    *out_next = *next_slot;
                }
                *next_slot = u32::MAX;
            }
            if let Operation::Pair(_, _, second) = instruction.operation {
                if secondary_uses.is_empty() {
                    secondary_uses.resize(n_inst, 0);
                    secondary_next.resize(n_inst, u32::MAX);
                }
                let sec_idx = usize::try_from(second).unwrap_or(usize::MAX);
                if let Some(slot) = live.get_mut(sec_idx) {
                    live_count = live_count.saturating_sub(u32::from(*slot > 0));
                    if let Some(out2) = secondary_uses.get_mut(idx) {
                        *out2 = *slot;
                    }
                    *slot = 0;
                }
                if let Some(next_slot) = next_read.get_mut(sec_idx) {
                    if let Some(out2_next) = secondary_next.get_mut(idx) {
                        *out2_next = *next_slot;
                    }
                    *next_slot = u32::MAX;
                }
            }
            if Self::instruction_needs_call(&instruction.operation, backend) {
                has_calls = true;
            }
            scratch_regs = scratch_regs.max(Self::instruction_scratch_regs(
                &instruction.operation,
                backend,
            ));
            // SAFETY: `program` passed `validate`, so every pooled span lies in `program.operands`.
            let inputs = unsafe { instruction.operation.inputs(&program.operands) };
            for &input in inputs.iter().rev() {
                let in_idx = usize::try_from(input).unwrap_or(usize::MAX);
                let after_ip = next_read.get(in_idx).copied().unwrap_or(u32::MAX);
                use_next.push(after_ip);
                if let Some(slot) = live.get_mut(in_idx) {
                    if *slot == 0 {
                        live_count = live_count.saturating_add(1);
                    }
                    *slot = slot.saturating_add(1);
                }
                if let Some(next_slot) = next_read.get_mut(in_idx) {
                    *next_slot = ip;
                }
            }
            max_live = max_live.max(live_count);
        }
        use_next.reverse();
        Self {
            initial_uses: live,
            initial_next: next_read,
            primary_uses,
            primary_next,
            secondary_uses,
            secondary_next,
            use_next,
            max_live,
            has_calls,
            scratch_regs,
        }
    }

    fn instruction_needs_call<B: ArchBackend>(operation: &Operation, backend: &B) -> bool {
        match operation {
            Operation::Power(_) | Operation::Ternary(_, _) | Operation::Pair(_, _, _) => true,
            Operation::Unary(builtin, _) => {
                !InlineUnary::of(*builtin).is_some_and(|op| backend.supports_inline_unary(op))
            }
            Operation::Binary(builtin, _) => {
                !InlineBinary::of(*builtin).is_some_and(|op| backend.supports_inline_binary(op))
            }
            Operation::Combine(_, _)
            | Operation::Divide(_)
            | Operation::Negate(_)
            | Operation::NegatedMultiply(_)
            | Operation::Fused(_, _)
            | Operation::Fold(_, _) => false,
        }
    }

    fn instruction_scratch_regs<B: ArchBackend>(operation: &Operation, backend: &B) -> u8 {
        match operation {
            Operation::Negate(_) | Operation::NegatedMultiply(_) => 1,
            Operation::Unary(BuiltinFunction::Abs, _) => {
                u8::from(backend.supports_inline_unary(InlineUnary::Abs))
            }
            Operation::Binary(builtin, _) => InlineBinary::of(*builtin)
                .filter(|&op| backend.supports_inline_binary(op))
                .map_or(0, |op| backend.inline_binary_scratch(op)),
            Operation::Fused(fused, _) => {
                u8::from(fused.negate_product && !backend.non_destructive_fused())
            }
            Operation::Combine(_, _)
            | Operation::Divide(_)
            | Operation::Power(_)
            | Operation::Fold(_, _)
            | Operation::Unary(_, _)
            | Operation::Ternary(_, _)
            | Operation::Pair(_, _, _) => 0,
        }
    }
}
