use crate::evaluator::logic::bytecode::instruction::Instruction;

#[allow(clippy::too_many_lines, reason = "DCE pass with multiple sub-steps")]
pub(super) fn eliminate_dead_code(
    mut out: Vec<Instruction>,
    arg_pool: &mut [u32],
    use_count: &mut [usize],
    param_count: usize,
    old_const_count: usize,
    max_reg_idx: u32,
) -> Vec<Instruction> {
    // --- Copy forwarding pass ---
    // Recompute use_count after fusion
    super::helper::calculate_use_count(&out, use_count, arg_pool);

    // Build a forwarding table: copy_of[reg_idx] = the register reg_idx was copied from (or reg_idx itself)
    let mut copy_of: Vec<u32> = (0..=max_reg_idx).collect();

    // Track next definition of each register to ensure safe forwarding of temporaries
    let mut next_def = vec![usize::MAX; (max_reg_idx + 1) as usize];
    let mut src_next_def_at_idx = vec![usize::MAX; out.len()];
    for (i, instr) in out.iter().enumerate().rev() {
        if let Instruction::Copy { src, .. } = *instr {
            src_next_def_at_idx[i] = next_def[src as usize];
        }
        let d = instr.dest_reg();
        next_def[d as usize] = i;
    }

    // Find last use of each register
    let mut last_use = vec![0_usize; (max_reg_idx + 1) as usize];
    for (i, instr) in out.iter().enumerate() {
        instr.for_each_read(|r| last_use[r as usize] = i);
        instr.for_each_pooled_reg(arg_pool, |reg_idx| last_use[reg_idx as usize] = i);
    }

    // First pass: record all Copy sources
    for (i, instr) in out.iter().enumerate() {
        if let Instruction::Copy { dest, src } = *instr {
            if dest == 0 || dest == src || use_count[dest as usize] > 1 {
                continue;
            }

            let is_immutable = src
                < u32::try_from(param_count + old_const_count).expect("Register index overflow");

            // Safe to forward if src is immutable (Param/Const)
            // OR if src is a temp that isn't redefined before the last use of dest.
            if is_immutable {
                copy_of[dest as usize] = copy_of[src as usize]; // chain-follow safe
            } else if src_next_def_at_idx[i] > last_use[dest as usize] {
                copy_of[dest as usize] = src; // don't chase through another temp alias
            }
        }
    }

    // Second pass: rewrite all reads through the forwarding table, then drop dead Copies
    for instr in &mut out {
        instr.map_reads(|reg_idx| copy_of[reg_idx as usize]);
        instr.map_pooled_regs(arg_pool, |reg_idx| copy_of[reg_idx as usize]);
    }
    out.retain(|instr| {
        if let Instruction::Copy { dest, src } = *instr {
            // Drop useless copies, but keep if dest was NOT forwarded (i.e. it's still needed as a named register)
            dest != src && copy_of[dest as usize] == dest
        } else {
            true
        }
    });

    // --- Backward Dead Code Elimination (DCE) ---
    // Copy forwarding or earlier fusion passes (e.g. Sub instead of Add(Neg)) can leave
    // dead instructions that compute values never read.
    let mut lives = vec![false; (max_reg_idx + 1) as usize];
    // The final expression result is always stored in register 0
    lives[0] = true;
    let mut live_instrs = vec![false; out.len()];

    for (i, instr) in out.iter().enumerate().rev() {
        let dest = instr.dest_reg();
        if lives[dest as usize] {
            live_instrs[i] = true;
            // Kill the destination (it's overwritten here, so previous value is dead)
            lives[dest as usize] = false;

            // Gen reads (they are needed to compute this instruction)
            instr.for_each_read(|r| lives[r as usize] = true);
            instr.for_each_pooled_reg(arg_pool, |reg_idx| lives[reg_idx as usize] = true);
        }
    }

    // Keep only live instructions
    let mut out_live = Vec::with_capacity(out.len());
    for (i, instr) in out.into_iter().enumerate() {
        if live_instrs[i] {
            out_live.push(instr);
        }
    }
    out = out_live;

    out
}
