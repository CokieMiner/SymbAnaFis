use super::helper::ConstantPool;
use super::instruction::{FnOp, Instruction};

/// Peephole optimizer that fuses consecutive instruction pairs into more efficient single
/// instructions.
///
/// The patterns are organised into category-specific sub-functions:
///
/// - [`try_fuse_negation`]: Neg/NegMul patterns
/// - [`try_fuse_load_const`]: `LoadConst` + arithmetic → *Const variant
/// - [`try_fuse_mul_add`]: Mul + Add/Sub → FMA patterns
/// - [`try_fuse_power`]: Square/Cube/Pow4 combinations
/// - [`try_fuse_inverse`]: X + `Recip` → `InvX`
/// - [`try_fuse_logarithmic`]: Log/Exp algebraic identities
/// - [`try_fuse_idempotent`]: Self-cancelling pairs (Neg+Neg, Abs+Abs, etc.)
/// - [`try_fuse_const_chain`]: Consecutive `AddConst`/`MulConst` folding
#[allow(
    clippy::too_many_lines,
    clippy::float_cmp,
    reason = "Peephole optimizer with multiple patterns; exact float comparison is intentional for safe algebraic folding"
)]
pub(super) fn fuse_instructions(
    instructions: &[Instruction],
    pool: &mut ConstantPool<'_>,
    use_count: &[usize],
) -> Vec<Instruction> {
    let single_use = |reg_idx: &u32| use_count[*reg_idx as usize] == 1;
    let instr_count = instructions.len();
    let mut out = Vec::with_capacity(instr_count);
    let mut instr_idx = 0;

    while instr_idx < instr_count {
        if instr_idx + 1 < instr_count {
            let prev = &instructions[instr_idx];
            let next = &instructions[instr_idx + 1];

            if let Some(fused) = try_fuse_negation(prev, next, pool, &single_use) {
                out.extend(fused);
                instr_idx += 2;
                continue;
            }
            if let Some(fused) = try_fuse_load_const(prev, next, &single_use) {
                out.extend(fused);
                instr_idx += 2;
                continue;
            }
            if let Some(fused) = try_fuse_mul_add(prev, next, &single_use) {
                out.extend(fused);
                instr_idx += 2;
                continue;
            }
            if let Some(fused) = try_fuse_power(prev, next, pool, &single_use) {
                out.extend(fused);
                instr_idx += 2;
                continue;
            }
            if let Some(fused) = try_fuse_inverse(prev, next, &single_use) {
                out.extend(fused);
                instr_idx += 2;
                continue;
            }
            if let Some(fused) = try_fuse_logarithmic(prev, next, pool, &single_use) {
                out.extend(fused);
                instr_idx += 2;
                continue;
            }
            if let Some(fused) = try_fuse_idempotent(prev, next, &single_use) {
                out.extend(fused);
                instr_idx += 2;
                continue;
            }
            if let Some(fused) = try_fuse_const_chain(prev, next, pool, &single_use) {
                out.extend(fused);
                instr_idx += 2;
                continue;
            }
        }

        // Single-instruction transform: `MulConst` with negative value → `NegMulConst` with positive
        if let Instruction::MulConst {
            dest,
            src,
            const_idx,
        } = instructions[instr_idx]
        {
            let value = pool.get(const_idx);
            if value.is_sign_negative() && value != 0.0 {
                let pos_const = pool.get_or_insert(-value);
                out.push(Instruction::NegMulConst {
                    dest,
                    src,
                    const_idx: pos_const,
                });
                instr_idx += 1;
                continue;
            }
        }

        out.push(instructions[instr_idx]);
        instr_idx += 1;
    }

    out
}

// ─── Negation fusions ────────────────────────────────────────────────────────

