use super::super::instruction::Instruction;
use super::vir::{VInstruction, VReg};

pub struct RegAllocator {
    param_count: u32,
    const_count: u32,
    num_temps: usize,
    last_use: Vec<Option<usize>>,
    deaths: Vec<(usize, u32)>,
    arg_pool: Vec<u32>,
    last_phys_0_read: Option<usize>,
}

impl RegAllocator {
    pub(crate) fn new(
        param_count: u32,
        const_count: u32,
        num_temps: usize,
        vinstrs: &[VInstruction],
    ) -> Self {
        let mut last_use = vec![None; num_temps];
        let mut last_phys_0_read = None;

        let phys_0_vreg = if param_count > 0 {
            Some(VReg::Param(0))
        } else if const_count > 0 {
            Some(VReg::Const(0))
        } else {
            None
        };

        for (idx, instr) in vinstrs.iter().enumerate() {
            instr.for_each_read(|src| match src {
                VReg::Temp(t) => {
                    last_use[t as usize] = Some(idx);
                }
                _ => {
                    if Some(src) == phys_0_vreg {
                        last_phys_0_read = Some(idx);
                    }
                }
            });
        }

        let mut deaths: Vec<(usize, u32)> = last_use
            .iter()
            .enumerate()
            .filter_map(|(t, lu_opt)| {
                lu_opt.map(|lu| (lu, u32::try_from(t).expect("Temp index too large")))
            })
            .collect();
        deaths.sort_unstable_by_key(|&(idx, _)| idx);

        Self {
            param_count,
            const_count,
            num_temps,
            last_use,
            deaths,
            arg_pool: Vec::with_capacity(128),
            last_phys_0_read,
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "Large match statement for IR dispatch"
    )]
    pub(crate) fn allocate(
        mut self,
        vinstrs: Vec<VInstruction>,
        final_vreg: Option<VReg>,
    ) -> (Vec<Instruction>, Vec<u32>, usize) {
        let n_instrs = vinstrs.len();
        let mut max_phys = self.param_count + self.const_count;
        let mut temp_to_phys: Vec<u32> = vec![u32::MAX; self.num_temps];
        let mut free_phys: Vec<u32> = Vec::new();
        let mut instructions = Vec::with_capacity(n_instrs);
        let mut death_cursor = 0;

        for (idx, instr) in vinstrs.into_iter().enumerate() {
            let map_vreg_phys = |vreg: VReg, t2p: &[u32]| -> u32 {
                match vreg {
                    VReg::Param(p) => p,
                    VReg::Const(c) => self.param_count + c,
                    VReg::Temp(t) => t2p[t as usize],
                }
            };

            let dest_vreg = instr.dest();
            let is_final_prod = final_vreg == Some(dest_vreg);

            let dest_phys = match dest_vreg {
                VReg::Param(p) => p,
                VReg::Const(c) => self.param_count + c,
                VReg::Temp(t) => {
                    // Optimization: If this instruction produces the final result, try to land it in
                    // register 0 immediately. This is safe if register 0 (Param(0) or Const(0))
                    // is not used after this instruction.
                    if is_final_prod && self.last_phys_0_read.is_none_or(|lu| lu <= idx) {
                        temp_to_phys[t as usize] = 0;
                        0
                    } else {
                        let p = free_phys.pop().unwrap_or_else(|| {
                            let p = max_phys;
                            max_phys += 1;
                            p
                        });
                        temp_to_phys[t as usize] = p;
                        if self.last_use[t as usize].is_none() {
                            free_phys.push(p);
                        }
                        p
                    }
                }
            };

            let map_vreg_local = |vreg: VReg| map_vreg_phys(vreg, &temp_to_phys);

            match instr {
                VInstruction::Add { srcs, .. } => match srcs.len() {
                    1 => instructions.push(Instruction::Copy {
                        dest: dest_phys,
                        src: map_vreg_local(srcs[0]),
                    }),
                    2 => self.emit_add2(
                        dest_phys,
                        srcs[0],
                        srcs[1],
                        &temp_to_phys,
                        &mut instructions,
                    ),
                    _ => {
                        let start_idx = u32::try_from(self.arg_pool.len())
                            .expect("Arg pool too large for u32 index");
                        for &s in &srcs {
                            self.arg_pool.push(map_vreg_local(s));
                        }
                        instructions.push(Instruction::AddN {
                            dest: dest_phys,
                            start_idx,
                            count: u32::try_from(srcs.len()).expect("Too many sources for AddN"),
                        });
                    }
                },
                VInstruction::Add2 { a, b, .. } => {
                    self.emit_add2(dest_phys, a, b, &temp_to_phys, &mut instructions);
                }
                VInstruction::Mul { srcs, .. } => match srcs.len() {
                    1 => instructions.push(Instruction::Copy {
                        dest: dest_phys,
                        src: map_vreg_local(srcs[0]),
                    }),
                    2 => self.emit_mul2(
                        dest_phys,
                        srcs[0],
                        srcs[1],
                        &temp_to_phys,
                        &mut instructions,
                    ),
                    _ => {
                        let start_idx = u32::try_from(self.arg_pool.len())
                            .expect("Arg pool too large for u32 index");
                        for &s in &srcs {
                            self.arg_pool.push(map_vreg_local(s));
                        }
                        instructions.push(Instruction::MulN {
                            dest: dest_phys,
                            start_idx,
                            count: u32::try_from(srcs.len()).expect("Too many sources for MulN"),
                        });
                    }
                },
                VInstruction::Mul2 { a, b, .. } => {
                    self.emit_mul2(dest_phys, a, b, &temp_to_phys, &mut instructions);
                }
                VInstruction::Sub { a, b, .. } => match (a, b) {
                    (VReg::Const(c), _) => instructions.push(Instruction::ConstSub {
                        dest: dest_phys,
                        src: map_vreg_local(b),
                        const_idx: c,
                    }),
                    (_, VReg::Const(c)) => instructions.push(Instruction::SubConst {
                        dest: dest_phys,
                        src: map_vreg_local(a),
                        const_idx: c,
                    }),
                    _ => instructions.push(Instruction::Sub {
                        dest: dest_phys,
                        a: map_vreg_local(a),
                        b: map_vreg_local(b),
                    }),
                },
                VInstruction::Div { num, den, .. } => match (num, den) {
                    (VReg::Const(c), _) => instructions.push(Instruction::ConstDiv {
                        dest: dest_phys,
                        src: map_vreg_local(den),
                        const_idx: c,
                    }),
                    (_, VReg::Const(c)) => instructions.push(Instruction::DivConst {
                        dest: dest_phys,
                        src: map_vreg_local(num),
                        const_idx: c,
                    }),
                    _ => instructions.push(Instruction::Div {
                        dest: dest_phys,
                        num: map_vreg_local(num),
                        den: map_vreg_local(den),
                    }),
                },
                VInstruction::Pow { base, exp, .. } => instructions.push(Instruction::Pow {
                    dest: dest_phys,
                    base: map_vreg_local(base),
                    exp: map_vreg_local(exp),
                }),
                VInstruction::Neg { src, .. } => instructions.push(Instruction::Neg {
                    dest: dest_phys,
                    src: map_vreg_local(src),
                }),
                VInstruction::BuiltinFun { op, args, .. } => match args.len() {
                    1 => instructions.push(Instruction::Builtin1 {
                        dest: dest_phys,
                        op,
                        arg: map_vreg_local(args[0]),
                    }),
                    2 => instructions.push(Instruction::Builtin2 {
                        dest: dest_phys,
                        op,
                        arg1: map_vreg_local(args[0]),
                        arg2: map_vreg_local(args[1]),
                    }),
                    _ => {
                        let start_idx = u32::try_from(self.arg_pool.len())
                            .expect("Arg pool too large for u32 index");
                        for &s in &args {
                            self.arg_pool.push(map_vreg_local(s));
                        }
                        if args.len() == 3 {
                            instructions.push(Instruction::Builtin3 {
                                dest: dest_phys,
                                op,
                                start_idx,
                            });
                        } else {
                            instructions.push(Instruction::Builtin4 {
                                dest: dest_phys,
                                op,
                                start_idx,
                            });
                        }
                    }
                },
                VInstruction::Builtin1 { op, arg, .. } => {
                    instructions.push(Instruction::Builtin1 {
                        dest: dest_phys,
                        op,
                        arg: map_vreg_local(arg),
                    });
                }
                VInstruction::Builtin2 { op, arg1, arg2, .. } => {
                    instructions.push(Instruction::Builtin2 {
                        dest: dest_phys,
                        op,
                        arg1: map_vreg_local(arg1),
                        arg2: map_vreg_local(arg2),
                    });
                }
                VInstruction::Square { src, .. } => instructions.push(Instruction::Square {
                    dest: dest_phys,
                    src: map_vreg_local(src),
                }),
                VInstruction::Cube { src, .. } => instructions.push(Instruction::Cube {
                    dest: dest_phys,
                    src: map_vreg_local(src),
                }),
                VInstruction::Pow4 { src, .. } => instructions.push(Instruction::Pow4 {
                    dest: dest_phys,
                    src: map_vreg_local(src),
                }),
                VInstruction::Pow3_2 { src, .. } => instructions.push(Instruction::Pow3_2 {
                    dest: dest_phys,
                    src: map_vreg_local(src),
                }),
                VInstruction::InvPow3_2 { src, .. } => instructions.push(Instruction::InvPow3_2 {
                    dest: dest_phys,
                    src: map_vreg_local(src),
                }),
                VInstruction::InvSqrt { src, .. } => instructions.push(Instruction::InvSqrt {
                    dest: dest_phys,
                    src: map_vreg_local(src),
                }),
                VInstruction::InvSquare { src, .. } => instructions.push(Instruction::InvSquare {
                    dest: dest_phys,
                    src: map_vreg_local(src),
                }),
                VInstruction::InvCube { src, .. } => instructions.push(Instruction::InvCube {
                    dest: dest_phys,
                    src: map_vreg_local(src),
                }),
                VInstruction::Recip { src, .. } => instructions.push(Instruction::Recip {
                    dest: dest_phys,
                    src: map_vreg_local(src),
                }),
                VInstruction::Powi { src, n, .. } => instructions.push(Instruction::Powi {
                    dest: dest_phys,
                    src: map_vreg_local(src),
                    n,
                }),
                VInstruction::MulAdd { a, b, c, .. } => {
                    if let VReg::Const(c_idx) = c {
                        instructions.push(Instruction::MulAddConst {
                            dest: dest_phys,
                            a: map_vreg_local(a),
                            b: map_vreg_local(b),
                            const_idx: c_idx,
                        });
                    } else {
                        instructions.push(Instruction::MulAdd {
                            dest: dest_phys,
                            a: map_vreg_local(a),
                            b: map_vreg_local(b),
                            c: map_vreg_local(c),
                        });
                    }
                }
                VInstruction::MulSub { a, b, c, .. } => {
                    if let VReg::Const(c_idx) = c {
                        instructions.push(Instruction::MulSubConst {
                            dest: dest_phys,
                            a: map_vreg_local(a),
                            b: map_vreg_local(b),
                            const_idx: c_idx,
                        });
                    } else {
                        instructions.push(Instruction::MulSub {
                            dest: dest_phys,
                            a: map_vreg_local(a),
                            b: map_vreg_local(b),
                            c: map_vreg_local(c),
                        });
                    }
                }
                VInstruction::NegMulAdd { a, b, c, .. } => {
                    if let VReg::Const(c_idx) = c {
                        instructions.push(Instruction::NegMulAddConst {
                            dest: dest_phys,
                            a: map_vreg_local(a),
                            b: map_vreg_local(b),
                            const_idx: c_idx,
                        });
                    } else {
                        instructions.push(Instruction::NegMulAdd {
                            dest: dest_phys,
                            a: map_vreg_local(a),
                            b: map_vreg_local(b),
                            c: map_vreg_local(c),
                        });
                    }
                }
                VInstruction::RecipExpm1 { src, .. } => {
                    instructions.push(Instruction::RecipExpm1 {
                        dest: dest_phys,
                        src: map_vreg_local(src),
                    });
                }
                VInstruction::ExpSqr { src, .. } => instructions.push(Instruction::ExpSqr {
                    dest: dest_phys,
                    src: map_vreg_local(src),
                }),
                VInstruction::ExpSqrNeg { src, .. } => instructions.push(Instruction::ExpSqrNeg {
                    dest: dest_phys,
                    src: map_vreg_local(src),
                }),
            }

            while death_cursor < self.deaths.len() && self.deaths[death_cursor].0 == idx {
                let t_id = self.deaths[death_cursor].1;
                let p = temp_to_phys[t_id as usize];
                if p != u32::MAX {
                    free_phys.push(p);
                }
                death_cursor += 1;
            }
        }

        if let Some(f_vreg) = final_vreg {
            let src_phys = match f_vreg {
                VReg::Param(p) => p,
                VReg::Const(c) => self.param_count + c,
                VReg::Temp(t) => temp_to_phys[t as usize],
            };
            if src_phys != 0 {
                instructions.push(Instruction::Copy {
                    dest: 0,
                    src: src_phys,
                });
            }
        } else {
            instructions.push(Instruction::LoadConst {
                dest: 0,
                const_idx: 0,
            });
        }

        (instructions, self.arg_pool, max_phys as usize)
    }

