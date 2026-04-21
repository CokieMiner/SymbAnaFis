use super::Instruction;

use super::helper::calculate_use_count;

pub(super) struct DceScratch {
    pub copy_of: Vec<u32>,
    pub next_def: Vec<usize>,
    pub src_next_def_at_idx: Vec<usize>,
    pub last_use: Vec<usize>,
    pub lives: Vec<bool>,
    pub dirty_uses: Vec<u32>,
}

impl DceScratch {
    pub const fn new() -> Self {
        Self {
            copy_of: Vec::new(),
            next_def: Vec::new(),
            src_next_def_at_idx: Vec::new(),
            last_use: Vec::new(),
            lives: Vec::new(),
            dirty_uses: Vec::new(),
        }
    }
}

#[allow(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    reason = "DCE pass with multiple sub-steps"
)]
pub(super) fn eliminate_dead_code(
    mut out: Vec<Instruction>,
    arg_pool: &mut [u32],
    use_count: &mut [usize],
    param_count: usize,
    const_count: usize,
    max_reg_idx: u32,
    output_reg: u32,
    scratch: &mut DceScratch,
) -> Vec<Instruction> {
    let max_reg_len = (max_reg_idx + 1) as usize;
    let DceScratch {
        copy_of,
        next_def,
        src_next_def_at_idx,
        last_use,
        lives,
        dirty_uses,
    } = scratch;

    // --- Copy forwarding pass ---
    // Recompute use_count after fusion
    calculate_use_count(&out, use_count, dirty_uses, arg_pool);

    // Build a forwarding table: copy_of[reg_idx] = the register reg_idx was copied from (or reg_idx itself)
    copy_of.resize(max_reg_len, 0);
    copy_of
        .iter_mut()
        .enumerate()
        .for_each(|(i, v)| *v = u32::try_from(i).expect("Register index overflow"));

    // Track next definition of each register to ensure safe forwarding of temporaries
    next_def.clear();
    next_def.resize(max_reg_len, usize::MAX);
    src_next_def_at_idx.clear();
    src_next_def_at_idx.resize(out.len(), usize::MAX);
    for (i, instr) in out.iter().enumerate().rev() {
        if let Instruction::Copy { src, .. } = *instr {
            src_next_def_at_idx[i] = next_def[src as usize];
        }
        instr.for_each_write(|d| next_def[d as usize] = i);
    }

    // Find last use of each register
    last_use.clear();
    last_use.resize(max_reg_len, 0);
    for (i, instr) in out.iter().enumerate() {
        instr.for_each_read(|r| last_use[r as usize] = i);
        instr.for_each_pooled_reg(arg_pool, |reg_idx| last_use[reg_idx as usize] = i);
    }

    // First pass: record all Copy sources
    let immutable_limit =
        u32::try_from(param_count + const_count).expect("Register index overflow");
    for (i, instr) in out.iter().enumerate() {
        if let Instruction::Copy { dest, src } = *instr {
            if dest == output_reg || dest == src {
                continue;
            }

            if src < immutable_limit {
                // Immutable source (Param/Const): chain-follow is always safe since
                // the value can never be redefined.
                copy_of[dest as usize] = copy_of[src as usize];
            } else {
                let root = copy_of[src as usize];
                if root < immutable_limit {
                    // src is a temp already forwarded to an immutable root.
                    // Jumping directly to the immutable root is always safe.
                    copy_of[dest as usize] = root;
                } else if root == src && src_next_def_at_idx[i] > last_use[dest as usize] {
                    // src is a non-forwarded temp that isn't redefined before the
                    // last use of dest. Safe to forward exactly one hop.
                    copy_of[dest as usize] = src;
                }
                // else: src chains to a mutable temp — skip forwarding to avoid
                // creating stale chains. The Copy survives and its src is rewritten
                // through copy_of correctly by the rewrite pass below.
            }
        }
    }

    // Second pass: rewrite all reads through the forwarding table, then drop dead Copies
    out.retain_mut(|instr| {
        instr.map_reads(|reg_idx| copy_of[reg_idx as usize]);
        instr.map_pooled_regs(arg_pool, |reg_idx| copy_of[reg_idx as usize]);
        if let Instruction::Copy { dest, src } = *instr {
            // Drop useless copies, but keep if dest was NOT forwarded (i.e. it's still needed as a named register)
            dest != src && copy_of[dest as usize] == dest
        } else {
            true
        }
    });

    // --- Backward Dead Code Elimination (DCE) and final instruction retention ---
    // Copy forwarding or earlier fusion passes can leave dead instructions.
    // This pass identifies and removes them, combining the final retain operations.
    lives.clear();
    lives.resize(max_reg_len, false);
    // Seed liveness with the required output register
    lives[output_reg as usize] = true;

    // We need to build `live_instrs` on the instructions *after* copy forwarding.
    // This array will store whether each instruction is live.
    let mut temp_live_instrs = vec![false; out.len()];

    for (i, instr) in out.iter().enumerate().rev() {
        // Iterate over the *current* `out`
        let mut is_live = false;
        instr.for_each_write(|d| {
            if lives[d as usize] {
                is_live = true;
            }
        });

        if is_live {
            temp_live_instrs[i] = true;
            // Kill the destination(s) - they are redefined here
            instr.for_each_write(|d| lives[d as usize] = false);

            // Gen reads (they are needed to compute this instruction)
            instr.for_each_read(|r| lives[r as usize] = true);
            instr.for_each_pooled_reg(arg_pool, |reg_idx| lives[reg_idx as usize] = true);
        }
    }

    // Combine the two retain passes into one final pass.
    // The `map_reads` and `map_pooled_regs` have already been applied to all instructions.
    let mut current_idx = 0;
    out.retain_mut(|instr| {
        let is_dead_copy = if let Instruction::Copy { dest, src } = *instr {
            dest == src || copy_of[dest as usize] != dest
        } else {
            false // Not a copy, so not a "dead copy"
        };
        let is_live = temp_live_instrs[current_idx];
        current_idx += 1;
        is_live && !is_dead_copy
    });

    out
}
