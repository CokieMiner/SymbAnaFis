use super::VirGenerator;
use super::analysis::GvnKey;
use super::vir::VReg;
use super::vir::node::{NodeData, compute_const_from_children, compute_is_cse_candidate};
use crate::core::Expr;
use crate::core::error::DiffError;
use rustc_hash::{FxHashMap, FxHashSet};
use std::ptr::from_ref;

/// Produces a post-order traversal of the expression tree using an iterative
/// approach to avoid stack overflow on deeply nested expressions.
///
/// All raw-pointer manipulation is confined to this function. The returned
/// references borrow from the immutable `root` tree and are valid for its
/// lifetime.
///
/// Each `&Expr` appears at most once in the output (deduplication via pointer
/// identity).
fn postorder_walk(root: &Expr, capacity: usize) -> Vec<&Expr> {
    let mut stack: Vec<(*const Expr, bool)> = Vec::with_capacity(1024.min(capacity));
    let mut visited = FxHashSet::default();
    visited.reserve(capacity);
    let mut result = Vec::with_capacity(capacity);

    let root_ptr = from_ref(root);
    stack.push((root_ptr, false));

    while let Some((ptr, processed)) = stack.pop() {
        if processed {
            #[allow(
                unsafe_code,
                reason = "Pointer derives from the immutable root expression tree and is valid for the lifetime of `root`."
            )]
            // SAFETY: `ptr` was obtained from `from_ref` or `Arc::as_ptr` on nodes
            // within the immutable `root` tree. The tree is not modified during traversal.
            let expr = unsafe { &*ptr };
            result.push(expr);
        } else {
            // Mark visited on first encounter. If already visited (shared subtree
            // pushed by another parent), skip entirely — prevents redundant stack
            // growth proportional to the DAG's sharing factor.
            if !visited.insert(ptr) {
                continue;
            }
            stack.push((ptr, true));
            #[allow(
                unsafe_code,
                reason = "Pointer derives from the immutable root expression tree and is valid for the lifetime of `root`."
            )]
            // SAFETY: Same as above — pointer is valid and immutable.
            let expr = unsafe { &*ptr };
            VirGenerator::push_children(expr, &mut stack);
        }
    }

    result
}

impl VirGenerator {
    pub(crate) fn compile_expr_iterative(
        &mut self,
        root: &Expr,
        node_count: usize,
    ) -> Result<VReg, DiffError> {
        let mut node_map: FxHashMap<*const Expr, NodeData> = FxHashMap::default();
        node_map.reserve(node_count);

        // Obtain a safe post-order traversal — all `unsafe` is confined to `postorder_walk`.
        let order = postorder_walk(root, node_count);

        for expr in order {
            let ptr = from_ref(expr);

            let const_val = compute_const_from_children(expr, &node_map);
            let is_cse_candidate = compute_is_cse_candidate(expr, &node_map);

            if is_cse_candidate && let Some(cached) = self.lookup_cse(expr) {
                let node_data = const_val.map_or_else(
                    || NodeData::runtime(cached),
                    |value| NodeData::constant(cached, value),
                );
                node_map.insert(ptr, node_data);
                continue;
            }

            if let Some(val) = const_val {
                let idx = self.add_const(val);
                let vreg = VReg::Const(idx);
                node_map.insert(ptr, NodeData::constant(vreg, val));
                continue;
            }

            let result_vreg = self.compile_nonconst_node(expr, &node_map)?;
            node_map.insert(ptr, NodeData::runtime(result_vreg));

            if is_cse_candidate {
                self.gvn_cache.insert(GvnKey::new(expr), result_vreg);
            }
        }

        let root_ptr = from_ref(root);
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