    fn emit_add2(
        &self,
        dest_phys: u32,
        a: VReg,
        b: VReg,
        t2p: &[u32],
        instrs: &mut Vec<Instruction>,
    ) {
        let map = |v| match v {
            VReg::Param(p) => p,
            VReg::Const(c) => self.param_count + c,
            VReg::Temp(t) => t2p[t as usize],
        };
        match (a, b) {
            (VReg::Const(c), _) => instrs.push(Instruction::AddConst {
                dest: dest_phys,
                src: map(b),
                const_idx: c,
            }),
            (_, VReg::Const(c)) => instrs.push(Instruction::AddConst {
                dest: dest_phys,
                src: map(a),
                const_idx: c,
            }),
            _ => instrs.push(Instruction::Add {
                dest: dest_phys,
                a: map(a),
                b: map(b),
            }),
        }
    }

    fn emit_mul2(
        &self,
        dest_phys: u32,
        a: VReg,
        b: VReg,
        t2p: &[u32],
        instrs: &mut Vec<Instruction>,
    ) {
        let map = |v| match v {
            VReg::Param(p) => p,
            VReg::Const(c) => self.param_count + c,
            VReg::Temp(t) => t2p[t as usize],
        };
        match (a, b) {
            (VReg::Const(c), _) => instrs.push(Instruction::MulConst {
                dest: dest_phys,
                src: map(b),
                const_idx: c,
            }),
            (_, VReg::Const(c)) => instrs.push(Instruction::MulConst {
                dest: dest_phys,
                src: map(a),
                const_idx: c,
            }),
            _ => instrs.push(Instruction::Mul {
                dest: dest_phys,
                a: map(a),
                b: map(b),
            }),
        }
    }
}
