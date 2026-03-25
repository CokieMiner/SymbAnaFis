use super::super::super::vir::node::{self, NodeData};
use super::super::super::vir::{VInstruction, VReg};
use crate::EPSILON;
use crate::Expr;
use crate::core::error::DiffError;
use crate::core::expr::ExprKind;
use crate::core::known_symbols::KS;
use crate::evaluator::logic::bytecode::instruction::FnOp;
use rustc_hash::FxHashMap;

use super::super::super::Compiler;

impl Compiler {
    pub(super) fn compile_div_node(
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
}
