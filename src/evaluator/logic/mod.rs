//! Evaluator Implementation Details (Core Engines)

pub(super) mod bytecode;
pub(super) mod tree;

// Re-exports for api.rs / Evaluator API Boundary
// Crate-internal re-exports (for other modules like diff/compiler)
pub use bytecode::{VirGenerator, Instruction, expand_user_functions};

#[cfg(feature = "parallel")]
pub use bytecode::{
    EvalResult, ExprInput, SKIP, Value, VarInput, eval_single_expr_chunked, evaluate_parallel,
};

#[cfg(all(feature = "parallel", feature = "python"))]
pub use bytecode::evaluate_parallel_with_hint;

pub use tree::VarLookup;

pub use super::CompiledEvaluator;

#[cfg(feature = "parallel")]
pub use super::ToParamName;
