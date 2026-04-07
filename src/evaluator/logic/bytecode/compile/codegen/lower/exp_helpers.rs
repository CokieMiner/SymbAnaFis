//! Helpers for compiling exponential patterns shared across `pow.rs` and `func.rs`.
//!
//! Moved here to avoid cross-module dependencies between the `pow` and `func`
//! lowering modules that both need to recognise `exp(-x)` patterns.

use super::VirGenerator;
use super::vir::VReg;
use super::vir::node::{NodeData, const_from_map};
use crate::EPSILON;
use crate::core::{Expr, ExprKind};
use rustc_hash::FxHashMap;
use std::sync::Arc;

use super::vir::VInstruction;

impl VirGenerator {
    /// Attempts to extract and compile the *positive* part of a negated exponent argument.
    ///
    /// Given an argument that is structurally `-(positive_expr)`, this function
    /// compiles `positive_expr` and returns its [`VReg`] so the caller can emit an
    /// `ExpNeg` instruction instead of a `Neg` + `Exp` pair.
    ///
    /// # Recognised patterns
    ///
    /// - `Product([-c, ...])` where `c < 0`: extracts `|c| * rest`
    /// - `Div(Product([-c, ...]), den)`: similarly for division
    /// - `Div(-c, den)`: constant negative numerator
    ///
    /// Returns `None` if the argument is not a recognisable negated form.
    pub(super) fn try_compile_positive_exp_argument(
        &mut self,
        arg: &Expr,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Option<VReg> {
        let compile_pos_product = |compiler: &mut Self, factors: &[Arc<Expr>]| -> Option<VReg> {
            let mut const_total = 1.0_f64;
            let mut has_const = false;
            for f in factors {
                if let Some(c) = const_from_map(node_map, (*f).as_ref()) {
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
                if const_from_map(node_map, (*f).as_ref()).is_none() {
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
                if let Some(n) = const_from_map(node_map, num.as_ref())
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
}
