//! Bytecode compilation pipeline (AST → Bytecode)

mod compiler;
pub mod expand;
mod node;
mod optimize;
mod reg_alloc;
mod registry;
mod vir;

#[cfg(test)]
mod tests;

pub use compiler::Compiler;
