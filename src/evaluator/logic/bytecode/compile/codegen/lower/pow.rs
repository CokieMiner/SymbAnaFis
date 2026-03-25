use crate::EPSILON;
use crate::Expr;
use crate::core::error::DiffError;
use crate::core::expr::ExprKind;
use crate::core::poly::Polynomial;
use super::super::super::vir::node::{self, NodeData};
use super::super::super::vir::{VInstruction, VReg};
use crate::evaluator::logic::bytecode::instruction::FnOp;
use rustc_hash::FxHashMap;
use std::sync::Arc;

use super::super::super::Compiler;

impl Compiler {
    pub(super) fn compile_exp_neg_arg(
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

    pub(super) fn compile_polynomial_with_base(
        &mut self,
        poly: &Polynomial,
        base_vreg: VReg,
    ) -> VReg {
        let terms = poly.terms();
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

    pub(super) fn compile_poly_estrin(
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

    pub(super) fn get_power_vreg(
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

    pub(super) fn is_const_zero(&self, vreg: VReg) -> bool {
        match vreg {
            VReg::Const(idx) => self.constants[idx as usize] == 0.0,
            _ => false,
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "Large dispatch function for power nodes"
    )]
    pub(super) fn compile_pow_node(
        &mut self,
        base: &Expr,
        exp: &Expr,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Result<VReg, DiffError> {
        if let Some(n_val) = node::const_from_map(node_map, exp) {
            let is_integer = (n_val - n_val.round()).abs() < EPSILON;
            if is_integer {
                #[allow(clippy::cast_possible_truncation, reason = "Integer powers fit in i64")]
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
}
