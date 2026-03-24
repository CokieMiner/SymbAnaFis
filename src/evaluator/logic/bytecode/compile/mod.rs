//! Bytecode compilation pipeline (AST → Bytecode)

pub mod analysis;
pub mod codegen;
mod compiler;
pub mod emit;
mod optimize;
pub mod vir;

pub use compiler::Compiler;
