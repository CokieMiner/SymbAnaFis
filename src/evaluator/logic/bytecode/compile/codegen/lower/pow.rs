use super::FnOp;
use super::VirGenerator;
use super::vir::matcher::exp_sqr_arg;
use super::vir::node::{NodeData, const_from_map};
use super::vir::{VInstruction, VReg};
use crate::EPSILON;
use crate::core::Expr;
use crate::core::Polynomial;
use crate::core::error::DiffError;
use rustc_hash::FxHashMap;
use std::f64::consts::E;

impl VirGenerator {
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
            let mut powers = FxHashMap::default();
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
        powers: &mut FxHashMap<usize, VReg>,
    ) -> VReg {
        if coeffs.iter().all(|&c| c == 0.0) {
            return VReg::Const(self.add_const(0.0));
        }
        if coeffs.len() == 1 {
            return VReg::Const(self.add_const(coeffs[0]));
        }
        if coeffs.len() == 2 {
            let c1 = coeffs[1];
            let c0 = coeffs[0];

            if c1 == 0.0 {
                return VReg::Const(self.add_const(c0));
            }

            let c1_idx = self.add_const(c1);
            let dest = self.alloc_vreg();

            if c0 == 0.0 {
                self.emit(VInstruction::Mul2 {
                    dest,
                    a: VReg::Const(c1_idx),
                    b: x,
                });
            } else {
                let c0_idx = self.add_const(c0);
                self.emit(VInstruction::MulAdd {
                    dest,
                    a: VReg::Const(c1_idx),
                    b: x,
                    c: VReg::Const(c0_idx),
                });
            }
            return dest;
        }

        let mid = coeffs.len().div_ceil(2);
        let left = &coeffs[..mid];
        let right = &coeffs[mid..];

        let left_vreg = self.compile_poly_estrin(left, x, powers);
        let right_vreg = self.compile_poly_estrin(right, x, powers);

        if self.is_const_zero(right_vreg) {
            return left_vreg;
        }

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
        powers: &mut FxHashMap<usize, VReg>,
    ) -> VReg {
        if n == 1 {
            return x;
        }
        if let Some(&cached) = powers.get(&n) {
            return cached;
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
        if let Some(n_val) = const_from_map(node_map, exp) {
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
                        // Exponents outside i32 range (-2^31..2^31) fall through to the
                        // generic f64 `Pow` instruction. For symbolic math expressions this
                        // is extremely rare (e.g. x^3_000_000_000) and the performance
                        // difference is negligible compared to computing such a large power.
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

        if let Some(base_val) = const_from_map(node_map, base)
            && (base_val - E).abs() < EPSILON
        {
            if let Some((src, neg)) = exp_sqr_arg(exp, node_map) {
                let dest = self.alloc_vreg();
                if neg {
                    self.emit(VInstruction::ExpSqrNeg { dest, src });
                } else {
                    self.emit(VInstruction::ExpSqr { dest, src });
                }
                return Ok(dest);
            }

            if let Some(pos_vreg) = self.try_compile_positive_exp_argument(exp, node_map) {
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
