use super::Instruction;
use rustc_hash::FxHashMap;
use std::cell::RefCell;

/// Compacts the constant pool and reassigns register indices.
///
/// In the Unified Memory Layout, constants occupy registers `param_count .. param_count + const_count`.
/// This pass identifies which constants are actually used and removes unused ones,
/// shifting the remaining constants and all temporary registers down.
pub(super) fn compact_constants(
    mut out: Vec<Instruction>,
    constants: &mut Vec<f64>,
    arg_pool: &mut [u32],
    param_count: usize,
    old_const_count: usize,
    output_reg: u32,
) -> (Vec<Instruction>, usize, u32) {
    let param_count_u32 = u32::try_from(param_count).expect("Param count overflow");
    let const_limit_u32 =
        u32::try_from(param_count + old_const_count).expect("Register index overflow");

    let all_used_indices = collect_used_constant_indices(
        &out,
        arg_pool,
        param_count_u32,
        const_limit_u32,
        old_const_count,
        output_reg,
    );

    // If all constants are used, just return the current state
    if all_used_indices.iter().all(|&used| used) {
        let final_max_reg = max_register_index(&out, arg_pool);
        return (
            out,
            (final_max_reg as usize + 1).max(param_count + old_const_count),
            output_reg,
        );
    }

    // Compact the constant vector and create a map from old register index to new register index
    let index_map = compact_constant_pool(constants, &all_used_indices, param_count_u32);

    // Remap all instructions and the output register
    let new_output_reg = remap_after_constant_compaction(
        &mut out,
        arg_pool,
        &index_map,
        param_count_u32,
        const_limit_u32,
        output_reg,
        constants.len(),
    );

    let final_max_reg = max_register_index(&out, arg_pool);

    (
        out,
        (final_max_reg as usize + 1).max(param_count + constants.len()),
        new_output_reg,
    )
}

fn collect_used_constant_indices(
    instructions: &[Instruction],
    arg_pool: &[u32],
    param_count_u32: u32,
    const_limit_u32: u32,
    constant_count: usize,
    output_reg: u32,
) -> Vec<bool> {
    let mut used_pool_indices = vec![false; constant_count];
    for instr in instructions {
        instr.for_each_read(|reg_idx| {
            if reg_idx >= param_count_u32 && reg_idx < const_limit_u32 {
                used_pool_indices[(reg_idx - param_count_u32) as usize] = true;
            }
        });
        instr.for_each_pooled_reg(arg_pool, |reg_idx| {
            if reg_idx >= param_count_u32 && reg_idx < const_limit_u32 {
                used_pool_indices[(reg_idx - param_count_u32) as usize] = true;
            }
        });
    }
    // The final result might be a literal constant
    if output_reg >= param_count_u32 && output_reg < const_limit_u32 {
        used_pool_indices[(output_reg - param_count_u32) as usize] = true;
    }
    used_pool_indices
}

fn compact_constant_pool(
    constants: &mut Vec<f64>,
    used_indices: &[bool],
    param_count_u32: u32,
) -> Vec<Option<u32>> {
    let mut new_constants = Vec::with_capacity(used_indices.len());
    let mut index_map = vec![None; used_indices.len()];

    for (old_rel_idx, &used) in used_indices.iter().enumerate() {
        if used {
            index_map[old_rel_idx] = Some(
                param_count_u32
                    + u32::try_from(new_constants.len()).expect("Constant index overflow"),
            );
            new_constants.push(constants[old_rel_idx]);
        }
    }

    *constants = new_constants;
    index_map
}

#[allow(
    clippy::too_many_arguments,
    reason = "Internal remapping requires multiple parameters to track old/new register boundaries"
)]
fn remap_after_constant_compaction(
    instructions: &mut [Instruction],
    arg_pool: &mut [u32],
    index_map: &[Option<u32>],
    param_count_u32: u32,
    const_limit_u32: u32,
    output_reg: u32,
    new_const_count: usize,
) -> u32 {
    let temp_start =
        param_count_u32 + u32::try_from(new_const_count).expect("New constant count overflow");
    let next_temp = RefCell::new(temp_start);
    let temp_map = RefCell::new(FxHashMap::default());

    let remap_register = |reg_idx: u32| {
        if reg_idx < param_count_u32 {
            reg_idx
        } else if reg_idx < const_limit_u32 {
            let old_rel_idx = reg_idx - param_count_u32;
            index_map[old_rel_idx as usize].expect("Constant index missing in map during remapping")
        } else {
            // This is a temporary register, densely pack it in first-encounter order
            *temp_map.borrow_mut().entry(reg_idx).or_insert_with(|| {
                let mut p = next_temp.borrow_mut();
                let v = *p;
                *p += 1;
                v
            })
        }
    };

    for instr in instructions {
        instr.map_all_regs(arg_pool, &remap_register);
    }

    remap_register(output_reg)
}

fn max_register_index(instrs: &[Instruction], arg_pool: &[u32]) -> u32 {
    let mut final_max_reg = 0;
    for instr in instrs {
        instr.for_each_reg(|reg_idx| final_max_reg = final_max_reg.max(reg_idx));
        instr.for_each_pooled_reg(arg_pool, |reg_idx| {
            final_max_reg = final_max_reg.max(reg_idx);
        });
    }
    final_max_reg
}
