use super::helper::ConstantPool;
use super::{FnOp, Instruction};

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
    clippy::float_cmp,
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

    (out, changed || sin_cos_changed)
}

fn fuse_sin_cos(instructions: &mut [Instruction], arg_pool: &[u32]) -> bool {
    use rustc_hash::FxHashMap;
    let mut changed = false;

    let mut sin_map: FxHashMap<u32, (usize, u32)> = FxHashMap::default();
    let mut cos_map: FxHashMap<u32, (usize, u32)> = FxHashMap::default();
    let mut pairs: Vec<(usize, usize, u32, u32, u32)> = Vec::new();

    for (idx, instr) in instructions.iter().enumerate() {
        match *instr {
            Instruction::Builtin1 {
                dest,
                op: FnOp::Sin,
                arg,
            } => {
                if let Some(&(cos_idx, cos_dest)) = cos_map.get(&arg) {
                    pairs.push((idx, cos_idx, arg, dest, cos_dest));
                    cos_map.remove(&arg);
                } else {
                    sin_map.insert(arg, (idx, dest));
                }
            }
            Instruction::Builtin1 {
                dest,
                op: FnOp::Cos,
                arg,
            } => {
                if let Some(&(sin_idx, sin_dest)) = sin_map.get(&arg) {
                    pairs.push((sin_idx, idx, arg, sin_dest, dest));
                    sin_map.remove(&arg);
                } else {
                    cos_map.insert(arg, (idx, dest));
                }
            }
            _ => {
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
        let earlier_dest = instructions[earlier_idx].dest_reg();

        let mut conflict = false;
        for instr in &instructions[earlier_idx + 1..later_idx] {
            instr.for_each_write(|d| {
                if d == earlier_dest {
                    conflict = true;
                }
            });
            instr.for_each_read(|r| {
                if r == earlier_dest {
                    conflict = true;
                }
            });
            instr.for_each_pooled_reg(arg_pool, |r| {
                if r == earlier_dest {
                    conflict = true;
                }
            });
            if conflict {
                break;
            }
        }

        if conflict {
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

#[allow(
    clippy::float_cmp,
    reason = "Exact comparison is intended for algebraic identity matching of constants"
)]
fn try_fuse_negation(
    prev: &Instruction,
    next: &Instruction,
    _pool: &ConstantPool<'_>,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match (prev, next) {
        (
            Instruction::Neg {
                dest: neg_dest,
                src: neg_src,
            },
            Instruction::Mul {
                dest: mul_dest,
                a: mul_a,
                b: mul_b,
            },
        ) if single_use(neg_dest) => {
            if mul_a == neg_dest {
                return Some(FuseResult::One(Instruction::NegMul {
                    dest: *mul_dest,
                    a: *neg_src,
                    b: *mul_b,
                }));
            } else if mul_b == neg_dest {
                return Some(FuseResult::One(Instruction::NegMul {
                    dest: *mul_dest,
                    a: *neg_src,
                    b: *mul_a,
                }));
            }
        }
        (
            Instruction::Neg {
                dest: neg_dest,
                src: neg_src,
            },
            Instruction::Builtin1 {
                dest: exp_dest,
                op: FnOp::Exp,
                arg: exp_arg,
            },
        ) if exp_arg == neg_dest && single_use(neg_dest) => {
            return Some(FuseResult::One(Instruction::Builtin1 {
                dest: *exp_dest,
                op: FnOp::ExpNeg,
                arg: *neg_src,
            }));
        }
        (
            Instruction::Neg {
                dest: tmp_dest,
                src: src_reg,
            },
            Instruction::Square {
                dest: dest_reg,
                src: tmp_src,
            },
        ) if tmp_src == tmp_dest && single_use(tmp_dest) => {
            return Some(FuseResult::One(Instruction::Square {
                dest: *dest_reg,
                src: *src_reg,
            }));
        }
        (
            Instruction::Neg {
                dest: tmp_dest,
                src: src_reg,
            },
            Instruction::Builtin1 {
                dest: dest_reg,
                op: FnOp::Abs,
                arg: tmp_arg,
            },
        ) if tmp_arg == tmp_dest && single_use(tmp_dest) => {
            return Some(FuseResult::One(Instruction::Builtin1 {
                dest: *dest_reg,
                op: FnOp::Abs,
                arg: *src_reg,
            }));
        }
        // Mul { a: x, b: -1.0 } -> Neg(x)
        _ => {}
    }
    None
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
    let (mul_dest, mul_a, mul_b, is_neg) = match prev {
        Instruction::Mul { dest, a, b } => (*dest, *a, *b, false),
        Instruction::NegMul { dest, a, b } => (*dest, *a, *b, true),
        _ => return None,
    };

    if !single_use(&mul_dest) {
        return None;
    }

    match next {
        Instruction::Add {
            dest: add_dest,
            a: add_a,
            b: add_b,
        } => {
            if *add_a == mul_dest {
                return Some(FuseResult::One(if is_neg {
                    Instruction::NegMulAdd {
                        dest: *add_dest,
                        a: mul_a,
                        b: mul_b,
                        c: *add_b,
                    }
                } else {
                    Instruction::MulAdd {
                        dest: *add_dest,
                        a: mul_a,
                        b: mul_b,
                        c: *add_b,
                    }
                }));
            } else if *add_b == mul_dest {
                return Some(FuseResult::One(if is_neg {
                    Instruction::NegMulAdd {
                        dest: *add_dest,
                        a: mul_a,
                        b: mul_b,
                        c: *add_a,
                    }
                } else {
                    Instruction::MulAdd {
                        dest: *add_dest,
                        a: mul_a,
                        b: mul_b,
                        c: *add_a,
                    }
                }));
            }
        }
        Instruction::Sub {
            dest: sub_dest,
            a: sub_a,
            b: sub_b,
        } if *sub_a == mul_dest => {
            return Some(FuseResult::One(if is_neg {
                Instruction::NegMulSub {
                    dest: *sub_dest,
                    a: mul_a,
                    b: mul_b,
                    c: *sub_b,
                }
            } else {
                Instruction::MulSub {
                    dest: *sub_dest,
                    a: mul_a,
                    b: mul_b,
                    c: *sub_b,
                }
            }));
        }
        Instruction::Sub {
            dest: sub_dest,
            a: sub_a,
            b: sub_b,
        } if *sub_b == mul_dest => {
            return Some(FuseResult::One(if is_neg {
                Instruction::MulAdd {
                    dest: *sub_dest,
                    a: mul_a,
                    b: mul_b,
                    c: *sub_a,
                }
            } else {
                Instruction::NegMulAdd {
                    dest: *sub_dest,
                    a: mul_a,
                    b: mul_b,
                    c: *sub_a,
                }
            }));
        }
        Instruction::Add3 {
            dest: add_dest,
            a,
            b,
            c,
        } => {
            let (other1, other2) = if *a == mul_dest {
                (*b, *c)
            } else if *b == mul_dest {
                (*a, *c)
            } else if *c == mul_dest {
                (*a, *b)
            } else {
                return None;
            };
            return Some(FuseResult::Two(
                if is_neg {
                    Instruction::NegMulAdd {
                        dest: mul_dest,
                        a: mul_a,
                        b: mul_b,
                        c: other1,
                    }
                } else {
                    Instruction::MulAdd {
                        dest: mul_dest,
                        a: mul_a,
                        b: mul_b,
                        c: other1,
                    }
                },
                Instruction::Add {
                    dest: *add_dest,
                    a: mul_dest,
                    b: other2,
                },
            ));
        }
        Instruction::Add4 {
            dest: add_dest,
            a,
            b,
            c,
            d,
        } => {
            let (other1, other2, other3) = if *a == mul_dest {
                (*b, *c, *d)
            } else if *b == mul_dest {
                (*a, *c, *d)
            } else if *c == mul_dest {
                (*a, *b, *d)
            } else if *d == mul_dest {
                (*a, *b, *c)
            } else {
                return None;
            };
            return Some(FuseResult::Two(
                if is_neg {
                    Instruction::NegMulAdd {
                        dest: mul_dest,
                        a: mul_a,
                        b: mul_b,
                        c: other1,
                    }
                } else {
                    Instruction::MulAdd {
                        dest: mul_dest,
                        a: mul_a,
                        b: mul_b,
                        c: other1,
                    }
                },
                Instruction::Add3 {
                    dest: *add_dest,
                    a: mul_dest,
                    b: other2,
                    c: other3,
                },
            ));
        }
        _ => {}
    }
    None
}

#[allow(
    clippy::too_many_lines,
    clippy::float_cmp,
    reason = "Function handles extensive pattern matching for peephole optimization fusions"
)]
fn try_fuse_power(
    prev: &Instruction,
    next: &Instruction,
    pool: &mut ConstantPool<'_>,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match (prev, next) {
        (
            Instruction::Square {
                dest: sq1_dest,
                src: sq1_src,
            },
            Instruction::Square {
                dest: sq2_dest,
                src: sq2_src,
            },
        ) if sq2_src == sq1_dest && single_use(sq1_dest) => {
            return Some(FuseResult::One(Instruction::Pow4 {
                dest: *sq2_dest,
                src: *sq1_src,
            }));
        }
        (
            Instruction::Square {
                dest: sq_dest,
                src: sq_src,
            },
            Instruction::Mul {
                dest: mul_dest,
                a: mul_a,
                b: mul_b,
            },
        ) if single_use(sq_dest)
            && ((*mul_a == *sq_dest && *mul_b == *sq_src)
                || (*mul_a == *sq_src && *mul_b == *sq_dest)) =>
        {
            return Some(FuseResult::One(Instruction::Cube {
                dest: *mul_dest,
                src: *sq_src,
            }));
        }
        (
            Instruction::Cube {
                dest: cube_dest,
                src: cube_src,
            },
            Instruction::Mul {
                dest: mul_dest,
                a: mul_a,
                b: mul_b,
            },
        ) if single_use(cube_dest)
            && ((*mul_a == *cube_dest && *mul_b == *cube_src)
                || (*mul_a == *cube_src && *mul_b == *cube_dest)) =>
        {
            return Some(FuseResult::One(Instruction::Pow4 {
                dest: *mul_dest,
                src: *cube_src,
            }));
        }
        (
            Instruction::Builtin1 {
                dest: exp_dest,
                op: FnOp::Exp,
                arg: exp_arg,
            },
            Instruction::Square {
                dest: sq_dest,
                src: sq_src,
            },
        ) if *sq_src == *exp_dest && single_use(exp_dest) => {
            // Exp(x)^2 = Exp(2x)
            let c_2 = pool.get_or_insert(2.0);
            return Some(FuseResult::Two(
                Instruction::Mul {
                    dest: *exp_dest,
                    a: *exp_arg,
                    b: c_2,
                },
                Instruction::Builtin1 {
                    dest: *sq_dest,
                    op: FnOp::Exp,
                    arg: *exp_dest,
                },
            ));
        }
        // Square { t, s }, Builtin1 { d, Exp, t } -> ExpSqr { d, s }
        (
            Instruction::Square {
                dest: tmp_dest,
                src: src_reg,
            },
            Instruction::Builtin1 {
                dest: dest_reg,
                op: FnOp::Exp,
                arg: tmp_arg,
            },
        ) if *tmp_arg == *tmp_dest && single_use(tmp_dest) => {
            return Some(FuseResult::One(Instruction::ExpSqr {
                dest: *dest_reg,
                src: *src_reg,
            }));
        }
        (
            Instruction::Pow4 {
                dest: p4_dest,
                src: p4_src,
            },
            Instruction::Recip {
                dest: recip_dest,
                src: recip_src,
            },
        ) if recip_src == p4_dest && single_use(p4_dest) => {
            return Some(FuseResult::Two(
                Instruction::Square {
                    dest: *p4_dest,
                    src: *p4_src,
                },
                Instruction::InvSquare {
                    dest: *recip_dest,
                    src: *p4_dest,
                },
            ));
        }
        _ => {}
    }
    None
}

