use super::{VInstruction, VReg};
use rustc_hash::{FxBuildHasher, FxHashMap};

/// Replaces redundant divisions by the same denominator with a single reciprocal
/// and multiple multiplications.
/// Returns the optimized instructions and the updated `next_vreg` counter.
pub fn optimize_div_to_recip(
    vinstrs: Vec<VInstruction>,
    next_vreg: u32,
) -> (Vec<VInstruction>, u32) {
    let mut den_counts: FxHashMap<VReg, usize> = FxHashMap::default();

    // First pass: count division denominators
    for instr in &vinstrs {
        if let VInstruction::Div { den, .. } = instr {
            *den_counts.entry(*den).or_insert(0) += 1;
        }
    }

    let mut optimized = Vec::with_capacity(vinstrs.len());
    let mut current_next_vreg = next_vreg;
    let mut recips_created: FxHashMap<VReg, VReg> = FxHashMap::default();

    // Second pass: replace redundant divisions
    for instr in vinstrs {
        if let VInstruction::Div { dest, num, den } = instr
            && den_counts.get(&den).copied().unwrap_or(0) > 1
        {
            let recip_reg = *recips_created.entry(den).or_insert_with(|| {
                let reg = VReg::Temp(current_next_vreg);
                current_next_vreg += 1;
                optimized.push(VInstruction::Recip {
                    dest: reg,
                    src: den,
                });
                reg
            });

            optimized.push(VInstruction::Mul2 {
                dest,
                a: num,
                b: recip_reg,
            });
            continue;
        }
        optimized.push(instr);
    }

    (optimized, current_next_vreg)
}

// ── Pre-scheduling FMA fusion ────────────────────────────────────────────

/// Light fusion pass for VIR instructions before scheduling.
///
/// This pass combines patterns like Mul + Add into `MulAdd` to ensure
/// the scheduler treats them as a single unit, preventing them from being
/// separated and thus missing FMA fusion opportunities in the physical pass.
pub fn fuse_vir(instructions: Vec<VInstruction>, next_vreg: VReg) -> Vec<VInstruction> {
    if instructions.is_empty() {
        return instructions;
    }

    let max_reg = match next_vreg {
        VReg::Temp(t) => t as usize,
        VReg::Param(_) | VReg::Const(_) => 1024,
    };

    let mut use_count = vec![0_u32; max_reg];
    for instr in &instructions {
        instr.for_each_read(|r| {
            if let VReg::Temp(t) = r {
                let idx = t as usize;
                if let Some(count) = use_count.get_mut(idx) {
                    *count += 1;
                }
            }
        });
    }

    let mut out: Vec<VInstruction> = Vec::with_capacity(instructions.len());
    let mut producer_idx: FxHashMap<VReg, usize> =
        FxHashMap::with_capacity_and_hasher(instructions.len(), FxBuildHasher);

    for instr in instructions {
        let mut fused = false;
        match instr {
            VInstruction::Add2 { dest, a, b } => {
                fused = process_add2(dest, a, b, &mut out, &mut producer_idx, &use_count);
            }
            VInstruction::Sub { dest, a, b } => {
                fused = process_sub(dest, a, b, &mut out, &mut producer_idx, &use_count);
            }
            VInstruction::Neg { dest, src } => {
                fused = process_neg(dest, src, &mut out, &mut producer_idx, &use_count);
            }
            VInstruction::Add { .. }
            | VInstruction::Mul { .. }
            | VInstruction::Mul2 { .. }
            | VInstruction::Div { .. }
            | VInstruction::Pow { .. }
            | VInstruction::NegMul { .. }
            | VInstruction::BuiltinFun { .. }
            | VInstruction::Builtin1 { .. }
            | VInstruction::Builtin2 { .. }
            | VInstruction::Square { .. }
            | VInstruction::Cube { .. }
            | VInstruction::Pow4 { .. }
            | VInstruction::Pow3_2 { .. }
            | VInstruction::InvPow3_2 { .. }
            | VInstruction::InvSqrt { .. }
            | VInstruction::InvSquare { .. }
            | VInstruction::InvCube { .. }
            | VInstruction::Recip { .. }
            | VInstruction::Powi { .. }
            | VInstruction::MulAdd { .. }
            | VInstruction::MulSub { .. }
            | VInstruction::NegMulAdd { .. }
            | VInstruction::NegMulSub { .. }
            | VInstruction::RecipExpm1 { .. }
            | VInstruction::ExpSqr { .. }
            | VInstruction::ExpSqrNeg { .. } => {}
        }

        if !fused {
            let dest = instr.dest();
            if let VReg::Temp(_) = dest {
                producer_idx.insert(dest, out.len());
            }
            out.push(instr);
        }
    }

    out
}

