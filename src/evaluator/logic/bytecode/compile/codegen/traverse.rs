use crate::Expr;
use crate::core::error::DiffError;
use crate::evaluator::logic::bytecode::compile::vir::VReg;
use crate::evaluator::logic::bytecode::compile::vir::node::{self, NodeData};
use rustc_hash::FxHashMap;

use super::super::Compiler;

impl Compiler {
    pub(crate) fn compile_expr_iterative(
        &mut self,
        root: &Expr,
        node_count: usize,
    ) -> Result<VReg, DiffError> {
        let mut stack: Vec<(*const Expr, bool)> = Vec::with_capacity(node_count);
        let mut node_map: FxHashMap<*const Expr, NodeData> = FxHashMap::default();
        node_map.reserve(node_count);

        let root_ptr = root as *const Expr;
        stack.push((root_ptr, false));

        while let Some((ptr, visited)) = stack.pop() {
            #[allow(
                unsafe_code,
                reason = "Iterative compilation uses raw pointers to Expr nodes to avoid recursion limits. These pointers are always valid as they point to nodes within the immutable root expression tree."
            )]
            // SAFETY: Pointers are derived from the immutable root expression tree and are valid during traversal.
            let expr = unsafe { &*ptr };

            if visited {
                if node_map.contains_key(&ptr) {
                    continue;
                }

                let const_val = node::compute_const_from_children(expr, &node_map);
                let is_expensive = node::compute_expensive_from_children(expr, &node_map);

                if is_expensive && let Some(cached) = self.lookup_cse(expr) {
                    let node_data = const_val.map_or_else(
                        || NodeData::runtime(cached, is_expensive),
                        |value| NodeData::constant(cached, value, is_expensive),
                    );
                    node_map.insert(ptr, node_data);
                    continue;
                }

                if let Some(val) = const_val {
                    let idx = self.add_const(val);
                    let vreg = VReg::Const(idx);
                    node_map.insert(ptr, NodeData::constant(vreg, val, is_expensive));
                    continue;
                }

                let result_vreg = self.compile_nonconst_node(expr, &node_map)?;
                node_map.insert(ptr, NodeData::runtime(result_vreg, is_expensive));

                if is_expensive {
                    self.cse_cache.insert(
                        crate::evaluator::logic::bytecode::compile::analysis::CseKey(ptr),
                        result_vreg,
                    );
                }
            } else if !node_map.contains_key(&ptr) {
                stack.push((ptr, true));
                Self::push_children(expr, &mut stack);
            }
        }

        node_map
            .get(&root_ptr)
            .map(|data| data.vreg())
            .ok_or_else(|| {
                DiffError::UnsupportedExpression(
                    "Internal compile error: missing root vreg".to_owned(),
                )
            })
    }
}
