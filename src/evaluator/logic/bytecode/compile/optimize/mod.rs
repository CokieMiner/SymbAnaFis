//! Physical-instruction optimization passes run after register allocation.
//!
//! These operate on [`Instruction`] streams with physical register indices.
//! The pipeline is orchestrated by [`super::VmEvaluator::optimize_instructions`]:
//!  1. [`reduce_strength`] + [`optimize_power_chains`] — algebraic rewrites
//!  2. [`eliminate_dead_code`] — backward-liveness DCE + copy forwarding
//!  3. [`fuse_instructions`] — peephole fusion (iterated to convergence)
//!  4. [`eliminate_dead_code`] — final cleanup
//!  5. [`compact_constants`] — constant-pool compaction + register re-indexing

mod compact;
mod dce;
mod fusion;
pub mod helper;
pub use helper::ConstantPool;
mod pipeline;
mod power_chain;

pub use super::{FnOp, Instruction, VmEvaluator};

#[cfg(test)]
mod tests;