/// Fuses negation patterns:
/// - `Neg` + `Mul` → `NegMul`
/// - `Neg` + `MulConst` → `NegMulConst`
/// - `MulConst(-1)` + `Mul` → `NegMul`
/// - `Neg` + `Exp` → `ExpNeg`
/// - `Neg` + `Square` → `Square` (square absorbs sign)
/// - `Neg` + `Abs` → `Abs` (abs absorbs sign)
/// - `Neg` + `AddConst` → `ConstSub`
#[allow(
    clippy::too_many_lines,
    clippy::float_cmp,
    reason = "Large match for negation patterns; exact float comparison is intentional for -1.0"
)]
fn try_fuse_negation(
    prev: &Instruction,
    next: &Instruction,
    pool: &ConstantPool<'_>,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<Vec<Instruction>> {
    match (prev, next) {
        // Neg{t, s}, Mul{d, t, b} → NegMul{d, s, b}
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
                return Some(vec![Instruction::NegMul {
                    dest: *mul_dest,
                    a: *neg_src,
                    b: *mul_b,
                }]);
            } else if mul_b == neg_dest {
                return Some(vec![Instruction::NegMul {
                    dest: *mul_dest,
                    a: *neg_src,
                    b: *mul_a,
                }]);
            }
        }
        // Neg{t, s}, MulConst{d, t, c} → NegMulConst{d, s, c}
        (
            Instruction::Neg {
                dest: neg_dest,
                src: neg_src,
            },
            Instruction::MulConst {
                dest: mul_dest,
                src: mul_src,
                const_idx: mul_c,
            },
        ) if mul_src == neg_dest && single_use(neg_dest) => {
            return Some(vec![Instruction::NegMulConst {
                dest: *mul_dest,
                src: *neg_src,
                const_idx: *mul_c,
            }]);
        }
        // MulConst{t, s, -1.0}, Mul{d, t, b} → NegMul{d, s, b}
        (
            Instruction::MulConst {
                dest: mc_dest,
                src: mc_src,
                const_idx: mc_c,
            },
            Instruction::Mul {
                dest: mul_dest,
                a: mul_a,
                b: mul_b,
            },
        ) if *mc_dest != *mc_src && single_use(mc_dest) && pool.get(*mc_c) == -1.0 => {
            if mul_a == mc_dest {
                return Some(vec![Instruction::NegMul {
                    dest: *mul_dest,
                    a: *mc_src,
                    b: *mul_b,
                }]);
            } else if mul_b == mc_dest {
                return Some(vec![Instruction::NegMul {
                    dest: *mul_dest,
                    a: *mc_src,
                    b: *mul_a,
                }]);
            }
        }
        // Neg{t, s}, Exp{d, t} → ExpNeg{d, s}
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
            return Some(vec![Instruction::Builtin1 {
                dest: *exp_dest,
                op: FnOp::ExpNeg,
                arg: *neg_src,
            }]);
        }
        // Neg{t, s}, Square{d, t} → Square{d, s}
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
            return Some(vec![Instruction::Square {
                dest: *dest_reg,
                src: *src_reg,
            }]);
        }
        // Neg{t, s}, Abs{d, t} → Abs{d, s}
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
            return Some(vec![Instruction::Builtin1 {
                dest: *dest_reg,
                op: FnOp::Abs,
                arg: *src_reg,
            }]);
        }
        // Neg{t, s}, AddConst{d, t, c} → ConstSub{d, s, c}
        (
            Instruction::Neg {
                dest: tmp_dest,
                src: src_reg,
            },
            Instruction::AddConst {
                dest: dest_reg,
                src: tmp_src,
                const_idx: const_idx_val,
            },
        ) if *tmp_src == *tmp_dest && single_use(tmp_dest) => {
            return Some(vec![Instruction::ConstSub {
                dest: *dest_reg,
                src: *src_reg,
                const_idx: *const_idx_val,
            }]);
        }
        _ => {}
    }
    None
}

// ─── LoadConst fusions ───────────────────────────────────────────────────────

