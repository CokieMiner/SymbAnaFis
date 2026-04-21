use super::vir::{VInstruction, VReg};

/// Performs backward Dead Code Elimination (DCE) on Virtual IR.
///
/// Returns the optimized instruction stream and the maximum temporary register index + 1.
pub fn eliminate_vir_dead_code(
    vinstrs: Vec<VInstruction>,
    final_vreg: Option<VReg>,
    next_vreg: u32,
) -> (Vec<VInstruction>, usize) {
    if vinstrs.is_empty() {
        return (vinstrs, 0);
    }

    // Temp IDs are dense u32s — Vec<bool> is O(1) lookup with no hashing.
    let mut live = vec![false; next_vreg as usize];
    if let Some(VReg::Temp(t)) = final_vreg {
        live[t as usize] = true;
    }

    let mut optimized = Vec::with_capacity(vinstrs.len());
    let mut max_temp = 0_u32;
    for instr in vinstrs.into_iter().rev() {
        #[allow(
            clippy::unreachable,
            reason = "VInstruction destination is guaranteed to be VReg::Temp"
        )]
        let keep = match instr.dest() {
            VReg::Temp(t) => live[t as usize],
            _ => unreachable!("VInstruction dest is always VReg::Temp"),
        };

        if keep {
            instr.for_each_read(|r| {
                if let VReg::Temp(t) = r {
                    live[t as usize] = true;
                    max_temp = max_temp.max(t + 1);
                }
            });
            if let VReg::Temp(t) = instr.dest() {
                max_temp = max_temp.max(t + 1);
            }
            optimized.push(instr);
        }
    }
    optimized.reverse();

    if let Some(VReg::Temp(t)) = final_vreg {
        max_temp = max_temp.max(t + 1);
    }

    (optimized, max_temp as usize)
}
