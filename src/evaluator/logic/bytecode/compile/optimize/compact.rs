use super::instruction::Instruction;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

#[allow(
    clippy::too_many_lines,
    clippy::float_cmp,
    reason = "Peephole optimizer with multiple patterns; exact float comparison is intentional for safe algebraic folding"
)]
pub(super) fn compact_constants(
    mut out: Vec<Instruction>,
    constants: &mut Vec<f64>,
    _const_map: &mut FxHashMap<u64, u32>,
    arg_pool: &mut [u32],
    param_count: usize,
    old_const_count: usize,
) -> (Vec<Instruction>, usize) {
    let param_count_u32 = u32::try_from(param_count).expect("Param count overflow");
    let const_limit_u32 =
        u32::try_from(param_count + old_const_count).expect("Register index overflow");

    let all_used_indices =
        collect_used_constant_indices(&out, arg_pool, param_count_u32, const_limit_u32);

    if all_used_indices.len() == old_const_count {
        let final_max_reg = max_register_index(&out, arg_pool);
        return (
            out,
            (final_max_reg as usize + 1).max(param_count + old_const_count),
        );
    }

    let index_map = compact_constant_pool(constants, &all_used_indices);
    remap_after_constant_compaction(
        &mut out,
        arg_pool,
        &index_map,
        param_count_u32,
        const_limit_u32,
        old_const_count,
        constants.len(),
    );

    let final_max_reg = max_register_index(&out, arg_pool);

    (
        out,
        (final_max_reg as usize + 1).max(param_count + constants.len()),
    )
}

fn collect_used_constant_indices(
    instructions: &[Instruction],
    arg_pool: &[u32],
    param_count_u32: u32,
    const_limit_u32: u32,
) -> FxHashSet<u32> {
    let mut used_pool_indices = FxHashSet::default();
    for instr in instructions {
        match instr {
            Instruction::LoadConst { const_idx, .. }
            | Instruction::MulAddConst { const_idx, .. }
            | Instruction::MulSubConst { const_idx, .. }
            | Instruction::NegMulAddConst { const_idx, .. }
            | Instruction::AddConst { const_idx, .. }
            | Instruction::MulConst { const_idx, .. }
            | Instruction::SubConst { const_idx, .. }
            | Instruction::ConstSub { const_idx, .. }
            | Instruction::DivConst { const_idx, .. }
            | Instruction::NegMulConst { const_idx, .. }
            | Instruction::ConstDiv { const_idx, .. } => {
                used_pool_indices.insert(*const_idx);
            }
            _ => {}
        }
    }

    for instr in instructions {
        instr.for_each_read(|reg_idx| {
            if reg_idx >= param_count_u32 && reg_idx < const_limit_u32 {
                used_pool_indices.insert(reg_idx - param_count_u32);
            }
        });
        instr.for_each_pooled_reg(arg_pool, |reg_idx| {
            if reg_idx >= param_count_u32 && reg_idx < const_limit_u32 {
                used_pool_indices.insert(reg_idx - param_count_u32);
            }
        });
    }

    used_pool_indices
}

fn compact_constant_pool(
    constants: &mut Vec<f64>,
    used_indices: &FxHashSet<u32>,
) -> FxHashMap<u32, u32> {
    let mut new_constants = Vec::with_capacity(used_indices.len());
    let mut index_map = FxHashMap::default();

    for (old_idx, &value) in constants.iter().enumerate() {
        let old_idx_u32 = u32::try_from(old_idx).expect("Constant pool index too large for u32");
        if used_indices.contains(&old_idx_u32) {
            index_map.insert(
                old_idx_u32,
                u32::try_from(new_constants.len()).expect("New constant pool index too large"),
            );
            new_constants.push(value);
        }
    }

    *constants = new_constants;
    index_map
}

/// Rewrites all register references and constant indices after constant pool compaction.
///
/// # Register layout
///
/// Registers are laid out as: `[params | constants | temps]`.
/// When constants are removed, temps must shift down to fill the gap.
///
/// ## Worked example
///
/// ```text
/// Before: 2 params, 5 constants, temps start at register 7
///   R0=param0  R1=param1  R2=C0  R3=C1  R4=C2  R5=C3  R6=C4  R7=temp0  R8=temp1
///
/// After compaction: only 3 constants survive (C0, C2, C4 kept)
///   R0=param0  R1=param1  R2=C0  R3=C2  R4=C4  R5=temp0  R6=temp1
///
/// shift = old_const_count - new_const_count = 5 - 3 = 2
/// Old R7 (temp0) → 7 - 2 = R5
/// Old R8 (temp1) → 8 - 2 = R6
/// ```
fn remap_after_constant_compaction(
    instructions: &mut [Instruction],
    arg_pool: &mut [u32],
    index_map: &FxHashMap<u32, u32>,
    param_count_u32: u32,
    const_limit_u32: u32,
    old_const_count: usize,
    new_const_count: usize,
) {
    // A positive shift means constants were removed (temps move down).
    // A negative shift means constants were added (temps move up).
    let shift = i32::try_from(old_const_count).expect("Old const count overflow")
        - i32::try_from(new_const_count).expect("New const count overflow");

    let remap_register = |reg_idx: u32| {
        if reg_idx < param_count_u32 {
            reg_idx
        } else if reg_idx < const_limit_u32 {
            let old_const_idx = reg_idx - param_count_u32;
            param_count_u32 + *index_map.get(&old_const_idx).unwrap_or(&0)
        } else {
            let new_idx = i32::try_from(reg_idx).expect("Register index overflow") - shift;
            u32::try_from(new_idx).expect("Register index underflow/overflow")
        }
    };

    for instr in instructions {
        remap_instruction_consts(instr, index_map);
        instr.map_regs(&remap_register);
        instr.map_pooled_regs(arg_pool, &remap_register);
    }
}

fn remap_instruction_consts(instr: &mut Instruction, index_map: &FxHashMap<u32, u32>) {
    match instr {
        Instruction::LoadConst { const_idx, .. }
        | Instruction::MulAddConst { const_idx, .. }
        | Instruction::MulSubConst { const_idx, .. }
        | Instruction::NegMulAddConst { const_idx, .. }
        | Instruction::AddConst { const_idx, .. }
        | Instruction::MulConst { const_idx, .. }
        | Instruction::SubConst { const_idx, .. }
        | Instruction::ConstSub { const_idx, .. }
        | Instruction::DivConst { const_idx, .. }
        | Instruction::ConstDiv { const_idx, .. }
        | Instruction::NegMulConst { const_idx, .. } => {
            if let Some(&new_idx) = index_map.get(const_idx) {
                *const_idx = new_idx;
            }
        }
        _ => {}
    }
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
