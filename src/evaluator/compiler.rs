//! Expression compiler for the bytecode evaluator.
//!
//! This module compiles symbolic [`Expr`] expressions into efficient bytecode
//! ([`Instruction`]s) that can be executed by the [`CompiledEvaluator`].

use super::instruction::{FnOp, Instruction};
use crate::core::error::DiffError;
use crate::core::known_symbols::KS;
use crate::core::poly::Polynomial;
use crate::core::symbol::InternedSymbol;
use crate::core::traits::EPSILON;
use crate::{Expr, ExprKind};
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;
use std::sync::{Arc, LazyLock};

static FN_MAP: LazyLock<FxHashMap<u64, FnOp>> = LazyLock::new(|| {
    let mut m = FxHashMap::default();
    let ks = &KS;
    // Arity 1
    m.insert(ks.sin, FnOp::Sin);
    m.insert(ks.cos, FnOp::Cos);
    m.insert(ks.tan, FnOp::Tan);
    m.insert(ks.cot, FnOp::Cot);
    m.insert(ks.sec, FnOp::Sec);
    m.insert(ks.csc, FnOp::Csc);
    m.insert(ks.asin, FnOp::Asin);
    m.insert(ks.acos, FnOp::Acos);
    m.insert(ks.atan, FnOp::Atan);
    m.insert(ks.acot, FnOp::Acot);
    m.insert(ks.asec, FnOp::Asec);
    m.insert(ks.acsc, FnOp::Acsc);
    m.insert(ks.sinh, FnOp::Sinh);
    m.insert(ks.cosh, FnOp::Cosh);
    m.insert(ks.tanh, FnOp::Tanh);
    m.insert(ks.coth, FnOp::Coth);
    m.insert(ks.sech, FnOp::Sech);
    m.insert(ks.csch, FnOp::Csch);
    m.insert(ks.asinh, FnOp::Asinh);
    m.insert(ks.acosh, FnOp::Acosh);
    m.insert(ks.atanh, FnOp::Atanh);
    m.insert(ks.acoth, FnOp::Acoth);
    m.insert(ks.acsch, FnOp::Acsch);
    m.insert(ks.asech, FnOp::Asech);
    m.insert(ks.exp, FnOp::Exp);
    m.insert(ks.ln, FnOp::Ln);
    m.insert(ks.sqrt, FnOp::Sqrt);
    m.insert(ks.cbrt, FnOp::Cbrt);
    m.insert(ks.abs, FnOp::Abs);
    m.insert(ks.signum, FnOp::Signum);
    m.insert(ks.sign, FnOp::Signum);
    m.insert(ks.sgn, FnOp::Signum);
    m.insert(ks.floor, FnOp::Floor);
    m.insert(ks.ceil, FnOp::Ceil);
    m.insert(ks.round, FnOp::Round);
    m.insert(ks.erf, FnOp::Erf);
    m.insert(ks.erfc, FnOp::Erfc);
    m.insert(ks.gamma, FnOp::Gamma);
    m.insert(ks.digamma, FnOp::Digamma);
    m.insert(ks.trigamma, FnOp::Trigamma);
    m.insert(ks.tetragamma, FnOp::Tetragamma);
    m.insert(ks.sinc, FnOp::Sinc);
    m.insert(ks.lambertw, FnOp::LambertW);
    m.insert(ks.elliptic_k, FnOp::EllipticK);
    m.insert(ks.elliptic_e, FnOp::EllipticE);
    m.insert(ks.zeta, FnOp::Zeta);
    m.insert(ks.exp_polar, FnOp::ExpPolar);

    // Arity 2
    m.insert(ks.atan2, FnOp::Atan2);
    m.insert(ks.log, FnOp::Log);
    m.insert(ks.besselj, FnOp::BesselJ);
    m.insert(ks.bessely, FnOp::BesselY);
    m.insert(ks.besseli, FnOp::BesselI);
    m.insert(ks.besselk, FnOp::BesselK);
    m.insert(ks.polygamma, FnOp::Polygamma);
    m.insert(ks.beta, FnOp::Beta);
    m.insert(ks.zeta_deriv, FnOp::ZetaDeriv);
    m.insert(ks.hermite, FnOp::Hermite);

    // Arity 3
    m.insert(ks.assoc_legendre, FnOp::AssocLegendre);

    // Arity 4
    m.insert(ks.spherical_harmonic, FnOp::SphericalHarmonic);
    m.insert(ks.ynm, FnOp::SphericalHarmonic);

    m
});

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VReg {
    Param(u32),
    Const(u32),
    Temp(u32),
}

#[derive(Clone, Debug)]
enum VInstruction {
    Add {
        dest: VReg,
        srcs: Vec<VReg>,
    },
    Add2 {
        dest: VReg,
        a: VReg,
        b: VReg,
    },
    Mul {
        dest: VReg,
        srcs: Vec<VReg>,
    },
    Mul2 {
        dest: VReg,
        a: VReg,
        b: VReg,
    },
    Sub {
        dest: VReg,
        a: VReg,
        b: VReg,
    },
    Div {
        dest: VReg,
        num: VReg,
        den: VReg,
    },
    Pow {
        dest: VReg,
        base: VReg,
        exp: VReg,
    },
    Neg {
        dest: VReg,
        src: VReg,
    },
    BuiltinFun {
        dest: VReg,
        op: FnOp,
        args: Vec<VReg>,
    },
    Builtin1 {
        dest: VReg,
        op: FnOp,
        arg: VReg,
    },
    Builtin2 {
        dest: VReg,
        op: FnOp,
        arg1: VReg,
        arg2: VReg,
    },
    Square {
        dest: VReg,
        src: VReg,
    },
    Cube {
        dest: VReg,
        src: VReg,
    },
    Pow4 {
        dest: VReg,
        src: VReg,
    },
    Pow3_2 {
        dest: VReg,
        src: VReg,
    },
    InvPow3_2 {
        dest: VReg,
        src: VReg,
    },
    InvSqrt {
        dest: VReg,
        src: VReg,
    },
    InvSquare {
        dest: VReg,
        src: VReg,
    },
    InvCube {
        dest: VReg,
        src: VReg,
    },
    Recip {
        dest: VReg,
        src: VReg,
    },
    Powi {
        dest: VReg,
        src: VReg,
        n: i32,
    },
    MulAdd {
        dest: VReg,
        a: VReg,
        b: VReg,
        c: VReg,
    },
    MulSub {
        dest: VReg,
        a: VReg,
        b: VReg,
        c: VReg,
    },
    NegMulAdd {
        dest: VReg,
        a: VReg,
        b: VReg,
        c: VReg,
    },
    PolyEval {
        dest: VReg,
        x: VReg,
        const_idx: u32,
    },
    RecipExpm1 {
        dest: VReg,
        src: VReg,
    },
    ExpSqr {
        dest: VReg,
        src: VReg,
    },
    ExpSqrNeg {
        dest: VReg,
        src: VReg,
    },
}

struct NodeData {
    vreg: Option<VReg>,
    const_val: Option<f64>,
    is_expensive: bool,
}

