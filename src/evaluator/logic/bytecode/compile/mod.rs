//! Bytecode compilation pipeline (AST → Bytecode)

pub mod analysis;
pub mod codegen;
pub mod compiler;
pub mod emit;
pub mod optimize;
pub mod vir;

pub use codegen::expand_user_functions;
pub use compiler::VirGenerator;
pub use emit::assemble_flat_bytecode;

pub use super::{CompiledEvaluator, FnOp, Instruction};
pub use vir::{VInstruction, VReg};
