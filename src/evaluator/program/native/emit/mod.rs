//! Architecture-independent liveness analysis, Belady register allocation,
//! and native machine-code lowering.

use super::{
    ArchBackend, Combine, FloatPrecision, Fold, Fused, FusedMemOperand, HwReg, InlineBinary,
    InlineUnary, Instruction, NativeLayout, Operation, ProgramData, Trampolines,
};

use complex::ComplexEmitter;
#[cfg(not(test))]
use liveness::LivenessPlan;
use regalloc::RealEmitter;

mod calls;
mod complex;
mod driver;
mod liveness;
mod real;
mod regalloc;

pub use driver::NativeEmitter;
#[cfg(test)]
pub use liveness::LivenessPlan;
