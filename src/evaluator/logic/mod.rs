//! Evaluator Implementation Details (Core Engines)
//!
//! The `bytecode` evaluator is the primary numeric fast path. The `tree`
//! evaluator provides symbolic partial evaluation as a fallback.
//!
//! Types re-exported as `pub` here are consumed by [`super::api`] for the
//! public evaluator API boundary (`compile`, `disassemble`, `eval_f64`, etc.).

pub(super) mod bytecode;
pub(super) mod tree;

// Re-exports for api.rs / Evaluator API Boundary
// Crate-internal re-exports (for other modules like diff/compiler)
pub use bytecode::{
    CompiledProgram, FnOp, Instruction, VirGenerator, assemble_flat_bytecode, expand_user_functions,
};

#[cfg(feature = "parallel")]
pub use bytecode::{
    EvalResult, ExprInput, SKIP, Value, VarInput, eval_single_expr_chunked, evaluate_parallel,
};

#[cfg(all(feature = "parallel", feature = "python"))]
pub use bytecode::evaluate_parallel_with_hint;

pub use tree::VarLookup;

pub use super::VmEvaluator;

#[cfg(feature = "parallel")]
pub use super::ToParamName;

#[cfg(test)]
mod tests;
