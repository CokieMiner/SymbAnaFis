use std::hash::{Hash, Hasher};
use std::mem::take;
use std::ptr::from_ref;

use rustc_hash::FxHashMap;

use super::vir::VReg;
use crate::core::Expr;

use super::vir::VInstruction;

/// Key for the AST-level CSE cache used during VIR generation.
///
/// Wraps a raw pointer to an `Expr` node and uses the pre-computed structural
/// hash for O(1) equality rejection.
///
/// # Lifetime invariant
///
/// Pointers stored in `CseKey` are valid for the lifetime of the `Arc<Expr>`
/// tree currently being compiled. The tree is immutable and pinned in memory
/// through `Arc`, so the pointers remain valid for the entire compilation.
#[derive(Clone, Copy, Debug)]
pub struct CseKey(*const Expr);

impl CseKey {
    /// Create a new CSE key from a reference to an expression node.
    ///
    /// The reference must come from the expression tree currently being
    /// compiled, ensuring the pointer remains valid for the duration of
    /// compilation.
    #[inline]
    pub(in crate::evaluator::logic::bytecode::compile) const fn new(expr: &Expr) -> Self {
        Self(from_ref(expr))
    }
}

impl PartialEq for CseKey {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        #[allow(
            unsafe_code,
            reason = "Pointers in CseKey are derived from valid Expr nodes in the tree currently being compiled. Accessing precomputed hashes provides O(1) equality rejection, which is critical for CSE performance."
        )]
        // SAFETY: Pointers are derived from the root expression tree and are valid during compilation.
        unsafe {
            self.0 == other.0 || ((*self.0).hash == (*other.0).hash && *self.0 == *other.0)
        }
    }
}

impl Eq for CseKey {}

impl Hash for CseKey {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        #[allow(
            unsafe_code,
            reason = "CseKey pointers are guaranteed valid during compilation. Using the precomputed hash field avoids expensive recursive structural hashing."
        )]
        // SAFETY: The pointer is valid as it comes from the tree currently being compiled.
        unsafe {
            (*self.0).hash.hash(state);
        }
    }
}

/// Perform VIR-level Common Subexpression Elimination.
///
/// This operates on the `VInstruction` stream after VIR generation, aliasing
/// duplicate instructions to a single destination register. It is separate
/// from the AST-level CSE (`CseKey` cache) which operates during tree traversal.
pub(in crate::evaluator::logic::bytecode::compile) fn optimize_vir_cse(
    vinstrs: &mut Vec<VInstruction>,
    final_vreg: &mut Option<VReg>,
) {
    let mut seen = FxHashMap::default();
    let mut alias = FxHashMap::default();
    let mut optimized = Vec::with_capacity(vinstrs.len());

    for mut instr in take(vinstrs) {
        instr.for_each_read_mut(|r| {
            if let Some(&canonical) = alias.get(r) {
                *r = canonical;
            }
        });

        instr.sort_operands();

        let mut key = instr.clone();
        key.set_dest(VReg::Temp(u32::MAX));

        if let Some(&existing_vreg) = seen.get(&key) {
            alias.insert(instr.dest(), existing_vreg);
        } else {
            seen.insert(key, instr.dest());
            optimized.push(instr);
        }
    }
    *vinstrs = optimized;

    if let Some(f) = final_vreg
        && let Some(&canonical) = alias.get(f)
    {
        *f = canonical;
    }
}
