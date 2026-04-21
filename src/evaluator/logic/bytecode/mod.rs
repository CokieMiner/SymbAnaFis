pub mod compile;
pub mod execute;
pub mod functions;
pub mod instruction;

// --- Core API ---
pub use super::CompiledEvaluator;
pub use functions::FnOp;
pub use instruction::Instruction;

// --- Compilation ---
pub use compile::{VirGenerator, assemble_flat_bytecode, expand_user_functions};

// --- Execution & Parallelism ---
#[cfg(all(feature = "parallel", feature = "python"))]
pub use execute::evaluate_parallel_with_hint;
#[cfg(feature = "parallel")]
pub use execute::{
    EvalResult, ExprInput, SKIP, Value, VarInput, eval_single_expr_chunked, evaluate_parallel,
};

#[cfg(feature = "parallel")]
pub use super::ToParamName;
