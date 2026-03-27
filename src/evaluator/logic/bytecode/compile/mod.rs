//! Bytecode compilation pipeline (AST → Bytecode)

pub mod analysis;
pub mod codegen;
pub mod compiler;
pub mod emit;
pub mod optimize;
pub mod vir;

pub use codegen::expand_user_functions;
pub use compiler::Compiler;

pub use super::{CompiledEvaluator, instruction};
