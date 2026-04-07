use super::VirGenerator;
use super::vir::node::{NodeData, const_from_map};
use super::vir::VReg;
use crate::EPSILON;
use crate::core::{DiffError, Expr};
use rustc_hash::FxHashMap;
use std::sync::Arc;

impl VirGenerator {
    pub(super) fn compile_product_node(
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
            let c0 = const_from_map(node_map, f0);
            let c1 = const_from_map(node_map, f1);

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
                        return Ok(self.emit_mul_vregs(vec![VReg::Const(c_idx), v1_reg]));
                    }
                }
                (None, Some(v1)) => {
                    if v1.is_finite() {
                        if (v1 - 1.0).abs() < EPSILON {
                            return Self::vreg_from_map(node_map, f0);
                        }
                        let c_idx = self.add_const(v1);
                        let v0_reg = Self::vreg_from_map(node_map, f0)?;
                        return Ok(self.emit_mul_vregs(vec![v0_reg, VReg::Const(c_idx)]));
                    }
                }
                _ => {}
            }
        }

        let mut constant_acc = 1.0_f64;
        let mut variable_vregs = Vec::with_capacity(factors.len());
        for f in factors {
            if let Some(c) = const_from_map(node_map, (*f).as_ref()) {
                constant_acc *= c;
            } else {
                variable_vregs.push(Self::vreg_from_map(node_map, (*f).as_ref())?);
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
                if let Some(c) = const_from_map(node_map, (*f).as_ref()) {
                    let c_idx = self.add_const(c);
                    vregs_all.push(VReg::Const(c_idx));
                }
            }
        }

        if vregs_all.is_empty() {
            let idx = self.add_const(1.0);
            return Ok(VReg::Const(idx));
        }

        Ok(self.emit_mul_vregs(vregs_all))
    }
}