impl VInstruction {
    const fn dest(&self) -> VReg {
        match self {
            Self::Add { dest, .. }
            | Self::Add2 { dest, .. }
            | Self::Mul { dest, .. }
            | Self::Mul2 { dest, .. }
            | Self::Sub { dest, .. }
            | Self::Div { dest, .. }
            | Self::Pow { dest, .. }
            | Self::Neg { dest, .. }
            | Self::BuiltinFun { dest, .. }
            | Self::Builtin1 { dest, .. }
            | Self::Builtin2 { dest, .. }
            | Self::Square { dest, .. }
            | Self::Cube { dest, .. }
            | Self::Pow4 { dest, .. }
            | Self::Pow3_2 { dest, .. }
            | Self::InvPow3_2 { dest, .. }
            | Self::InvSqrt { dest, .. }
            | Self::InvSquare { dest, .. }
            | Self::InvCube { dest, .. }
            | Self::Recip { dest, .. }
            | Self::Powi { dest, .. }
            | Self::MulAdd { dest, .. }
            | Self::MulSub { dest, .. }
            | Self::NegMulAdd { dest, .. }
            | Self::PolyEval { dest, .. }
            | Self::RecipExpm1 { dest, .. }
            | Self::ExpSqr { dest, .. }
            | Self::ExpSqrNeg { dest, .. } => *dest,
        }
    }

    fn for_each_read(&self, mut f: impl FnMut(VReg)) {
        match self {
            Self::Add { srcs, .. } | Self::Mul { srcs, .. } => {
                for &s in srcs {
                    f(s);
                }
            }
            Self::Add2 { a, b, .. }
            | Self::Mul2 { a, b, .. }
            | Self::Sub { a, b, .. }
            | Self::Div { num: a, den: b, .. }
            | Self::Pow {
                base: a, exp: b, ..
            } => {
                f(*a);
                f(*b);
            }
            Self::BuiltinFun { args, .. } => {
                for &a in args {
                    f(a);
                }
            }
            Self::Builtin1 { arg, .. } => f(*arg),
            Self::Builtin2 { arg1, arg2, .. } => {
                f(*arg1);
                f(*arg2);
            }
            Self::MulAdd { a, b, c, .. }
            | Self::MulSub { a, b, c, .. }
            | Self::NegMulAdd { a, b, c, .. } => {
                f(*a);
                f(*b);
                f(*c);
            }
            Self::PolyEval { x, .. } => f(*x),
            Self::Neg { src, .. }
            | Self::Square { src, .. }
            | Self::Cube { src, .. }
            | Self::Pow4 { src, .. }
            | Self::Pow3_2 { src, .. }
            | Self::InvPow3_2 { src, .. }
            | Self::InvSqrt { src, .. }
            | Self::InvSquare { src, .. }
            | Self::InvCube { src, .. }
            | Self::Recip { src, .. }
            | Self::Powi { src, .. }
            | Self::RecipExpm1 { src, .. }
            | Self::ExpSqr { src, .. }
            | Self::ExpSqrNeg { src, .. } => f(*src),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct CseKey(*const Expr);

impl PartialEq for CseKey {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
            || (
                // SAFETY: CseKey pointers always point to valid nodes in the current expression tree
                unsafe { &*self.0 }
                    ==
                // SAFETY: CseKey pointers always point to valid nodes in the current expression tree
                unsafe { &*other.0 }
            )
    }
}

impl Eq for CseKey {}

impl std::hash::Hash for CseKey {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // SAFETY: CseKey pointers always point to valid nodes in the current expression tree
        unsafe {
            (*self.0).hash.hash(state);
        }
    }
}

pub struct Compiler {
    vinstrs: Vec<VInstruction>,
    param_ids: Vec<u64>,
    param_index: FxHashMap<u64, usize>,
    cse_cache: FxHashMap<CseKey, VReg>,
    constants: Vec<f64>,
    const_map: FxHashMap<u64, u32>,
    /// Pool for N-ary instruction register indices
    arg_pool: Vec<u32>,
    next_vreg: u32,
    final_vreg: Option<VReg>,
    used_params: Vec<bool>,
    used_constants: Vec<bool>,
}

impl Compiler {
    pub fn new(param_ids: &[u64]) -> Self {
        let param_index = param_ids
            .iter()
            .enumerate()
            .map(|(idx, &id)| (id, idx))
            .collect();
        let mut compiler = Self {
            vinstrs: Vec::with_capacity(64),
            param_ids: param_ids.to_vec(),
            param_index,
            cse_cache: FxHashMap::default(),
            constants: Vec::new(),
            const_map: FxHashMap::default(),
            arg_pool: Vec::with_capacity(128),
            next_vreg: 0,
            final_vreg: None,
            used_params: vec![false; param_ids.len()],
            used_constants: Vec::new(),
        };
        // Pre-add 0.0 so it's always available (e.g. for empty expressions)
        compiler.add_const(0.0);
        compiler
    }

    #[inline]
    const fn alloc_vreg(&mut self) -> VReg {
        let r = self.next_vreg;
        self.next_vreg += 1;
        VReg::Temp(r)
    }

    #[inline]
    pub(crate) fn add_const(&mut self, val: f64) -> u32 {
        let bits = val.to_bits();
        match self.const_map.entry(bits) {
            Entry::Occupied(o) => {
                let idx = *o.get();
                self.used_constants[idx as usize] = true;
                idx
            }
            Entry::Vacant(v) => {
                let idx = u32::try_from(self.constants.len()).unwrap_or(u32::MAX);
                self.constants.push(val);
                self.used_constants.push(true);
                v.insert(idx);
                idx
            }
        }
    }

    #[inline]
    fn emit(&mut self, instr: VInstruction) {
        self.vinstrs.push(instr);
    }

