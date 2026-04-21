mod compact;
mod dce;
mod fusion;
pub mod helper;
pub use helper::ConstantPool;
mod pipeline;
mod power_chain;
pub mod schedule;
mod strength_reduction;

pub use super::{CompiledEvaluator, FnOp, Instruction, VInstruction, VReg};

#[cfg(test)]
mod tests;
