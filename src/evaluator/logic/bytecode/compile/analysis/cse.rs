use std::hash::{Hash, Hasher};
use std::mem::take;

use rustc_hash::FxHashMap;

use super::Compiler;
use super::vir::VReg;
use crate::core::Expr;

#[derive(Clone, Copy, Debug)]
pub struct CseKey(pub *const Expr);

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

impl Compiler {
    pub(crate) fn optimize_vir_cse(&mut self) {
        let mut seen = FxHashMap::default();
        let mut alias = FxHashMap::default();
        let mut optimized = Vec::with_capacity(self.vinstrs.len());

        for mut instr in take(&mut self.vinstrs) {
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
        self.vinstrs = optimized;

        if let Some(f) = &mut self.final_vreg
            && let Some(&canonical) = alias.get(f)
        {
            *f = canonical;
        }
    }
}
