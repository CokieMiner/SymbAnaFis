//! Bytecode compilation pipeline (AST → Bytecode)

mod compiler;
pub(super) mod expand;
mod node;
mod optimize;
mod reg_alloc;
mod registry;
mod vir;

pub use compiler::Compiler;
