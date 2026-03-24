pub mod compile;
pub mod execute;
pub mod instruction;

pub use instruction::Instruction;

#[cfg(test)]
pub mod execute_tests;