/// Fuses `LoadConst` patterns into *Const instruction variants:
/// - `LoadConst` + `Add` → `AddConst`
/// - `LoadConst` + `Mul` → `MulConst`
/// - `LoadConst` + `Sub` → `SubConst` or `ConstSub`
/// - `LoadConst` + `Div` → `DivConst` or `ConstDiv`
/// - `LoadConst` + `MulAdd` → `MulAddConst`
/// - `LoadConst` + `MulSub` → `MulSubConst`
/// - `LoadConst` + `NegMulAdd` → `NegMulAddConst`
#[allow(
    clippy::too_many_lines,
    reason = "Large match for LoadConst fusion patterns"
)]
fn try_fuse_load_const(
    prev: &Instruction,
    next: &Instruction,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<Vec<Instruction>> {
    let Instruction::LoadConst {
        dest: ld_dest,
        const_idx: ld_c,
    } = prev
    else {
        return None;
    };
    if !single_use(ld_dest) {
        return None;
    }

    match next {
        Instruction::Add {
            dest: add_dest,
            a: add_a,
            b: add_b,
        } => {
            if add_a == ld_dest {
                return Some(vec![Instruction::AddConst {
                    dest: *add_dest,
                    src: *add_b,
                    const_idx: *ld_c,
                }]);
            } else if add_b == ld_dest {
                return Some(vec![Instruction::AddConst {
                    dest: *add_dest,
                    src: *add_a,
                    const_idx: *ld_c,
                }]);
            }
        }
        Instruction::Mul {
            dest: mul_dest,
            a: mul_a,
            b: mul_b,
        } => {
            if mul_a == ld_dest {
                return Some(vec![Instruction::MulConst {
                    dest: *mul_dest,
                    src: *mul_b,
                    const_idx: *ld_c,
                }]);
            } else if mul_b == ld_dest {
                return Some(vec![Instruction::MulConst {
                    dest: *mul_dest,
                    src: *mul_a,
                    const_idx: *ld_c,
                }]);
            }
        }
        Instruction::Sub {
            dest: sub_dest,
            a: sub_a,
            b: sub_b,
        } => {
            if *sub_b == *ld_dest {
                return Some(vec![Instruction::SubConst {
                    dest: *sub_dest,
                    src: *sub_a,
                    const_idx: *ld_c,
                }]);
            } else if *sub_a == *ld_dest {
                return Some(vec![Instruction::ConstSub {
                    dest: *sub_dest,
                    src: *sub_b,
                    const_idx: *ld_c,
                }]);
            }
        }
        Instruction::Div {
            dest: div_dest,
            num: div_num,
            den: div_den,
        } => {
            if *div_den == *ld_dest {
                return Some(vec![Instruction::DivConst {
                    dest: *div_dest,
                    src: *div_num,
                    const_idx: *ld_c,
                }]);
            } else if *div_num == *ld_dest {
                return Some(vec![Instruction::ConstDiv {
                    dest: *div_dest,
                    src: *div_den,
                    const_idx: *ld_c,
                }]);
            }
        }
        Instruction::MulAdd {
            dest: add_dest,
            a: add_a,
            b: add_b,
            c: add_c,
        } if *add_c == *ld_dest => {
            return Some(vec![Instruction::MulAddConst {
                dest: *add_dest,
                a: *add_a,
                b: *add_b,
                const_idx: *ld_c,
            }]);
        }
        Instruction::MulSub {
            dest: sub_dest,
            a: sub_a,
            b: sub_b,
            c: sub_c,
        } if *sub_c == *ld_dest => {
            return Some(vec![Instruction::MulSubConst {
                dest: *sub_dest,
                a: *sub_a,
                b: *sub_b,
                const_idx: *ld_c,
            }]);
        }
        Instruction::NegMulAdd {
            dest: add_dest,
            a: add_a,
            b: add_b,
            c: add_c,
        } if *add_c == *ld_dest => {
            return Some(vec![Instruction::NegMulAddConst {
                dest: *add_dest,
                a: *add_a,
                b: *add_b,
                const_idx: *ld_c,
            }]);
        }
        _ => {}
    }
    None
}

// ─── Mul+Add/Sub fusions (FMA) ──────────────────────────────────────────────

/// Fuses `Mul` + add/sub into FMA patterns:
/// - `Mul` + `Add` → `MulAdd`
/// - `Mul` + `AddConst` → `MulAddConst`
/// - `Mul` + `Sub(mul_result, x)` → `MulSub`
/// - `Mul` + `SubConst` → `MulSubConst`
/// - `Mul` + `Sub(x, mul_result)` → `NegMulAdd`
/// - `Mul` + `ConstSub` → `NegMulAddConst`
fn try_fuse_mul_add(
    prev: &Instruction,
    next: &Instruction,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<Vec<Instruction>> {
    let Instruction::Mul {
        dest: mul_dest,
        a: mul_a,
        b: mul_b,
    } = prev
    else {
        return None;
    };
    if !single_use(mul_dest) {
        return None;
    }

    match next {
        // Mul{t,a,b}, Add{d, t, c} → MulAdd{d, a, b, c}
        Instruction::Add {
            dest: add_dest,
            a: add_a,
            b: add_b,
        } => {
            if add_a == mul_dest {
                return Some(vec![Instruction::MulAdd {
                    dest: *add_dest,
                    a: *mul_a,
                    b: *mul_b,
                    c: *add_b,
                }]);
            } else if add_b == mul_dest {
                return Some(vec![Instruction::MulAdd {
                    dest: *add_dest,
                    a: *mul_a,
                    b: *mul_b,
                    c: *add_a,
                }]);
            }
        }
        // Mul{t,a,b}, AddConst{d, t, c} → MulAddConst{d, a, b, c}
        Instruction::AddConst {
            dest: add_dest,
            src: add_src,
            const_idx: add_c,
        } if add_src == mul_dest => {
            return Some(vec![Instruction::MulAddConst {
                dest: *add_dest,
                a: *mul_a,
                b: *mul_b,
                const_idx: *add_c,
            }]);
        }
        // Mul{t,a,b}, Sub{d, t, c} → MulSub{d, a, b, c}
        Instruction::Sub {
            dest: sub_dest,
            a: sub_a,
            b: sub_b,
        } if sub_a == mul_dest => {
            return Some(vec![Instruction::MulSub {
                dest: *sub_dest,
                a: *mul_a,
                b: *mul_b,
                c: *sub_b,
            }]);
        }
        // Mul{t,a,b}, SubConst{d, t, c} → MulSubConst{d, a, b, c}
        Instruction::SubConst {
            dest: sub_dest,
            src: sub_src,
            const_idx: sub_c,
        } if sub_src == mul_dest => {
            return Some(vec![Instruction::MulSubConst {
                dest: *sub_dest,
                a: *mul_a,
                b: *mul_b,
                const_idx: *sub_c,
            }]);
        }
        // Mul{t,a,b}, Sub{d, c, t} → NegMulAdd{d, a, b, c}
        Instruction::Sub {
            dest: sub_dest,
            a: sub_a,
            b: sub_b,
        } if sub_b == mul_dest => {
            return Some(vec![Instruction::NegMulAdd {
                dest: *sub_dest,
                a: *mul_a,
                b: *mul_b,
                c: *sub_a,
            }]);
        }
        // Mul{t,a,b}, ConstSub{d, t, c} → NegMulAddConst{d, a, b, c}
        Instruction::ConstSub {
            dest: sub_dest,
            src: sub_src,
            const_idx: sub_c,
        } if sub_src == mul_dest => {
            return Some(vec![Instruction::NegMulAddConst {
                dest: *sub_dest,
                a: *mul_a,
                b: *mul_b,
                const_idx: *sub_c,
            }]);
        }
        _ => {}
    }
    None
}

// ─── Power fusions ───────────────────────────────────────────────────────────

/// Fuses power-related patterns:
/// - `Square` + `Square` → `Pow4`
/// - `Square` + `Mul(sq, base)` → `Cube`
/// - `Cube` + `Mul(cu, base)` → `Pow4`
/// - `Exp` + `Square` → `MulConst(2)` + `Exp`
/// - `Square` + `Ln` → `Abs` + `Ln` + `MulConst(2)`
/// - `Pow4` + `Recip` → `Square` + `InvSquare`
#[allow(
    clippy::too_many_lines,
    reason = "Large match for power fusion patterns"
)]
fn try_fuse_power(
    prev: &Instruction,
    next: &Instruction,
    pool: &mut ConstantPool<'_>,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<Vec<Instruction>> {
    match (prev, next) {
        // Square{t, s}, Square{d, t} → Pow4{d, s}
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
            return Some(vec![Instruction::Pow4 {
                dest: *sq2_dest,
                src: *sq1_src,
            }]);
        }
        // Square{t, s}, Mul{d, t, s} or Mul{d, s, t} → Cube{d, s}
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
            return Some(vec![Instruction::Cube {
                dest: *mul_dest,
                src: *sq_src,
            }]);
        }
        // Cube{t, s}, Mul{d, t, s} or Mul{d, s, t} → Pow4{d, s}
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
            return Some(vec![Instruction::Pow4 {
                dest: *mul_dest,
                src: *cube_src,
            }]);
        }
        // Exp{t, s}, Square{d, t} → MulConst{t, s, 2.0}, Exp{d, t}
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
        ) if sq_src == exp_dest && single_use(exp_dest) => {
            let c_idx = pool.get_or_insert(2.0);
            return Some(vec![
                Instruction::MulConst {
                    dest: *exp_dest,
                    src: *exp_arg,
                    const_idx: c_idx,
                },
                Instruction::Builtin1 {
                    dest: *sq_dest,
                    op: FnOp::Exp,
                    arg: *exp_dest,
                },
            ]);
        }
        // Square{t, s}, Builtin1{d, Ln, t} → Abs{t, s}, Ln{t, t}, MulConst{d, t, 2.0}
        (
            Instruction::Square {
                dest: sq_dest,
                src: sq_src,
            },
            Instruction::Builtin1 {
                dest: ln_dest,
                op: FnOp::Ln,
                arg: ln_arg,
            },
        ) if ln_arg == sq_dest && single_use(sq_dest) => {
            let c_idx = pool.get_or_insert(2.0);
            return Some(vec![
                Instruction::Builtin1 {
                    dest: *sq_dest,
                    op: FnOp::Abs,
                    arg: *sq_src,
                },
                Instruction::Builtin1 {
                    dest: *sq_dest,
                    op: FnOp::Ln,
                    arg: *sq_dest,
                },
                Instruction::MulConst {
                    dest: *ln_dest,
                    src: *sq_dest,
                    const_idx: c_idx,
                },
            ]);
        }
        // Pow4{t, s}, Recip{d, t} → Square{t, s}, InvSquare{d, t}
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
            return Some(vec![
                Instruction::Square {
                    dest: *p4_dest,
                    src: *p4_src,
                },
                Instruction::InvSquare {
                    dest: *recip_dest,
                    src: *p4_dest,
                },
            ]);
        }
        _ => {}
    }
    None
}

