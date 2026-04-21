use super::FnOp;
use super::VirGenerator;
use super::vir::matcher::exp_sqr_arg;
use super::vir::node::NodeData;
use super::vir::registry::FN_MAP;
use super::vir::{VInstruction, VReg};
use crate::core::InternedSymbol;
use crate::core::known_symbols::KS;
use crate::core::{DiffError, Expr};
use rustc_hash::FxHashMap;
use std::sync::Arc;

impl VirGenerator {
    pub(super) fn compile_function_node(
        &mut self,
        name: &InternedSymbol,
        args: &[Arc<Expr>],
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Result<VReg, DiffError> {
        let id = name.id();
        let ks = &*KS;

        if id == ks.exp && args.len() == 1 {
            if let Some((src, neg)) = exp_sqr_arg(args[0].as_ref(), node_map) {
                let dest = self.alloc_vreg();
                if neg {
                    self.emit(VInstruction::ExpSqrNeg { dest, src });
                } else {
                    self.emit(VInstruction::ExpSqr { dest, src });
                }
                return Ok(dest);
            }

            if let Some(pos_vreg) = self.try_compile_positive_exp_argument(&args[0], node_map) {
                let dest = self.alloc_vreg();
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

            let dest = self.alloc_vreg();
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
                let dest = self.alloc_vreg();
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
}