fn process_add2(
    dest: VReg,
    a: VReg,
    b: VReg,
    out: &mut Vec<VInstruction>,
    producer_idx: &mut FxHashMap<VReg, usize>,
    use_count: &[u32],
) -> bool {
    if let Some(new_instr) = try_fuse_add(dest, a, b, out, producer_idx, use_count) {
        let op_to_remove = if let VReg::Temp(ta) = a {
            if use_count[ta as usize] == 1 { a } else { b }
        } else {
            b
        };

        if matches!(producer_idx.get(&op_to_remove), Some(&idx) if idx == out.len() - 1) {
            out.pop();
            producer_idx.remove(&op_to_remove);
            if let VReg::Temp(_) = dest {
                producer_idx.insert(dest, out.len());
            }
            out.push(new_instr);
            return true;
        }
    }
    false
}

fn process_sub(
    dest: VReg,
    a: VReg,
    b: VReg,
    out: &mut Vec<VInstruction>,
    producer_idx: &mut FxHashMap<VReg, usize>,
    use_count: &[u32],
) -> bool {
    if let Some(new_instr) = try_fuse_sub(dest, a, b, out, producer_idx, use_count) {
        let op_to_remove = match a {
            VReg::Temp(ta) if use_count[ta as usize] == 1 => {
                if let Some(&idx) = producer_idx.get(&a) {
                    if matches!(
                        out.get(idx),
                        Some(VInstruction::Mul2 { .. } | VInstruction::NegMul { .. })
                    ) {
                        a
                    } else {
                        b
                    }
                } else {
                    b
                }
            }
            VReg::Param(_) | VReg::Const(_) | VReg::Temp(_) => b,
        };

        if matches!(producer_idx.get(&op_to_remove), Some(&idx) if idx == out.len() - 1) {
            out.pop();
            producer_idx.remove(&op_to_remove);
            if let VReg::Temp(_) = dest {
                producer_idx.insert(dest, out.len());
            }
            out.push(new_instr);
            return true;
        }
    }
    false
}

fn process_neg(
    dest: VReg,
    src: VReg,
    out: &mut Vec<VInstruction>,
    producer_idx: &mut FxHashMap<VReg, usize>,
    use_count: &[u32],
) -> bool {
    let last_idx = out.len().wrapping_sub(1);
    match src {
        VReg::Temp(ts) if use_count[ts as usize] == 1 => {
            match (producer_idx.get(&src), out.get(last_idx)) {
                (Some(&idx), Some(VInstruction::Mul2 { a: ma, b: mb, .. })) if idx == last_idx => {
                    let new_instr = VInstruction::NegMul {
                        dest,
                        a: *ma,
                        b: *mb,
                    };
                    out.pop();
                    producer_idx.remove(&src);
                    if let VReg::Temp(_) = dest {
                        producer_idx.insert(dest, out.len());
                    }
                    out.push(new_instr);
                    true
                }
                (
                    _,
                    Some(
                        VInstruction::Add { .. }
                        | VInstruction::Add2 { .. }
                        | VInstruction::Mul { .. }
                        | VInstruction::Mul2 { .. }
                        | VInstruction::Sub { .. }
                        | VInstruction::Div { .. }
                        | VInstruction::Pow { .. }
                        | VInstruction::Neg { .. }
                        | VInstruction::NegMul { .. }
                        | VInstruction::BuiltinFun { .. }
                        | VInstruction::Builtin1 { .. }
                        | VInstruction::Builtin2 { .. }
                        | VInstruction::Square { .. }
                        | VInstruction::Cube { .. }
                        | VInstruction::Pow4 { .. }
                        | VInstruction::Pow3_2 { .. }
                        | VInstruction::InvPow3_2 { .. }
                        | VInstruction::InvSqrt { .. }
                        | VInstruction::InvSquare { .. }
                        | VInstruction::InvCube { .. }
                        | VInstruction::Recip { .. }
                        | VInstruction::Powi { .. }
                        | VInstruction::MulAdd { .. }
                        | VInstruction::MulSub { .. }
                        | VInstruction::NegMulAdd { .. }
                        | VInstruction::NegMulSub { .. }
                        | VInstruction::RecipExpm1 { .. }
                        | VInstruction::ExpSqr { .. }
                        | VInstruction::ExpSqrNeg { .. },
                    )
                    | None,
                ) => false,
            }
        }
        VReg::Param(_) | VReg::Const(_) | VReg::Temp(_) => false,
    }
}