// ─── Inverse fusions ─────────────────────────────────────────────────────────

/// Fuses X + `Recip` → `InvX` patterns:
/// - `Sqrt` + `Recip` → `InvSqrt`
/// - `Square` + `Recip` → `InvSquare`
/// - `Cube` + `Recip` → `InvCube`
/// - `Pow3_2` + `Recip` → `InvPow3_2`
/// - `Exp` + `Recip` → `ExpNeg`
/// - `Expm1` + `Recip` → `RecipExpm1`
fn try_fuse_inverse(
    prev: &Instruction,
    next: &Instruction,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<Vec<Instruction>> {
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
            return Some(vec![Instruction::InvSqrt {
                dest: *recip_dest,
                src: *sqrt_src,
            }]);
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
            return Some(vec![Instruction::InvSquare {
                dest: *recip_dest,
                src: *sq_src,
            }]);
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
            return Some(vec![Instruction::InvCube {
                dest: *recip_dest,
                src: *cube_src,
            }]);
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
            return Some(vec![Instruction::InvPow3_2 {
                dest: *recip_dest,
                src: *p32_src,
            }]);
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
            return Some(vec![Instruction::Builtin1 {
                dest: *recip_dest,
                op: FnOp::ExpNeg,
                arg: *exp_arg,
            }]);
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
            return Some(vec![Instruction::RecipExpm1 {
                dest: *recip_dest,
                src: *expm1_arg,
            }]);
        }
        _ => {}
    }
    None
}

