use super::VirGenerator;
use super::vir::matcher::{negated_product_two_vregs, product_two_vregs};
use super::vir::node::{NodeData, const_from_map};
use super::vir::{VInstruction, VReg};
use crate::EPSILON;
use crate::core::error::DiffError;
use crate::core::{Expr, ExprKind};
use rustc_hash::FxHashMap;
use std::ptr::from_ref;
use std::sync::Arc;

/// Tries `f(a, b)` then `f(b, a)`, returning the first `Some`.
fn try_both_orderings<T>(
    a: &Expr,
    b: &Expr,
    mut f: impl FnMut(&Expr, &Expr) -> Option<T>,
) -> Option<T> {
    f(a, b).or_else(|| f(b, a))
}

impl VirGenerator {
    pub(super) fn vreg_from_map(
        node_map: &FxHashMap<*const Expr, NodeData>,
        expr: &Expr,
    ) -> Result<VReg, DiffError> {
        node_map
            .get(&from_ref(expr))
            .map(|data| data.vreg())
            .ok_or_else(|| {
                DiffError::UnsupportedExpression(
                    "Internal compile error: missing child vreg".to_owned(),
                )
            })
    }

    /// Attempts to extract the inner product of a negated term.
    ///
    /// Given a `Product([-1, a, b, ...])`, returns the [`VReg`] for `a * b * ...`
    /// (i.e. the product without the `-1` factor). This enables emitting
    /// `Sub` or `MulSub` instead of adding a negated term.
    ///
    /// Returns `None` if `term` is not a product containing exactly one `-1` factor.
    pub(super) fn try_extract_negated_product(
        &mut self,
        term: &Expr,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Option<VReg> {
        if let ExprKind::Product(factors) = &term.kind {
            let neg_idx = factors.iter().position(|f| {
                const_from_map(node_map, (*f).as_ref()).is_some_and(|n| (n + 1.0).abs() < EPSILON)
            })?;

            // Fast path for common 2-factor case to avoid Vec allocation
            if factors.len() == 2 {
                let other_idx = 1 - neg_idx;
                return Some(node_map.get(&Arc::as_ptr(&factors[other_idx]))?.vreg());
            }

            let mut remaining = Vec::with_capacity(factors.len() - 1);
            for (i, f) in factors.iter().enumerate() {
                if i != neg_idx {
                    remaining.push(node_map.get(&Arc::as_ptr(f))?.vreg());
                }
            }
            return Some(self.emit_mul_vregs(remaining));
        }
        None
    }

    /// Tries to fold two constant terms, returning a constant [`VReg`] if possible.
    ///
    /// Also handles the identity cases: `0 + x = x` and `x + 0 = x`.
    fn try_fold_two_sum_constants(
        &mut self,
        t0: &Expr,
        t1: &Expr,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Option<Result<VReg, DiffError>> {
        let c0 = const_from_map(node_map, t0);
        let c1 = const_from_map(node_map, t1);
        match (c0, c1) {
            (Some(v0), Some(v1)) => {
                let val = v0 + v1;
                if val.is_finite() {
                    let idx = self.add_const(val);
                    return Some(Ok(VReg::Const(idx)));
                }
            }
            (Some(v0), None) if v0.is_finite() && v0.abs() < EPSILON => {
                return Some(Self::vreg_from_map(node_map, t1));
            }
            (None, Some(v1)) if v1.is_finite() && v1.abs() < EPSILON => {
                return Some(Self::vreg_from_map(node_map, t0));
            }
            _ => {}
        }
        None
    }

    /// Tries to emit an FMA or subtraction pattern for a 2-term sum.
    ///
    /// Detects patterns like `a*b - c` → `MulSub`, `c - a*b` → `NegMulAdd`,
    /// `a*b + c` → `MulAdd`, and plain `x - y` → `Sub`.
    fn try_fma_two_terms(
        &mut self,
        t0: &Expr,
        t1: &Expr,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Option<VReg> {
        // Short-circuit: FMA and Sub patterns all require at least one Product node
        let t0_is_product = matches!(t0.kind, ExprKind::Product(_));
        let t1_is_product = matches!(t1.kind, ExprKind::Product(_));
        if !t0_is_product && !t1_is_product {
            return None;
        }

        // Pattern: a*b - neg(c) → MulSub
        if let Some(result) = try_both_orderings(t0, t1, |pos, neg| {
            let (a, b) = product_two_vregs(pos, node_map)?;
            let c = self.try_extract_negated_product(neg, node_map)?;
            let dest = self.alloc_vreg();
            self.emit(VInstruction::MulSub { dest, a, b, c });
            Some(dest)
        }) {
            return Some(result);
        }

        // Pattern: x - neg(a*b) → NegMulAdd  (i.e. c - a*b)
        if let Some(result) = try_both_orderings(t0, t1, |neg_term, other| {
            let (a, b) = negated_product_two_vregs(neg_term, node_map)?;
            let c = Self::vreg_from_map(node_map, other).ok()?;
            let dest = self.alloc_vreg();
            self.emit(VInstruction::NegMulAdd { dest, a, b, c });
            Some(dest)
        }) {
            return Some(result);
        }

        // Pattern: a*b + c → MulAdd
        if let Some(result) = try_both_orderings(t0, t1, |prod, other| {
            let (a, b) = product_two_vregs(prod, node_map)?;
            let c = Self::vreg_from_map(node_map, other).ok()?;
            let dest = self.alloc_vreg();
            self.emit(VInstruction::MulAdd { dest, a, b, c });
            Some(dest)
        }) {
            return Some(result);
        }

        // Pattern: x - y → Sub  (one of the terms is negated)
        if let Some(result) = try_both_orderings(t0, t1, |neg_candidate, other| {
            let a = self.try_extract_negated_product(neg_candidate, node_map)?;
            let b = Self::vreg_from_map(node_map, other).ok()?;
            let dest = self.alloc_vreg();
            self.emit(VInstruction::Sub { dest, a: b, b: a });
            Some(dest)
        }) {
            return Some(result);
        }

        None
    }

    pub(super) fn compile_sum_node(
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

        // --- 2-term fast path ---
        if terms.len() == 2 {
            let t0 = terms[0].as_ref();
            let t1 = terms[1].as_ref();

            if let Some(result) = self.try_fold_two_sum_constants(t0, t1, node_map) {
                return result;
            }

            if let Some(result) = self.try_fma_two_terms(t0, t1, node_map) {
                return Ok(result);
            }

            let a = Self::vreg_from_map(node_map, t0)?;
            let b = Self::vreg_from_map(node_map, t1)?;
            let dest = self.alloc_vreg();
            self.emit(VInstruction::Add2 { dest, a, b });
            return Ok(dest);
        }

        // --- N-term path: separate positive, negative, and constant terms ---
        let mut pos_vregs = Vec::with_capacity(terms.len());
        let mut neg_vregs = Vec::with_capacity(terms.len());
        let mut constant_acc = 0.0_f64;
        let mut has_const = false;

        // FMA candidates: we pick one product from each side if available
        let mut pos_prod_candidate = None;
        let mut neg_prod_candidate = None;

        for term in terms {
            if let Some(c) = const_from_map(node_map, term.as_ref()) {
                constant_acc += c;
                has_const = true;
            } else if let Some((a, b)) = negated_product_two_vregs(term.as_ref(), node_map) {
                if neg_prod_candidate.is_none() {
                    neg_prod_candidate = Some((a, b));
                } else {
                    let dest = self.emit_mul_two(a, b);
                    neg_vregs.push(dest);
                }
            } else if let Some((a, b)) = product_two_vregs(term.as_ref(), node_map) {
                if pos_prod_candidate.is_none() {
                    pos_prod_candidate = Some((a, b));
                } else {
                    let dest = self.emit_mul_two(a, b);
                    pos_vregs.push(dest);
                }
            } else if let Some(inner) = self.try_extract_negated_product(term.as_ref(), node_map) {
                neg_vregs.push(inner);
            } else {
                pos_vregs.push(Self::vreg_from_map(node_map, term.as_ref())?);
            }
        }

        if has_const {
            self.add_constant_to_vectors(constant_acc, &mut pos_vregs, &mut neg_vregs);
        }

        Ok(self.assemble_nary_sum(pos_prod_candidate, neg_prod_candidate, pos_vregs, neg_vregs))
    }

    fn add_constant_to_vectors(
        &mut self,
        constant_acc: f64,
        pos_vregs: &mut Vec<VReg>,
        neg_vregs: &mut Vec<VReg>,
    ) {
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

    fn assemble_nary_sum(
        &mut self,
        pos_prod_candidate: Option<(VReg, VReg)>,
        neg_prod_candidate: Option<(VReg, VReg)>,
        pos_vregs: Vec<VReg>,
        mut neg_vregs: Vec<VReg>,
    ) -> VReg {
        match (pos_prod_candidate, neg_prod_candidate) {
            // Case 1: Purely positive sum (Add or MulAdd)
            (Some((pa, pb)), None) if neg_vregs.is_empty() => {
                if pos_vregs.is_empty() {
                    return self.emit_mul_two(pa, pb);
                }
                let rest_pos = self.emit_add_vregs(pos_vregs);
                let dest = self.alloc_vreg();
                self.emit(VInstruction::MulAdd {
                    dest,
                    a: pa,
                    b: pb,
                    c: rest_pos,
                });
                dest
            }
            (None, None) if neg_vregs.is_empty() => self.emit_add_vregs(pos_vregs),

            // Case 2: Purely negative sum (Neg(Add) or Neg(Mul))
            (None, Some((na, nb))) if pos_vregs.is_empty() => {
                let inner = if neg_vregs.is_empty() {
                    self.emit_mul_two(na, nb)
                } else {
                    neg_vregs.push(self.emit_mul_two(na, nb));
                    self.emit_add_vregs(neg_vregs)
                };
                let dest = self.alloc_vreg();
                self.emit(VInstruction::Neg { dest, src: inner });
                dest
            }
            (None, None) if pos_vregs.is_empty() => {
                let inner = self.emit_add_vregs(neg_vregs);
                let dest = self.alloc_vreg();
                self.emit(VInstruction::Neg { dest, src: inner });
                dest
            }

            // Case 3: Mixed sum (pos - neg)
            (p_pair, n_pair) => {
                match (p_pair, n_pair) {
                    // Sub-case: pos_prod - neg_sum -> MulSub
                    (Some((pa, pb)), None) if pos_vregs.is_empty() => {
                        let neg_v = self.emit_add_vregs(neg_vregs);
                        let dest = self.alloc_vreg();
                        self.emit(VInstruction::MulSub {
                            dest,
                            a: pa,
                            b: pb,
                            c: neg_v,
                        });
                        dest
                    }
                    // Sub-case: pos_sum - neg_prod -> NegMulAdd (which is c - a*b)
                    (None, Some((na, nb))) if neg_vregs.is_empty() => {
                        let pos_v = self.emit_add_vregs(pos_vregs);
                        let dest = self.alloc_vreg();
                        self.emit(VInstruction::NegMulAdd {
                            dest,
                            a: na,
                            b: nb,
                            c: pos_v,
                        });
                        dest
                    }
                    // Sub-case: Both are products or sums, or fallback
                    _ => {
                        let pos_v = self.assemble_positive_part(pos_prod_candidate, pos_vregs);
                        if let Some((na, nb)) = neg_prod_candidate {
                            neg_vregs.push(self.emit_mul_two(na, nb));
                        }
                        let neg_v = self.emit_add_vregs(neg_vregs);
                        self.emit_final_sub(pos_v, neg_v)
                    }
                }
            }
        }
    }

    fn assemble_positive_part(
        &mut self,
        prod_candidate: Option<(VReg, VReg)>,
        pos_vregs: Vec<VReg>,
    ) -> Option<VReg> {
        match prod_candidate {
            Some((a, b)) if pos_vregs.is_empty() => Some(self.emit_mul_two(a, b)),
            Some((a, b)) => {
                let rest_pos = self.emit_add_vregs(pos_vregs);
                let dest = self.alloc_vreg();
                self.emit(VInstruction::MulAdd {
                    dest,
                    a,
                    b,
                    c: rest_pos,
                });
                Some(dest)
            }
            None if pos_vregs.is_empty() => None,
            None => Some(self.emit_add_vregs(pos_vregs)),
        }
    }

    /// Helper to emit the final subtraction
    fn emit_final_sub(&mut self, a: Option<VReg>, b: VReg) -> VReg {
        let dest = self.alloc_vreg();
        if let Some(a_v) = a {
            self.emit(VInstruction::Sub { dest, a: a_v, b });
        } else {
            self.emit(VInstruction::Neg { dest, src: b });
        }
        dest
    }

    /// Helper to emit Mul2
    fn emit_mul_two(&mut self, a: VReg, b: VReg) -> VReg {
        let dest = self.alloc_vreg();
        self.emit(VInstruction::Mul2 { dest, a, b });
        dest
    }
}
