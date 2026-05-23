use super::helper::ConstantPool;
use super::{FnOp, Instruction};
use rustc_hash::FxHashMap;

/// Stack-only replacement for `Vec<Instruction>` in fusion results.
/// Eliminates heap allocation for every matched pattern (up to 3 instructions).
enum FuseResult {
    One(Instruction),
    Two(Instruction, Instruction),
}

impl FuseResult {
    #[inline]
    fn push_to(self, out: &mut Vec<Instruction>) {
        match self {
            Self::One(a) => out.push(a),
            Self::Two(a, b) => {
                out.push(a);
                out.push(b);
            }
        }
    }
}

/// Peephole optimizer that fuses consecutive instruction pairs into more efficient single
/// instructions.
///
/// Simplified for the Unified Memory Layout: constants are now treated as regular registers,
/// so we no longer need specialized *Const variants or `LoadConst` fusions.
#[allow(
    clippy::too_many_lines,
    reason = "Peephole optimizer with multiple patterns; exact float comparison is intentional for safe algebraic folding"
)]
pub(super) fn fuse_instructions(
    instructions: &[Instruction],
    pool: &mut ConstantPool<'_>,
    use_count: &[usize],
    arg_pool: &[u32],
) -> (Vec<Instruction>, bool) {
    let single_use = |reg_idx: &u32| use_count[*reg_idx as usize] == 1;
    let instr_count = instructions.len();
    let mut out = Vec::with_capacity(instr_count);
    let mut instr_idx = 0;
    let mut changed = false;

    while instr_idx < instr_count {
        if instr_idx + 1 < instr_count {
            let prev = &instructions[instr_idx];
            let next = &instructions[instr_idx + 1];

            if let Some(fused) = try_fuse_negation(prev, next, pool, &single_use) {
                fused.push_to(&mut out);
                instr_idx += 2;
                changed = true;
                continue;
            }
            if let Some(fused) = try_fuse_mul_add(prev, next, &single_use) {
                fused.push_to(&mut out);
                instr_idx += 2;
                changed = true;
                continue;
            }
            if let Some(fused) = try_fuse_power(prev, next, pool, &single_use) {
                fused.push_to(&mut out);
                instr_idx += 2;
                changed = true;
                continue;
            }
            if let Some(fused) = try_fuse_inverse(prev, next, &single_use) {
                fused.push_to(&mut out);
                instr_idx += 2;
                changed = true;
                continue;
            }
            if let Some(fused) = try_fuse_logarithmic(prev, next, pool, &single_use) {
                fused.push_to(&mut out);
                instr_idx += 2;
                changed = true;
                continue;
            }
            if let Some(fused) = try_fuse_idempotent(prev, next, &single_use) {
                fused.push_to(&mut out);
                instr_idx += 2;
                changed = true;
                continue;
            }
            if let Some(fused) = try_fuse_const_chain(prev, next, pool, &single_use) {
                fused.push_to(&mut out);
                instr_idx += 2;
                changed = true;
                continue;
            }
        }

        out.push(instructions[instr_idx]);
        instr_idx += 1;
    }

    // P2: Non-adjacent SinCos fusion
    let sin_cos_changed = fuse_sin_cos(&mut out, arg_pool);

    // P3: Non-adjacent AsinAcos fusion
    let asin_acos_changed = fuse_asin_acos(&mut out, arg_pool);

    (out, changed || sin_cos_changed || asin_acos_changed)
}

fn has_conflict(instructions: &[Instruction], dest: u32, arg_pool: &[u32]) -> bool {
    for instr in instructions {
        let mut conflict = false;
        instr.for_each_write(|d| {
            if d == dest {
                conflict = true;
            }
        });
        if conflict {
            return true;
        }
        instr.for_each_read(|r| {
            if r == dest {
                conflict = true;
            }
        });
        if conflict {
            return true;
        }
        instr.for_each_pooled_reg(arg_pool, |r| {
            if r == dest {
                conflict = true;
            }
        });
        if conflict {
            return true;
        }
    }
    false
}

fn fuse_sin_cos(instructions: &mut [Instruction], arg_pool: &[u32]) -> bool {
    let mut changed = false;

    let mut sin_map: FxHashMap<u32, (usize, u32)> = FxHashMap::default();
    let mut cos_map: FxHashMap<u32, (usize, u32)> = FxHashMap::default();
    let mut pairs: Vec<(usize, usize, u32, u32, u32)> = Vec::new();

    for (idx, instr) in instructions.iter().enumerate() {
        match *instr {
            Instruction::Sin { dest, arg } => {
                if let Some(&(cos_idx, cos_dest)) = cos_map.get(&arg) {
                    pairs.push((idx, cos_idx, arg, dest, cos_dest));
                    cos_map.remove(&arg);
                } else {
                    sin_map.insert(arg, (idx, dest));
                }
            }
            Instruction::Cos { dest, arg } => {
                if let Some(&(sin_idx, sin_dest)) = sin_map.get(&arg) {
                    pairs.push((sin_idx, idx, arg, sin_dest, dest));
                    sin_map.remove(&arg);
                } else {
                    cos_map.insert(arg, (idx, dest));
                }
            }
            Instruction::End { .. }
            | Instruction::Copy { .. }
            | Instruction::Neg { .. }
            | Instruction::SinCos { .. }
            | Instruction::AsinAcos { .. }
            | Instruction::Add { .. }
            | Instruction::Add3 { .. }
            | Instruction::Add4 { .. }
            | Instruction::AddN { .. }
            | Instruction::Mul { .. }
            | Instruction::Mul3 { .. }
            | Instruction::Mul4 { .. }
            | Instruction::MulN { .. }
            | Instruction::Sub { .. }
            | Instruction::Div { .. }
            | Instruction::Pow { .. }
            | Instruction::MulAdd { .. }
            | Instruction::MulSub { .. }
            | Instruction::NegMul { .. }
            | Instruction::NegMulAdd { .. }
            | Instruction::NegMulSub { .. }
            | Instruction::Square { .. }
            | Instruction::Cube { .. }
            | Instruction::Pow4 { .. }
            | Instruction::Pow3_2 { .. }
            | Instruction::InvPow3_2 { .. }
            | Instruction::InvSqrt { .. }
            | Instruction::InvSquare { .. }
            | Instruction::InvCube { .. }
            | Instruction::Recip { .. }
            | Instruction::Powi { .. }
            | Instruction::Exp { .. }
            | Instruction::Ln { .. }
            | Instruction::Sqrt { .. }
            | Instruction::RecipExpm1 { .. }
            | Instruction::ExpSqr { .. }
            | Instruction::ExpSqrNeg { .. }
            | Instruction::Builtin1 { .. }
            | Instruction::Builtin2 { .. }
            | Instruction::Builtin3 { .. }
            | Instruction::Builtin4 { .. } => {
                instr.for_each_write(|dest| {
                    sin_map.remove(&dest);
                    cos_map.remove(&dest);
                });
            }
        }
    }

    for (sin_idx, cos_idx, arg, sin_dest, cos_dest) in pairs {
        let later_idx = sin_idx.max(cos_idx);
        let earlier_idx = sin_idx.min(cos_idx);
        let earlier_dest = if earlier_idx == sin_idx {
            sin_dest
        } else {
            cos_dest
        };

        if has_conflict(
            &instructions[earlier_idx + 1..later_idx],
            earlier_dest,
            arg_pool,
        ) {
            continue;
        }

        instructions[later_idx] = Instruction::SinCos {
            sin_dest,
            cos_dest,
            arg,
        };
        instructions[earlier_idx] = Instruction::Copy {
            dest: earlier_dest,
            src: earlier_dest,
        };
        changed = true;
    }
    changed
}