// ─── Logarithmic / Exponential algebraic fusions ─────────────────────────────

/// Fuses log/exp algebraic identities:
#[allow(
    clippy::float_cmp,
    reason = "Exact comparison against 1.0 is intentional for Expm1/Log1p identity matching"
)]
/// - `Exp` + `SubConst(1)` → `Expm1`
/// - `AddConst(1)` + `Ln` → `Log1p`
/// - `Recip` + `Ln` → `Ln` + `Neg`
/// - `Sqrt` + `Ln` → `Ln` + `MulConst(0.5)`
fn try_fuse_logarithmic(
    prev: &Instruction,
    next: &Instruction,
    pool: &mut ConstantPool<'_>,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<Vec<Instruction>> {
    match (prev, next) {
        // Exp{t, s}, SubConst{d, t, c=1.0} → Expm1{d, s}
        (
            Instruction::Builtin1 {
                dest: exp_dest,
                op: FnOp::Exp,
                arg: exp_arg,
            },
            Instruction::SubConst {
                dest: sub_dest,
                src: sub_src,
                const_idx: sub_c,
            },
        ) if sub_src == exp_dest && single_use(exp_dest) && pool.get(*sub_c) == 1.0 => {
            return Some(vec![Instruction::Builtin1 {
                dest: *sub_dest,
                op: FnOp::Expm1,
                arg: *exp_arg,
            }]);
        }
        // AddConst{t, s, c=1.0}, Ln{d, t} → Log1p{d, s}
        (
            Instruction::AddConst {
                dest: add_dest,
                src: add_src,
                const_idx: add_c,
            },
            Instruction::Builtin1 {
                dest: ln_dest,
                op: FnOp::Ln,
                arg: ln_arg,
            },
        ) if ln_arg == add_dest && single_use(add_dest) && pool.get(*add_c) == 1.0 => {
            return Some(vec![Instruction::Builtin1 {
                dest: *ln_dest,
                op: FnOp::Log1p,
                arg: *add_src,
            }]);
        }
        // Recip{t, s}, Builtin1{d, Ln, t} → Ln{t, s}, Neg{d, t}
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
        ) if tmp_arg == tmp_dest && single_use(tmp_dest) => {
            return Some(vec![
                Instruction::Builtin1 {
                    dest: *tmp_dest,
                    op: FnOp::Ln,
                    arg: *src_reg,
                },
                Instruction::Neg {
                    dest: *dest_reg,
                    src: *tmp_dest,
                },
            ]);
        }
        // Sqrt{t, s}, Builtin1{d, Ln, t} → Ln{t, s}, MulConst{d, t, 0.5}
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
        ) if tmp_arg == tmp_dest && single_use(tmp_dest) => {
            let c_idx = pool.get_or_insert(0.5);
            return Some(vec![
                Instruction::Builtin1 {
                    dest: *tmp_dest,
                    op: FnOp::Ln,
                    arg: *src_reg,
                },
                Instruction::MulConst {
                    dest: *dest_reg,
                    src: *tmp_dest,
                    const_idx: c_idx,
                },
            ]);
        }
        _ => {}
    }
    None
}

