use super::super::super::instruction::{FnOp, Instruction};
use super::helper::ConstantPool;

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
            match (&instructions[instr_idx], &instructions[instr_idx + 1]) {
                // Neg{t, s}, Mul{d, t, b} -> NegMul{d, s, b}
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
                        out.push(Instruction::NegMul {
                            dest: *mul_dest,
                            a: *neg_src,
                            b: *mul_b,
                        });
                        instr_idx += 2;
                        continue;
                    } else if mul_b == neg_dest {
                        out.push(Instruction::NegMul {
                            dest: *mul_dest,
                            a: *neg_src,
                            b: *mul_a,
                        });
                        instr_idx += 2;
                        continue;
                    }
                }
                // Neg{t, s}, MulConst{d, t, c} -> NegMulConst{d, s, c}
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
                    out.push(Instruction::NegMulConst {
                        dest: *mul_dest,
                        src: *neg_src,
                        const_idx: *mul_c,
                    });
                    instr_idx += 2;
                    continue;
                }
                // MulConst{t, s, -1.0}, Mul{d, t, b} -> NegMul{d, s, b}
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
                        out.push(Instruction::NegMul {
                            dest: *mul_dest,
                            a: *mc_src,
                            b: *mul_b,
                        });
                        instr_idx += 2;
                        continue;
                    } else if mul_b == mc_dest {
                        out.push(Instruction::NegMul {
                            dest: *mul_dest,
                            a: *mc_src,
                            b: *mul_a,
                        });
                        instr_idx += 2;
                        continue;
                    }
                }
                // LoadConst{t, c}, Add{d, t, s} or Add{d, s, t} -> AddConst{d, s, c}
                (
                    Instruction::LoadConst {
                        dest: ld_dest,
                        const_idx: ld_c,
                    },
                    Instruction::Add {
                        dest: add_dest,
                        a: add_a,
                        b: add_b,
                    },
                ) if single_use(ld_dest) => {
                    if add_a == ld_dest {
                        out.push(Instruction::AddConst {
                            dest: *add_dest,
                            src: *add_b,
                            const_idx: *ld_c,
                        });
                        instr_idx += 2;
                        continue;
                    } else if add_b == ld_dest {
                        out.push(Instruction::AddConst {
                            dest: *add_dest,
                            src: *add_a,
                            const_idx: *ld_c,
                        });
                        instr_idx += 2;
                        continue;
                    }
                }
                // LoadConst{t, c}, MulAdd{d, a, b, t} -> MulAddConst{d, a, b, c}
                (
                    Instruction::LoadConst {
                        dest: ld_dest,
                        const_idx: ld_c,
                    },
                    Instruction::MulAdd {
                        dest: add_dest,
                        a: add_a,
                        b: add_b,
                        c: add_c,
                    },
                ) if *add_c == *ld_dest && single_use(ld_dest) => {
                    out.push(Instruction::MulAddConst {
                        dest: *add_dest,
                        a: *add_a,
                        b: *add_b,
                        const_idx: *ld_c,
                    });
                    instr_idx += 2;
                    continue;
                }
                // LoadConst{t, c}, MulSub{d, a, b, t} -> MulSubConst{d, a, b, c}
                (
                    Instruction::LoadConst {
                        dest: ld_dest,
                        const_idx: ld_c,
                    },
                    Instruction::MulSub {
                        dest: sub_dest,
                        a: sub_a,
                        b: sub_b,
                        c: sub_c,
                    },
                ) if *sub_c == *ld_dest && single_use(ld_dest) => {
                    out.push(Instruction::MulSubConst {
                        dest: *sub_dest,
                        a: *sub_a,
                        b: *sub_b,
                        const_idx: *ld_c,
                    });
                    instr_idx += 2;
                    continue;
                }
                // LoadConst{t, c}, NegMulAdd{d, a, b, t} -> NegMulAddConst{d, a, b, c}
                (
                    Instruction::LoadConst {
                        dest: ld_dest,
                        const_idx: ld_c,
                    },
                    Instruction::NegMulAdd {
                        dest: add_dest,
                        a: add_a,
                        b: add_b,
                        c: add_c,
                    },
                ) if *add_c == *ld_dest && single_use(ld_dest) => {
                    out.push(Instruction::NegMulAddConst {
                        dest: *add_dest,
                        a: *add_a,
                        b: *add_b,
                        const_idx: *ld_c,
                    });
                    instr_idx += 2;
                    continue;
                }
                // LoadConst{t, c}, Mul{d, t, s} or Mul{d, s, t} -> MulConst{d, s, c}
                (
                    Instruction::LoadConst {
                        dest: ld_dest,
                        const_idx: ld_c,
                    },
                    Instruction::Mul {
                        dest: mul_dest,
                        a: mul_a,
                        b: mul_b,
                    },
                ) if single_use(ld_dest) => {
                    if mul_a == ld_dest {
                        out.push(Instruction::MulConst {
                            dest: *mul_dest,
                            src: *mul_b,
                            const_idx: *ld_c,
                        });
                        instr_idx += 2;
                        continue;
                    } else if mul_b == ld_dest {
                        out.push(Instruction::MulConst {
                            dest: *mul_dest,
                            src: *mul_a,
                            const_idx: *ld_c,
                        });
                        instr_idx += 2;
                        continue;
                    }
                }
                // LoadConst{t, c}, Sub{d, s, t} -> SubConst{d, s, c}
                (
                    Instruction::LoadConst {
                        dest: ld_dest,
                        const_idx: ld_c,
                    },
                    Instruction::Sub {
                        dest: sub_dest,
                        a: sub_a,
                        b: sub_b,
                    },
                ) if *sub_b == *ld_dest && single_use(ld_dest) => {
                    out.push(Instruction::SubConst {
                        dest: *sub_dest,
                        src: *sub_a,
                        const_idx: *ld_c,
                    });
                    instr_idx += 2;
                    continue;
                }
                // LoadConst{t, c}, Sub{d, t, s} -> ConstSub{d, s, c}
                (
                    Instruction::LoadConst {
                        dest: ld_dest,
                        const_idx: ld_c,
                    },
                    Instruction::Sub {
                        dest: sub_dest,
                        a: sub_a,
                        b: sub_b,
                    },
                ) if *sub_a == *ld_dest && single_use(ld_dest) => {
                    out.push(Instruction::ConstSub {
                        dest: *sub_dest,
                        src: *sub_b,
                        const_idx: *ld_c,
                    });
                    instr_idx += 2;
                    continue;
                }
                // LoadConst{t, c}, Div{d, s, t} -> DivConst{d, s, c}
                (
                    Instruction::LoadConst {
                        dest: ld_dest,
                        const_idx: ld_c,
                    },
                    Instruction::Div {
                        dest: div_dest,
                        num: div_num,
                        den: div_den,
                    },
                ) if *div_den == *ld_dest && single_use(ld_dest) => {
                    out.push(Instruction::DivConst {
                        dest: *div_dest,
                        src: *div_num,
                        const_idx: *ld_c,
                    });
                    instr_idx += 2;
                    continue;
                }
                // LoadConst{t, c}, Div{d, t, s} -> ConstDiv{d, s, c}
                (
                    Instruction::LoadConst {
                        dest: ld_dest,
                        const_idx: ld_c,
                    },
                    Instruction::Div {
                        dest: div_dest,
                        num: div_num,
                        den: div_den,
                    },
                ) if *div_num == *ld_dest && single_use(ld_dest) => {
                    out.push(Instruction::ConstDiv {
                        dest: *div_dest,
                        src: *div_den,
                        const_idx: *ld_c,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Mul{t,a,b}, Add{d, t, c} -> MulAdd{d, a, b, c}
                (
                    Instruction::Mul {
                        dest: mul_dest,
                        a: mul_a,
                        b: mul_b,
                    },
                    Instruction::Add {
                        dest: add_dest,
                        a: add_a,
                        b: add_b,
                    },
                ) if single_use(mul_dest) => {
                    if add_a == mul_dest {
                        out.push(Instruction::MulAdd {
                            dest: *add_dest,
                            a: *mul_a,
                            b: *mul_b,
                            c: *add_b,
                        });
                        instr_idx += 2;
                        continue;
                    } else if add_b == mul_dest {
                        out.push(Instruction::MulAdd {
                            dest: *add_dest,
                            a: *mul_a,
                            b: *mul_b,
                            c: *add_a,
                        });
                        instr_idx += 2;
                        continue;
                    }
                }
                // Mul{t,a,b}, AddConst{d, t, c} -> MulAddConst{d, a, b, c}
                (
                    Instruction::Mul {
                        dest: mul_dest,
                        a: mul_a,
                        b: mul_b,
                    },
                    Instruction::AddConst {
                        dest: add_dest,
                        src: add_src,
                        const_idx: add_c,
                    },
                ) if add_src == mul_dest && single_use(mul_dest) => {
                    out.push(Instruction::MulAddConst {
                        dest: *add_dest,
                        a: *mul_a,
                        b: *mul_b,
                        const_idx: *add_c,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Mul{t,a,b}, Sub{d, t, c} -> MulSub{d, a, b, c}
                (
                    Instruction::Mul {
                        dest: mul_dest,
                        a: mul_a,
                        b: mul_b,
                    },
                    Instruction::Sub {
                        dest: sub_dest,
                        a: sub_a,
                        b: sub_b,
                    },
                ) if sub_a == mul_dest && single_use(mul_dest) => {
                    out.push(Instruction::MulSub {
                        dest: *sub_dest,
                        a: *mul_a,
                        b: *mul_b,
                        c: *sub_b,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Mul{t,a,b}, SubConst{d, t, c} -> MulSubConst{d, a, b, c}
                (
                    Instruction::Mul {
                        dest: mul_dest,
                        a: mul_a,
                        b: mul_b,
                    },
                    Instruction::SubConst {
                        dest: sub_dest,
                        src: sub_src,
                        const_idx: sub_c,
                    },
                ) if sub_src == mul_dest && single_use(mul_dest) => {
                    out.push(Instruction::MulSubConst {
                        dest: *sub_dest,
                        a: *mul_a,
                        b: *mul_b,
                        const_idx: *sub_c,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Mul{t,a,b}, Sub{d, c, t} -> NegMulAdd{d, a, b, c}
                (
                    Instruction::Mul {
                        dest: mul_dest,
                        a: mul_a,
                        b: mul_b,
                    },
                    Instruction::Sub {
                        dest: sub_dest,
                        a: sub_a,
                        b: sub_b,
                    },
                ) if sub_b == mul_dest && single_use(mul_dest) => {
                    out.push(Instruction::NegMulAdd {
                        dest: *sub_dest,
                        a: *mul_a,
                        b: *mul_b,
                        c: *sub_a,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Mul{t,a,b}, ConstSub{d, t, c} -> NegMulAddConst{d, a, b, c}
                (
                    Instruction::Mul {
                        dest: mul_dest,
                        a: mul_a,
                        b: mul_b,
                    },
                    Instruction::ConstSub {
                        dest: sub_dest,
                        src: sub_src,
                        const_idx: sub_c,
                    },
                ) if sub_src == mul_dest && single_use(mul_dest) => {
                    out.push(Instruction::NegMulAddConst {
                        dest: *sub_dest,
                        a: *mul_a,
                        b: *mul_b,
                        const_idx: *sub_c,
                    });
                    instr_idx += 2;
                    continue;
                }
                // NegMulAdd{t,a,b,c}, Sub{d, c', t} -> NegMulAdd{d, a, b, c' - c} -- wait, too complex.
                // Just basic ones for now.
                // Neg{t, s}, Exp{d, t} -> ExpNeg{d, s}
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
                    out.push(Instruction::Builtin1 {
                        dest: *exp_dest,
                        op: FnOp::ExpNeg,
                        arg: *neg_src,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Exp{t, s}, SubConst{d, t, c=1.0} -> Expm1{d, s}
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
                    out.push(Instruction::Builtin1 {
                        dest: *sub_dest,
                        op: FnOp::Expm1,
                        arg: *exp_arg,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Exp{t, s}, Recip{d, t} -> ExpNeg{d, s}
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
                    out.push(Instruction::Builtin1 {
                        dest: *recip_dest,
                        op: FnOp::ExpNeg,
                        arg: *exp_arg,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Square{t, s}, Square{d, t} -> Pow4{d, s}
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
                    out.push(Instruction::Pow4 {
                        dest: *sq2_dest,
                        src: *sq1_src,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Expm1{t, s}, Recip{d, t} -> RecipExpm1{d, s}
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
                    out.push(Instruction::RecipExpm1 {
                        dest: *recip_dest,
                        src: *expm1_arg,
                    });
                    instr_idx += 2;
                    continue;
                }
                // AddConst{t, s, c=1.0}, Ln{d, t} -> Log1p{d, s}
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
                    out.push(Instruction::Builtin1 {
                        dest: *ln_dest,
                        op: FnOp::Log1p,
                        arg: *add_src,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Sqrt{t, s}, Recip{d, t} -> InvSqrt{d, s}
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
                    out.push(Instruction::InvSqrt {
                        dest: *recip_dest,
                        src: *sqrt_src,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Square{t, s}, Recip{d, t} -> InvSquare{d, s}
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
                    out.push(Instruction::InvSquare {
                        dest: *recip_dest,
                        src: *sq_src,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Cube{t, s}, Recip{d, t} -> InvCube{d, s}
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
                    out.push(Instruction::InvCube {
                        dest: *recip_dest,
                        src: *cube_src,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Pow3_2{t, s}, Recip{d, t} -> InvPow3_2{d, s}
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
                    out.push(Instruction::InvPow3_2 {
                        dest: *recip_dest,
                        src: *p32_src,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Pow4{t, s}, Recip{d, t} -> InvSquare{d, t'} where t' = Square(s)
                // Note: We can reuse the temp register from Pow4 for the intermediate Square.
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
                    // Reuse p4_dest as a temporary for x^2
                    out.push(Instruction::Square {
                        dest: *p4_dest,
                        src: *p4_src,
                    });
                    out.push(Instruction::InvSquare {
                        dest: *recip_dest,
                        src: *p4_dest,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Square{t, s}, Mul{d, t, s} or Mul{d, s, t} -> Cube{d, s}
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
                    out.push(Instruction::Cube {
                        dest: *mul_dest,
                        src: *sq_src,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Cube{t, s}, Mul{d, t, s} or Mul{d, s, t} -> Pow4{d, s}
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
                    out.push(Instruction::Pow4 {
                        dest: *mul_dest,
                        src: *cube_src,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Exp{t, s}, Square{d, t} -> MulConst{t, s, 2.0}, Exp{d, t}
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
                    // We need a constant 2.0. If it's not there, we'll just keep it.
                    // Optimization pass 3 will deduplicate this later.
                    let c_idx = pool.get_or_insert(2.0);
                    out.push(Instruction::MulConst {
                        dest: *exp_dest,
                        src: *exp_arg,
                        const_idx: c_idx,
                    });
                    out.push(Instruction::Builtin1 {
                        dest: *sq_dest,
                        op: FnOp::Exp,
                        arg: *exp_dest,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Square{t, s}, Builtin1{d, Ln, t} -> Builtin1{t, Ln, s}, MulConst{d, t, 2.0}
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
                    out.push(Instruction::Builtin1 {
                        dest: *sq_dest,
                        op: FnOp::Abs,
                        arg: *sq_src,
                    });
                    out.push(Instruction::Builtin1 {
                        dest: *sq_dest,
                        op: FnOp::Ln,
                        arg: *sq_dest,
                    });
                    out.push(Instruction::MulConst {
                        dest: *ln_dest,
                        src: *sq_dest,
                        const_idx: c_idx,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Abs{tmp_dest, src_reg}, Abs{dest_reg, tmp_dest} -> Abs{dest_reg, src_reg}
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
                    out.push(Instruction::Builtin1 {
                        dest: *dest_reg,
                        op: FnOp::Abs,
                        arg: *src_reg,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Neg{tmp_dest, src_reg}, Neg{dest_reg, tmp_dest} -> Copy{dest_reg, src_reg}
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
                    out.push(Instruction::Copy {
                        dest: *dest_reg,
                        src: *src_reg,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Recip{tmp_dest, src_reg}, Recip{dest_reg, tmp_dest} -> Copy{dest_reg, src_reg}
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
                    out.push(Instruction::Copy {
                        dest: *dest_reg,
                        src: *src_reg,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Neg{tmp_dest, src_reg}, Abs{dest_reg, tmp_dest} -> Abs{dest_reg, src_reg}
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
                    out.push(Instruction::Builtin1 {
                        dest: *dest_reg,
                        op: FnOp::Abs,
                        arg: *src_reg,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Neg{tmp_dest, src_reg}, Square{dest_reg, tmp_dest} -> Square{dest_reg, src_reg}
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
                    out.push(Instruction::Square {
                        dest: *dest_reg,
                        src: *src_reg,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Abs{tmp_dest, src_reg}, Square{dest_reg, tmp_dest} -> Square{dest_reg, src_reg}
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
                    out.push(Instruction::Square {
                        dest: *dest_reg,
                        src: *src_reg,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Recip{tmp_dest, src_reg}, Builtin1{dest_reg, Ln, tmp_dest} -> Builtin1{tmp_dest, Ln, src_reg}, Neg{dest_reg, tmp_dest}
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
                    out.push(Instruction::Builtin1 {
                        dest: *tmp_dest,
                        op: FnOp::Ln,
                        arg: *src_reg,
                    });
                    out.push(Instruction::Neg {
                        dest: *dest_reg,
                        src: *tmp_dest,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Square{tmp_dest, src_reg}, Builtin1{dest_reg, Sqrt, tmp_dest} -> Builtin1{dest_reg, Abs, src_reg}
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
                    out.push(Instruction::Builtin1 {
                        dest: *dest_reg,
                        op: FnOp::Abs,
                        arg: *src_reg,
                    });
                    instr_idx += 2;
                    continue;
                }
                // InvSqrt{tmp_dest, src_reg}, Recip{dest_reg, tmp_dest} -> Builtin1{dest_reg, Sqrt, src_reg}
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
                    out.push(Instruction::Builtin1 {
                        dest: *dest_reg,
                        op: FnOp::Sqrt,
                        arg: *src_reg,
                    });
                    instr_idx += 2;
                    continue;
                }
                // ExpNeg{tmp_dest, src_reg}, Recip{dest_reg, tmp_dest} -> Builtin1{dest_reg, Exp, src_reg}
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
                    out.push(Instruction::Builtin1 {
                        dest: *dest_reg,
                        op: FnOp::Exp,
                        arg: *src_reg,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Sqrt{tmp_dest, src_reg}, Builtin1{dest_reg, Ln, tmp_dest} -> Builtin1{tmp_dest, Ln, src_reg}, MulConst{dest_reg, tmp_dest, 0.5}
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
                    out.push(Instruction::Builtin1 {
                        dest: *tmp_dest,
                        op: FnOp::Ln,
                        arg: *src_reg,
                    });
                    out.push(Instruction::MulConst {
                        dest: *dest_reg,
                        src: *tmp_dest,
                        const_idx: c_idx,
                    });
                    instr_idx += 2;
                    continue;
                }
                // AddConst{tmp_dest, src_reg, c1}, AddConst{dest_reg, tmp_dest, c2} -> AddConst{dest_reg, src_reg, c1+c2}
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
                    out.push(Instruction::AddConst {
                        dest: *dest_reg,
                        src: *src_reg,
                        const_idx: new_idx,
                    });
                    instr_idx += 2;
                    continue;
                }
                // MulConst{tmp_dest, src_reg, c1}, MulConst{dest_reg, tmp_dest, c2} -> MulConst{dest_reg, src_reg, c1*c2}
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
                    out.push(Instruction::MulConst {
                        dest: *dest_reg,
                        src: *src_reg,
                        const_idx: new_idx,
                    });
                    instr_idx += 2;
                    continue;
                }
                // Neg{tmp_dest, src_reg}, AddConst{dest_reg, tmp_dest, const_idx_val} -> ConstSub{dest_reg, src_reg, const_idx_val}
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
                    out.push(Instruction::ConstSub {
                        dest: *dest_reg,
                        src: *src_reg,
                        const_idx: *const_idx_val,
                    });
                    instr_idx += 2;
                    continue;
                }
                _ => {}
            }
        }

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
