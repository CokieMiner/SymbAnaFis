//! VIR generator for the bytecode evaluator.
//!
//! This module compiles symbolic [`Expr`] expressions into Virtual Intermediate
//! Representation ([`VInstruction`]s) that are then lowered to physical bytecode.

use super::Instruction;
use super::analysis::{GvnKey, eliminate_vir_dead_code, optimize_vir_gvn};
use super::emit::RegAllocator;
use super::optimize::schedule::greedy_schedule;
use super::vir::{VInstruction, VReg};
use crate::core::Expr;
use crate::core::error::DiffError;
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;

pub struct VirGenerator {
    pub(super) vinstrs: Vec<VInstruction>,
    pub(super) param_ids: Vec<u64>,
    pub(super) param_index: FxHashMap<u64, usize>,
    pub(super) gvn_cache: FxHashMap<GvnKey, VReg>,
    pub(super) constants: Vec<f64>,
    pub(super) const_map: FxHashMap<u64, u32>,
    pub(super) next_vreg: u32,
    pub(super) final_vreg: Option<VReg>,
}

impl VirGenerator {
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
            gvn_cache: FxHashMap::default(),
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
                let idx = u32::try_from(self.constants.len())
                    .expect("constant pool overflow: more than u32::MAX constants");
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

    #[allow(
        clippy::type_complexity,
        reason = "Internal signature for bytecode decomposition"
    )]
    pub(crate) fn into_parts(
        mut self,
    ) -> (
        Vec<Instruction>,
        Vec<f64>,
        FxHashMap<u64, u32>,
        Vec<u32>,
        usize,
        usize,
        u32,
    ) {
        let param_count = u32::try_from(self.param_ids.len()).expect("Param count too large");
        optimize_vir_gvn(
            &mut self.vinstrs,
            &mut self.final_vreg,
            &mut self.constants,
            &mut self.const_map,
            param_count,
        );
        let const_count = u32::try_from(self.constants.len()).expect("Const count too large");

        // Greedy Instruction Scheduling (Minimizes Register Pressure)
        self.vinstrs = greedy_schedule(self.vinstrs, self.next_vreg);

        // VIR Backward Dead Code Elimination
        let (vinstrs, num_temps) =
            eliminate_vir_dead_code(self.vinstrs, self.final_vreg, self.next_vreg);

        let allocator = RegAllocator::new(
            param_count,
            const_count,
            num_temps,
            &vinstrs,
            self.final_vreg,
        );
        let (instructions, arg_pool, max_phys, result_reg) =
            allocator.allocate(vinstrs, self.final_vreg);

        (
            instructions,
            self.constants,
            self.const_map,
            arg_pool,
            param_count as usize,
            max_phys,
            result_reg,
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
        self.gvn_cache.reserve(node_count / 8);
        let vreg = self.compile_expr_iterative(expr, node_count)?;
        self.final_vreg = Some(vreg);
        Ok(vreg)
    }
}
