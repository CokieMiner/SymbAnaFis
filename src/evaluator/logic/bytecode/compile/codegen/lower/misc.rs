use super::VirGenerator;
use super::analysis::GvnKey;
use super::vir::VReg;
use super::vir::node::NodeData;
use crate::core::InternedSymbol;
use crate::core::error::DiffError;
use crate::core::known_symbols::get_constant_value_by_id;
use crate::core::{Expr, ExprKind};
use rustc_hash::FxHashMap;
use std::sync::Arc;

impl VirGenerator {
    pub(super) fn compile_symbol_node(&mut self, sym: &InternedSymbol) -> Result<VReg, DiffError> {
        let sym_id = sym.id();
        if let Some(&idx) = self.param_index.get(&sym_id) {
            Ok(VReg::Param(
                u32::try_from(idx).map_err(|_err| DiffError::RegisterOverflow)?,
            ))
        } else if let Some(val) = get_constant_value_by_id(sym_id) {
            let idx = self.add_const(val);
            Ok(VReg::Const(idx))
        } else {
            Err(DiffError::UnboundVariable(sym.as_str().to_owned()))
        }
    }

    pub(super) fn map_args_vregs(
        args: &[Arc<Expr>],
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Result<Vec<VReg>, DiffError> {
        let mut out = Vec::with_capacity(args.len());
        for arg in args {
            out.push(Self::vreg_from_map(node_map, arg.as_ref())?);
        }
        Ok(out)
    }

    pub(in crate::evaluator::logic::bytecode::compile) fn lookup_cse(
        &self,
        expr: &Expr,
    ) -> Option<VReg> {
        self.gvn_cache.get(&GvnKey::new(expr)).copied()
    }

    pub(in crate::evaluator::logic::bytecode::compile) fn push_children(
        expr: &Expr,
        stack: &mut Vec<(*const Expr, bool)>,
    ) {
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
}