// ─── Idempotent / self-cancelling fusions ────────────────────────────────────

/// Fuses self-cancelling or idempotent pairs:
/// - `Abs` + `Abs` → `Abs`
/// - `Neg` + `Neg` → `Copy`
/// - `Recip` + `Recip` → `Copy`
/// - `Square` + `Sqrt` → `Abs`
/// - `Abs` + `Square` → `Square`
/// - `InvSqrt` + `Recip` → `Sqrt`
/// - `ExpNeg` + `Recip` → `Exp`
#[allow(
    clippy::too_many_lines,
    reason = "Large match for idempotent/self-cancelling patterns"
)]
fn try_fuse_idempotent(
    prev: &Instruction,
    next: &Instruction,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<Vec<Instruction>> {
    match (prev, next) {
        // Abs{t, s}, Abs{d, t} → Abs{d, s}
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
            return Some(vec![Instruction::Builtin1 {
                dest: *dest_reg,
                op: FnOp::Abs,
                arg: *src_reg,
            }]);
        }
        // Neg{t, s}, Neg{d, t} → Copy{d, s}
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
            return Some(vec![Instruction::Copy {
                dest: *dest_reg,
                src: *src_reg,
            }]);
        }
        // Recip{t, s}, Recip{d, t} → Copy{d, s}
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
            return Some(vec![Instruction::Copy {
                dest: *dest_reg,
                src: *src_reg,
            }]);
        }
        // Square{t, s}, Builtin1{d, Sqrt, t} → Abs{d, s}
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
            return Some(vec![Instruction::Builtin1 {
                dest: *dest_reg,
                op: FnOp::Abs,
                arg: *src_reg,
            }]);
        }
        // Abs{t, s}, Square{d, t} → Square{d, s}
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
            return Some(vec![Instruction::Square {
                dest: *dest_reg,
                src: *src_reg,
            }]);
        }
        // InvSqrt{t, s}, Recip{d, t} → Sqrt{d, s}
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
            return Some(vec![Instruction::Builtin1 {
                dest: *dest_reg,
                op: FnOp::Sqrt,
                arg: *src_reg,
            }]);
        }
        // ExpNeg{t, s}, Recip{d, t} → Exp{d, s}
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
        ) if tmp_src == tmp_dest && single_use(tmp_dest) => {
            return Some(vec![Instruction::Builtin1 {
                dest: *dest_reg,
                op: FnOp::Exp,
                arg: *src_reg,
            }]);
        }
        _ => {}
    }
    None
}

