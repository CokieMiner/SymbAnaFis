//! Portable scalar and blocked bytecode VM execution for [`Program`] and [`MultiProgram`].

use super::{
    BLOCK, BatchTarget, Combine, EvaluationValue, Fold, InlineBinary, InlineUnary, Instruction,
    MultiProgram, Operation, PairedFunction, Program, ProgramData, STACK_REGS, Workspace,
};

mod execute;
mod kernel;
mod multi;
