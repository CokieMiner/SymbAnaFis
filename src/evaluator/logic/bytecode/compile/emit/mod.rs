pub mod assemble;
pub mod reg_alloc;

pub use assemble::assemble_flat_bytecode;
pub use reg_alloc::RegAllocator;

pub use super::{Instruction, vir};
