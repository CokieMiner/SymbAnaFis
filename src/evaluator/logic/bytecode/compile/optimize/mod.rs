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
