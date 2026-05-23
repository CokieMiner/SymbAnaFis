use super::FnOp;
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
                VReg::Param(_) | VReg::Const(_) => {
                    if Some(src) == phys_0_vreg {
                        last_phys_0_read = Some(idx);
                    }
                }
            });
        }

        // Ensure the final result register is kept alive until the very end
        // so its physical register is not overwritten by intermediate computations.
        if let (Some(VReg::Temp(t)), Some(last_idx)) = (final_vreg, vinstrs.len().checked_sub(1)) {
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

            self.emit_instruction(instr, dest_phys, &temp_to_phys, &mut instructions);

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
            self.param_count
        };

        (instructions, self.arg_pool, max_phys as usize, result_phys)
    }

    fn map(&self, v: VReg, temp_to_phys: &[u32]) -> u32 {
        map_vreg(v, self.param_count, temp_to_phys)
    }

    fn emit_instruction(
        &mut self,
        instr: VInstruction,
        dest_phys: u32,
        temp_to_phys: &[u32],
        instructions: &mut Vec<Instruction>,
    ) {
        match instr {
            VInstruction::Add { srcs, .. } => {
                self.emit_add_nary(dest_phys, &srcs, temp_to_phys, instructions);
            }
            VInstruction::Add2 { a, b, .. } => {
                self.emit_add2(dest_phys, a, b, temp_to_phys, instructions);
            }
            VInstruction::Mul { srcs, .. } => {
                self.emit_mul_nary(dest_phys, &srcs, temp_to_phys, instructions);
            }
            VInstruction::Mul2 { a, b, .. } => {
                self.emit_mul2(dest_phys, a, b, temp_to_phys, instructions);
            }
            VInstruction::Sub { a, b, .. } => instructions.push(Instruction::Sub {
                dest: dest_phys,
                a: map_vreg(a, self.param_count, temp_to_phys),
                b: map_vreg(b, self.param_count, temp_to_phys),
            }),
            VInstruction::Div { num, den, .. } => instructions.push(Instruction::Div {
                dest: dest_phys,
                num: map_vreg(num, self.param_count, temp_to_phys),
                den: map_vreg(den, self.param_count, temp_to_phys),
            }),
            VInstruction::Pow { base, exp, .. } => instructions.push(Instruction::Pow {
                dest: dest_phys,
                base: map_vreg(base, self.param_count, temp_to_phys),
                exp: map_vreg(exp, self.param_count, temp_to_phys),
            }),
            VInstruction::Neg { src, .. } => instructions.push(Instruction::Neg {
                dest: dest_phys,
                src: map_vreg(src, self.param_count, temp_to_phys),
            }),
            VInstruction::NegMul { a, b, .. } => instructions.push(Instruction::NegMul {
                dest: dest_phys,
                a: map_vreg(a, self.param_count, temp_to_phys),
                b: map_vreg(b, self.param_count, temp_to_phys),
            }),
            VInstruction::BuiltinFun { op, args, .. } => {
                self.emit_builtin_fun(dest_phys, op, &args, temp_to_phys, instructions);
            }
            VInstruction::Builtin1 { op, arg, .. } => {
                self.emit_builtin_unary(dest_phys, op, arg, temp_to_phys, instructions);
            }
            VInstruction::Builtin2 { op, arg1, arg2, .. } => {
                instructions.push(Instruction::Builtin2 {
                    dest: dest_phys,
                    op,
                    arg1: map_vreg(arg1, self.param_count, temp_to_phys),
                    arg2: map_vreg(arg2, self.param_count, temp_to_phys),
                });
            }
            VInstruction::Square { .. }
            | VInstruction::Cube { .. }
            | VInstruction::Pow4 { .. }
            | VInstruction::Pow3_2 { .. }
            | VInstruction::InvPow3_2 { .. }
            | VInstruction::InvSqrt { .. }
            | VInstruction::InvSquare { .. }
            | VInstruction::InvCube { .. }
            | VInstruction::Recip { .. } => {
                self.emit_power_unary(&instr, dest_phys, temp_to_phys, instructions);
            }
            VInstruction::Powi { src, n, .. } => instructions.push(Instruction::Powi {
                dest: dest_phys,
                src: map_vreg(src, self.param_count, temp_to_phys),
                n,
            }),
            VInstruction::MulAdd { .. }
            | VInstruction::MulSub { .. }
            | VInstruction::NegMulAdd { .. }
            | VInstruction::NegMulSub { .. } => {
                self.emit_mul_add_family(&instr, dest_phys, temp_to_phys, instructions);
            }

            VInstruction::RecipExpm1 { src, .. } => {
                instructions.push(Instruction::RecipExpm1 {
                    dest: dest_phys,
                    src: map_vreg(src, self.param_count, temp_to_phys),
                });
            }
            VInstruction::ExpSqr { src, .. } => instructions.push(Instruction::ExpSqr {
                dest: dest_phys,
                src: map_vreg(src, self.param_count, temp_to_phys),
            }),
            VInstruction::ExpSqrNeg { src, .. } => instructions.push(Instruction::ExpSqrNeg {
                dest: dest_phys,
                src: map_vreg(src, self.param_count, temp_to_phys),
            }),
        }
    }

    fn emit_power_unary(
        &self,
        instr: &VInstruction,
        dest_phys: u32,
        temp_to_phys: &[u32],
        instructions: &mut Vec<Instruction>,
    ) {
        match instr {
            VInstruction::Square { src, .. } => instructions.push(Instruction::Square {
                dest: dest_phys,
                src: self.map(*src, temp_to_phys),
            }),
            VInstruction::Cube { src, .. } => instructions.push(Instruction::Cube {
                dest: dest_phys,
                src: self.map(*src, temp_to_phys),
            }),
            VInstruction::Pow4 { src, .. } => instructions.push(Instruction::Pow4 {
                dest: dest_phys,
                src: self.map(*src, temp_to_phys),
            }),
            VInstruction::Pow3_2 { src, .. } => instructions.push(Instruction::Pow3_2 {
                dest: dest_phys,
                src: self.map(*src, temp_to_phys),
            }),
            VInstruction::InvPow3_2 { src, .. } => instructions.push(Instruction::InvPow3_2 {
                dest: dest_phys,
                src: self.map(*src, temp_to_phys),
            }),
            VInstruction::InvSqrt { src, .. } => instructions.push(Instruction::InvSqrt {
                dest: dest_phys,
                src: self.map(*src, temp_to_phys),
            }),
            VInstruction::InvSquare { src, .. } => instructions.push(Instruction::InvSquare {
                dest: dest_phys,
                src: self.map(*src, temp_to_phys),
            }),
            VInstruction::InvCube { src, .. } => instructions.push(Instruction::InvCube {
                dest: dest_phys,
                src: self.map(*src, temp_to_phys),
            }),
            VInstruction::Recip { src, .. } => instructions.push(Instruction::Recip {
                dest: dest_phys,
                src: self.map(*src, temp_to_phys),
            }),
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
            | VInstruction::Powi { .. }
            | VInstruction::MulAdd { .. }
            | VInstruction::MulSub { .. }
            | VInstruction::NegMulAdd { .. }
            | VInstruction::NegMulSub { .. }
            | VInstruction::RecipExpm1 { .. }
            | VInstruction::ExpSqr { .. }
            | VInstruction::ExpSqrNeg { .. } => {}
        }
    }

    fn emit_mul_add_family(
        &self,
        instr: &VInstruction,
        dest_phys: u32,
        temp_to_phys: &[u32],
        instructions: &mut Vec<Instruction>,
    ) {
        match instr {
            VInstruction::MulAdd { a, b, c, .. } => {
                instructions.push(Instruction::MulAdd {
                    dest: dest_phys,
                    a: self.map(*a, temp_to_phys),
                    b: self.map(*b, temp_to_phys),
                    c: self.map(*c, temp_to_phys),
                });
            }
            VInstruction::MulSub { a, b, c, .. } => {
                instructions.push(Instruction::MulSub {
                    dest: dest_phys,
                    a: self.map(*a, temp_to_phys),
                    b: self.map(*b, temp_to_phys),
                    c: self.map(*c, temp_to_phys),
                });
            }
            VInstruction::NegMulAdd { a, b, c, .. } => {
                instructions.push(Instruction::NegMulAdd {
                    dest: dest_phys,
                    a: self.map(*a, temp_to_phys),
                    b: self.map(*b, temp_to_phys),
                    c: self.map(*c, temp_to_phys),
                });
            }
            VInstruction::NegMulSub { a, b, c, .. } => {
                instructions.push(Instruction::NegMulSub {
                    dest: dest_phys,
                    a: self.map(*a, temp_to_phys),
                    b: self.map(*b, temp_to_phys),
                    c: self.map(*c, temp_to_phys),
                });
            }
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
            | VInstruction::RecipExpm1 { .. }
            | VInstruction::ExpSqr { .. }
            | VInstruction::ExpSqrNeg { .. } => {}
        }
    }

    fn emit_add_nary(
        &mut self,
        dest_phys: u32,
        srcs: &[VReg],
        temp_to_phys: &[u32],
        instructions: &mut Vec<Instruction>,
    ) {
        if srcs.len() == 2 {
            self.emit_add2(dest_phys, srcs[0], srcs[1], temp_to_phys, instructions);
        } else if srcs.len() == 3 {
            instructions.push(Instruction::Add3 {
                dest: dest_phys,
                a: map_vreg(srcs[0], self.param_count, temp_to_phys),
                b: map_vreg(srcs[1], self.param_count, temp_to_phys),
                c: map_vreg(srcs[2], self.param_count, temp_to_phys),
            });
        } else if srcs.len() == 4 {
            instructions.push(Instruction::Add4 {
                dest: dest_phys,
                a: map_vreg(srcs[0], self.param_count, temp_to_phys),
                b: map_vreg(srcs[1], self.param_count, temp_to_phys),
                c: map_vreg(srcs[2], self.param_count, temp_to_phys),
                d: map_vreg(srcs[3], self.param_count, temp_to_phys),
            });
        } else {
            self.arg_pool.reserve(srcs.len());
            let start_idx =
                u32::try_from(self.arg_pool.len()).expect("Arg pool too large for u32 index");
            for &s in srcs {
                self.arg_pool
                    .push(map_vreg(s, self.param_count, temp_to_phys));
            }
            instructions.push(Instruction::AddN {
                dest: dest_phys,
                start_idx,
                count: u32::try_from(srcs.len()).expect("Too many sources for AddN"),
            });
        }
    }

    fn emit_mul_nary(
        &mut self,
        dest_phys: u32,
        srcs: &[VReg],
        temp_to_phys: &[u32],
        instructions: &mut Vec<Instruction>,
    ) {
        if srcs.len() == 2 {
            self.emit_mul2(dest_phys, srcs[0], srcs[1], temp_to_phys, instructions);
        } else if srcs.len() == 3 {
            instructions.push(Instruction::Mul3 {
                dest: dest_phys,
                a: map_vreg(srcs[0], self.param_count, temp_to_phys),
                b: map_vreg(srcs[1], self.param_count, temp_to_phys),
                c: map_vreg(srcs[2], self.param_count, temp_to_phys),
            });
        } else if srcs.len() == 4 {
            instructions.push(Instruction::Mul4 {
                dest: dest_phys,
                a: map_vreg(srcs[0], self.param_count, temp_to_phys),
                b: map_vreg(srcs[1], self.param_count, temp_to_phys),
                c: map_vreg(srcs[2], self.param_count, temp_to_phys),
                d: map_vreg(srcs[3], self.param_count, temp_to_phys),
            });
        } else {
            self.arg_pool.reserve(srcs.len());
            let start_idx =
                u32::try_from(self.arg_pool.len()).expect("Arg pool too large for u32 index");
            for &s in srcs {
                self.arg_pool
                    .push(map_vreg(s, self.param_count, temp_to_phys));
            }
            instructions.push(Instruction::MulN {
                dest: dest_phys,
                start_idx,
                count: u32::try_from(srcs.len()).expect("Too many sources for MulN"),
            });
        }
    }

    fn emit_builtin_fun(
        &self,
        dest_phys: u32,
        op: FnOp,
        args: &[VReg],
        temp_to_phys: &[u32],
        instructions: &mut Vec<Instruction>,
    ) {
        match args.len() {
            1 => self.emit_builtin_unary(dest_phys, op, args[0], temp_to_phys, instructions),
            2 => instructions.push(Instruction::Builtin2 {
                dest: dest_phys,
                op,
                arg1: map_vreg(args[0], self.param_count, temp_to_phys),
                arg2: map_vreg(args[1], self.param_count, temp_to_phys),
            }),
            3 => instructions.push(Instruction::Builtin3 {
                dest: dest_phys,
                op,
                arg1: map_vreg(args[0], self.param_count, temp_to_phys),
                arg2: map_vreg(args[1], self.param_count, temp_to_phys),
                arg3: map_vreg(args[2], self.param_count, temp_to_phys),
            }),
            4 => instructions.push(Instruction::Builtin4 {
                dest: dest_phys,
                op,
                arg1: map_vreg(args[0], self.param_count, temp_to_phys),
                arg2: map_vreg(args[1], self.param_count, temp_to_phys),
                arg3: map_vreg(args[2], self.param_count, temp_to_phys),
                arg4: map_vreg(args[3], self.param_count, temp_to_phys),
            }),
            _ => {}
        }
    }

    fn emit_builtin_unary(
        &self,
        dest_phys: u32,
        op: FnOp,
        arg: VReg,
        temp_to_phys: &[u32],
        instructions: &mut Vec<Instruction>,
    ) {
        match op {
            FnOp::Sin => instructions.push(Instruction::Sin {
                dest: dest_phys,
                arg: map_vreg(arg, self.param_count, temp_to_phys),
            }),
            FnOp::Cos => instructions.push(Instruction::Cos {
                dest: dest_phys,
                arg: map_vreg(arg, self.param_count, temp_to_phys),
            }),
            FnOp::Exp => instructions.push(Instruction::Exp {
                dest: dest_phys,
                arg: map_vreg(arg, self.param_count, temp_to_phys),
            }),
            FnOp::Ln => instructions.push(Instruction::Ln {
                dest: dest_phys,
                arg: map_vreg(arg, self.param_count, temp_to_phys),
            }),
            FnOp::Sqrt => instructions.push(Instruction::Sqrt {
                dest: dest_phys,
                arg: map_vreg(arg, self.param_count, temp_to_phys),
            }),
            FnOp::Tan
            | FnOp::Cot
            | FnOp::Sec
            | FnOp::Csc
            | FnOp::Asin
            | FnOp::Acos
            | FnOp::Atan
            | FnOp::Acot
            | FnOp::Asec
            | FnOp::Acsc
            | FnOp::Sinh
            | FnOp::Cosh
            | FnOp::Tanh
            | FnOp::Coth
            | FnOp::Sech
            | FnOp::Csch
            | FnOp::Asinh
            | FnOp::Acosh
            | FnOp::Atanh
            | FnOp::Acoth
            | FnOp::Acsch
            | FnOp::Asech
            | FnOp::Expm1
            | FnOp::ExpNeg
            | FnOp::Log1p
            | FnOp::Cbrt
            | FnOp::Abs
            | FnOp::Signum
            | FnOp::Floor
            | FnOp::Ceil
            | FnOp::Round
            | FnOp::Erf
            | FnOp::Erfc
            | FnOp::Gamma
            | FnOp::Lgamma
            | FnOp::Digamma
            | FnOp::Trigamma
            | FnOp::Tetragamma
            | FnOp::Sinc
            | FnOp::LambertW
            | FnOp::EllipticK
            | FnOp::EllipticE
            | FnOp::Zeta
            | FnOp::ExpPolar
            | FnOp::Atan2
            | FnOp::Log
            | FnOp::BesselJ
            | FnOp::BesselY
            | FnOp::BesselI
            | FnOp::BesselK
            | FnOp::Polygamma
            | FnOp::Beta
            | FnOp::ZetaDeriv
            | FnOp::Hermite
            | FnOp::AssocLegendre
            | FnOp::SphericalHarmonic => {
                instructions.push(Instruction::Builtin1 {
                    dest: dest_phys,
                    op,
                    arg: map_vreg(arg, self.param_count, temp_to_phys),
                });
            }
        }
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