    #[allow(
        clippy::too_many_lines,
        reason = "Main compiler pass which needs to handle all instruction variants inline"
    )]
    pub(crate) fn into_parts(mut self) -> (Vec<Instruction>, Vec<f64>, Vec<u32>, usize, usize) {
        let num_temps = self.next_vreg as usize;
        let n_instrs = self.vinstrs.len();

        // Forward pass: record the last instruction index where each temp is READ
        let mut last_use: Vec<Option<usize>> = vec![None; num_temps];
        for (idx, instr) in self.vinstrs.iter().enumerate() {
            instr.for_each_read(|src| {
                if let VReg::Temp(t) = src {
                    last_use[t as usize] = Some(idx);
                }
            });
        }

        // deaths: sorted list of (last_use_idx, temp_id) pairs
        #[allow(
            clippy::cast_possible_truncation,
            reason = "Temp index comes from u32 so it will fit"
        )]
        let mut deaths: Vec<(usize, u32)> = last_use
            .iter()
            .enumerate()
            .filter_map(|(t, lu_opt)| lu_opt.map(|lu| (lu, t as u32)))
            .collect();
        deaths.sort_unstable_by_key(|&(idx, _)| idx);
        let mut death_cursor = 0;

        let param_count = u32::try_from(self.param_ids.len()).unwrap_or(u32::MAX);
        let const_count = u32::try_from(self.constants.len()).unwrap_or(u32::MAX);
        let mut max_phys = param_count + const_count;

        // Vec-indexed maps — no HashMap overhead in the hot loop
        let mut temp_to_phys: Vec<u32> = vec![u32::MAX; num_temps];
        let mut free_phys: Vec<u32> = Vec::new();

        let mut instructions = Vec::with_capacity(n_instrs);
        let vinstrs = std::mem::take(&mut self.vinstrs);

        for (idx, instr) in vinstrs.into_iter().enumerate() {
            let map_vreg_phys = |vreg: VReg, t2p: &[u32]| -> u32 {
                match vreg {
                    VReg::Param(p) => p,
                    VReg::Const(c) => param_count + c,
                    VReg::Temp(t) => t2p[t as usize],
                }
            };

            let dest_vreg = instr.dest();
            let dest_phys = match dest_vreg {
                VReg::Param(p) => p,
                VReg::Const(c) => param_count + c,
                VReg::Temp(t) => {
                    let p = free_phys.pop().unwrap_or_else(|| {
                        let p = max_phys;
                        max_phys += 1;
                        p
                    });
                    temp_to_phys[t as usize] = p;
                    // Temp that is never read: free immediately so next instr can reuse it
                    if last_use[t as usize].is_none() {
                        free_phys.push(p);
                    }
                    p
                }
            };

            let map_vreg_local = |vreg: VReg| map_vreg_phys(vreg, &temp_to_phys);

            match instr {
                VInstruction::Add { srcs, .. } => {
                    debug_assert!(!srcs.is_empty(), "Empty Add sources in into_parts");
                    match srcs.len() {
                        1 => {
                            instructions.push(Instruction::Copy {
                                dest: dest_phys,
                                src: map_vreg_local(srcs[0]),
                            });
                        }
                        2 => {
                            instructions.push(Instruction::Add {
                                dest: dest_phys,
                                a: map_vreg_local(srcs[0]),
                                b: map_vreg_local(srcs[1]),
                            });
                        }
                        _ => {
                            let start_idx = u32::try_from(self.arg_pool.len()).unwrap_or(u32::MAX);
                            for &s in &srcs {
                                self.arg_pool.push(map_vreg_local(s));
                            }
                            instructions.push(Instruction::AddN {
                                dest: dest_phys,
                                start_idx,
                                count: u32::try_from(srcs.len()).unwrap_or(u32::MAX),
                            });
                        }
                    }
                }
                VInstruction::Add2 { a, b, .. } => {
                    instructions.push(Instruction::Add {
                        dest: dest_phys,
                        a: map_vreg_local(a),
                        b: map_vreg_local(b),
                    });
                }
                VInstruction::Mul { srcs, .. } => {
                    debug_assert!(!srcs.is_empty(), "Empty Mul sources in into_parts");
                    match srcs.len() {
                        1 => {
                            instructions.push(Instruction::Copy {
                                dest: dest_phys,
                                src: map_vreg_local(srcs[0]),
                            });
                        }
                        2 => {
                            instructions.push(Instruction::Mul {
                                dest: dest_phys,
                                a: map_vreg_local(srcs[0]),
                                b: map_vreg_local(srcs[1]),
                            });
                        }
                        _ => {
                            let start_idx = u32::try_from(self.arg_pool.len()).unwrap_or(u32::MAX);
                            for &s in &srcs {
                                self.arg_pool.push(map_vreg_local(s));
                            }
                            instructions.push(Instruction::MulN {
                                dest: dest_phys,
                                start_idx,
                                count: u32::try_from(srcs.len()).unwrap_or(u32::MAX),
                            });
                        }
                    }
                }
                VInstruction::Mul2 { a, b, .. } => {
                    instructions.push(Instruction::Mul {
                        dest: dest_phys,
                        a: map_vreg_local(a),
                        b: map_vreg_local(b),
                    });
                }
                VInstruction::Sub { a, b, .. } => {
                    instructions.push(Instruction::Sub {
                        dest: dest_phys,
                        a: map_vreg_local(a),
                        b: map_vreg_local(b),
                    });
                }
                VInstruction::Div { num, den, .. } => {
                    instructions.push(Instruction::Div {
                        dest: dest_phys,
                        num: map_vreg_local(num),
                        den: map_vreg_local(den),
                    });
                }
                VInstruction::Pow { base, exp, .. } => {
                    instructions.push(Instruction::Pow {
                        dest: dest_phys,
                        base: map_vreg_local(base),
                        exp: map_vreg_local(exp),
                    });
                }
                VInstruction::Neg { src, .. } => {
                    instructions.push(Instruction::Neg {
                        dest: dest_phys,
                        src: map_vreg_local(src),
                    });
                }
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
                    3 => {
                        let start_idx = u32::try_from(self.arg_pool.len()).unwrap_or(u32::MAX);
                        for &s in &args {
                            self.arg_pool.push(map_vreg_local(s));
                        }
                        instructions.push(Instruction::Builtin3 {
                            dest: dest_phys,
                            op,
                            start_idx,
                        });
                    }
                    4 => {
                        let start_idx = u32::try_from(self.arg_pool.len()).unwrap_or(u32::MAX);
                        for &s in &args {
                            self.arg_pool.push(map_vreg_local(s));
                        }
                        instructions.push(Instruction::Builtin4 {
                            dest: dest_phys,
                            op,
                            start_idx,
                        });
                    }
                    _ => {
                        debug_assert!(
                            false,
                            "Unsupported arity {} for function {op:?}",
                            args.len()
                        );
                        instructions.push(Instruction::LoadConst {
                            dest: dest_phys,
                            const_idx: 0,
                        });
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
                VInstruction::Square { src, .. } => {
                    instructions.push(Instruction::Square {
                        dest: dest_phys,
                        src: map_vreg_local(src),
                    });
                }
                VInstruction::Cube { src, .. } => {
                    instructions.push(Instruction::Cube {
                        dest: dest_phys,
                        src: map_vreg_local(src),
                    });
                }
                VInstruction::Pow4 { src, .. } => {
                    instructions.push(Instruction::Pow4 {
                        dest: dest_phys,
                        src: map_vreg_local(src),
                    });
                }
                VInstruction::Pow3_2 { src, .. } => {
                    instructions.push(Instruction::Pow3_2 {
                        dest: dest_phys,
                        src: map_vreg_local(src),
                    });
                }
                VInstruction::InvPow3_2 { src, .. } => {
                    instructions.push(Instruction::InvPow3_2 {
                        dest: dest_phys,
                        src: map_vreg_local(src),
                    });
                }
                VInstruction::InvSqrt { src, .. } => {
                    instructions.push(Instruction::InvSqrt {
                        dest: dest_phys,
                        src: map_vreg_local(src),
                    });
                }
                VInstruction::InvSquare { src, .. } => {
                    instructions.push(Instruction::InvSquare {
                        dest: dest_phys,
                        src: map_vreg_local(src),
                    });
                }
                VInstruction::InvCube { src, .. } => {
                    instructions.push(Instruction::InvCube {
                        dest: dest_phys,
                        src: map_vreg_local(src),
                    });
                }
                VInstruction::Recip { src, .. } => {
                    instructions.push(Instruction::Recip {
                        dest: dest_phys,
                        src: map_vreg_local(src),
                    });
                }
                VInstruction::Powi { src, n, .. } => {
                    instructions.push(Instruction::Powi {
                        dest: dest_phys,
                        src: map_vreg_local(src),
                        n,
                    });
                }
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
                VInstruction::PolyEval { x, const_idx, .. } => {
                    instructions.push(Instruction::PolyEval {
                        dest: dest_phys,
                        x: map_vreg_local(x),
                        const_idx,
                    });
                }
                VInstruction::RecipExpm1 { src, .. } => {
                    instructions.push(Instruction::RecipExpm1 {
                        dest: dest_phys,
                        src: map_vreg_local(src),
                    });
                }
                VInstruction::ExpSqr { src, .. } => {
                    instructions.push(Instruction::ExpSqr {
                        dest: dest_phys,
                        src: map_vreg_local(src),
                    });
                }
                VInstruction::ExpSqrNeg { src, .. } => {
                    instructions.push(Instruction::ExpSqrNeg {
                        dest: dest_phys,
                        src: map_vreg_local(src),
                    });
                }
            }

            // Free dead temps whose last use was THIS instruction
            while death_cursor < deaths.len() && deaths[death_cursor].0 == idx {
                let t_id = deaths[death_cursor].1;
                let p = temp_to_phys[t_id as usize];
                if p != u32::MAX {
                    free_phys.push(p);
                }
                death_cursor += 1;
            }
        }

        if let Some(f_vreg) = self.final_vreg {
            let src_phys = match f_vreg {
                VReg::Param(p) => p,
                VReg::Const(c) => param_count + c,
                VReg::Temp(t) => temp_to_phys[t as usize],
            };
            if src_phys != 0 {
                instructions.push(Instruction::Copy {
                    dest: 0,
                    src: src_phys,
                });
            }
        } else {
            // Empty expression? Emit a load const 0.0 to 0
            // Index 0 is guaranteed to be 0.0 because of Compiler::new
            instructions.push(Instruction::LoadConst {
                dest: 0,
                const_idx: 0,
            });
        }

        let register_count = max_phys as usize;

        // return tuple: (instructions, constants, arg_pool, stack_size(now register_count), param_count)
        (
            instructions,
            self.constants,
            self.arg_pool,
            register_count,
            param_count as usize,
        )
    }

    fn compute_expensive_from_children(
        expr: &Expr,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> bool {
        match &expr.kind {
            ExprKind::FunctionCall { .. } | ExprKind::Div(..) | ExprKind::Poly(_) => true,
            ExprKind::Pow(base, exp) => {
                if Self::const_from_map(node_map, exp.as_ref())
                    .is_none_or(|n| (n - n.round()).abs() > EPSILON)
                {
                    true
                } else {
                    !matches!(base.kind, ExprKind::Number(_))
                }
            }
            ExprKind::Sum(terms) | ExprKind::Product(terms) => match terms.len().cmp(&2) {
                std::cmp::Ordering::Greater => true,
                std::cmp::Ordering::Equal => {
                    // Only consider it expensive if at least one term is not a simple terminal
                    // or is already marked as expensive.
                    terms.iter().any(|t| {
                        if node_map
                            .get(&Arc::as_ptr(t))
                            .is_some_and(|data| data.is_expensive)
                        {
                            return true;
                        }
                        !matches!(t.kind, ExprKind::Number(_) | ExprKind::Symbol(_))
                    })
                }
                std::cmp::Ordering::Less => terms.iter().any(|t| {
                    node_map
                        .get(&Arc::as_ptr(t))
                        .is_some_and(|data| data.is_expensive)
                }),
            },
            _ => false,
        }
    }

    pub(crate) fn compile_expr(&mut self, expr: &Expr) -> Result<VReg, DiffError> {
        let node_count = expr.node_count();
        self.vinstrs.reserve(node_count);
        #[allow(
            clippy::integer_division,
            reason = "Heuristic sizing for reserve; integer division is intentional"
        )]
        let const_reserve = node_count / 8 + 8;
        self.constants.reserve(const_reserve);
        self.const_map.reserve(const_reserve);
        #[allow(
            clippy::integer_division,
            reason = "Heuristic sizing for reserve; integer division is intentional"
        )]
        self.cse_cache.reserve(node_count / 8);
        let vreg = self.compile_expr_iterative(expr, node_count)?;
        self.final_vreg = Some(vreg);
        Ok(vreg)
    }

    fn const_from_map(node_map: &FxHashMap<*const Expr, NodeData>, expr: &Expr) -> Option<f64> {
        node_map
            .get(&std::ptr::from_ref(expr))
            .and_then(|data| data.const_val)
    }

    fn vreg_from_map(
        node_map: &FxHashMap<*const Expr, NodeData>,
        expr: &Expr,
    ) -> Result<VReg, DiffError> {
        node_map
            .get(&std::ptr::from_ref(expr))
            .and_then(|data| data.vreg)
            .ok_or_else(|| {
                DiffError::UnsupportedExpression(
                    "Internal compile error: missing child vreg".to_owned(),
                )
            })
    }

    fn compute_const_from_children(
        expr: &Expr,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Option<f64> {
        match &expr.kind {
            ExprKind::Number(n) => Some(*n),
            ExprKind::Symbol(s) => crate::core::known_symbols::get_constant_value_by_id(s.id()),
            ExprKind::Sum(terms) => {
                let mut sum = 0.0;
                for t in terms {
                    sum += Self::const_from_map(node_map, t.as_ref())?;
                }
                Some(sum)
            }
            ExprKind::Product(factors) => {
                let mut product = 1.0;
                for f in factors {
                    product *= Self::const_from_map(node_map, f.as_ref())?;
                }
                Some(product)
            }
            ExprKind::Div(num, den) => Some(
                Self::const_from_map(node_map, num.as_ref())?
                    / Self::const_from_map(node_map, den.as_ref())?,
            ),
            ExprKind::Pow(base, exp) => Some(
                Self::const_from_map(node_map, base.as_ref())?
                    .powf(Self::const_from_map(node_map, exp.as_ref())?),
            ),
            ExprKind::FunctionCall { name, args } => match args.len() {
                1 => {
                    let x = Self::const_from_map(node_map, args[0].as_ref())?;
                    let id = name.id();
                    let ks = &*KS;
                    if id == ks.sin {
                        Some(x.sin())
                    } else if id == ks.cos {
                        Some(x.cos())
                    } else if id == ks.tan {
                        Some(x.tan())
                    } else if id == ks.exp {
                        Some(x.exp())
                    } else if id == ks.ln || id == ks.log {
                        Some(x.ln())
                    } else if id == ks.sqrt {
                        Some(x.sqrt())
                    } else if id == ks.abs {
                        Some(x.abs())
                    } else if id == ks.floor {
                        Some(x.floor())
                    } else if id == ks.ceil {
                        Some(x.ceil())
                    } else if id == ks.round {
                        Some(x.round())
                    } else {
                        None
                    }
                }
                2 => {
                    let a = Self::const_from_map(node_map, args[0].as_ref())?;
                    let b = Self::const_from_map(node_map, args[1].as_ref())?;
                    let id = name.id();
                    let ks = &*KS;
                    if id == ks.atan2 {
                        Some(a.atan2(b))
                    } else if id == ks.log {
                        Some(b.log(a))
                    } else {
                        None
                    }
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn negated_inner_vreg(
        &mut self,
        term: &Expr,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Option<VReg> {
        if let ExprKind::Product(factors) = &term.kind {
            let neg_idx = factors.iter().position(|f| {
                Self::const_from_map(node_map, f.as_ref())
                    .is_some_and(|n| (n + 1.0).abs() < EPSILON)
            })?;
            // Determine how many actual factors we have
            let num_inner = factors.len().saturating_sub(1);
            if num_inner == 0 {
                let idx = self.add_const(1.0);
                return Some(VReg::Const(idx));
            }
            if num_inner == 1 {
                for (i, f) in factors.iter().enumerate() {
                    if i != neg_idx {
                        return node_map.get(&Arc::as_ptr(f))?.vreg;
                    }
                }
            }
            if num_inner == 2 {
                let mut iter = factors.iter().enumerate().filter(|(i, _)| *i != neg_idx);
                let (_, f1) = iter.next()?;
                let (_, f2) = iter.next()?;
                let a = node_map.get(&Arc::as_ptr(f1))?.vreg?;
                let b = node_map.get(&Arc::as_ptr(f2))?.vreg?;
                let d = self.alloc_vreg();
                self.emit(VInstruction::Mul2 { dest: d, a, b });
                return Some(d);
            }
            let mut inner_vregs: Vec<VReg> = Vec::with_capacity(num_inner);
            for (i, f) in factors.iter().enumerate() {
                if i != neg_idx {
                    inner_vregs.push(node_map.get(&Arc::as_ptr(f))?.vreg?);
                }
            }
            return Some(match inner_vregs.len() {
                0 => {
                    let idx = self.add_const(1.0);
                    VReg::Const(idx)
                }
                1 => inner_vregs[0],
                _ => {
                    let d = self.alloc_vreg();
                    self.emit(VInstruction::Mul {
                        dest: d,
                        srcs: inner_vregs,
                    });
                    d
                }
            });
        }
        None
    }

    fn product_two_vregs(
        term: &Expr,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Option<(VReg, VReg)> {
        if let ExprKind::Product(factors) = &term.kind
            && factors.len() == 2
        {
            if factors.iter().any(|f| {
                Self::const_from_map(node_map, f.as_ref())
                    .is_some_and(|n| (n + 1.0).abs() < EPSILON)
            }) {
                return None;
            }
            let a = Self::vreg_from_map(node_map, factors[0].as_ref()).ok()?;
            let b = Self::vreg_from_map(node_map, factors[1].as_ref()).ok()?;
            return Some((a, b));
        }
        None
    }

    fn negated_product_two_vregs(
        term: &Expr,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Option<(VReg, VReg)> {
        let ExprKind::Product(factors) = &term.kind else {
            return None;
        };
        if factors.len() != 3 {
            return None;
        }
        let mut neg_idx = None;
        for (i, f) in factors.iter().enumerate() {
            if Self::const_from_map(node_map, f.as_ref()).is_some_and(|n| (n + 1.0).abs() < EPSILON)
            {
                if neg_idx.is_some() {
                    return None;
                }
                neg_idx = Some(i);
            }
        }
        let neg_idx = neg_idx?;
        let mut iter = factors
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != neg_idx)
            .map(|(_, f)| f);
        let a = Self::vreg_from_map(node_map, iter.next()?.as_ref()).ok()?;
        let b = Self::vreg_from_map(node_map, iter.next()?.as_ref()).ok()?;
        if iter.next().is_some() {
            return None;
        }
        Some((a, b))
    }

    fn exp_call_arg(expr: &Expr) -> Option<&Expr> {
        if let ExprKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.exp
            && args.len() == 1
        {
            return Some(args[0].as_ref());
        }
        None
    }

    fn pow2_base<'expr>(
        expr: &'expr Expr,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Option<&'expr Expr> {
        if let ExprKind::Pow(base, exp) = &expr.kind
            && Self::const_from_map(node_map, exp.as_ref())
                .is_some_and(|n| (n - 2.0).abs() < EPSILON)
        {
            return Some(base.as_ref());
        }
        None
    }

    fn recip_expm1_arg<'expr>(
        den: &'expr Expr,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Option<&'expr Expr> {
        let ExprKind::Sum(terms) = &den.kind else {
            return None;
        };
        if terms.len() != 2 {
            return None;
        }
        let a = terms[0].as_ref();
        let b = terms[1].as_ref();

        let is_neg_one = |expr: &Expr| -> bool {
            Self::const_from_map(node_map, expr).is_some_and(|n| (n + 1.0).abs() < EPSILON)
        };

        if let Some(arg) = Self::exp_call_arg(a)
            && is_neg_one(b)
        {
            return Some(arg);
        }
        if let Some(arg) = Self::exp_call_arg(b)
            && is_neg_one(a)
        {
            return Some(arg);
        }
        None
    }

    fn exp_sqr_arg(
        arg: &Expr,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Option<(VReg, bool)> {
        if let Some(base) = Self::pow2_base(arg, node_map) {
            let base_v = Self::vreg_from_map(node_map, base).ok()?;
            return Some((base_v, false));
        }

        if let ExprKind::Product(factors) = &arg.kind
            && factors.len() == 2
        {
            let (_neg_idx, other_idx) = if Self::const_from_map(node_map, factors[0].as_ref())
                .is_some_and(|n| (n + 1.0).abs() < EPSILON)
            {
                (0, 1)
            } else if Self::const_from_map(node_map, factors[1].as_ref())
                .is_some_and(|n| (n + 1.0).abs() < EPSILON)
            {
                (1, 0)
            } else {
                return None;
            };
            if let Some(base) = Self::pow2_base(factors[other_idx].as_ref(), node_map) {
                let base_v = Self::vreg_from_map(node_map, base).ok()?;
                return Some((base_v, true));
            }
        }

        None
    }

    fn compile_exp_neg_arg(
        &mut self,
        arg: &Expr,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Option<VReg> {
        let compile_pos_product = |compiler: &mut Self, factors: &[Arc<Expr>]| -> Option<VReg> {
            let mut const_total = 1.0_f64;
            let mut has_const = false;
            for f in factors {
                if let Some(c) = Self::const_from_map(node_map, f.as_ref()) {
                    const_total *= c;
                    has_const = true;
                }
            }
            if !has_const || const_total >= 0.0 || !const_total.is_finite() {
                return None;
            }
            let pos_c = -const_total;
            let mut vregs_local: Vec<VReg> = Vec::new();
            for f in factors {
                if Self::const_from_map(node_map, f.as_ref()).is_none() {
                    vregs_local.push(node_map.get(&Arc::as_ptr(f))?.vreg?);
                }
            }
            if (pos_c - 1.0).abs() > EPSILON {
                let idx = compiler.add_const(pos_c);
                vregs_local.push(VReg::Const(idx));
            }
            Some(match vregs_local.len() {
                0 => {
                    let idx = compiler.add_const(pos_c);
                    VReg::Const(idx)
                }
                1 => vregs_local[0],
                _ => {
                    let d = compiler.alloc_vreg();
                    compiler.emit(VInstruction::Mul {
                        dest: d,
                        srcs: vregs_local,
                    });
                    d
                }
            })
        };

        match &arg.kind {
            ExprKind::Product(factors) => compile_pos_product(self, factors),
            ExprKind::Div(num, den) => {
                if let ExprKind::Product(nf) = &num.kind
                    && let Some(pos_num) = compile_pos_product(self, nf)
                {
                    let den_v = node_map.get(&Arc::as_ptr(den))?.vreg?;
                    let d = self.alloc_vreg();
                    self.emit(VInstruction::Div {
                        dest: d,
                        num: pos_num,
                        den: den_v,
                    });
                    return Some(d);
                }
                if let Some(n) = Self::const_from_map(node_map, num.as_ref())
                    && n < 0.0
                    && n.is_finite()
                {
                    let pos_idx = self.add_const(-n);
                    let den_v = node_map.get(&Arc::as_ptr(den))?.vreg?;
                    let d = self.alloc_vreg();
                    self.emit(VInstruction::Div {
                        dest: d,
                        num: VReg::Const(pos_idx),
                        den: den_v,
                    });
                    return Some(d);
                }
                None
            }
            _ => None,
        }
    }

    fn compile_polynomial_with_base(&mut self, poly: &Polynomial, base_vreg: VReg) -> VReg {
        let terms = poly.terms();
        if terms.is_empty() {
            let idx = self.add_const(0.0);
            return VReg::Const(idx);
        }

        let degree = terms.last().map_or(0, |t| t.0);
        let start_idx = u32::try_from(self.constants.len()).unwrap_or(u32::MAX);

        self.constants.push(f64::from_bits(u64::from(degree)));
        self.used_constants.push(true);
        let mut term_idx = terms.len();
        for current_pow in (0..=degree).rev() {
            let coeff = if term_idx > 0 && terms[term_idx - 1].0 == current_pow {
                term_idx -= 1;
                terms[term_idx].1
            } else {
                0.0
            };
            self.constants.push(coeff);
            self.used_constants.push(true);
        }

        let dest = self.alloc_vreg();
        self.emit(VInstruction::PolyEval {
            dest,
            x: base_vreg,
            const_idx: start_idx,
        });
        dest
    }

    fn compile_symbol_node(&mut self, sym: &InternedSymbol) -> Result<VReg, DiffError> {
        let sym_id = sym.id();
        if let Some(&idx) = self.param_index.get(&sym_id) {
            self.used_params[idx] = true;
            Ok(VReg::Param(u32::try_from(idx).unwrap_or(u32::MAX)))
        } else if let Some(val) = crate::core::known_symbols::get_constant_value_by_id(sym_id) {
            let idx = self.add_const(val);
            self.used_constants[idx as usize] = true;
            Ok(VReg::Const(idx))
        } else {
            Err(DiffError::UnboundVariable(sym.as_str().to_owned()))
        }
    }

    fn map_args_vregs(
        args: &[Arc<Expr>],
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Result<Vec<VReg>, DiffError> {
        let mut out = Vec::with_capacity(args.len());
        for arg in args {
            out.push(Self::vreg_from_map(node_map, arg.as_ref())?);
        }
        Ok(out)
    }

    #[allow(
        clippy::too_many_lines,
        clippy::integer_division,
        reason = "Sum compilation handles many algebraic optimizations inline"
    )]
    fn compile_sum_node(
        &mut self,
        terms: &[Arc<Expr>],
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Result<VReg, DiffError> {
        if terms.is_empty() {
            let idx = self.add_const(0.0);
            return Ok(VReg::Const(idx));
        }
        if terms.len() == 1 {
            return Self::vreg_from_map(node_map, terms[0].as_ref());
        }

        if terms.len() == 2 {
            let t0 = terms[0].as_ref();
            let t1 = terms[1].as_ref();

            if let Some((a, b)) = Self::product_two_vregs(t0, node_map)
                && let Some(c) = self.negated_inner_vreg(t1, node_map)
            {
                let dest = self.alloc_vreg();
                self.emit(VInstruction::MulSub { dest, a, b, c });
                return Ok(dest);
            }
            if let Some((a, b)) = Self::product_two_vregs(t1, node_map)
                && let Some(c) = self.negated_inner_vreg(t0, node_map)
            {
                let dest = self.alloc_vreg();
                self.emit(VInstruction::MulSub { dest, a, b, c });
                return Ok(dest);
            }
            if let Some((a, b)) = Self::negated_product_two_vregs(t0, node_map) {
                let c = Self::vreg_from_map(node_map, t1)?;
                let dest = self.alloc_vreg();
                self.emit(VInstruction::NegMulAdd { dest, a, b, c });
                return Ok(dest);
            }
            if let Some((a, b)) = Self::negated_product_two_vregs(t1, node_map) {
                let c = Self::vreg_from_map(node_map, t0)?;
                let dest = self.alloc_vreg();
                self.emit(VInstruction::NegMulAdd { dest, a, b, c });
                return Ok(dest);
            }
            if let Some((a, b)) = Self::product_two_vregs(t0, node_map) {
                let c = Self::vreg_from_map(node_map, t1)?;
                let dest = self.alloc_vreg();
                self.emit(VInstruction::MulAdd { dest, a, b, c });
                return Ok(dest);
            }
            if let Some((a, b)) = Self::product_two_vregs(t1, node_map) {
                let c = Self::vreg_from_map(node_map, t0)?;
                let dest = self.alloc_vreg();
                self.emit(VInstruction::MulAdd { dest, a, b, c });
                return Ok(dest);
            }

            // Simple 2-term sum without FMA
            let a = Self::vreg_from_map(node_map, t0)?;
            let b = Self::vreg_from_map(node_map, t1)?;
            let dest = self.alloc_vreg();
            self.emit(VInstruction::Add2 { dest, a, b });
            return Ok(dest);
        }

        let mut pos_vregs = Vec::with_capacity(terms.len());
        let mut neg_vregs = Vec::new();

        for term in terms {
            if let Some(inner) = self.negated_inner_vreg(term.as_ref(), node_map) {
                if neg_vregs.capacity() == 0 {
                    neg_vregs.reserve(terms.len() / 2 + 1);
                }
                neg_vregs.push(inner);
            } else {
                pos_vregs.push(Self::vreg_from_map(node_map, term.as_ref())?);
            }
        }

        if pos_vregs.is_empty() && neg_vregs.is_empty() {
            let idx = self.add_const(0.0);
            return Ok(VReg::Const(idx));
        }

        if neg_vregs.is_empty() {
            if pos_vregs.len() == 1 {
                return Ok(pos_vregs[0]);
            }
            if pos_vregs.len() == 2 {
                let dest = self.alloc_vreg();
                self.emit(VInstruction::Add2 {
                    dest,
                    a: pos_vregs[0],
                    b: pos_vregs[1],
                });
                return Ok(dest);
            }
            let dest = self.alloc_vreg();
            self.emit(VInstruction::Add {
                dest,
                srcs: pos_vregs,
            });
            return Ok(dest);
        }

        if pos_vregs.is_empty() {
            let inner = if neg_vregs.len() == 1 {
                neg_vregs[0]
            } else if neg_vregs.len() == 2 {
                let s_v = self.alloc_vreg();
                self.emit(VInstruction::Add2 {
                    dest: s_v,
                    a: neg_vregs[0],
                    b: neg_vregs[1],
                });
                s_v
            } else {
                let s_v = self.alloc_vreg();
                self.emit(VInstruction::Add {
                    dest: s_v,
                    srcs: neg_vregs,
                });
                s_v
            };
            let dest = self.alloc_vreg();
            self.emit(VInstruction::Neg { dest, src: inner });
            return Ok(dest);
        }

        let pos_v = if pos_vregs.len() == 1 {
            pos_vregs[0]
        } else if pos_vregs.len() == 2 {
            let s_v = self.alloc_vreg();
            self.emit(VInstruction::Add2 {
                dest: s_v,
                a: pos_vregs[0],
                b: pos_vregs[1],
            });
            s_v
        } else {
            let s_v = self.alloc_vreg();
            self.emit(VInstruction::Add {
                dest: s_v,
                srcs: pos_vregs,
            });
            s_v
        };
        let neg_v = if neg_vregs.len() == 1 {
            neg_vregs[0]
        } else if neg_vregs.len() == 2 {
            let s_v = self.alloc_vreg();
            self.emit(VInstruction::Add2 {
                dest: s_v,
                a: neg_vregs[0],
                b: neg_vregs[1],
            });
            s_v
        } else {
            let s_v = self.alloc_vreg();
            self.emit(VInstruction::Add {
                dest: s_v,
                srcs: neg_vregs,
            });
            s_v
        };
        let dest = self.alloc_vreg();
        self.emit(VInstruction::Sub {
            dest,
            a: pos_v,
            b: neg_v,
        });
        Ok(dest)
    }

    #[allow(
        clippy::too_many_lines,
        reason = "Product compilation handles constant folding and arity optimizations inline"
    )]
    fn compile_product_node(
        &mut self,
        factors: &[Arc<Expr>],
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Result<VReg, DiffError> {
        if factors.is_empty() {
            let idx = self.add_const(1.0);
            return Ok(VReg::Const(idx));
        }

        // Optimize 2-factor products to avoid Vec allocation
        if factors.len() == 2 {
            let f0 = factors[0].as_ref();
            let f1 = factors[1].as_ref();
            let c0 = Self::const_from_map(node_map, f0);
            let c1 = Self::const_from_map(node_map, f1);

            match (c0, c1) {
                (Some(v0), Some(v1)) => {
                    let val = v0 * v1;
                    if val.is_finite() {
                        let idx = self.add_const(val);
                        return Ok(VReg::Const(idx));
                    }
                }
                (Some(v0), None) => {
                    if v0.is_finite() {
                        if (v0 - 1.0).abs() < EPSILON {
                            return Self::vreg_from_map(node_map, f1);
                        }
                        let c_idx = self.add_const(v0);
                        let v1_reg = Self::vreg_from_map(node_map, f1)?;
                        let dest = self.alloc_vreg();
                        self.emit(VInstruction::Mul2 {
                            dest,
                            a: VReg::Const(c_idx),
                            b: v1_reg,
                        });
                        return Ok(dest);
                    }
                }
                (None, Some(v1)) => {
                    if v1.is_finite() {
                        if (v1 - 1.0).abs() < EPSILON {
                            return Self::vreg_from_map(node_map, f0);
                        }
                        let c_idx = self.add_const(v1);
                        let v0_reg = Self::vreg_from_map(node_map, f0)?;
                        let dest = self.alloc_vreg();
                        self.emit(VInstruction::Mul2 {
                            dest,
                            a: v0_reg,
                            b: VReg::Const(c_idx),
                        });
                        return Ok(dest);
                    }
                }
                (None, None) => {
                    let v0_reg = Self::vreg_from_map(node_map, f0)?;
                    let v1_reg = Self::vreg_from_map(node_map, f1)?;
                    let dest = self.alloc_vreg();
                    self.emit(VInstruction::Mul2 {
                        dest,
                        a: v0_reg,
                        b: v1_reg,
                    });
                    return Ok(dest);
                }
            }
        }

        let mut constant_acc = 1.0_f64;
        let mut variable_vregs = Vec::with_capacity(factors.len());
        for f in factors {
            if let Some(c) = Self::const_from_map(node_map, f.as_ref()) {
                constant_acc *= c;
            } else {
                variable_vregs.push(Self::vreg_from_map(node_map, f.as_ref())?);
            }
        }

        let mut vregs_all = variable_vregs;
        if constant_acc.is_finite() {
            if (constant_acc - 1.0).abs() > EPSILON {
                let c_idx = self.add_const(constant_acc);
                vregs_all.push(VReg::Const(c_idx));
            }
        } else {
            // If the constant accumulator is non-finite (NaN or Inf),
            // we must include the constant factors individually to preserve
            // the exact non-finite behavior (e.g., which specific Inf it was).
            // vregs_all already has capacity factors.len()
            for f in factors {
                if let Some(c) = Self::const_from_map(node_map, f.as_ref()) {
                    let c_idx = self.add_const(c);
                    vregs_all.push(VReg::Const(c_idx));
                }
            }
        }

        if vregs_all.is_empty() {
            let idx = self.add_const(1.0);
            return Ok(VReg::Const(idx));
        }
        if vregs_all.len() == 1 {
            return Ok(vregs_all[0]);
        }
        if vregs_all.len() == 2 {
            let dest = self.alloc_vreg();
            self.emit(VInstruction::Mul2 {
                dest,
                a: vregs_all[0],
                b: vregs_all[1],
            });
            return Ok(dest);
        }

        let dest = self.alloc_vreg();
        self.emit(VInstruction::Mul {
            dest,
            srcs: vregs_all,
        });
        Ok(dest)
    }

    fn compile_div_node(
        &mut self,
        num: &Expr,
        den: &Expr,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Result<VReg, DiffError> {
        if num == den {
            let idx = self.add_const(1.0);
            return Ok(VReg::Const(idx));
        }

        if let Some(n) = Self::const_from_map(node_map, num)
            && (n - 1.0).abs() < EPSILON
            && let Some(arg) = Self::recip_expm1_arg(den, node_map)
        {
            let src = Self::vreg_from_map(node_map, arg)?;
            let dest = self.alloc_vreg();
            self.emit(VInstruction::RecipExpm1 { dest, src });
            return Ok(dest);
        }

        if let ExprKind::FunctionCall { name, args } = &num.kind
            && args.len() == 1
            && name.id() == KS.sin
            && args[0].as_ref() == den
        {
            let den_v = Self::vreg_from_map(node_map, args[0].as_ref())?;
            let dest = self.alloc_vreg();
            self.emit(VInstruction::BuiltinFun {
                dest,
                op: FnOp::Sinc,
                args: vec![den_v],
            });
            return Ok(dest);
        }

        let num_v = Self::vreg_from_map(node_map, num)?;
        let den_v = Self::vreg_from_map(node_map, den)?;
        let dest = self.alloc_vreg();
        self.emit(VInstruction::Div {
            dest,
            num: num_v,
            den: den_v,
        });
        Ok(dest)
    }

    #[allow(
        clippy::too_many_lines,
        reason = "Power compilation handles many optimized exponent cases inline"
    )]
    fn compile_pow_node(
        &mut self,
        base: &Expr,
        exp: &Expr,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Result<VReg, DiffError> {
        if let Some(n_val) = Self::const_from_map(node_map, exp) {
            let is_integer = (n_val - n_val.round()).abs() < EPSILON;
            if is_integer {
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "Checked for equality with rounded value so it's safe"
                )]
                let n_int = n_val.round() as i64;
                if n_int == 0 {
                    let idx = self.add_const(1.0);
                    return Ok(VReg::Const(idx));
                }

                let base_v = Self::vreg_from_map(node_map, base)?;
                let out = match n_int {
                    1 => base_v,
                    2 => {
                        let dest = self.alloc_vreg();
                        self.emit(VInstruction::Square { dest, src: base_v });
                        dest
                    }
                    3 => {
                        let dest = self.alloc_vreg();
                        self.emit(VInstruction::Cube { dest, src: base_v });
                        dest
                    }
                    4 => {
                        let dest = self.alloc_vreg();
                        self.emit(VInstruction::Pow4 { dest, src: base_v });
                        dest
                    }
                    -1 => {
                        let dest = self.alloc_vreg();
                        self.emit(VInstruction::Recip { dest, src: base_v });
                        dest
                    }
                    -2 => {
                        let dest = self.alloc_vreg();
                        self.emit(VInstruction::InvSquare { dest, src: base_v });
                        dest
                    }
                    -3 => {
                        let dest = self.alloc_vreg();
                        self.emit(VInstruction::InvCube { dest, src: base_v });
                        dest
                    }
                    n => {
                        if let Ok(n_i32) = i32::try_from(n) {
                            let dest = self.alloc_vreg();
                            self.emit(VInstruction::Powi {
                                dest,
                                src: base_v,
                                n: n_i32,
                            });
                            dest
                        } else {
                            let exp_v = Self::vreg_from_map(node_map, exp)?;
                            let dest = self.alloc_vreg();
                            self.emit(VInstruction::Pow {
                                dest,
                                base: base_v,
                                exp: exp_v,
                            });
                            dest
                        }
                    }
                };
                return Ok(out);
            }
            if (n_val - 0.5).abs() < EPSILON {
                let base_v = Self::vreg_from_map(node_map, base)?;
                let dest = self.alloc_vreg();
                self.emit(VInstruction::BuiltinFun {
                    dest,
                    op: FnOp::Sqrt,
                    args: vec![base_v],
                });
                return Ok(dest);
            }
            if (n_val + 0.5).abs() < EPSILON {
                let base_v = Self::vreg_from_map(node_map, base)?;
                let dest = self.alloc_vreg();
                self.emit(VInstruction::InvSqrt { dest, src: base_v });
                return Ok(dest);
            }
            if (n_val - 1.5).abs() < EPSILON {
                let base_v = Self::vreg_from_map(node_map, base)?;
                let dest = self.alloc_vreg();
                self.emit(VInstruction::Pow3_2 { dest, src: base_v });
                return Ok(dest);
            }
            if (n_val + 1.5).abs() < EPSILON {
                let base_v = Self::vreg_from_map(node_map, base)?;
                let dest = self.alloc_vreg();
                self.emit(VInstruction::InvPow3_2 { dest, src: base_v });
                return Ok(dest);
            }
        }

        let base_v = Self::vreg_from_map(node_map, base)?;
        let exp_v = Self::vreg_from_map(node_map, exp)?;
        let dest = self.alloc_vreg();
        self.emit(VInstruction::Pow {
            dest,
            base: base_v,
            exp: exp_v,
        });
        Ok(dest)
    }

    fn compile_function_node(
        &mut self,
        name: &InternedSymbol,
        args: &[Arc<Expr>],
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Result<VReg, DiffError> {
        let id = name.id();
        let ks = &*KS;
        let dest = self.alloc_vreg();

        if id == ks.exp
            && args.len() == 1
            && let Some((src, neg)) = Self::exp_sqr_arg(args[0].as_ref(), node_map)
        {
            if neg {
                self.emit(VInstruction::ExpSqrNeg { dest, src });
            } else {
                self.emit(VInstruction::ExpSqr { dest, src });
            }
            return Ok(dest);
        }

        if id == ks.exp
            && args.len() == 1
            && let Some(pos_vreg) = self.compile_exp_neg_arg(&args[0], node_map)
        {
            self.emit(VInstruction::Builtin1 {
                dest,
                op: FnOp::ExpNeg,
                arg: pos_vreg,
            });
            return Ok(dest);
        }

        if let Some(&op) = FN_MAP.get(&id) {
            match args.len() {
                1 => {
                    let arg = Self::vreg_from_map(node_map, args[0].as_ref())?;
                    if id == ks.log {
                        self.emit(VInstruction::Builtin1 {
                            dest,
                            op: FnOp::Ln,
                            arg,
                        });
                    } else {
                        self.emit(VInstruction::Builtin1 { dest, op, arg });
                    }
                }
                2 => {
                    let vreg1 = Self::vreg_from_map(node_map, args[0].as_ref())?;
                    let vreg2 = Self::vreg_from_map(node_map, args[1].as_ref())?;
                    self.emit(VInstruction::Builtin2 {
                        dest,
                        op,
                        arg1: vreg1,
                        arg2: vreg2,
                    });
                }
                _ => {
                    let arg_vregs = Self::map_args_vregs(args, node_map)?;
                    self.emit(VInstruction::BuiltinFun {
                        dest,
                        op,
                        args: arg_vregs,
                    });
                }
            }
            return Ok(dest);
        }

        if args.len() == 1 {
            let base_val = if id == ks.log2 {
                Some(2.0)
            } else if id == ks.log10 {
                Some(10.0)
            } else {
                None
            };
            if let Some(bv) = base_val {
                let base_idx = self.add_const(bv);
                let arg = Self::vreg_from_map(node_map, args[0].as_ref())?;
                self.emit(VInstruction::Builtin2 {
                    dest,
                    op: FnOp::Log,
                    arg1: VReg::Const(base_idx),
                    arg2: arg,
                });
                return Ok(dest);
            }
        }

        Err(DiffError::UnsupportedFunction(name.as_str().to_owned()))
    }

    fn compile_poly_node(
        &mut self,
        poly: &Polynomial,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Result<VReg, DiffError> {
        let base_v = Self::vreg_from_map(node_map, poly.base().as_ref())?;
        Ok(self.compile_polynomial_with_base(poly, base_v))
    }

    fn lookup_cse(&self, expr: &Expr) -> Option<VReg> {
        self.cse_cache
            .get(&CseKey(std::ptr::from_ref::<Expr>(expr)))
            .copied()
    }

    fn push_children(expr: &Expr, stack: &mut Vec<(*const Expr, bool)>) {
        match &expr.kind {
            ExprKind::Sum(terms) | ExprKind::Product(terms) => {
                for t in terms.iter().rev() {
                    stack.push((Arc::as_ptr(t), false));
                }
            }
            ExprKind::Div(num, den) => {
                stack.push((Arc::as_ptr(den), false));
                stack.push((Arc::as_ptr(num), false));
            }
            ExprKind::Pow(base, exp) => {
                stack.push((Arc::as_ptr(exp), false));
                stack.push((Arc::as_ptr(base), false));
            }
            ExprKind::FunctionCall { args, .. } => {
                for a in args.iter().rev() {
                    stack.push((Arc::as_ptr(a), false));
                }
            }
            ExprKind::Poly(poly) => {
                stack.push((Arc::as_ptr(poly.base()), false));
            }
            ExprKind::Derivative { inner, .. } => {
                stack.push((Arc::as_ptr(inner), false));
            }
            ExprKind::Number(_) | ExprKind::Symbol(_) => {}
        }
    }

    fn compile_nonconst_node(
        &mut self,
        expr: &Expr,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Result<VReg, DiffError> {
        match &expr.kind {
            ExprKind::Number(_) => Err(DiffError::UnsupportedExpression(
                "Numerical values are unreachable here".to_owned(),
            )),
            ExprKind::Symbol(s) => self.compile_symbol_node(s),
            ExprKind::Sum(terms) => self.compile_sum_node(terms, node_map),
            ExprKind::Product(factors) => self.compile_product_node(factors, node_map),
            ExprKind::Div(num, den) => self.compile_div_node(num.as_ref(), den.as_ref(), node_map),
            ExprKind::Pow(base, exp) => {
                self.compile_pow_node(base.as_ref(), exp.as_ref(), node_map)
            }
            ExprKind::FunctionCall { name, args } => {
                self.compile_function_node(name, args, node_map)
            }
            ExprKind::Poly(poly) => self.compile_poly_node(poly, node_map),
            ExprKind::Derivative { .. } => Err(DiffError::UnsupportedExpression(
                "Derivatives cannot be numerically evaluated - simplify first".to_owned(),
            )),
        }
    }

    fn compile_expr_iterative(
        &mut self,
        root: &Expr,
        node_count: usize,
    ) -> Result<VReg, DiffError> {
        let mut stack: Vec<(*const Expr, bool)> = Vec::with_capacity(node_count);
        let mut node_map: FxHashMap<*const Expr, NodeData> = FxHashMap::default();
        node_map.reserve(node_count);

        let root_ptr = root as *const Expr;
        stack.push((root_ptr, false));

        while let Some((ptr, visited)) = stack.pop() {
            // SAFETY: ptrs come from the current expression tree, which outlives compilation.
            let expr = unsafe { &*ptr };

            if visited {
                if node_map.get(&ptr).and_then(|data| data.vreg).is_some() {
                    continue;
                }

                let const_val = Self::compute_const_from_children(expr, &node_map);
                let is_expensive = Self::compute_expensive_from_children(expr, &node_map);

                if is_expensive && let Some(cached) = self.lookup_cse(expr) {
                    node_map.insert(
                        ptr,
                        NodeData {
                            vreg: Some(cached),
                            const_val,
                            is_expensive,
                        },
                    );
                    continue;
                }

                if let Some(val) = const_val
                    && val.is_finite()
                {
                    let idx = self.add_const(val);
                    let vreg = VReg::Const(idx);
                    node_map.insert(
                        ptr,
                        NodeData {
                            vreg: Some(vreg),
                            const_val,
                            is_expensive,
                        },
                    );
                    continue;
                }

                let result_vreg = self.compile_nonconst_node(expr, &node_map)?;
                node_map.insert(
                    ptr,
                    NodeData {
                        vreg: Some(result_vreg),
                        const_val,
                        is_expensive,
                    },
                );

                if is_expensive {
                    self.cse_cache.insert(CseKey(ptr), result_vreg);
                }
            } else if !node_map.contains_key(&ptr) {
                stack.push((ptr, true));
                Self::push_children(expr, &mut stack);
            }
        }

        node_map
            .get(&root_ptr)
            .and_then(|data| data.vreg)
            .ok_or_else(|| {
                DiffError::UnsupportedExpression(
                    "Internal compile error: missing root vreg".to_owned(),
                )
            })
    }
}
