//! Expression compiler for the bytecode evaluator.
//!
//! This module compiles symbolic [`Expr`] expressions into efficient bytecode
//! ([`Instruction`]s) that can be executed by the [`CompiledEvaluator`].

use super::node::{self, NodeData};
use super::reg_alloc::RegAllocator;
use super::registry::FN_MAP;
use super::vir::{VInstruction, VReg};
use crate::EPSILON;
use crate::Expr;
use crate::core::error::DiffError;
use crate::core::expr::ExprKind;
use crate::core::known_symbols::KS;
use crate::core::poly::Polynomial;
use crate::core::symbol::InternedSymbol;
use crate::evaluator::logic::instruction::{FnOp, Instruction};
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
struct CseKey(*const Expr);

impl PartialEq for CseKey {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        #[allow(
            unsafe_code,
            reason = "Pointers in CseKey are derived from valid Expr nodes in the tree currently being compiled. Accessing precomputed hashes provides O(1) equality rejection, which is critical for CSE performance."
        )]
        // SAFETY: Pointers are derived from the root expression tree and are valid during compilation.
        unsafe {
            self.0 == other.0 || ((*self.0).hash == (*other.0).hash && *self.0 == *other.0)
        }
    }
}

impl Eq for CseKey {}

impl std::hash::Hash for CseKey {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        #[allow(
            unsafe_code,
            reason = "CseKey pointers are guaranteed valid during compilation. Using the precomputed hash field avoids expensive recursive structural hashing."
        )]
        // SAFETY: The pointer is valid as it comes from the tree currently being compiled.
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
    next_vreg: u32,
    final_vreg: Option<VReg>,
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
    const fn alloc_vreg(&mut self) -> VReg {
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
    fn emit(&mut self, instr: VInstruction) {
        self.vinstrs.push(instr);
    }

    pub(crate) fn into_parts(mut self) -> (Vec<Instruction>, Vec<f64>, Vec<u32>, usize, usize) {
        let param_count = u32::try_from(self.param_ids.len()).expect("Param count too large");
        let const_count = u32::try_from(self.constants.len()).expect("Const count too large");
        let num_temps = self.next_vreg as usize;

        self.optimize_vir_cse();

        let vinstrs = std::mem::take(&mut self.vinstrs);

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

    fn vreg_from_map(
        node_map: &FxHashMap<*const Expr, NodeData>,
        expr: &Expr,
    ) -> Result<VReg, DiffError> {
        node_map
            .get(&std::ptr::from_ref(expr))
            .map(|data| data.vreg())
            .ok_or_else(|| {
                DiffError::UnsupportedExpression(
                    "Internal compile error: missing child vreg".to_owned(),
                )
            })
    }

    fn negated_inner_vreg(
        &mut self,
        term: &Expr,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Option<VReg> {
        if let ExprKind::Product(factors) = &term.kind {
            let neg_idx = factors.iter().position(|f| {
                node::const_from_map(node_map, f.as_ref())
                    .is_some_and(|n| (n + 1.0).abs() < EPSILON)
            })?;
            let num_inner = factors.len().saturating_sub(1);
            match num_inner {
                0 => {
                    let idx = self.add_const(1.0);
                    return Some(VReg::Const(idx));
                }
                1 => {
                    for (i, f) in factors.iter().enumerate() {
                        if i != neg_idx {
                            return node_map.get(&Arc::as_ptr(f)).map(|data| data.vreg());
                        }
                    }
                }
                2 => {
                    let mut iter = factors.iter().enumerate().filter(|(i, _)| *i != neg_idx);
                    let (_, f1) = iter.next()?;
                    let (_, f2) = iter.next()?;
                    let a = node_map.get(&Arc::as_ptr(f1)).map(|data| data.vreg())?;
                    let b = node_map.get(&Arc::as_ptr(f2)).map(|data| data.vreg())?;
                    let d = self.alloc_vreg();
                    self.emit(VInstruction::Mul2 { dest: d, a, b });
                    return Some(d);
                }
                _ => {
                    let mut inner_vregs: Vec<VReg> = Vec::with_capacity(num_inner);
                    for (i, f) in factors.iter().enumerate() {
                        if i != neg_idx {
                            inner_vregs
                                .push(node_map.get(&Arc::as_ptr(f)).map(|data| data.vreg())?);
                        }
                    }
                    let d = self.alloc_vreg();
                    self.emit(VInstruction::Mul {
                        dest: d,
                        srcs: inner_vregs,
                    });
                    return Some(d);
                }
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
                if let Some(c) = node::const_from_map(node_map, f.as_ref()) {
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
                if node::const_from_map(node_map, f.as_ref()).is_none() {
                    vregs_local.push(node_map.get(&Arc::as_ptr(f)).map(|data| data.vreg())?);
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
                    let den_v = node_map.get(&Arc::as_ptr(den)).map(|data| data.vreg())?;
                    let d = self.alloc_vreg();
                    self.emit(VInstruction::Div {
                        dest: d,
                        num: pos_num,
                        den: den_v,
                    });
                    return Some(d);
                }
                if let Some(n) = node::const_from_map(node_map, num.as_ref())
                    && n < 0.0
                    && n.is_finite()
                {
                    let pos_idx = self.add_const(-n);
                    let den_v = node_map.get(&Arc::as_ptr(den)).map(|data| data.vreg())?;
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

        if degree >= 4 {
            let mut coeffs = vec![0.0; (degree + 1) as usize];
            for &(p, c) in terms {
                coeffs[p as usize] = c;
            }
            let mut powers = rustc_hash::FxHashMap::default();
            powers.insert(1, base_vreg);
            return self.compile_poly_estrin(&coeffs, base_vreg, &mut powers);
        }

        let mut term_idx = terms.len();
        let mut current_vreg = VReg::Const(self.add_const(0.0));
        let mut first = true;

        for current_pow in (0..=degree).rev() {
            let coeff = if term_idx > 0 && terms[term_idx - 1].0 == current_pow {
                term_idx -= 1;
                terms[term_idx].1
            } else {
                0.0
            };

            if first {
                let idx = self.add_const(coeff);
                current_vreg = VReg::Const(idx);
                first = false;
            } else {
                let next_vreg = self.alloc_vreg();
                let c_idx = self.add_const(coeff);
                self.emit(VInstruction::MulAdd {
                    dest: next_vreg,
                    a: current_vreg,
                    b: base_vreg,
                    c: VReg::Const(c_idx),
                });
                current_vreg = next_vreg;
            }
        }

        current_vreg
    }

    fn compile_poly_estrin(
        &mut self,
        coeffs: &[f64],
        x: VReg,
        powers: &mut rustc_hash::FxHashMap<usize, VReg>,
    ) -> VReg {
        if coeffs.iter().all(|&c| c == 0.0) {
            return VReg::Const(self.add_const(0.0));
        }
        if coeffs.len() == 1 {
            return VReg::Const(self.add_const(coeffs[0]));
        }
        if coeffs.len() == 2 {
            let dest = self.alloc_vreg();
            let c1 = self.add_const(coeffs[1]);
            let c0 = self.add_const(coeffs[0]);
            self.emit(VInstruction::MulAdd {
                dest,
                a: VReg::Const(c1),
                b: x,
                c: VReg::Const(c0),
            });
            return dest;
        }

        let mid = coeffs.len().div_ceil(2);
        let left = &coeffs[..mid];
        let right = &coeffs[mid..];

        let left_vreg = self.compile_poly_estrin(left, x, powers);
        let right_vreg = self.compile_poly_estrin(right, x, powers);

        let x_pow_mid = self.get_power_vreg(x, mid, powers);

        let dest = self.alloc_vreg();
        if self.is_const_zero(left_vreg) {
            self.emit(VInstruction::Mul2 {
                dest,
                a: right_vreg,
                b: x_pow_mid,
            });
        } else {
            self.emit(VInstruction::MulAdd {
                dest,
                a: right_vreg,
                b: x_pow_mid,
                c: left_vreg,
            });
        }
        dest
    }

    fn get_power_vreg(
        &mut self,
        x: VReg,
        n: usize,
        powers: &mut rustc_hash::FxHashMap<usize, VReg>,
    ) -> VReg {
        if n == 1 {
            return x;
        }
        #[allow(
            clippy::manual_is_multiple_of,
            clippy::integer_division,
            reason = "Intentional integer power splitting"
        )]
        let v = if n % 2 == 0 {
            let half = self.get_power_vreg(x, n / 2, powers);
            let dest = self.alloc_vreg();
            self.emit(VInstruction::Mul2 {
                dest,
                a: half,
                b: half,
            });
            dest
        } else {
            let prev = self.get_power_vreg(x, n - 1, powers);
            let dest = self.alloc_vreg();
            self.emit(VInstruction::Mul2 {
                dest,
                a: prev,
                b: x,
            });
            dest
        };
        powers.insert(n, v);
        v
    }

    fn is_const_zero(&self, vreg: VReg) -> bool {
        match vreg {
            VReg::Const(idx) => self.constants[idx as usize] == 0.0,
            _ => false,
        }
    }

    fn optimize_vir_cse(&mut self) {
        let mut seen = rustc_hash::FxHashMap::default();
        let mut alias = rustc_hash::FxHashMap::default();
        let mut optimized = Vec::with_capacity(self.vinstrs.len());

        for mut instr in std::mem::take(&mut self.vinstrs) {
            instr.for_each_read_mut(|r| {
                if let Some(&canonical) = alias.get(r) {
                    *r = canonical;
                }
            });

            instr.sort_operands();

            let mut key = instr.clone();
            key.set_dest(VReg::Temp(u32::MAX));

            if let Some(&existing_vreg) = seen.get(&key) {
                alias.insert(instr.dest(), existing_vreg);
            } else {
                seen.insert(key, instr.dest());
                optimized.push(instr);
            }
        }
        self.vinstrs = optimized;

        if let Some(f) = &mut self.final_vreg {
            if let Some(&canonical) = alias.get(f) {
                *f = canonical;
            }
        }
    }

    fn compile_symbol_node(&mut self, sym: &InternedSymbol) -> Result<VReg, DiffError> {
        let sym_id = sym.id();
        if let Some(&idx) = self.param_index.get(&sym_id) {
            Ok(VReg::Param(
                u32::try_from(idx).expect("Param index too large"),
            ))
        } else if let Some(val) = crate::core::known_symbols::get_constant_value_by_id(sym_id) {
            let idx = self.add_const(val);
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
        reason = "Large dispatch function for sum nodes"
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

            let c0 = node::const_from_map(node_map, t0);
            let c1 = node::const_from_map(node_map, t1);
            match (c0, c1) {
                (Some(v0), Some(v1)) => {
                    let val = v0 + v1;
                    if val.is_finite() {
                        let idx = self.add_const(val);
                        return Ok(VReg::Const(idx));
                    }
                }
                (Some(v0), None) if v0.is_finite() && v0.abs() < EPSILON => {
                    return Self::vreg_from_map(node_map, t1);
                }
                (None, Some(v1)) if v1.is_finite() && v1.abs() < EPSILON => {
                    return Self::vreg_from_map(node_map, t0);
                }
                _ => {}
            }

            if let Some((a, b)) = node::product_two_vregs(t0, node_map)
                && let Some(c) = self.negated_inner_vreg(t1, node_map)
            {
                let dest = self.alloc_vreg();
                self.emit(VInstruction::MulSub { dest, a, b, c });
                return Ok(dest);
            }
            if let Some((a, b)) = node::product_two_vregs(t1, node_map)
                && let Some(c) = self.negated_inner_vreg(t0, node_map)
            {
                let dest = self.alloc_vreg();
                self.emit(VInstruction::MulSub { dest, a, b, c });
                return Ok(dest);
            }
            if let Some((a, b)) = node::negated_product_two_vregs(t0, node_map) {
                let c = Self::vreg_from_map(node_map, t1)?;
                let dest = self.alloc_vreg();
                self.emit(VInstruction::NegMulAdd { dest, a, b, c });
                return Ok(dest);
            }
            if let Some((a, b)) = node::negated_product_two_vregs(t1, node_map) {
                let c = Self::vreg_from_map(node_map, t0)?;
                let dest = self.alloc_vreg();
                self.emit(VInstruction::NegMulAdd { dest, a, b, c });
                return Ok(dest);
            }
            if let Some((a, b)) = node::product_two_vregs(t0, node_map) {
                let c = Self::vreg_from_map(node_map, t1)?;
                let dest = self.alloc_vreg();
                self.emit(VInstruction::MulAdd { dest, a, b, c });
                return Ok(dest);
            }
            if let Some((a, b)) = node::product_two_vregs(t1, node_map) {
                let c = Self::vreg_from_map(node_map, t0)?;
                let dest = self.alloc_vreg();
                self.emit(VInstruction::MulAdd { dest, a, b, c });
                return Ok(dest);
            }
            if let Some(a) = self.negated_inner_vreg(t1, node_map) {
                let b = Self::vreg_from_map(node_map, t0)?;
                let dest = self.alloc_vreg();
                self.emit(VInstruction::Sub { dest, a: b, b: a });
                return Ok(dest);
            }

            if let Some(a) = self.negated_inner_vreg(t0, node_map) {
                let b = Self::vreg_from_map(node_map, t1)?;
                let dest = self.alloc_vreg();
                self.emit(VInstruction::Sub { dest, a: b, b: a });
                return Ok(dest);
            }

            let a = Self::vreg_from_map(node_map, t0)?;
            let b = Self::vreg_from_map(node_map, t1)?;
            let dest = self.alloc_vreg();
            self.emit(VInstruction::Add2 { dest, a, b });
            return Ok(dest);
        }

        let mut pos_vregs = Vec::with_capacity(terms.len());
        let mut neg_vregs = Vec::with_capacity(terms.len());
        let mut constant_acc = 0.0_f64;
        let mut has_const = false;

        for term in terms {
            if let Some(c) = node::const_from_map(node_map, term.as_ref()) {
                constant_acc += c;
                has_const = true;
            } else if let Some(inner) = self.negated_inner_vreg(term.as_ref(), node_map) {
                neg_vregs.push(inner);
            } else {
                pos_vregs.push(Self::vreg_from_map(node_map, term.as_ref())?);
            }
        }

        if has_const {
            if constant_acc.is_finite() {
                if constant_acc.abs() > EPSILON {
                    if constant_acc > 0.0 {
                        let idx = self.add_const(constant_acc);
                        pos_vregs.push(VReg::Const(idx));
                    } else {
                        let idx = self.add_const(-constant_acc);
                        neg_vregs.push(VReg::Const(idx));
                    }
                }
            } else {
                let idx = self.add_const(constant_acc);
                pos_vregs.push(VReg::Const(idx));
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
        reason = "Large dispatch function for product nodes"
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

        if factors.len() == 2 {
            let f0 = factors[0].as_ref();
            let f1 = factors[1].as_ref();
            let c0 = node::const_from_map(node_map, f0);
            let c1 = node::const_from_map(node_map, f1);

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
            if let Some(c) = node::const_from_map(node_map, f.as_ref()) {
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
            for f in factors {
                if let Some(c) = node::const_from_map(node_map, f.as_ref()) {
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

        if let Some(n) = node::const_from_map(node_map, num)
            && (n - 1.0).abs() < EPSILON
            && let Some(arg) = node::recip_expm1_arg(den, node_map)
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
            self.emit(VInstruction::Builtin1 {
                dest,
                op: FnOp::Sinc,
                arg: den_v,
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
        reason = "Large dispatch function for power nodes"
    )]
    fn compile_pow_node(
        &mut self,
        base: &Expr,
        exp: &Expr,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Result<VReg, DiffError> {
        if let Some(n_val) = node::const_from_map(node_map, exp) {
            let is_integer = (n_val - n_val.round()).abs() < EPSILON;
            if is_integer {
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "Integer powers are expected to fit in i64"
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
                self.emit(VInstruction::Builtin1 {
                    dest,
                    op: FnOp::Sqrt,
                    arg: base_v,
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

        if let Some(base_val) = node::const_from_map(node_map, base)
            && (base_val - std::f64::consts::E).abs() < EPSILON
        {
            if let Some((src, neg)) = node::exp_sqr_arg(exp, node_map) {
                let dest = self.alloc_vreg();
                if neg {
                    self.emit(VInstruction::ExpSqrNeg { dest, src });
                } else {
                    self.emit(VInstruction::ExpSqr { dest, src });
                }
                return Ok(dest);
            }

            if let Some(pos_vreg) = self.compile_exp_neg_arg(exp, node_map) {
                let dest = self.alloc_vreg();
                self.emit(VInstruction::Builtin1 {
                    dest,
                    op: FnOp::ExpNeg,
                    arg: pos_vreg,
                });
                return Ok(dest);
            }

            let exp_v = Self::vreg_from_map(node_map, exp)?;
            let dest = self.alloc_vreg();
            self.emit(VInstruction::Builtin1 {
                dest,
                op: FnOp::Exp,
                arg: exp_v,
            });
            return Ok(dest);
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

        if id == ks.exp && args.len() == 1 {
            if let Some((src, neg)) = node::exp_sqr_arg(args[0].as_ref(), node_map) {
                if neg {
                    self.emit(VInstruction::ExpSqrNeg { dest, src });
                } else {
                    self.emit(VInstruction::ExpSqr { dest, src });
                }
                return Ok(dest);
            }

            if let Some(pos_vreg) = self.compile_exp_neg_arg(&args[0], node_map) {
                self.emit(VInstruction::Builtin1 {
                    dest,
                    op: FnOp::ExpNeg,
                    arg: pos_vreg,
                });
                return Ok(dest);
            }
        }

        if let Some(&op) = FN_MAP.get(&id) {
            let accepts_arity = (id == ks.log && args.len() == 1) || op.arity() == args.len();
            if !accepts_arity {
                return Err(DiffError::UnsupportedFunction(name.as_str().to_owned()));
            }

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
            #[allow(
                unsafe_code,
                reason = "Iterative compilation uses raw pointers to Expr nodes to avoid recursion limits. These pointers are always valid as they point to nodes within the immutable root expression tree."
            )]
            // SAFETY: Pointers are derived from the immutable root expression tree and are valid during traversal.
            let expr = unsafe { &*ptr };

            if visited {
                if node_map.contains_key(&ptr) {
                    continue;
                }

                let const_val = node::compute_const_from_children(expr, &node_map);
                let is_expensive = node::compute_expensive_from_children(expr, &node_map);

                if is_expensive && let Some(cached) = self.lookup_cse(expr) {
                    let node_data = const_val.map_or_else(
                        || NodeData::runtime(cached, is_expensive),
                        |value| NodeData::constant(cached, value, is_expensive),
                    );
                    node_map.insert(ptr, node_data);
                    continue;
                }

                if let Some(val) = const_val {
                    let idx = self.add_const(val);
                    let vreg = VReg::Const(idx);
                    node_map.insert(ptr, NodeData::constant(vreg, val, is_expensive));
                    continue;
                }

                let result_vreg = self.compile_nonconst_node(expr, &node_map)?;
                node_map.insert(ptr, NodeData::runtime(result_vreg, is_expensive));

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
            .map(|data| data.vreg())
            .ok_or_else(|| {
                DiffError::UnsupportedExpression(
                    "Internal compile error: missing root vreg".to_owned(),
                )
            })
    }
}
