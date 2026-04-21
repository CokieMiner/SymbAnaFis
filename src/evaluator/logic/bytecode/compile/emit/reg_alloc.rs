use super::Instruction;
use super::vir::{VInstruction, VReg};

pub struct RegAllocator {
    param_count: u32,
    const_count: u32,
    num_temps: usize,
    last_use: Vec<Option<usize>>,
    death_heads: Vec<u32>,
    death_next: Vec<u32>,
    arg_pool: Vec<u32>,
    last_phys_0_read: Option<usize>,
}

#[inline]
fn map_vreg(vreg: VReg, param_count: u32, t2p: &[u32]) -> u32 {
    match vreg {
        VReg::Param(p) => p,
        VReg::Const(c) => param_count + c,
        VReg::Temp(t) => t2p[t as usize],
    }
}

impl RegAllocator {
    pub(crate) fn new(
        param_count: u32,
        const_count: u32,
        num_temps: usize,
        vinstrs: &[VInstruction],
        final_vreg: Option<VReg>,
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

        // Ensure the final result register is kept alive until the very end
        // so its physical register is not overwritten by intermediate computations.
        if let Some(VReg::Temp(t)) = final_vreg {
            let last_idx = vinstrs.len().saturating_sub(1);
            if let Some(lu) = last_use[t as usize] {
                last_use[t as usize] = Some(lu.max(last_idx));
            } else {
                last_use[t as usize] = Some(last_idx);
            }
        }

        // O(N) linked bucket sort for temporal register deaths
        let mut death_heads = vec![u32::MAX; vinstrs.len()];
        let mut death_next = vec![u32::MAX; num_temps];
        for (t, lu_opt) in last_use.iter().enumerate() {
            if let Some(lu) = lu_opt {
                let t_u32 = u32::try_from(t).expect("Temp index too large");
                death_next[t] = death_heads[*lu];
                death_heads[*lu] = t_u32;
            }
        }

        Self {
            param_count,
            const_count,
            num_temps,
            last_use,
            death_heads,
            death_next,
            arg_pool: Vec::with_capacity(128),
            last_phys_0_read,
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "Register allocation is a complex linear pass that is easier to maintain and faster to execute when kept in a single function"
    )]
    pub(crate) fn allocate(
        mut self,
        vinstrs: Vec<VInstruction>,
        final_vreg: Option<VReg>,
    ) -> (Vec<Instruction>, Vec<u32>, usize, u32) {
        let n_instrs = vinstrs.len();
        let mut max_phys = self.param_count + self.const_count;
        let mut temp_to_phys: Vec<u32> = vec![u32::MAX; self.num_temps];
        let mut free_phys: Vec<u32> = Vec::with_capacity(self.num_temps.min(64));
        let mut instructions = Vec::with_capacity(n_instrs);

        for (idx, instr) in vinstrs.into_iter().enumerate() {
            let dest_vreg = instr.dest();
            let is_final_prod = final_vreg == Some(dest_vreg);

            let dest_phys = match dest_vreg {
                VReg::Param(p) => p,
                VReg::Const(c) => self.param_count + c,
                VReg::Temp(t) => {
                    if is_final_prod
                        && self.last_phys_0_read.is_none_or(|lu| lu <= idx)
                        && self.param_count == 0
                        && self.const_count == 0
                    {
                        temp_to_phys[t as usize] = 0;
                        0
                    } else {
                        let p = free_phys.pop().unwrap_or_else(|| {
                            let p = max_phys;
                            max_phys += 1;
                            p
                        });
                        temp_to_phys[t as usize] = p;
                        debug_assert!(
                            self.last_use[t as usize].is_some(),
                            "Temp {t} has no last_use — should have been DCE'd"
                        );
                        if self.last_use[t as usize].is_none() {
                            free_phys.push(p);
                        }
                        p
                    }
                }
            };

            // Inlined macro for mapping VReg to physical register to avoid closure overhead
            // and ensure direct inlining at each call site.
            macro_rules! map_vreg_to_phys {
                ($vreg:expr) => {
                    match $vreg {
                        VReg::Param(p) => p,
                        VReg::Const(c) => self.param_count + c,
                        VReg::Temp(t) => temp_to_phys[t as usize],
                    }
                };
            }
            match instr {
                VInstruction::Add { srcs, .. } => {
                    debug_assert!(
                        srcs.len() >= 2,
                        "1-element Add should be simplified in VIR lowering"
                    );
                    if srcs.len() == 2 {
                        self.emit_add2(
                            dest_phys,
                            srcs[0],
                            srcs[1],
                            &temp_to_phys,
                            &mut instructions,
                        );
                    } else if srcs.len() == 3 {
                        instructions.push(Instruction::Add3 {
                            dest: dest_phys,
                            a: map_vreg_to_phys!(srcs[0]),
                            b: map_vreg_to_phys!(srcs[1]),
                            c: map_vreg_to_phys!(srcs[2]),
                        });
                    } else if srcs.len() == 4 {
                        instructions.push(Instruction::Add4 {
                            dest: dest_phys,
                            a: map_vreg_to_phys!(srcs[0]),
                            b: map_vreg_to_phys!(srcs[1]),
                            c: map_vreg_to_phys!(srcs[2]),
                            d: map_vreg_to_phys!(srcs[3]),
                        });
                    } else {
                        self.arg_pool.reserve(srcs.len());
                        let start_idx = u32::try_from(self.arg_pool.len())
                            .expect("Arg pool too large for u32 index");
                        for &s in &srcs {
                            self.arg_pool.push(map_vreg_to_phys!(s));
                        }
                        instructions.push(Instruction::AddN {
                            dest: dest_phys,
                            start_idx,
                            count: u32::try_from(srcs.len()).expect("Too many sources for AddN"),
                        });
                    }
                }
                VInstruction::Add2 { a, b, .. } => {
                    self.emit_add2(dest_phys, a, b, &temp_to_phys, &mut instructions);
                }
                VInstruction::Mul { srcs, .. } => {
                    debug_assert!(
                        srcs.len() >= 2,
                        "1-element Mul should be simplified in VIR lowering"
                    );
                    if srcs.len() == 2 {
                        self.emit_mul2(
                            dest_phys,
                            srcs[0],
                            srcs[1],
                            &temp_to_phys,
                            &mut instructions,
                        );
                    } else if srcs.len() == 3 {
                        instructions.push(Instruction::Mul3 {
                            dest: dest_phys,
                            a: map_vreg_to_phys!(srcs[0]),
                            b: map_vreg_to_phys!(srcs[1]),
                            c: map_vreg_to_phys!(srcs[2]),
                        });
                    } else if srcs.len() == 4 {
                        instructions.push(Instruction::Mul4 {
                            dest: dest_phys,
                            a: map_vreg_to_phys!(srcs[0]),
                            b: map_vreg_to_phys!(srcs[1]),
                            c: map_vreg_to_phys!(srcs[2]),
                            d: map_vreg_to_phys!(srcs[3]),
                        });
                    } else {
                        self.arg_pool.reserve(srcs.len());
                        let start_idx = u32::try_from(self.arg_pool.len())
                            .expect("Arg pool too large for u32 index");
                        for &s in &srcs {
                            self.arg_pool.push(map_vreg_to_phys!(s));
                        }
                        instructions.push(Instruction::MulN {
                            dest: dest_phys,
                            start_idx,
                            count: u32::try_from(srcs.len()).expect("Too many sources for MulN"),
                        });
                    }
                }
                VInstruction::Mul2 { a, b, .. } => {
                    self.emit_mul2(dest_phys, a, b, &temp_to_phys, &mut instructions);
                }
                VInstruction::Sub { a, b, .. } => instructions.push(Instruction::Sub {
                    dest: dest_phys,
                    a: map_vreg_to_phys!(a),
                    b: map_vreg_to_phys!(b),
                }),
                VInstruction::Div { num, den, .. } => instructions.push(Instruction::Div {
                    dest: dest_phys,
                    num: map_vreg_to_phys!(num),
                    den: map_vreg_to_phys!(den),
                }),
                VInstruction::Pow { base, exp, .. } => instructions.push(Instruction::Pow {
                    dest: dest_phys,
                    base: map_vreg_to_phys!(base),
                    exp: map_vreg_to_phys!(exp),
                }),
                VInstruction::Neg { src, .. } => instructions.push(Instruction::Neg {
                    dest: dest_phys,
                    src: map_vreg_to_phys!(src),
                }),
                VInstruction::BuiltinFun { op, args, .. } => match args.len() {
                    1 => instructions.push(Instruction::Builtin1 {
                        dest: dest_phys,
                        op,
                        arg: map_vreg_to_phys!(args[0]),
                    }),
                    2 => instructions.push(Instruction::Builtin2 {
                        dest: dest_phys,
                        op,
                        arg1: map_vreg_to_phys!(args[0]),
                        arg2: map_vreg_to_phys!(args[1]),
                    }),
                    _ => {
                        let start_idx = u32::try_from(self.arg_pool.len())
                            .expect("Arg pool too large for u32 index");
                        for &s in &args {
                            self.arg_pool.push(map_vreg_to_phys!(s));
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
                        arg: map_vreg_to_phys!(arg),
                    });
                }
                VInstruction::Builtin2 { op, arg1, arg2, .. } => {
                    instructions.push(Instruction::Builtin2 {
                        dest: dest_phys,
                        op,
                        arg1: map_vreg_to_phys!(arg1),
                        arg2: map_vreg_to_phys!(arg2),
                    });
                }
                VInstruction::Square { src, .. } => instructions.push(Instruction::Square {
                    dest: dest_phys,
                    src: map_vreg_to_phys!(src),
                }),
                VInstruction::Cube { src, .. } => instructions.push(Instruction::Cube {
                    dest: dest_phys,
                    src: map_vreg_to_phys!(src),
                }),
                VInstruction::Pow4 { src, .. } => instructions.push(Instruction::Pow4 {
                    dest: dest_phys,
                    src: map_vreg_to_phys!(src),
                }),
                VInstruction::Pow3_2 { src, .. } => instructions.push(Instruction::Pow3_2 {
                    dest: dest_phys,
                    src: map_vreg_to_phys!(src),
                }),
                VInstruction::InvPow3_2 { src, .. } => instructions.push(Instruction::InvPow3_2 {
                    dest: dest_phys,
                    src: map_vreg_to_phys!(src),
                }),
                VInstruction::InvSqrt { src, .. } => instructions.push(Instruction::InvSqrt {
                    dest: dest_phys,
                    src: map_vreg_to_phys!(src),
                }),
                VInstruction::InvSquare { src, .. } => instructions.push(Instruction::InvSquare {
                    dest: dest_phys,
                    src: map_vreg_to_phys!(src),
                }),
                VInstruction::InvCube { src, .. } => instructions.push(Instruction::InvCube {
                    dest: dest_phys,
                    src: map_vreg_to_phys!(src),
                }),
                VInstruction::Recip { src, .. } => instructions.push(Instruction::Recip {
                    dest: dest_phys,
                    src: map_vreg_to_phys!(src),
                }),
                VInstruction::Powi { src, n, .. } => instructions.push(Instruction::Powi {
                    dest: dest_phys,
                    src: map_vreg_to_phys!(src),
                    n,
                }),
                VInstruction::MulAdd { a, b, c, .. } => {
                    instructions.push(Instruction::MulAdd {
                        dest: dest_phys,
                        a: map_vreg_to_phys!(a),
                        b: map_vreg_to_phys!(b),
                        c: map_vreg_to_phys!(c),
                    });
                }
                VInstruction::MulSub { a, b, c, .. } => {
                    instructions.push(Instruction::MulSub {
                        dest: dest_phys,
                        a: map_vreg_to_phys!(a),
                        b: map_vreg_to_phys!(b),
                        c: map_vreg_to_phys!(c),
                    });
                }
                VInstruction::NegMulAdd { a, b, c, .. } => {
                    instructions.push(Instruction::NegMulAdd {
                        dest: dest_phys,
                        a: map_vreg_to_phys!(a),
                        b: map_vreg_to_phys!(b),
                        c: map_vreg_to_phys!(c),
                    });
                }

                VInstruction::RecipExpm1 { src, .. } => {
                    instructions.push(Instruction::RecipExpm1 {
                        dest: dest_phys,
                        src: map_vreg_to_phys!(src),
                    });
                }
                VInstruction::ExpSqr { src, .. } => instructions.push(Instruction::ExpSqr {
                    dest: dest_phys,
                    src: map_vreg_to_phys!(src),
                }),
                VInstruction::ExpSqrNeg { src, .. } => instructions.push(Instruction::ExpSqrNeg {
                    dest: dest_phys,
                    src: map_vreg_to_phys!(src),
                }),
            }

            let mut curr_death = self.death_heads[idx];
            while curr_death != u32::MAX {
                let t_id = curr_death;
                let p = temp_to_phys[t_id as usize];
                if p != u32::MAX {
                    free_phys.push(p);
                }
                curr_death = self.death_next[t_id as usize];
            }
        }

        let result_phys = if let Some(f_vreg) = final_vreg {
            match f_vreg {
                VReg::Param(p) => p,
                VReg::Const(c) => self.param_count + c,
                VReg::Temp(t) => temp_to_phys[t as usize],
            }
        } else {
            // No result produced (e.g. empty program), use default zero (first constant)
            // In the unified layout, constant 0 (0.0) is at param_count + 0.
            self.param_count
        };

        (instructions, self.arg_pool, max_phys as usize, result_phys)
    }

    fn emit_add2(
        &self,
        dest_phys: u32,
        a: VReg,
        b: VReg,
        t2p: &[u32],
        instrs: &mut Vec<Instruction>,
    ) {
        instrs.push(Instruction::Add {
            dest: dest_phys,
            a: map_vreg(a, self.param_count, t2p),
            b: map_vreg(b, self.param_count, t2p),
        });
    }

    fn emit_mul2(
        &self,
        dest_phys: u32,
        a: VReg,
        b: VReg,
        t2p: &[u32],
        instrs: &mut Vec<Instruction>,
    ) {
        instrs.push(Instruction::Mul {
            dest: dest_phys,
            a: map_vreg(a, self.param_count, t2p),
            b: map_vreg(b, self.param_count, t2p),
        });
    }
}