// ─── Constant chain fusions ─────────────────────────────────────────────────

/// Fuses consecutive constant operations:
/// - `AddConst(c1)` + `AddConst(c2)` → `AddConst(c1 + c2)`
/// - `MulConst(c1)` + `MulConst(c2)` → `MulConst(c1 * c2)`
fn try_fuse_const_chain(
    prev: &Instruction,
    next: &Instruction,
    pool: &mut ConstantPool<'_>,
    single_use: &impl Fn(&u32) -> bool,
) -> Option<Vec<Instruction>> {
    match (prev, next) {
        // AddConst{t, s, c1}, AddConst{d, t, c2} → AddConst{d, s, c1+c2}
        (
            Instruction::AddConst {
                dest: tmp_dest,
                src: src_reg,
                const_idx: const_idx1,
            },
            Instruction::AddConst {
                dest: dest_reg,
                src: tmp_src,
                const_idx: const_idx2,
            },
        ) if tmp_src == tmp_dest && single_use(tmp_dest) => {
            let val = pool.get(*const_idx1) + pool.get(*const_idx2);
            let new_idx = pool.get_or_insert(val);
            return Some(vec![Instruction::AddConst {
                dest: *dest_reg,
                src: *src_reg,
                const_idx: new_idx,
            }]);
        }
        // MulConst{t, s, c1}, MulConst{d, t, c2} → MulConst{d, s, c1*c2}
        (
            Instruction::MulConst {
                dest: tmp_dest,
                src: src_reg,
                const_idx: const_idx1,
            },
            Instruction::MulConst {
                dest: dest_reg,
                src: tmp_src,
                const_idx: const_idx2,
            },
        ) if tmp_src == tmp_dest && single_use(tmp_dest) => {
            let val = pool.get(*const_idx1) * pool.get(*const_idx2);
            let new_idx = pool.get_or_insert(val);
            return Some(vec![Instruction::MulConst {
                dest: *dest_reg,
                src: *src_reg,
                const_idx: new_idx,
            }]);
        }
        _ => {}
    }
    None
}
