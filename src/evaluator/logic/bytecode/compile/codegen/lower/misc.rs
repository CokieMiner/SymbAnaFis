use super::Compiler;
use super::analysis::CseKey;
use super::vir::VReg;
use super::vir::node::NodeData;
use crate::core::InternedSymbol;
use crate::core::Polynomial;
use crate::core::error::DiffError;
use crate::core::known_symbols::get_constant_value_by_id;
use crate::core::{Expr, ExprKind};
use rustc_hash::FxHashMap;
use std::ptr::from_ref;
use std::sync::Arc;

impl Compiler {
    pub(super) fn compile_symbol_node(&mut self, sym: &InternedSymbol) -> Result<VReg, DiffError> {
        let sym_id = sym.id();
        if let Some(&idx) = self.param_index.get(&sym_id) {
            Ok(VReg::Param(
                u32::try_from(idx).expect("Param index too large"),
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

    pub(super) fn compile_poly_node(
        &mut self,
        poly: &Polynomial,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Result<VReg, DiffError> {
        let base_v = Self::vreg_from_map(node_map, poly.base().as_ref())?;
        Ok(self.compile_polynomial_with_base(poly, base_v))
    }

    pub(in crate::evaluator::logic::bytecode::compile) fn lookup_cse(
        &self,
        expr: &Expr,
    ) -> Option<VReg> {
        self.cse_cache.get(&CseKey(from_ref(expr))).copied()
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
