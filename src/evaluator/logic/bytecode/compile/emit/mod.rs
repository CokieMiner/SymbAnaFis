//! Physical lowering: virtual-register VIR → physical-register bytecode.
//!
//! [`RegAllocator`] maps unbounded `VReg` temps to a dense physical workspace
//! via linear-scan allocation.  [`assemble_flat_bytecode`] encodes the
//! final [`Instruction`] stream into a flat `Vec<u32>` for cache-friendly
//! dispatch.

pub mod assemble;
pub mod reg_alloc;

pub use assemble::assemble_flat_bytecode;
pub use reg_alloc::RegAllocator;

pub use super::{FnOp, Instruction, vir};
