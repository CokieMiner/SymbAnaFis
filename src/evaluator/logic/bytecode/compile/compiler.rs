//! Expression compiler for the bytecode evaluator.
//!
//! This module compiles symbolic [`Expr`] expressions into efficient bytecode
//! ([`Instruction`]s) that can be executed by the [`CompiledEvaluator`].

use super::emit::RegAllocator;
use super::instruction::Instruction;
use super::vir::{VInstruction, VReg};
use crate::Expr;
use crate::core::error::DiffError;
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;
use std::mem::take;

use super::analysis::CseKey;

pub struct Compiler {
    pub(super) vinstrs: Vec<VInstruction>,
    pub(super) param_ids: Vec<u64>,
    pub(super) param_index: FxHashMap<u64, usize>,
    pub(super) cse_cache: FxHashMap<CseKey, VReg>,
    pub(super) constants: Vec<f64>,
    pub(super) const_map: FxHashMap<u64, u32>,
    pub(super) next_vreg: u32,
    pub(super) final_vreg: Option<VReg>,
}

impl Compiler {
    pub(crate) fn new(param_ids: &[u64]) -> Self {
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
            next_vreg: 0,
            final_vreg: None,
        };
        // Pre-add 0.0 so it's always available (e.g. for empty expressions)
        compiler.add_const(0.0);
        compiler
    }

    #[inline]
    pub(super) const fn alloc_vreg(&mut self) -> VReg {
        let r = self.next_vreg;
        self.next_vreg += 1;
        VReg::Temp(r)
    }

    #[inline]
    pub(crate) fn add_const(&mut self, val: f64) -> u32 {
        let bits = val.to_bits();
        match self.const_map.entry(bits) {
            Entry::Occupied(o) => *o.get(),
            Entry::Vacant(v) => {
                let idx = u32::try_from(self.constants.len()).unwrap_or(u32::MAX);
                self.constants.push(val);
                v.insert(idx);
                idx
            }
        }
    }

    #[inline]
    pub(super) fn emit(&mut self, instr: VInstruction) {
        self.vinstrs.push(instr);
    }

    pub(crate) fn into_parts(mut self) -> (Vec<Instruction>, Vec<f64>, Vec<u32>, usize, usize) {
        let param_count = u32::try_from(self.param_ids.len()).expect("Param count too large");
        let const_count = u32::try_from(self.constants.len()).expect("Const count too large");
        let num_temps = self.next_vreg as usize;

        self.optimize_vir_cse();

        let vinstrs = take(&mut self.vinstrs);

        let allocator = RegAllocator::new(param_count, const_count, num_temps, &vinstrs);
        let (instructions, arg_pool, register_count) = allocator.allocate(vinstrs, self.final_vreg);

        (
            instructions,
            self.constants,
            arg_pool,
            register_count,
            param_count as usize,
        )
    }

    pub(crate) fn compile_expr(&mut self, expr: &Expr) -> Result<VReg, DiffError> {
        let node_count = expr.node_count();
        self.vinstrs.reserve(node_count);
        #[allow(
            clippy::integer_division,
            reason = "Intentional integer division for capacity reservation"
        )]
        let const_reserve = node_count / 8 + 8;
        self.constants.reserve(const_reserve);
        self.const_map.reserve(const_reserve);
        #[allow(
            clippy::integer_division,
            reason = "Intentional integer division for capacity reservation"
        )]
        self.cse_cache.reserve(node_count / 8);
        let vreg = self.compile_expr_iterative(expr, node_count)?;
        self.final_vreg = Some(vreg);
        Ok(vreg)
    }
}