fn find_asin_acos_pairs(instructions: &[Instruction]) -> Vec<(usize, usize, u32, u32, u32)> {
    let mut asin_map: FxHashMap<u32, (usize, u32)> = FxHashMap::default();
    let mut acos_map: FxHashMap<u32, (usize, u32)> = FxHashMap::default();
    let mut pairs: Vec<(usize, usize, u32, u32, u32)> = Vec::new();

    for (idx, instr) in instructions.iter().enumerate() {
        match *instr {
            Instruction::Builtin1 {
                dest,
                op: FnOp::Asin,
                arg,
            } => {
                if let Some(&(acos_idx, acos_dest)) = acos_map.get(&arg) {
                    pairs.push((idx, acos_idx, arg, dest, acos_dest));
                    acos_map.remove(&arg);
                } else {
                    asin_map.insert(arg, (idx, dest));
                }
            }
            Instruction::Builtin1 {
                dest,
                op: FnOp::Acos,
                arg,
            } => {
                if let Some(&(asin_idx, asin_dest)) = asin_map.get(&arg) {
                    pairs.push((asin_idx, idx, arg, asin_dest, dest));
                    asin_map.remove(&arg);
                } else {
                    acos_map.insert(arg, (idx, dest));
                }
            }
            Instruction::End { .. }
            | Instruction::Copy { .. }
            | Instruction::Neg { .. }
            | Instruction::SinCos { .. }
            | Instruction::AsinAcos { .. }
            | Instruction::Add { .. }
            | Instruction::Add3 { .. }
            | Instruction::Add4 { .. }
            | Instruction::AddN { .. }
            | Instruction::Mul { .. }
            | Instruction::Mul3 { .. }
            | Instruction::Mul4 { .. }
            | Instruction::MulN { .. }
            | Instruction::Sub { .. }
            | Instruction::Div { .. }
            | Instruction::Pow { .. }
            | Instruction::MulAdd { .. }
            | Instruction::MulSub { .. }
            | Instruction::NegMul { .. }
            | Instruction::NegMulAdd { .. }
            | Instruction::NegMulSub { .. }
            | Instruction::Square { .. }
            | Instruction::Cube { .. }
            | Instruction::Pow4 { .. }
            | Instruction::Pow3_2 { .. }
            | Instruction::InvPow3_2 { .. }
            | Instruction::InvSqrt { .. }
            | Instruction::InvSquare { .. }
            | Instruction::InvCube { .. }
            | Instruction::Recip { .. }
            | Instruction::Powi { .. }
            | Instruction::Sin { .. }
            | Instruction::Cos { .. }
            | Instruction::Exp { .. }
            | Instruction::Ln { .. }
            | Instruction::Sqrt { .. }
            | Instruction::RecipExpm1 { .. }
            | Instruction::ExpSqr { .. }
            | Instruction::ExpSqrNeg { .. }
            | Instruction::Builtin2 { .. }
            | Instruction::Builtin3 { .. }
            | Instruction::Builtin4 { .. } => {
                instr.for_each_write(|write_dest| {
                    asin_map.remove(&write_dest);
                    acos_map.remove(&write_dest);
                });
            }
            Instruction::Builtin1 { op, .. } => {
                if !matches!(op, FnOp::Asin | FnOp::Acos) {
                    instr.for_each_write(|write_dest| {
                        asin_map.remove(&write_dest);
                        acos_map.remove(&write_dest);
                    });
                }
            }
        }
    }
    pairs
}

fn fuse_asin_acos(instructions: &mut [Instruction], arg_pool: &[u32]) -> bool {
    let mut changed = false;
    let pairs = find_asin_acos_pairs(instructions);

    for (asin_idx, acos_idx, arg, asin_dest, acos_dest) in pairs {
        let later_idx = asin_idx.max(acos_idx);
        let earlier_idx = asin_idx.min(acos_idx);
        let earlier_dest = if earlier_idx == asin_idx {
            asin_dest
        } else {
            acos_dest
        };

        if has_conflict(
            &instructions[earlier_idx + 1..later_idx],
            earlier_dest,
            arg_pool,
        ) {
            continue;
        }

        instructions[later_idx] = Instruction::AsinAcos {
            asin_dest,
            acos_dest,
            arg,
        };
        instructions[earlier_idx] = Instruction::Copy {
            dest: earlier_dest,
            src: earlier_dest,
        };
        changed = true;
    }
    changed
}