fn try_fuse_add(
    dest: VReg,
    a: VReg,
    b: VReg,
    out: &[VInstruction],
    producer_idx: &FxHashMap<VReg, usize>,
    use_count: &[u32],
) -> Option<VInstruction> {
    // Check 'a'
    match a {
        VReg::Temp(ta) if use_count[ta as usize] == 1 => match producer_idx.get(&a) {
            Some(&idx) => match &out[idx] {
                VInstruction::Mul2 { a: ma, b: mb, .. } => Some(VInstruction::MulAdd {
                    dest,
                    a: *ma,
                    b: *mb,
                    c: b,
                }),
                VInstruction::NegMul { a: ma, b: mb, .. } => Some(VInstruction::NegMulAdd {
                    dest,
                    a: *ma,
                    b: *mb,
                    c: b,
                }),
                VInstruction::Add { .. }
                | VInstruction::Add2 { .. }
                | VInstruction::Mul { .. }
                | VInstruction::Sub { .. }
                | VInstruction::Div { .. }
                | VInstruction::Pow { .. }
                | VInstruction::Neg { .. }
                | VInstruction::BuiltinFun { .. }
                | VInstruction::Builtin1 { .. }
                | VInstruction::Builtin2 { .. }
                | VInstruction::Square { .. }
                | VInstruction::Cube { .. }
                | VInstruction::Pow4 { .. }
                | VInstruction::Pow3_2 { .. }
                | VInstruction::InvPow3_2 { .. }
                | VInstruction::InvSqrt { .. }
                | VInstruction::InvSquare { .. }
                | VInstruction::InvCube { .. }
                | VInstruction::Recip { .. }
                | VInstruction::Powi { .. }
                | VInstruction::MulAdd { .. }
                | VInstruction::MulSub { .. }
                | VInstruction::NegMulAdd { .. }
                | VInstruction::NegMulSub { .. }
                | VInstruction::RecipExpm1 { .. }
                | VInstruction::ExpSqr { .. }
                | VInstruction::ExpSqrNeg { .. } => None,
            },
            None => None,
        },
        VReg::Param(_) | VReg::Const(_) | VReg::Temp(_) => None,
    }
    .or_else(|| {
        // Check 'b'
        match b {
            VReg::Temp(tb) if use_count[tb as usize] == 1 => match producer_idx.get(&b) {
                Some(&idx) => match &out[idx] {
                    VInstruction::Mul2 { a: ma, b: mb, .. } => Some(VInstruction::MulAdd {
                        dest,
                        a: *ma,
                        b: *mb,
                        c: a,
                    }),
                    VInstruction::NegMul { a: ma, b: mb, .. } => Some(VInstruction::NegMulAdd {
                        dest,
                        a: *ma,
                        b: *mb,
                        c: a,
                    }),
                    VInstruction::Add { .. }
                    | VInstruction::Add2 { .. }
                    | VInstruction::Mul { .. }
                    | VInstruction::Sub { .. }
                    | VInstruction::Div { .. }
                    | VInstruction::Pow { .. }
                    | VInstruction::Neg { .. }
                    | VInstruction::BuiltinFun { .. }
                    | VInstruction::Builtin1 { .. }
                    | VInstruction::Builtin2 { .. }
                    | VInstruction::Square { .. }
                    | VInstruction::Cube { .. }
                    | VInstruction::Pow4 { .. }
                    | VInstruction::Pow3_2 { .. }
                    | VInstruction::InvPow3_2 { .. }
                    | VInstruction::InvSqrt { .. }
                    | VInstruction::InvSquare { .. }
                    | VInstruction::InvCube { .. }
                    | VInstruction::Recip { .. }
                    | VInstruction::Powi { .. }
                    | VInstruction::MulAdd { .. }
                    | VInstruction::MulSub { .. }
                    | VInstruction::NegMulAdd { .. }
                    | VInstruction::NegMulSub { .. }
                    | VInstruction::RecipExpm1 { .. }
                    | VInstruction::ExpSqr { .. }
                    | VInstruction::ExpSqrNeg { .. } => None,
                },
                None => None,
            },
            VReg::Param(_) | VReg::Const(_) | VReg::Temp(_) => None,
        }
    })
}