fn try_fuse_inverse(
    prev: &Instruction,
    next: &Instruction,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match (prev, next) {
        (
            Instruction::Builtin1 {
                dest: sqrt_dest,
                op: FnOp::Sqrt,
                arg: sqrt_src,
            },
            Instruction::Recip {
                dest: recip_dest,
                src: recip_src,
            },
        ) if recip_src == sqrt_dest && single_use(sqrt_dest) => {
            return Some(FuseResult::One(Instruction::InvSqrt {
                dest: *recip_dest,
                src: *sqrt_src,
            }));
        }
        (
            Instruction::Square {
                dest: sq_dest,
                src: sq_src,
            },
            Instruction::Recip {
                dest: recip_dest,
                src: recip_src,
            },
        ) if recip_src == sq_dest && single_use(sq_dest) => {
            return Some(FuseResult::One(Instruction::InvSquare {
                dest: *recip_dest,
                src: *sq_src,
            }));
        }
        (
            Instruction::Cube {
                dest: cube_dest,
                src: cube_src,
            },
            Instruction::Recip {
                dest: recip_dest,
                src: recip_src,
            },
        ) if recip_src == cube_dest && single_use(cube_dest) => {
            return Some(FuseResult::One(Instruction::InvCube {
                dest: *recip_dest,
                src: *cube_src,
            }));
        }
        (
            Instruction::Pow3_2 {
                dest: p32_dest,
                src: p32_src,
            },
            Instruction::Recip {
                dest: recip_dest,
                src: recip_src,
            },
        ) if recip_src == p32_dest && single_use(p32_dest) => {
            return Some(FuseResult::One(Instruction::InvPow3_2 {
                dest: *recip_dest,
                src: *p32_src,
            }));
        }
        (
            Instruction::Builtin1 {
                dest: exp_dest,
                op: FnOp::Exp,
                arg: exp_arg,
            },
            Instruction::Recip {
                dest: recip_dest,
                src: recip_src,
            },
        ) if recip_src == exp_dest && single_use(exp_dest) => {
            return Some(FuseResult::One(Instruction::Builtin1 {
                dest: *recip_dest,
                op: FnOp::ExpNeg,
                arg: *exp_arg,
            }));
        }
        (
            Instruction::Builtin1 {
                dest: expm1_dest,
                op: FnOp::Expm1,
                arg: expm1_arg,
            },
            Instruction::Recip {
                dest: recip_dest,
                src: recip_src,
            },
        ) if recip_src == expm1_dest && single_use(expm1_dest) => {
            return Some(FuseResult::One(Instruction::RecipExpm1 {
                dest: *recip_dest,
                src: *expm1_arg,
            }));
        }
        _ => {}
    }
    None
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
    match (prev, next) {
        // Ln(Exp(x)) -> x
        (
            Instruction::Builtin1 {
                dest: tmp_dest,
                op: FnOp::Exp,
                arg: src,
            },
            Instruction::Builtin1 {
                dest: final_dest,
                op: FnOp::Ln,
                arg: tmp_arg,
            },
        ) if *tmp_arg == *tmp_dest && single_use(tmp_dest) => {
            Some(FuseResult::One(Instruction::Copy {
                dest: *final_dest,
                src: *src,
            }))
        }
        // Ln(Pow(x, n)) -> n * Ln(x)
        (
            Instruction::Pow {
                dest: tmp_dest,
                base: src_x,
                exp: src_n,
            },
            Instruction::Builtin1 {
                dest: final_dest,
                op: FnOp::Ln,
                arg: tmp_arg,
            },
        ) if *tmp_arg == *tmp_dest && single_use(tmp_dest) && pool.is_constant(*src_n) => {
            let n = pool.get(*src_n);
            let ln_x = Instruction::Builtin1 {
                dest: *final_dest,
                op: FnOp::Ln,
                arg: *src_x,
            };
            let c_n = pool.get_or_insert(n);
            Some(FuseResult::Two(
                ln_x,
                Instruction::Mul {
                    dest: *final_dest,
                    a: *final_dest,
                    b: c_n,
                },
            ))
        }
        (
            Instruction::Builtin1 {
                dest: tmp_dest,
                op: FnOp::Sqrt,
                arg: src_reg,
            },
            Instruction::Builtin1 {
                dest: dest_reg,
                op: FnOp::Ln,
                arg: tmp_arg,
            },
        ) if *tmp_arg == *tmp_dest && single_use(tmp_dest) => {
            // Ln(Sqrt(x)) = 0.5 * Ln(x)
            let ln_x = Instruction::Builtin1 {
                dest: *dest_reg,
                op: FnOp::Ln,
                arg: *src_reg,
            };
            let c_0_5 = pool.get_or_insert(0.5);
            Some(FuseResult::Two(
                ln_x,
                Instruction::Mul {
                    dest: *dest_reg,
                    a: *dest_reg,
                    b: c_0_5,
                },
            ))
        }
        // Ln(Recip(x)) -> -Ln(x)
        (
            Instruction::Recip {
                dest: tmp_dest,
                src: src_reg,
            },
            Instruction::Builtin1 {
                dest: dest_reg,
                op: FnOp::Ln,
                arg: tmp_arg,
            },
        ) if *tmp_arg == *tmp_dest && single_use(tmp_dest) => Some(FuseResult::Two(
            Instruction::Builtin1 {
                dest: *tmp_dest,
                op: FnOp::Ln,
                arg: *src_reg,
            },
            Instruction::Neg {
                dest: *dest_reg,
                src: *tmp_dest,
            },
        )),
        // Exp(x) - 1 -> Expm1(x)
        (
            Instruction::Builtin1 {
                dest: exp_dest,
                op: FnOp::Exp,
                arg: exp_arg,
            },
            Instruction::Sub {
                dest: final_dest,
                a: tmp_arg,
                b: c_reg,
            },
        ) if *tmp_arg == *exp_dest
            && single_use(exp_dest)
            && pool.is_constant(*c_reg)
            && pool.get(*c_reg) == 1.0 =>
        {
            Some(FuseResult::One(Instruction::Builtin1 {
                dest: *final_dest,
                op: FnOp::Expm1,
                arg: *exp_arg,
            }))
        }
        // 1 + x -> Add(x, 1) handled by canonicalization, then Add(x, 1) + Ln -> Log1p
        (
            Instruction::Add {
                dest: tmp_dest,
                a: src_reg,
                b: c_reg,
            },
            Instruction::Builtin1 {
                dest: final_dest,
                op: FnOp::Ln,
                arg: tmp_arg,
            },
        ) if *tmp_arg == *tmp_dest
            && single_use(tmp_dest)
            && pool.is_constant(*c_reg)
            && pool.get(*c_reg) == 1.0 =>
        {
            Some(FuseResult::One(Instruction::Builtin1 {
                dest: *final_dest,
                op: FnOp::Log1p,
                arg: *src_reg,
            }))
        }
        _ => None,
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
    match (prev, next) {
        (
            Instruction::Builtin1 {
                dest: tmp_dest,
                op: FnOp::Abs,
                arg: src_reg,
            },
            Instruction::Builtin1 {
                dest: dest_reg,
                op: FnOp::Abs,
                arg: tmp_arg,
            },
        ) if tmp_arg == tmp_dest && single_use(tmp_dest) => {
            Some(FuseResult::One(Instruction::Builtin1 {
                dest: *dest_reg,
                op: FnOp::Abs,
                arg: *src_reg,
            }))
        }
        (
            Instruction::Neg {
                dest: tmp_dest,
                src: src_reg,
            },
            Instruction::Neg {
                dest: dest_reg,
                src: tmp_src,
            },
        ) if tmp_src == tmp_dest && single_use(tmp_dest) => {
            Some(FuseResult::One(Instruction::Copy {
                dest: *dest_reg,
                src: *src_reg,
            }))
        }
        _ => None,
    }
}

fn try_fuse_recip_idempotent(
    prev: &Instruction,
    next: &Instruction,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match (prev, next) {
        (
            Instruction::Recip {
                dest: tmp_dest,
                src: src_reg,
            },
            Instruction::Recip {
                dest: dest_reg,
                src: tmp_src,
            },
        ) if tmp_src == tmp_dest && single_use(tmp_dest) => {
            Some(FuseResult::One(Instruction::Copy {
                dest: *dest_reg,
                src: *src_reg,
            }))
        }
        (
            Instruction::InvSqrt {
                dest: tmp_dest,
                src: src_reg,
            },
            Instruction::Recip {
                dest: dest_reg,
                src: tmp_src,
            },
        ) if tmp_src == tmp_dest && single_use(tmp_dest) => {
            Some(FuseResult::One(Instruction::Builtin1 {
                dest: *dest_reg,
                op: FnOp::Sqrt,
                arg: *src_reg,
            }))
        }
        _ => None,
    }
}

fn try_fuse_power_idempotent(
    prev: &Instruction,
    next: &Instruction,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match (prev, next) {
        (
            Instruction::Square {
                dest: tmp_dest,
                src: src_reg,
            },
            Instruction::Builtin1 {
                dest: dest_reg,
                op: FnOp::Sqrt,
                arg: tmp_arg,
            },
        ) if tmp_arg == tmp_dest && single_use(tmp_dest) => {
            Some(FuseResult::One(Instruction::Builtin1 {
                dest: *dest_reg,
                op: FnOp::Abs,
                arg: *src_reg,
            }))
        }
        (
            Instruction::Builtin1 {
                dest: tmp_dest,
                op: FnOp::Abs,
                arg: src_reg,
            },
            Instruction::Square {
                dest: dest_reg,
                src: tmp_src,
            },
        ) if tmp_src == tmp_dest && single_use(tmp_dest) => {
            Some(FuseResult::One(Instruction::Square {
                dest: *dest_reg,
                src: *src_reg,
            }))
        }
        _ => None,
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
    match (prev, next) {
        // (x + c1) + c2 -> x + (c1 + c2)
        (
            Instruction::Add {
                dest: t_d,
                a: prev_a,
                b: prev_b,
            },
            Instruction::Add {
                dest: f_d,
                a: n_a,
                b: n_b,
            },
        ) if single_use(t_d) => {
            // Check if one of the operands in the second Add is the result of the first Add
            let c2_reg = if *n_a == *t_d {
                *n_b
            } else if *n_b == *t_d {
                *n_a
            } else {
                return None;
            };

            // Ensure the other operand in the second Add is a constant
            if !pool.is_constant(c2_reg) {
                return None;
            }

            // In the first Add, find which one is the source and which one is the constant
            let (src, c1_reg) = if pool.is_constant(*prev_b) {
                (*prev_a, *prev_b)
            } else if pool.is_constant(*prev_a) {
                (*prev_b, *prev_a)
            } else {
                return None;
            };

            let sum = pool.get(c1_reg) + pool.get(c2_reg);
            if sum == 0.0 {
                Some(FuseResult::One(Instruction::Copy { dest: *f_d, src }))
            } else {
                let c_sum = pool.get_or_insert(sum);
                Some(FuseResult::One(Instruction::Add {
                    dest: *f_d,
                    a: src,
                    b: c_sum,
                }))
            }
        }
        // (x * c1) * c2 -> x * (c1 * c2)
        (
            Instruction::Mul {
                dest: t_d,
                a: prev_a,
                b: prev_b,
            },
            Instruction::Mul {
                dest: f_d,
                a: n_a,
                b: n_b,
            },
        ) if single_use(t_d) => {
            let (_t_res, c2_reg) = if *n_a == *t_d {
                (*n_a, *n_b)
            } else if *n_b == *t_d {
                (*n_b, *n_a)
            } else {
                return None;
            };

            if !pool.is_constant(c2_reg) {
                return None;
            }

            let (src, c1_reg) = if pool.is_constant(*prev_b) {
                (*prev_a, *prev_b)
            } else if pool.is_constant(*prev_a) {
                (*prev_b, *prev_a)
            } else {
                return None;
            };

            let prod = pool.get(c1_reg) * pool.get(c2_reg);
            if prod == 0.0 {
                let c_zero = pool.get_or_insert(0.0);
                Some(FuseResult::One(Instruction::Copy {
                    dest: *f_d,
                    src: c_zero,
                }))
            } else if prod == 1.0 {
                Some(FuseResult::One(Instruction::Copy { dest: *f_d, src }))
            } else {
                let c_prod = pool.get_or_insert(prod);
                Some(FuseResult::One(Instruction::Mul {
                    dest: *f_d,
                    a: src,
                    b: c_prod,
                }))
            }
        }
        _ => None,
    }
}

fn try_fuse_transcendental_idempotent(
    prev: &Instruction,
    next: &Instruction,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<FuseResult> {
    match (prev, next) {
        (
            Instruction::Builtin1 {
                dest: tmp_dest,
                op: FnOp::ExpNeg,
                arg: src_reg,
            },
            Instruction::Recip {
                dest: dest_reg,
                src: tmp_src,
            },
        ) if *tmp_src == *tmp_dest && single_use(tmp_dest) => {
            Some(FuseResult::One(Instruction::Builtin1 {
                dest: *dest_reg,
                op: FnOp::Exp,
                arg: *src_reg,
            }))
        }
        _ => None,
    }
}
