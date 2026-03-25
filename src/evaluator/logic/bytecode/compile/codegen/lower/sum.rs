use crate::EPSILON;
use crate::Expr;
use crate::core::error::DiffError;
use crate::core::expr::ExprKind;
use super::super::super::vir::node::{self, NodeData};
use super::super::super::vir::{VInstruction, VReg};
use rustc_hash::FxHashMap;
use std::sync::Arc;

use super::super::super::Compiler;

impl Compiler {
    pub(super) fn vreg_from_map(
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

    pub(super) fn negated_inner_vreg(
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

    #[allow(
        clippy::too_many_lines,
        reason = "Large dispatch function for sum nodes"
    )]
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
}
