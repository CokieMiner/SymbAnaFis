//! DAG lowering, SSA optimization, and register allocation for [`Program`].

use super::{
    BLOCK, Combine, Compilation, EvaluationValue, Fold, Fused, GROUP, Instruction, MultiProgram,
    NativeSlot, OperandSpan, Operation, PairedFunction, Program, ProgramData, Source, Traversal,
};

mod compact;
mod floating;
mod lower;
mod optimize;
mod pairs;
mod registers;