fn try_fuse_sub(
    dest: VReg,
    a: VReg,
    b: VReg,
    out: &[VInstruction],
    producer_idx: &FxHashMap<VReg, usize>,
    use_count: &[u32],
) -> Option<VInstruction> {
    // a - b
    // case 1: (Mul x y) - b -> MulSub(x, y, b)
    match a {
        VReg::Temp(ta) if use_count[ta as usize] == 1 => match producer_idx.get(&a) {
            Some(&idx) => match &out[idx] {
                VInstruction::Mul2 { a: ma, b: mb, .. } => Some(VInstruction::MulSub {
                    dest,
                    a: *ma,
                    b: *mb,
                    c: b,
                }),
                VInstruction::NegMul { a: ma, b: mb, .. } => Some(VInstruction::NegMulSub {
                    dest,
                    a: *ma,
                    b: *mb,
                    c: b,
                }),
                VInstruction::Add { .. }
                | VInstruction::Add2 { .. }
                | VInstruction::Mul { .. }
                | VInstruction::Sub { .. }
                | VInstruction::Div { .. }
                | VInstruction::Pow { .. }
                | VInstruction::Neg { .. }
                | VInstruction::BuiltinFun { .. }
                | VInstruction::Builtin1 { .. }
                | VInstruction::Builtin2 { .. }
                | VInstruction::Square { .. }
                | VInstruction::Cube { .. }
                | VInstruction::Pow4 { .. }
                | VInstruction::Pow3_2 { .. }
                | VInstruction::InvPow3_2 { .. }
                | VInstruction::InvSqrt { .. }
                | VInstruction::InvSquare { .. }
                | VInstruction::InvCube { .. }
                | VInstruction::Recip { .. }
                | VInstruction::Powi { .. }
                | VInstruction::MulAdd { .. }
                | VInstruction::MulSub { .. }
                | VInstruction::NegMulAdd { .. }
                | VInstruction::NegMulSub { .. }
                | VInstruction::RecipExpm1 { .. }
                | VInstruction::ExpSqr { .. }
                | VInstruction::ExpSqrNeg { .. } => None,
            },
            None => None,
        },
        VReg::Param(_) | VReg::Const(_) | VReg::Temp(_) => None,
    }
    .or_else(|| {
        // case 2: a - (Mul x y) -> NegMulAdd(x, y, a)
        match b {
            VReg::Temp(tb) if use_count[tb as usize] == 1 => match producer_idx.get(&b) {
                Some(&idx) => match &out[idx] {
                    VInstruction::Mul2 { a: ma, b: mb, .. } => Some(VInstruction::NegMulAdd {
                        dest,
                        a: *ma,
                        b: *mb,
                        c: a,
                    }),
                    VInstruction::NegMul { a: ma, b: mb, .. } => Some(VInstruction::MulAdd {
                        dest,
                        a: *ma,
                        b: *mb,
                        c: a,
                    }),
                    VInstruction::Add { .. }
                    | VInstruction::Add2 { .. }
                    | VInstruction::Mul { .. }
                    | VInstruction::Sub { .. }
                    | VInstruction::Div { .. }
                    | VInstruction::Pow { .. }
                    | VInstruction::Neg { .. }
                    | VInstruction::BuiltinFun { .. }
                    | VInstruction::Builtin1 { .. }
                    | VInstruction::Builtin2 { .. }
                    | VInstruction::Square { .. }
                    | VInstruction::Cube { .. }
                    | VInstruction::Pow4 { .. }
                    | VInstruction::Pow3_2 { .. }
                    | VInstruction::InvPow3_2 { .. }
                    | VInstruction::InvSqrt { .. }
                    | VInstruction::InvSquare { .. }
                    | VInstruction::InvCube { .. }
                    | VInstruction::Recip { .. }
                    | VInstruction::Powi { .. }
                    | VInstruction::MulAdd { .. }
                    | VInstruction::MulSub { .. }
                    | VInstruction::NegMulAdd { .. }
                    | VInstruction::NegMulSub { .. }
                    | VInstruction::RecipExpm1 { .. }
                    | VInstruction::ExpSqr { .. }
                    | VInstruction::ExpSqrNeg { .. } => None,
                },
                None => None,
            },
            VReg::Param(_) | VReg::Const(_) | VReg::Temp(_) => None,
        }
    })
}
