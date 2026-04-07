//! Shared helpers for emitting N-ary add/mul VIR instructions.
//!
//! [`emit_add_vregs`] and [`emit_mul_vregs`] handle the 0/1/2/N-ary cases
//! that appear repeatedly in sum, product, and negated-product lowering.

use super::VirGenerator;
use super::vir::{VInstruction, VReg};

impl VirGenerator {
    /// Emits an addition of `vregs`, returning a single [`VReg`] with the result.
    ///
    /// Handles the special cases:
    /// - **0 vregs** → constant `0.0`
    /// - **1 vreg**  → pass-through (no instruction emitted)
    /// - **2 vregs** → `Add2`
    /// - **N vregs** → `Add { srcs }`
    pub(super) fn emit_add_vregs(&mut self, vregs: Vec<VReg>) -> VReg {
        match vregs.len() {
            0 => VReg::Const(self.add_const(0.0)),
            1 => vregs[0],
            2 => {
                let dest = self.alloc_vreg();
                self.emit(VInstruction::Add2 {
                    dest,
                    a: vregs[0],
                    b: vregs[1],
                });
                dest
            }
            _ => {
                let dest = self.alloc_vreg();
                self.emit(VInstruction::Add { dest, srcs: vregs });
                dest
            }
        }
    }

    /// Emits a multiplication of `vregs`, returning a single [`VReg`] with the result.
    ///
    /// Handles the special cases:
    /// - **0 vregs** → constant `1.0`
    /// - **1 vreg**  → pass-through (no instruction emitted)
    /// - **2 vregs** → `Mul2`
    /// - **N vregs** → `Mul { srcs }`
    pub(super) fn emit_mul_vregs(&mut self, vregs: Vec<VReg>) -> VReg {
        match vregs.len() {
            0 => VReg::Const(self.add_const(1.0)),
            1 => vregs[0],
            2 => {
                let dest = self.alloc_vreg();
                self.emit(VInstruction::Mul2 {
                    dest,
                    a: vregs[0],
                    b: vregs[1],
                });
                dest
            }
            _ => {
                let dest = self.alloc_vreg();
                self.emit(VInstruction::Mul { dest, srcs: vregs });
                dest
            }
        }
    }
}