fn try_fuse_negation(
    prev: &Instruction,
    next: &Instruction,
    _pool: &ConstantPool<'_>,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match *prev {
        Instruction::Neg {
            dest: neg_dest,
            src: neg_src,
        } => fuse_neg_with_next(next, neg_dest, neg_src, single_use),
        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::Mul { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Square { .. }
        | Instruction::Cube { .. }
        | Instruction::Pow4 { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::InvSquare { .. }
        | Instruction::InvCube { .. }
        | Instruction::Recip { .. }
        | Instruction::Powi { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Exp { .. }
        | Instruction::Ln { .. }
        | Instruction::Sqrt { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}

fn fuse_neg_with_next(
    next: &Instruction,
    neg_dest: u32,
    neg_src: u32,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match *next {
        Instruction::Mul {
            dest: mul_dest,
            a: mul_a,
            b: mul_b,
        } if single_use(&neg_dest) => {
            if mul_a == neg_dest {
                Some(FuseResult::One(Instruction::NegMul {
                    dest: mul_dest,
                    a: neg_src,
                    b: mul_b,
                }))
            } else if mul_b == neg_dest {
                Some(FuseResult::One(Instruction::NegMul {
                    dest: mul_dest,
                    a: neg_src,
                    b: mul_a,
                }))
            } else {
                None
            }
        }
        Instruction::Exp {
            dest: exp_dest,
            arg: exp_arg,
        } if exp_arg == neg_dest && single_use(&neg_dest) => {
            Some(FuseResult::One(Instruction::Builtin1 {
                dest: exp_dest,
                op: FnOp::ExpNeg,
                arg: neg_src,
            }))
        }
        Instruction::Square {
            dest: dest_reg,
            src: tmp_src,
        } if tmp_src == neg_dest && single_use(&neg_dest) => {
            Some(FuseResult::One(Instruction::Square {
                dest: dest_reg,
                src: neg_src,
            }))
        }
        Instruction::Builtin1 {
            dest: dest_reg,
            op: FnOp::Abs,
            arg: tmp_arg,
        } if tmp_arg == neg_dest && single_use(&neg_dest) => {
            Some(FuseResult::One(Instruction::Builtin1 {
                dest: dest_reg,
                op: FnOp::Abs,
                arg: neg_src,
            }))
        }
        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::Neg { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Cube { .. }
        | Instruction::Pow4 { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::InvSquare { .. }
        | Instruction::InvCube { .. }
        | Instruction::Recip { .. }
        | Instruction::Powi { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Ln { .. }
        | Instruction::Sqrt { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Mul { .. }
        | Instruction::Square { .. }
        | Instruction::Exp { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "Function handles extensive pattern matching for peephole optimization fusions"
)]
fn try_fuse_mul_add(
    prev: &Instruction,
    next: &Instruction,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match prev {
        Instruction::Mul { dest, a, b } => {
            let mul_dest = *dest;
            let mul_a = *a;
            let mul_b = *b;
            let is_neg = false;
            if !single_use(&mul_dest) {
                return None;
            }
            handle_mul_add_fusion(
                next,
                MulAddParams {
                    mul_dest,
                    mul_a,
                    mul_b,
                    is_neg,
                },
            )
        }
        Instruction::NegMul { dest, a, b } => {
            let mul_dest = *dest;
            let mul_a = *a;
            let mul_b = *b;
            let is_neg = true;
            if !single_use(&mul_dest) {
                return None;
            }
            handle_mul_add_fusion(
                next,
                MulAddParams {
                    mul_dest,
                    mul_a,
                    mul_b,
                    is_neg,
                },
            )
        }
        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::Neg { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Square { .. }
        | Instruction::Cube { .. }
        | Instruction::Pow4 { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::InvSquare { .. }
        | Instruction::InvCube { .. }
        | Instruction::Recip { .. }
        | Instruction::Powi { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Exp { .. }
        | Instruction::Ln { .. }
        | Instruction::Sqrt { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}

#[derive(Clone, Copy)]
struct MulAddParams {
    mul_dest: u32,
    mul_a: u32,
    mul_b: u32,
    is_neg: bool,
}

const fn handle_mul_add_fusion(next: &Instruction, params: MulAddParams) -> Option<FuseResult> {
    match *next {
        Instruction::Add { dest, a, b } => fuse_with_add(dest, a, b, params),
        Instruction::Sub { dest, a, b } => fuse_with_sub(dest, a, b, params),
        Instruction::Add3 { dest, a, b, c } => fuse_with_add3(dest, a, b, c, params),
        Instruction::Add4 { dest, a, b, c, d } => fuse_with_add4(dest, a, b, c, d, params),
        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::Neg { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Square { .. }
        | Instruction::Cube { .. }
        | Instruction::Pow4 { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::InvSquare { .. }
        | Instruction::InvCube { .. }
        | Instruction::Recip { .. }
        | Instruction::Powi { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Exp { .. }
        | Instruction::Ln { .. }
        | Instruction::Sqrt { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}

const fn fuse_with_add(dest: u32, a: u32, b: u32, params: MulAddParams) -> Option<FuseResult> {
    if a == params.mul_dest {
        Some(FuseResult::One(if params.is_neg {
            Instruction::NegMulAdd {
                dest,
                a: params.mul_a,
                b: params.mul_b,
                c: b,
            }
        } else {
            Instruction::MulAdd {
                dest,
                a: params.mul_a,
                b: params.mul_b,
                c: b,
            }
        }))
    } else if b == params.mul_dest {
        Some(FuseResult::One(if params.is_neg {
            Instruction::NegMulAdd {
                dest,
                a: params.mul_a,
                b: params.mul_b,
                c: a,
            }
        } else {
            Instruction::MulAdd {
                dest,
                a: params.mul_a,
                b: params.mul_b,
                c: a,
            }
        }))
    } else {
        None
    }
}

const fn fuse_with_sub(dest: u32, a: u32, b: u32, params: MulAddParams) -> Option<FuseResult> {
    if a == params.mul_dest {
        Some(FuseResult::One(if params.is_neg {
            Instruction::NegMulSub {
                dest,
                a: params.mul_a,
                b: params.mul_b,
                c: b,
            }
        } else {
            Instruction::MulSub {
                dest,
                a: params.mul_a,
                b: params.mul_b,
                c: b,
            }
        }))
    } else if b == params.mul_dest {
        Some(FuseResult::One(if params.is_neg {
            Instruction::MulAdd {
                dest,
                a: params.mul_a,
                b: params.mul_b,
                c: a,
            }
        } else {
            Instruction::NegMulAdd {
                dest,
                a: params.mul_a,
                b: params.mul_b,
                c: a,
            }
        }))
    } else {
        None
    }
}

const fn fuse_with_add3(
    dest: u32,
    a: u32,
    b: u32,
    c: u32,
    params: MulAddParams,
) -> Option<FuseResult> {
    let (other1, other2) = if a == params.mul_dest {
        (b, c)
    } else if b == params.mul_dest {
        (a, c)
    } else if c == params.mul_dest {
        (a, b)
    } else {
        return None;
    };
    Some(FuseResult::Two(
        if params.is_neg {
            Instruction::NegMulAdd {
                dest: params.mul_dest,
                a: params.mul_a,
                b: params.mul_b,
                c: other1,
            }
        } else {
            Instruction::MulAdd {
                dest: params.mul_dest,
                a: params.mul_a,
                b: params.mul_b,
                c: other1,
            }
        },
        Instruction::Add {
            dest,
            a: params.mul_dest,
            b: other2,
        },
    ))
}

const fn fuse_with_add4(
    dest: u32,
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    params: MulAddParams,
) -> Option<FuseResult> {
    let (other1, other2, other3) = if a == params.mul_dest {
        (b, c, d)
    } else if b == params.mul_dest {
        (a, c, d)
    } else if c == params.mul_dest {
        (a, b, d)
    } else if d == params.mul_dest {
        (a, b, c)
    } else {
        return None;
    };
    Some(FuseResult::Two(
        if params.is_neg {
            Instruction::NegMulAdd {
                dest: params.mul_dest,
                a: params.mul_a,
                b: params.mul_b,
                c: other1,
            }
        } else {
            Instruction::MulAdd {
                dest: params.mul_dest,
                a: params.mul_a,
                b: params.mul_b,
                c: other1,
            }
        },
        Instruction::Add3 {
            dest,
            a: params.mul_dest,
            b: other2,
            c: other3,
        },
    ))
}

#[allow(
    clippy::too_many_lines,
    reason = "Function handles extensive pattern matching for peephole optimization fusions"
)]
fn try_fuse_power(
    prev: &Instruction,
    next: &Instruction,
    pool: &mut ConstantPool<'_>,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match *prev {
        Instruction::Square {
            dest: sq1_dest,
            src: sq1_src,
        } => match *next {
            Instruction::Square {
                dest: sq2_dest,
                src: sq2_src,
            } if sq2_src == sq1_dest && single_use(&sq1_dest) => {
                Some(FuseResult::One(Instruction::Pow4 {
                    dest: sq2_dest,
                    src: sq1_src,
                }))
            }
            Instruction::Mul {
                dest: mul_dest,
                a: mul_a,
                b: mul_b,
            } if single_use(&sq1_dest)
                && ((mul_a == sq1_dest && mul_b == sq1_src)
                    || (mul_a == sq1_src && mul_b == sq1_dest)) =>
            {
                Some(FuseResult::One(Instruction::Cube {
                    dest: mul_dest,
                    src: sq1_src,
                }))
            }
            Instruction::Exp {
                dest: dest_reg,
                arg: tmp_arg,
            } if tmp_arg == sq1_dest && single_use(&sq1_dest) => {
                Some(FuseResult::One(Instruction::ExpSqr {
                    dest: dest_reg,
                    src: sq1_src,
                }))
            }
            Instruction::End { .. }
            | Instruction::Copy { .. }
            | Instruction::Neg { .. }
            | Instruction::SinCos { .. }
            | Instruction::AsinAcos { .. }
            | Instruction::Add { .. }
            | Instruction::Add3 { .. }
            | Instruction::Add4 { .. }
            | Instruction::AddN { .. }
            | Instruction::Mul { .. }
            | Instruction::Mul3 { .. }
            | Instruction::Mul4 { .. }
            | Instruction::MulN { .. }
            | Instruction::Sub { .. }
            | Instruction::Div { .. }
            | Instruction::Pow { .. }
            | Instruction::MulAdd { .. }
            | Instruction::MulSub { .. }
            | Instruction::NegMul { .. }
            | Instruction::NegMulAdd { .. }
            | Instruction::NegMulSub { .. }
            | Instruction::Square { .. }
            | Instruction::Cube { .. }
            | Instruction::Pow4 { .. }
            | Instruction::Pow3_2 { .. }
            | Instruction::InvPow3_2 { .. }
            | Instruction::InvSqrt { .. }
            | Instruction::InvSquare { .. }
            | Instruction::InvCube { .. }
            | Instruction::Recip { .. }
            | Instruction::Powi { .. }
            | Instruction::Sin { .. }
            | Instruction::Cos { .. }
            | Instruction::Exp { .. }
            | Instruction::Ln { .. }
            | Instruction::Sqrt { .. }
            | Instruction::RecipExpm1 { .. }
            | Instruction::ExpSqr { .. }
            | Instruction::ExpSqrNeg { .. }
            | Instruction::Builtin1 { .. }
            | Instruction::Builtin2 { .. }
            | Instruction::Builtin3 { .. }
            | Instruction::Builtin4 { .. } => None,
        },
        Instruction::Cube {
            dest: cube_dest,
            src: cube_src,
        } => match *next {
            Instruction::Mul {
                dest: mul_dest,
                a: mul_a,
                b: mul_b,
            } if single_use(&cube_dest)
                && ((mul_a == cube_dest && mul_b == cube_src)
                    || (mul_a == cube_src && mul_b == cube_dest)) =>
            {
                Some(FuseResult::One(Instruction::Pow4 {
                    dest: mul_dest,
                    src: cube_src,
                }))
            }
            Instruction::End { .. }
            | Instruction::Copy { .. }
            | Instruction::Neg { .. }
            | Instruction::SinCos { .. }
            | Instruction::AsinAcos { .. }
            | Instruction::Add { .. }
            | Instruction::Add3 { .. }
            | Instruction::Add4 { .. }
            | Instruction::AddN { .. }
            | Instruction::Mul { .. }
            | Instruction::Mul3 { .. }
            | Instruction::Mul4 { .. }
            | Instruction::MulN { .. }
            | Instruction::Sub { .. }
            | Instruction::Div { .. }
            | Instruction::Pow { .. }
            | Instruction::MulAdd { .. }
            | Instruction::MulSub { .. }
            | Instruction::NegMul { .. }
            | Instruction::NegMulAdd { .. }
            | Instruction::NegMulSub { .. }
            | Instruction::Square { .. }
            | Instruction::Cube { .. }
            | Instruction::Pow4 { .. }
            | Instruction::Pow3_2 { .. }
            | Instruction::InvPow3_2 { .. }
            | Instruction::InvSqrt { .. }
            | Instruction::InvSquare { .. }
            | Instruction::InvCube { .. }
            | Instruction::Recip { .. }
            | Instruction::Powi { .. }
            | Instruction::Sin { .. }
            | Instruction::Cos { .. }
            | Instruction::Exp { .. }
            | Instruction::Ln { .. }
            | Instruction::Sqrt { .. }
            | Instruction::RecipExpm1 { .. }
            | Instruction::ExpSqr { .. }
            | Instruction::ExpSqrNeg { .. }
            | Instruction::Builtin1 { .. }
            | Instruction::Builtin2 { .. }
            | Instruction::Builtin3 { .. }
            | Instruction::Builtin4 { .. } => None,
        },
        Instruction::Exp {
            dest: exp_dest,
            arg: exp_arg,
        } => match *next {
            Instruction::Square {
                dest: sq_dest,
                src: sq_src,
            } if sq_src == exp_dest && single_use(&exp_dest) => {
                let c_2 = pool.get_or_insert(2.0);
                Some(FuseResult::Two(
                    Instruction::Mul {
                        dest: exp_dest,
                        a: exp_arg,
                        b: c_2,
                    },
                    Instruction::Builtin1 {
                        dest: sq_dest,
                        op: FnOp::Exp,
                        arg: exp_dest,
                    },
                ))
            }
            Instruction::End { .. }
            | Instruction::Copy { .. }
            | Instruction::Neg { .. }
            | Instruction::SinCos { .. }
            | Instruction::AsinAcos { .. }
            | Instruction::Add { .. }
            | Instruction::Add3 { .. }
            | Instruction::Add4 { .. }
            | Instruction::AddN { .. }
            | Instruction::Mul { .. }
            | Instruction::Mul3 { .. }
            | Instruction::Mul4 { .. }
            | Instruction::MulN { .. }
            | Instruction::Sub { .. }
            | Instruction::Div { .. }
            | Instruction::Pow { .. }
            | Instruction::MulAdd { .. }
            | Instruction::MulSub { .. }
            | Instruction::NegMul { .. }
            | Instruction::NegMulAdd { .. }
            | Instruction::NegMulSub { .. }
            | Instruction::Square { .. }
            | Instruction::Cube { .. }
            | Instruction::Pow4 { .. }
            | Instruction::Pow3_2 { .. }
            | Instruction::InvPow3_2 { .. }
            | Instruction::InvSqrt { .. }
            | Instruction::InvSquare { .. }
            | Instruction::InvCube { .. }
            | Instruction::Recip { .. }
            | Instruction::Powi { .. }
            | Instruction::Sin { .. }
            | Instruction::Cos { .. }
            | Instruction::Exp { .. }
            | Instruction::Ln { .. }
            | Instruction::Sqrt { .. }
            | Instruction::RecipExpm1 { .. }
            | Instruction::ExpSqr { .. }
            | Instruction::ExpSqrNeg { .. }
            | Instruction::Builtin1 { .. }
            | Instruction::Builtin2 { .. }
            | Instruction::Builtin3 { .. }
            | Instruction::Builtin4 { .. } => None,
        },
        Instruction::Pow4 {
            dest: p4_dest,
            src: p4_src,
        } => match *next {
            Instruction::Recip {
                dest: recip_dest,
                src: recip_src,
            } if recip_src == p4_dest && single_use(&p4_dest) => Some(FuseResult::Two(
                Instruction::Square {
                    dest: p4_dest,
                    src: p4_src,
                },
                Instruction::InvSquare {
                    dest: recip_dest,
                    src: p4_dest,
                },
            )),
            Instruction::End { .. }
            | Instruction::Copy { .. }
            | Instruction::Neg { .. }
            | Instruction::SinCos { .. }
            | Instruction::AsinAcos { .. }
            | Instruction::Add { .. }
            | Instruction::Add3 { .. }
            | Instruction::Add4 { .. }
            | Instruction::AddN { .. }
            | Instruction::Mul { .. }
            | Instruction::Mul3 { .. }
            | Instruction::Mul4 { .. }
            | Instruction::MulN { .. }
            | Instruction::Sub { .. }
            | Instruction::Div { .. }
            | Instruction::Pow { .. }
            | Instruction::MulAdd { .. }
            | Instruction::MulSub { .. }
            | Instruction::NegMul { .. }
            | Instruction::NegMulAdd { .. }
            | Instruction::NegMulSub { .. }
            | Instruction::Square { .. }
            | Instruction::Cube { .. }
            | Instruction::Pow4 { .. }
            | Instruction::Pow3_2 { .. }
            | Instruction::InvPow3_2 { .. }
            | Instruction::InvSqrt { .. }
            | Instruction::InvSquare { .. }
            | Instruction::InvCube { .. }
            | Instruction::Recip { .. }
            | Instruction::Powi { .. }
            | Instruction::Sin { .. }
            | Instruction::Cos { .. }
            | Instruction::Exp { .. }
            | Instruction::Ln { .. }
            | Instruction::Sqrt { .. }
            | Instruction::RecipExpm1 { .. }
            | Instruction::ExpSqr { .. }
            | Instruction::ExpSqrNeg { .. }
            | Instruction::Builtin1 { .. }
            | Instruction::Builtin2 { .. }
            | Instruction::Builtin3 { .. }
            | Instruction::Builtin4 { .. } => None,
        },
        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::Neg { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::InvSquare { .. }
        | Instruction::InvCube { .. }
        | Instruction::Recip { .. }
        | Instruction::Powi { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Ln { .. }
        | Instruction::Sqrt { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}

fn try_fuse_inverse(
    prev: &Instruction,
    next: &Instruction,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match *prev {
        Instruction::Sqrt {
            dest: sqrt_dest,
            arg: sqrt_src,
        } => fuse_sqrt_with_next(next, sqrt_dest, sqrt_src, single_use),

        Instruction::Square {
            dest: sq_dest,
            src: sq_src,
        } => fuse_square_with_next(next, sq_dest, sq_src, single_use),

        Instruction::Cube {
            dest: cube_dest,
            src: cube_src,
        } => fuse_cube_with_next(next, cube_dest, cube_src, single_use),

        Instruction::Pow3_2 {
            dest: p32_dest,
            src: p32_src,
        } => fuse_pow32_with_next(next, p32_dest, p32_src, single_use),

        Instruction::Exp {
            dest: exp_dest,
            arg: exp_arg,
        } => fuse_exp_with_next_inverse(next, exp_dest, exp_arg, single_use),

        Instruction::Builtin1 {
            dest: expm1_dest,
            op: FnOp::Expm1,
            arg: expm1_arg,
        } => fuse_expm1_with_next(next, expm1_dest, expm1_arg, single_use),

        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::Neg { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Pow4 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::InvSquare { .. }
        | Instruction::InvCube { .. }
        | Instruction::Recip { .. }
        | Instruction::Powi { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Ln { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}

fn fuse_sqrt_with_next(
    next: &Instruction,
    sqrt_dest: u32,
    sqrt_src: u32,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match *next {
        Instruction::Recip {
            dest: recip_dest,
            src: recip_src,
        } if recip_src == sqrt_dest && single_use(&sqrt_dest) => {
            Some(FuseResult::One(Instruction::InvSqrt {
                dest: recip_dest,
                src: sqrt_src,
            }))
        }
        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::Neg { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Square { .. }
        | Instruction::Cube { .. }
        | Instruction::Pow4 { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::InvSquare { .. }
        | Instruction::InvCube { .. }
        | Instruction::Recip { .. }
        | Instruction::Powi { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Exp { .. }
        | Instruction::Ln { .. }
        | Instruction::Sqrt { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}

fn fuse_square_with_next(
    next: &Instruction,
    sq_dest: u32,
    sq_src: u32,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match *next {
        Instruction::Recip {
            dest: recip_dest,
            src: recip_src,
        } if recip_src == sq_dest && single_use(&sq_dest) => {
            Some(FuseResult::One(Instruction::InvSquare {
                dest: recip_dest,
                src: sq_src,
            }))
        }
        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::Neg { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Square { .. }
        | Instruction::Cube { .. }
        | Instruction::Pow4 { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::InvSquare { .. }
        | Instruction::InvCube { .. }
        | Instruction::Recip { .. }
        | Instruction::Powi { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Exp { .. }
        | Instruction::Ln { .. }
        | Instruction::Sqrt { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}

fn fuse_cube_with_next(
    next: &Instruction,
    cube_dest: u32,
    cube_src: u32,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match *next {
        Instruction::Recip {
            dest: recip_dest,
            src: recip_src,
        } if recip_src == cube_dest && single_use(&cube_dest) => {
            Some(FuseResult::One(Instruction::InvCube {
                dest: recip_dest,
                src: cube_src,
            }))
        }
        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::Neg { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Square { .. }
        | Instruction::Cube { .. }
        | Instruction::Pow4 { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::InvSquare { .. }
        | Instruction::InvCube { .. }
        | Instruction::Recip { .. }
        | Instruction::Powi { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Exp { .. }
        | Instruction::Ln { .. }
        | Instruction::Sqrt { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}

fn fuse_pow32_with_next(
    next: &Instruction,
    p32_dest: u32,
    p32_src: u32,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match *next {
        Instruction::Recip {
            dest: recip_dest,
            src: recip_src,
        } if recip_src == p32_dest && single_use(&p32_dest) => {
            Some(FuseResult::One(Instruction::InvPow3_2 {
                dest: recip_dest,
                src: p32_src,
            }))
        }
        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::Neg { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Square { .. }
        | Instruction::Cube { .. }
        | Instruction::Pow4 { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::InvSquare { .. }
        | Instruction::InvCube { .. }
        | Instruction::Recip { .. }
        | Instruction::Powi { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Exp { .. }
        | Instruction::Ln { .. }
        | Instruction::Sqrt { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}

fn fuse_exp_with_next_inverse(
    next: &Instruction,
    exp_dest: u32,
    exp_arg: u32,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match *next {
        Instruction::Recip {
            dest: recip_dest,
            src: recip_src,
        } if recip_src == exp_dest && single_use(&exp_dest) => {
            Some(FuseResult::One(Instruction::Builtin1 {
                dest: recip_dest,
                op: FnOp::ExpNeg,
                arg: exp_arg,
            }))
        }
        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::Neg { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Square { .. }
        | Instruction::Cube { .. }
        | Instruction::Pow4 { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::InvSquare { .. }
        | Instruction::InvCube { .. }
        | Instruction::Recip { .. }
        | Instruction::Powi { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Exp { .. }
        | Instruction::Ln { .. }
        | Instruction::Sqrt { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}

fn fuse_expm1_with_next(
    next: &Instruction,
    expm1_dest: u32,
    expm1_arg: u32,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match *next {
        Instruction::Recip {
            dest: recip_dest,
            src: recip_src,
        } if recip_src == expm1_dest && single_use(&expm1_dest) => {
            Some(FuseResult::One(Instruction::RecipExpm1 {
                dest: recip_dest,
                src: expm1_arg,
            }))
        }
        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::Neg { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Square { .. }
        | Instruction::Cube { .. }
        | Instruction::Pow4 { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::InvSquare { .. }
        | Instruction::InvCube { .. }
        | Instruction::Recip { .. }
        | Instruction::Powi { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Exp { .. }
        | Instruction::Ln { .. }
        | Instruction::Sqrt { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}

#[allow(
    clippy::too_many_lines,
    clippy::float_cmp,
    reason = "Function handles extensive pattern matching for peephole optimization fusions"
)]
fn try_fuse_logarithmic(
    prev: &Instruction,
    next: &Instruction,
    pool: &mut ConstantPool<'_>,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match *prev {
        Instruction::Exp {
            dest: exp_dest,
            arg: exp_arg,
        } => match *next {
            Instruction::Ln {
                dest: final_dest,
                arg: tmp_arg,
            } if tmp_arg == exp_dest && single_use(&exp_dest) => {
                Some(FuseResult::One(Instruction::Copy {
                    dest: final_dest,
                    src: exp_arg,
                }))
            }
            Instruction::Sub {
                dest: final_dest,
                a: tmp_arg,
                b: c_reg,
            } if tmp_arg == exp_dest
                && single_use(&exp_dest)
                && pool.is_constant(c_reg)
                && (pool.get(c_reg) - 1.0).abs() < f64::EPSILON =>
            {
                Some(FuseResult::One(Instruction::Builtin1 {
                    dest: final_dest,
                    op: FnOp::Expm1,
                    arg: exp_arg,
                }))
            }
            Instruction::End { .. }
            | Instruction::Copy { .. }
            | Instruction::Neg { .. }
            | Instruction::SinCos { .. }
            | Instruction::AsinAcos { .. }
            | Instruction::Add { .. }
            | Instruction::Add3 { .. }
            | Instruction::Add4 { .. }
            | Instruction::AddN { .. }
            | Instruction::Mul { .. }
            | Instruction::Mul3 { .. }
            | Instruction::Mul4 { .. }
            | Instruction::MulN { .. }
            | Instruction::Sub { .. }
            | Instruction::Div { .. }
            | Instruction::Pow { .. }
            | Instruction::MulAdd { .. }
            | Instruction::MulSub { .. }
            | Instruction::NegMul { .. }
            | Instruction::NegMulAdd { .. }
            | Instruction::NegMulSub { .. }
            | Instruction::Square { .. }
            | Instruction::Cube { .. }
            | Instruction::Pow4 { .. }
            | Instruction::Pow3_2 { .. }
            | Instruction::InvPow3_2 { .. }
            | Instruction::InvSqrt { .. }
            | Instruction::InvSquare { .. }
            | Instruction::InvCube { .. }
            | Instruction::Recip { .. }
            | Instruction::Powi { .. }
            | Instruction::Sin { .. }
            | Instruction::Cos { .. }
            | Instruction::Exp { .. }
            | Instruction::Ln { .. }
            | Instruction::Sqrt { .. }
            | Instruction::RecipExpm1 { .. }
            | Instruction::ExpSqr { .. }
            | Instruction::ExpSqrNeg { .. }
            | Instruction::Builtin1 { .. }
            | Instruction::Builtin2 { .. }
            | Instruction::Builtin3 { .. }
            | Instruction::Builtin4 { .. } => None,
        },
        Instruction::Pow {
            dest: tmp_dest,
            base: src_x,
            exp: src_n,
        } => match *next {
            Instruction::Ln {
                dest: final_dest,
                arg: tmp_arg,
            } if tmp_arg == tmp_dest && single_use(&tmp_dest) && pool.is_constant(src_n) => {
                let n = pool.get(src_n);
                let ln_x = Instruction::Ln {
                    dest: final_dest,
                    arg: src_x,
                };
                let c_n = pool.get_or_insert(n);
                Some(FuseResult::Two(
                    ln_x,
                    Instruction::Mul {
                        dest: final_dest,
                        a: final_dest,
                        b: c_n,
                    },
                ))
            }
            Instruction::End { .. }
            | Instruction::Copy { .. }
            | Instruction::Neg { .. }
            | Instruction::SinCos { .. }
            | Instruction::AsinAcos { .. }
            | Instruction::Add { .. }
            | Instruction::Add3 { .. }
            | Instruction::Add4 { .. }
            | Instruction::AddN { .. }
            | Instruction::Mul { .. }
            | Instruction::Mul3 { .. }
            | Instruction::Mul4 { .. }
            | Instruction::MulN { .. }
            | Instruction::Sub { .. }
            | Instruction::Div { .. }
            | Instruction::Pow { .. }
            | Instruction::MulAdd { .. }
            | Instruction::MulSub { .. }
            | Instruction::NegMul { .. }
            | Instruction::NegMulAdd { .. }
            | Instruction::NegMulSub { .. }
            | Instruction::Square { .. }
            | Instruction::Cube { .. }
            | Instruction::Pow4 { .. }
            | Instruction::Pow3_2 { .. }
            | Instruction::InvPow3_2 { .. }
            | Instruction::InvSqrt { .. }
            | Instruction::InvSquare { .. }
            | Instruction::InvCube { .. }
            | Instruction::Recip { .. }
            | Instruction::Powi { .. }
            | Instruction::Sin { .. }
            | Instruction::Cos { .. }
            | Instruction::Exp { .. }
            | Instruction::Ln { .. }
            | Instruction::Sqrt { .. }
            | Instruction::RecipExpm1 { .. }
            | Instruction::ExpSqr { .. }
            | Instruction::ExpSqrNeg { .. }
            | Instruction::Builtin1 { .. }
            | Instruction::Builtin2 { .. }
            | Instruction::Builtin3 { .. }
            | Instruction::Builtin4 { .. } => None,
        },
        Instruction::Sqrt {
            dest: tmp_dest,
            arg: src_reg,
        } => match *next {
            Instruction::Ln {
                dest: dest_reg,
                arg: tmp_arg,
            } if tmp_arg == tmp_dest && single_use(&tmp_dest) => {
                let ln_x = Instruction::Ln {
                    dest: dest_reg,
                    arg: src_reg,
                };
                let c_0_5 = pool.get_or_insert(0.5);
                Some(FuseResult::Two(
                    ln_x,
                    Instruction::Mul {
                        dest: dest_reg,
                        a: dest_reg,
                        b: c_0_5,
                    },
                ))
            }
            Instruction::End { .. }
            | Instruction::Copy { .. }
            | Instruction::Neg { .. }
            | Instruction::SinCos { .. }
            | Instruction::AsinAcos { .. }
            | Instruction::Add { .. }
            | Instruction::Add3 { .. }
            | Instruction::Add4 { .. }
            | Instruction::AddN { .. }
            | Instruction::Mul { .. }
            | Instruction::Mul3 { .. }
            | Instruction::Mul4 { .. }
            | Instruction::MulN { .. }
            | Instruction::Sub { .. }
            | Instruction::Div { .. }
            | Instruction::Pow { .. }
            | Instruction::MulAdd { .. }
            | Instruction::MulSub { .. }
            | Instruction::NegMul { .. }
            | Instruction::NegMulAdd { .. }
            | Instruction::NegMulSub { .. }
            | Instruction::Square { .. }
            | Instruction::Cube { .. }
            | Instruction::Pow4 { .. }
            | Instruction::Pow3_2 { .. }
            | Instruction::InvPow3_2 { .. }
            | Instruction::InvSqrt { .. }
            | Instruction::InvSquare { .. }
            | Instruction::InvCube { .. }
            | Instruction::Recip { .. }
            | Instruction::Powi { .. }
            | Instruction::Sin { .. }
            | Instruction::Cos { .. }
            | Instruction::Exp { .. }
            | Instruction::Ln { .. }
            | Instruction::Sqrt { .. }
            | Instruction::RecipExpm1 { .. }
            | Instruction::ExpSqr { .. }
            | Instruction::ExpSqrNeg { .. }
            | Instruction::Builtin1 { .. }
            | Instruction::Builtin2 { .. }
            | Instruction::Builtin3 { .. }
            | Instruction::Builtin4 { .. } => None,
        },
        Instruction::Recip {
            dest: tmp_dest,
            src: src_reg,
        } => match *next {
            Instruction::Ln {
                dest: dest_reg,
                arg: tmp_arg,
            } if tmp_arg == tmp_dest && single_use(&tmp_dest) => {
                let ln_x = Instruction::Ln {
                    dest: tmp_dest,
                    arg: src_reg,
                };
                Some(FuseResult::Two(
                    ln_x,
                    Instruction::Neg {
                        dest: dest_reg,
                        src: tmp_dest,
                    },
                ))
            }
            Instruction::End { .. }
            | Instruction::Copy { .. }
            | Instruction::Neg { .. }
            | Instruction::SinCos { .. }
            | Instruction::AsinAcos { .. }
            | Instruction::Add { .. }
            | Instruction::Add3 { .. }
            | Instruction::Add4 { .. }
            | Instruction::AddN { .. }
            | Instruction::Mul { .. }
            | Instruction::Mul3 { .. }
            | Instruction::Mul4 { .. }
            | Instruction::MulN { .. }
            | Instruction::Sub { .. }
            | Instruction::Div { .. }
            | Instruction::Pow { .. }
            | Instruction::MulAdd { .. }
            | Instruction::MulSub { .. }
            | Instruction::NegMul { .. }
            | Instruction::NegMulAdd { .. }
            | Instruction::NegMulSub { .. }
            | Instruction::Square { .. }
            | Instruction::Cube { .. }
            | Instruction::Pow4 { .. }
            | Instruction::Pow3_2 { .. }
            | Instruction::InvPow3_2 { .. }
            | Instruction::InvSqrt { .. }
            | Instruction::InvSquare { .. }
            | Instruction::InvCube { .. }
            | Instruction::Recip { .. }
            | Instruction::Powi { .. }
            | Instruction::Sin { .. }
            | Instruction::Cos { .. }
            | Instruction::Exp { .. }
            | Instruction::Ln { .. }
            | Instruction::Sqrt { .. }
            | Instruction::RecipExpm1 { .. }
            | Instruction::ExpSqr { .. }
            | Instruction::ExpSqrNeg { .. }
            | Instruction::Builtin1 { .. }
            | Instruction::Builtin2 { .. }
            | Instruction::Builtin3 { .. }
            | Instruction::Builtin4 { .. } => None,
        },
        Instruction::Add {
            dest: tmp_dest,
            a: src_reg,
            b: c_reg,
        } => match *next {
            Instruction::Ln {
                dest: final_dest,
                arg: tmp_arg,
            } if tmp_arg == tmp_dest
                && single_use(&tmp_dest)
                && pool.is_constant(c_reg)
                && (pool.get(c_reg) - 1.0).abs() < f64::EPSILON =>
            {
                Some(FuseResult::One(Instruction::Builtin1 {
                    dest: final_dest,
                    op: FnOp::Log1p,
                    arg: src_reg,
                }))
            }
            Instruction::End { .. }
            | Instruction::Copy { .. }
            | Instruction::Neg { .. }
            | Instruction::SinCos { .. }
            | Instruction::AsinAcos { .. }
            | Instruction::Add { .. }
            | Instruction::Add3 { .. }
            | Instruction::Add4 { .. }
            | Instruction::AddN { .. }
            | Instruction::Mul { .. }
            | Instruction::Mul3 { .. }
            | Instruction::Mul4 { .. }
            | Instruction::MulN { .. }
            | Instruction::Sub { .. }
            | Instruction::Div { .. }
            | Instruction::Pow { .. }
            | Instruction::MulAdd { .. }
            | Instruction::MulSub { .. }
            | Instruction::NegMul { .. }
            | Instruction::NegMulAdd { .. }
            | Instruction::NegMulSub { .. }
            | Instruction::Square { .. }
            | Instruction::Cube { .. }
            | Instruction::Pow4 { .. }
            | Instruction::Pow3_2 { .. }
            | Instruction::InvPow3_2 { .. }
            | Instruction::InvSqrt { .. }
            | Instruction::InvSquare { .. }
            | Instruction::InvCube { .. }
            | Instruction::Recip { .. }
            | Instruction::Powi { .. }
            | Instruction::Sin { .. }
            | Instruction::Cos { .. }
            | Instruction::Exp { .. }
            | Instruction::Ln { .. }
            | Instruction::Sqrt { .. }
            | Instruction::RecipExpm1 { .. }
            | Instruction::ExpSqr { .. }
            | Instruction::ExpSqrNeg { .. }
            | Instruction::Builtin1 { .. }
            | Instruction::Builtin2 { .. }
            | Instruction::Builtin3 { .. }
            | Instruction::Builtin4 { .. } => None,
        },
        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::Neg { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Square { .. }
        | Instruction::Cube { .. }
        | Instruction::Pow4 { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::InvSquare { .. }
        | Instruction::InvCube { .. }
        | Instruction::Powi { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Ln { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}

fn try_fuse_idempotent(
    prev: &Instruction,
    next: &Instruction,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    if let Some(fused) = try_fuse_abs_neg_idempotent(prev, next, single_use) {
        return Some(fused);
    }
    if let Some(fused) = try_fuse_recip_idempotent(prev, next, single_use) {
        return Some(fused);
    }
    if let Some(fused) = try_fuse_power_idempotent(prev, next, single_use) {
        return Some(fused);
    }
    if let Some(fused) = try_fuse_transcendental_idempotent(prev, next, single_use) {
        return Some(fused);
    }
    None
}

fn try_fuse_abs_neg_idempotent(
    prev: &Instruction,
    next: &Instruction,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match *prev {
        Instruction::Builtin1 {
            dest: tmp_dest,
            op: FnOp::Abs,
            arg: src_reg,
        } => fuse_abs_with_next(next, tmp_dest, src_reg, single_use),

        Instruction::Neg {
            dest: tmp_dest,
            src: src_reg,
        } => fuse_neg_with_next_idempotent(next, tmp_dest, src_reg, single_use),

        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Square { .. }
        | Instruction::Cube { .. }
        | Instruction::Pow4 { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::InvSquare { .. }
        | Instruction::InvCube { .. }
        | Instruction::Recip { .. }
        | Instruction::Powi { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Exp { .. }
        | Instruction::Ln { .. }
        | Instruction::Sqrt { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}

fn fuse_abs_with_next(
    next: &Instruction,
    tmp_dest: u32,
    src_reg: u32,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match *next {
        Instruction::Builtin1 {
            dest: dest_reg,
            op: FnOp::Abs,
            arg: tmp_arg,
        } if tmp_arg == tmp_dest && single_use(&tmp_dest) => {
            Some(FuseResult::One(Instruction::Builtin1 {
                dest: dest_reg,
                op: FnOp::Abs,
                arg: src_reg,
            }))
        }
        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::Neg { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Square { .. }
        | Instruction::Cube { .. }
        | Instruction::Pow4 { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::InvSquare { .. }
        | Instruction::InvCube { .. }
        | Instruction::Recip { .. }
        | Instruction::Powi { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Exp { .. }
        | Instruction::Ln { .. }
        | Instruction::Sqrt { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}

fn fuse_neg_with_next_idempotent(
    next: &Instruction,
    tmp_dest: u32,
    src_reg: u32,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match *next {
        Instruction::Neg {
            dest: dest_reg,
            src: tmp_src,
        } if tmp_src == tmp_dest && single_use(&tmp_dest) => {
            Some(FuseResult::One(Instruction::Copy {
                dest: dest_reg,
                src: src_reg,
            }))
        }
        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::Neg { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Square { .. }
        | Instruction::Cube { .. }
        | Instruction::Pow4 { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::InvSquare { .. }
        | Instruction::InvCube { .. }
        | Instruction::Recip { .. }
        | Instruction::Powi { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Exp { .. }
        | Instruction::Ln { .. }
        | Instruction::Sqrt { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "Exhaustive matching of ISA is required for safety"
)]
fn try_fuse_recip_idempotent(
    prev: &Instruction,
    next: &Instruction,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match *prev {
        Instruction::Recip {
            dest: tmp_dest,
            src: src_reg,
        } => match *next {
            Instruction::Recip {
                dest: dest_reg,
                src: tmp_src,
            } if tmp_src == tmp_dest && single_use(&tmp_dest) => {
                Some(FuseResult::One(Instruction::Copy {
                    dest: dest_reg,
                    src: src_reg,
                }))
            }
            Instruction::End { .. }
            | Instruction::Copy { .. }
            | Instruction::Neg { .. }
            | Instruction::SinCos { .. }
            | Instruction::AsinAcos { .. }
            | Instruction::Add { .. }
            | Instruction::Add3 { .. }
            | Instruction::Add4 { .. }
            | Instruction::AddN { .. }
            | Instruction::Mul { .. }
            | Instruction::Mul3 { .. }
            | Instruction::Mul4 { .. }
            | Instruction::MulN { .. }
            | Instruction::Sub { .. }
            | Instruction::Div { .. }
            | Instruction::Pow { .. }
            | Instruction::MulAdd { .. }
            | Instruction::MulSub { .. }
            | Instruction::NegMul { .. }
            | Instruction::NegMulAdd { .. }
            | Instruction::NegMulSub { .. }
            | Instruction::Square { .. }
            | Instruction::Cube { .. }
            | Instruction::Pow4 { .. }
            | Instruction::Pow3_2 { .. }
            | Instruction::InvPow3_2 { .. }
            | Instruction::InvSqrt { .. }
            | Instruction::InvSquare { .. }
            | Instruction::InvCube { .. }
            | Instruction::Recip { .. }
            | Instruction::Powi { .. }
            | Instruction::Sin { .. }
            | Instruction::Cos { .. }
            | Instruction::Exp { .. }
            | Instruction::Ln { .. }
            | Instruction::Sqrt { .. }
            | Instruction::RecipExpm1 { .. }
            | Instruction::ExpSqr { .. }
            | Instruction::ExpSqrNeg { .. }
            | Instruction::Builtin1 { .. }
            | Instruction::Builtin2 { .. }
            | Instruction::Builtin3 { .. }
            | Instruction::Builtin4 { .. } => None,
        },
        Instruction::InvSqrt {
            dest: tmp_dest,
            src: src_reg,
        } => match *next {
            Instruction::Recip {
                dest: dest_reg,
                src: tmp_src,
            } if tmp_src == tmp_dest && single_use(&tmp_dest) => {
                Some(FuseResult::One(Instruction::Builtin1 {
                    dest: dest_reg,
                    op: FnOp::Sqrt,
                    arg: src_reg,
                }))
            }
            Instruction::End { .. }
            | Instruction::Copy { .. }
            | Instruction::Neg { .. }
            | Instruction::SinCos { .. }
            | Instruction::AsinAcos { .. }
            | Instruction::Add { .. }
            | Instruction::Add3 { .. }
            | Instruction::Add4 { .. }
            | Instruction::AddN { .. }
            | Instruction::Mul { .. }
            | Instruction::Mul3 { .. }
            | Instruction::Mul4 { .. }
            | Instruction::MulN { .. }
            | Instruction::Sub { .. }
            | Instruction::Div { .. }
            | Instruction::Pow { .. }
            | Instruction::MulAdd { .. }
            | Instruction::MulSub { .. }
            | Instruction::NegMul { .. }
            | Instruction::NegMulAdd { .. }
            | Instruction::NegMulSub { .. }
            | Instruction::Square { .. }
            | Instruction::Cube { .. }
            | Instruction::Pow4 { .. }
            | Instruction::Pow3_2 { .. }
            | Instruction::InvPow3_2 { .. }
            | Instruction::InvSqrt { .. }
            | Instruction::InvSquare { .. }
            | Instruction::InvCube { .. }
            | Instruction::Recip { .. }
            | Instruction::Powi { .. }
            | Instruction::Sin { .. }
            | Instruction::Cos { .. }
            | Instruction::Exp { .. }
            | Instruction::Ln { .. }
            | Instruction::Sqrt { .. }
            | Instruction::RecipExpm1 { .. }
            | Instruction::ExpSqr { .. }
            | Instruction::ExpSqrNeg { .. }
            | Instruction::Builtin1 { .. }
            | Instruction::Builtin2 { .. }
            | Instruction::Builtin3 { .. }
            | Instruction::Builtin4 { .. } => None,
        },
        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::Neg { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Square { .. }
        | Instruction::Cube { .. }
        | Instruction::Pow4 { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSquare { .. }
        | Instruction::InvCube { .. }
        | Instruction::Powi { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Exp { .. }
        | Instruction::Ln { .. }
        | Instruction::Sqrt { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "Exhaustive matching of ISA is required for safety"
)]
fn try_fuse_power_idempotent(
    prev: &Instruction,
    next: &Instruction,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match *prev {
        Instruction::Square {
            dest: tmp_dest,
            src: src_reg,
        } => match *next {
            Instruction::Sqrt {
                dest: dest_reg,
                arg: tmp_arg,
            } if tmp_arg == tmp_dest && single_use(&tmp_dest) => {
                Some(FuseResult::One(Instruction::Builtin1 {
                    dest: dest_reg,
                    op: FnOp::Abs,
                    arg: src_reg,
                }))
            }
            Instruction::End { .. }
            | Instruction::Copy { .. }
            | Instruction::Neg { .. }
            | Instruction::SinCos { .. }
            | Instruction::AsinAcos { .. }
            | Instruction::Add { .. }
            | Instruction::Add3 { .. }
            | Instruction::Add4 { .. }
            | Instruction::AddN { .. }
            | Instruction::Mul { .. }
            | Instruction::Mul3 { .. }
            | Instruction::Mul4 { .. }
            | Instruction::MulN { .. }
            | Instruction::Sub { .. }
            | Instruction::Div { .. }
            | Instruction::Pow { .. }
            | Instruction::MulAdd { .. }
            | Instruction::MulSub { .. }
            | Instruction::NegMul { .. }
            | Instruction::NegMulAdd { .. }
            | Instruction::NegMulSub { .. }
            | Instruction::Square { .. }
            | Instruction::Cube { .. }
            | Instruction::Pow4 { .. }
            | Instruction::Pow3_2 { .. }
            | Instruction::InvPow3_2 { .. }
            | Instruction::InvSqrt { .. }
            | Instruction::InvSquare { .. }
            | Instruction::InvCube { .. }
            | Instruction::Recip { .. }
            | Instruction::Powi { .. }
            | Instruction::Sin { .. }
            | Instruction::Cos { .. }
            | Instruction::Exp { .. }
            | Instruction::Ln { .. }
            | Instruction::Sqrt { .. }
            | Instruction::RecipExpm1 { .. }
            | Instruction::ExpSqr { .. }
            | Instruction::ExpSqrNeg { .. }
            | Instruction::Builtin1 { .. }
            | Instruction::Builtin2 { .. }
            | Instruction::Builtin3 { .. }
            | Instruction::Builtin4 { .. } => None,
        },
        Instruction::Builtin1 {
            dest: tmp_dest,
            op: FnOp::Abs,
            arg: src_reg,
        } => match *next {
            Instruction::Square {
                dest: dest_reg,
                src: tmp_src,
            } if tmp_src == tmp_dest && single_use(&tmp_dest) => {
                Some(FuseResult::One(Instruction::Square {
                    dest: dest_reg,
                    src: src_reg,
                }))
            }
            Instruction::End { .. }
            | Instruction::Copy { .. }
            | Instruction::Neg { .. }
            | Instruction::SinCos { .. }
            | Instruction::AsinAcos { .. }
            | Instruction::Add { .. }
            | Instruction::Add3 { .. }
            | Instruction::Add4 { .. }
            | Instruction::AddN { .. }
            | Instruction::Mul { .. }
            | Instruction::Mul3 { .. }
            | Instruction::Mul4 { .. }
            | Instruction::MulN { .. }
            | Instruction::Sub { .. }
            | Instruction::Div { .. }
            | Instruction::Pow { .. }
            | Instruction::MulAdd { .. }
            | Instruction::MulSub { .. }
            | Instruction::NegMul { .. }
            | Instruction::NegMulAdd { .. }
            | Instruction::NegMulSub { .. }
            | Instruction::Square { .. }
            | Instruction::Cube { .. }
            | Instruction::Pow4 { .. }
            | Instruction::Pow3_2 { .. }
            | Instruction::InvPow3_2 { .. }
            | Instruction::InvSqrt { .. }
            | Instruction::InvSquare { .. }
            | Instruction::InvCube { .. }
            | Instruction::Recip { .. }
            | Instruction::Powi { .. }
            | Instruction::Sin { .. }
            | Instruction::Cos { .. }
            | Instruction::Exp { .. }
            | Instruction::Ln { .. }
            | Instruction::Sqrt { .. }
            | Instruction::RecipExpm1 { .. }
            | Instruction::ExpSqr { .. }
            | Instruction::ExpSqrNeg { .. }
            | Instruction::Builtin1 { .. }
            | Instruction::Builtin2 { .. }
            | Instruction::Builtin3 { .. }
            | Instruction::Builtin4 { .. } => None,
        },
        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::Neg { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Cube { .. }
        | Instruction::Pow4 { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::InvSquare { .. }
        | Instruction::InvCube { .. }
        | Instruction::Recip { .. }
        | Instruction::Powi { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Exp { .. }
        | Instruction::Ln { .. }
        | Instruction::Sqrt { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}

#[allow(
    clippy::too_many_lines,
    clippy::float_cmp,
    reason = "Exact comparison is intended for algebraic identity matching of constants"
)]
fn try_fuse_const_chain(
    prev: &Instruction,
    next: &Instruction,
    pool: &mut ConstantPool<'_>,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match *prev {
        Instruction::Add {
            dest: t_d,
            a: prev_a,
            b: prev_b,
        } => match *next {
            Instruction::Add {
                dest: f_d,
                a: n_a,
                b: n_b,
            } if single_use(&t_d) => {
                let c2_reg = if n_a == t_d {
                    n_b
                } else if n_b == t_d {
                    n_a
                } else {
                    return None;
                };

                if !pool.is_constant(c2_reg) {
                    return None;
                }

                let (src, c1_reg) = if pool.is_constant(prev_b) {
                    (prev_a, prev_b)
                } else if pool.is_constant(prev_a) {
                    (prev_b, prev_a)
                } else {
                    return None;
                };

                let sum = pool.get(c1_reg) + pool.get(c2_reg);
                if sum == 0.0 {
                    Some(FuseResult::One(Instruction::Copy { dest: f_d, src }))
                } else {
                    let c_sum = pool.get_or_insert(sum);
                    Some(FuseResult::One(Instruction::Add {
                        dest: f_d,
                        a: src,
                        b: c_sum,
                    }))
                }
            }
            Instruction::End { .. }
            | Instruction::Copy { .. }
            | Instruction::Neg { .. }
            | Instruction::SinCos { .. }
            | Instruction::AsinAcos { .. }
            | Instruction::Add { .. }
            | Instruction::Add3 { .. }
            | Instruction::Add4 { .. }
            | Instruction::AddN { .. }
            | Instruction::Mul { .. }
            | Instruction::Mul3 { .. }
            | Instruction::Mul4 { .. }
            | Instruction::MulN { .. }
            | Instruction::Sub { .. }
            | Instruction::Div { .. }
            | Instruction::Pow { .. }
            | Instruction::MulAdd { .. }
            | Instruction::MulSub { .. }
            | Instruction::NegMul { .. }
            | Instruction::NegMulAdd { .. }
            | Instruction::NegMulSub { .. }
            | Instruction::Square { .. }
            | Instruction::Cube { .. }
            | Instruction::Pow4 { .. }
            | Instruction::Pow3_2 { .. }
            | Instruction::InvPow3_2 { .. }
            | Instruction::InvSqrt { .. }
            | Instruction::InvSquare { .. }
            | Instruction::InvCube { .. }
            | Instruction::Recip { .. }
            | Instruction::Powi { .. }
            | Instruction::Sin { .. }
            | Instruction::Cos { .. }
            | Instruction::Exp { .. }
            | Instruction::Ln { .. }
            | Instruction::Sqrt { .. }
            | Instruction::RecipExpm1 { .. }
            | Instruction::ExpSqr { .. }
            | Instruction::ExpSqrNeg { .. }
            | Instruction::Builtin1 { .. }
            | Instruction::Builtin2 { .. }
            | Instruction::Builtin3 { .. }
            | Instruction::Builtin4 { .. } => None,
        },
        Instruction::Mul {
            dest: t_d,
            a: prev_a,
            b: prev_b,
        } => match *next {
            Instruction::Mul {
                dest: f_d,
                a: n_a,
                b: n_b,
            } if single_use(&t_d) => {
                let (_t_res, c2_reg) = if n_a == t_d {
                    (n_a, n_b)
                } else if n_b == t_d {
                    (n_b, n_a)
                } else {
                    return None;
                };

                if !pool.is_constant(c2_reg) {
                    return None;
                }

                let (src, c1_reg) = if pool.is_constant(prev_b) {
                    (prev_a, prev_b)
                } else if pool.is_constant(prev_a) {
                    (prev_b, prev_a)
                } else {
                    return None;
                };

                let prod = pool.get(c1_reg) * pool.get(c2_reg);
                if prod == 0.0 {
                    let c_zero = pool.get_or_insert(0.0);
                    Some(FuseResult::One(Instruction::Copy {
                        dest: f_d,
                        src: c_zero,
                    }))
                } else if prod == 1.0 {
                    Some(FuseResult::One(Instruction::Copy { dest: f_d, src }))
                } else {
                    let c_prod = pool.get_or_insert(prod);
                    Some(FuseResult::One(Instruction::Mul {
                        dest: f_d,
                        a: src,
                        b: c_prod,
                    }))
                }
            }
            Instruction::End { .. }
            | Instruction::Copy { .. }
            | Instruction::Neg { .. }
            | Instruction::SinCos { .. }
            | Instruction::AsinAcos { .. }
            | Instruction::Add { .. }
            | Instruction::Add3 { .. }
            | Instruction::Add4 { .. }
            | Instruction::AddN { .. }
            | Instruction::Mul { .. }
            | Instruction::Mul3 { .. }
            | Instruction::Mul4 { .. }
            | Instruction::MulN { .. }
            | Instruction::Sub { .. }
            | Instruction::Div { .. }
            | Instruction::Pow { .. }
            | Instruction::MulAdd { .. }
            | Instruction::MulSub { .. }
            | Instruction::NegMul { .. }
            | Instruction::NegMulAdd { .. }
            | Instruction::NegMulSub { .. }
            | Instruction::Square { .. }
            | Instruction::Cube { .. }
            | Instruction::Pow4 { .. }
            | Instruction::Pow3_2 { .. }
            | Instruction::InvPow3_2 { .. }
            | Instruction::InvSqrt { .. }
            | Instruction::InvSquare { .. }
            | Instruction::InvCube { .. }
            | Instruction::Recip { .. }
            | Instruction::Powi { .. }
            | Instruction::Sin { .. }
            | Instruction::Cos { .. }
            | Instruction::Exp { .. }
            | Instruction::Ln { .. }
            | Instruction::Sqrt { .. }
            | Instruction::RecipExpm1 { .. }
            | Instruction::ExpSqr { .. }
            | Instruction::ExpSqrNeg { .. }
            | Instruction::Builtin1 { .. }
            | Instruction::Builtin2 { .. }
            | Instruction::Builtin3 { .. }
            | Instruction::Builtin4 { .. } => None,
        },
        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::Neg { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Square { .. }
        | Instruction::Cube { .. }
        | Instruction::Pow4 { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::InvSquare { .. }
        | Instruction::InvCube { .. }
        | Instruction::Recip { .. }
        | Instruction::Powi { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Exp { .. }
        | Instruction::Ln { .. }
        | Instruction::Sqrt { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "Exhaustive matching of ISA is required for safety"
)]
fn try_fuse_transcendental_idempotent(
    prev: &Instruction,
    next: &Instruction,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match *prev {
        Instruction::Builtin1 {
            dest: tmp_dest,
            op: FnOp::ExpNeg,
            arg: src_reg,
        } => match *next {
            Instruction::Recip {
                dest: dest_reg,
                src: tmp_src,
            } if tmp_src == tmp_dest && single_use(&tmp_dest) => {
                Some(FuseResult::One(Instruction::Builtin1 {
                    dest: dest_reg,
                    op: FnOp::Exp,
                    arg: src_reg,
                }))
            }
            Instruction::End { .. }
            | Instruction::Copy { .. }
            | Instruction::Neg { .. }
            | Instruction::SinCos { .. }
            | Instruction::AsinAcos { .. }
            | Instruction::Add { .. }
            | Instruction::Add3 { .. }
            | Instruction::Add4 { .. }
            | Instruction::AddN { .. }
            | Instruction::Mul { .. }
            | Instruction::Mul3 { .. }
            | Instruction::Mul4 { .. }
            | Instruction::MulN { .. }
            | Instruction::Sub { .. }
            | Instruction::Div { .. }
            | Instruction::Pow { .. }
            | Instruction::MulAdd { .. }
            | Instruction::MulSub { .. }
            | Instruction::NegMul { .. }
            | Instruction::NegMulAdd { .. }
            | Instruction::NegMulSub { .. }
            | Instruction::Square { .. }
            | Instruction::Cube { .. }
            | Instruction::Pow4 { .. }
            | Instruction::Pow3_2 { .. }
            | Instruction::InvPow3_2 { .. }
            | Instruction::InvSqrt { .. }
            | Instruction::InvSquare { .. }
            | Instruction::InvCube { .. }
            | Instruction::Recip { .. }
            | Instruction::Powi { .. }
            | Instruction::Sin { .. }
            | Instruction::Cos { .. }
            | Instruction::Exp { .. }
            | Instruction::Ln { .. }
            | Instruction::Sqrt { .. }
            | Instruction::RecipExpm1 { .. }
            | Instruction::ExpSqr { .. }
            | Instruction::ExpSqrNeg { .. }
            | Instruction::Builtin1 { .. }
            | Instruction::Builtin2 { .. }
            | Instruction::Builtin3 { .. }
            | Instruction::Builtin4 { .. } => None,
        },
        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::Neg { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Square { .. }
        | Instruction::Cube { .. }
        | Instruction::Pow4 { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::InvSquare { .. }
        | Instruction::InvCube { .. }
        | Instruction::Recip { .. }
        | Instruction::Powi { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Exp { .. }
        | Instruction::Ln { .. }
        | Instruction::Sqrt { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}
